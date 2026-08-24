//! macOS: the arm64e exec guard (tebako#448).
//!
//! The interpose dylib is built per host architecture (arm64-only on
//! Apple Silicon). When a virtualized process execs a target whose Mach-O
//! slice dyld will SELECT is a different ABI — an arm64e slice (dyld
//! prefers arm64e over arm64 when both exist), or a binary with no
//! host-arch slice at all (x86_64-only under Rosetta) — dyld tries to
//! load the inherited `DYLD_INSERT_LIBRARIES` entry into the child, finds
//! no compatible slice in it, and TERMINATES the child:
//!
//! ```text
//! dyld[…]: terminating because inserted dylib '…/libtfs_preload.dylib'
//! could not be loaded: … (mach-o file, but is an incompatible
//! architecture (have 'arm64', need 'arm64e'))
//! ```
//!
//! SIP-protected platform binaries strip `DYLD_*` silently and were never
//! affected; the dying children are third-party arm64e binaries (the
//! macos-14 CI toolchain — the 0.16.7 `native_ext_press_builds_and_
//! packages` leg). Before the fork/exec propagation fix the insertion
//! never crossed exec, so this is a shipped regression on that line.
//!
//! The guard: the interposed execve/posix_spawn/posix_spawnp probe the
//! target's Mach-O header (one open+read of the first page through the
//! REAL libc — the probe must not re-enter the engine through the
//! interposed open, and the loader's exec mechanics are not a policy op)
//! and, when the slice table says this dylib cannot load, forward a
//! rebuilt envp with `DYLD_INSERT_LIBRARIES` REMOVED for that one exec.
//! Scripts and unreadable/non-Mach-O targets keep insertion — today's
//! behavior; the shebang interpreter's own arch decides there.

use std::ffi::{c_char, CStr, CString};

/// The environment variable the guard strips.
const INSERT_VAR: &[u8] = b"DYLD_INSERT_LIBRARIES";

/// The CPU types the decision knows (mach/machine.h).
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_TYPE_ARM64: u32 = 0x0100_000c;

/// One Mach-O slice's identity — a (cputype, cpusubtype) pair from the
/// mach_header_64 or a fat_arch entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Slice {
    cputype: u32,
    cpusubtype: u32,
}

/// The host architecture the decision is compiled for — the interpose
/// dylib's own (only) slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostArch {
    X86_64,
    Arm64,
}

/// This build's host arch.
pub(crate) const HOST_ARCH: HostArch = if cfg!(target_arch = "aarch64") {
    HostArch::Arm64
} else {
    HostArch::X86_64
};

/// An arm64e slice: CPU_SUBTYPE_ARM64E (2) with the 0x8000_0000
/// capability bit (what Xcode's `-arch arm64e` emits: 0x8000_0002).
fn is_arm64e(cpusubtype: u32) -> bool {
    (cpusubtype & 0x00ff_ffff) == 2 && (cpusubtype & 0x8000_0000) != 0
}

/// May this dylib load into a process running one of `slices`? dyld
/// selects the best slice for the host: on Apple Silicon that is arm64e
/// when the table offers it (preferred over arm64), else arm64, else an
/// x86_64 slice under Rosetta; on Intel, x86_64. The dylib loads iff the
/// SELECTED slice is its own ABI — so on arm64 hosts an offered arm64e
/// slice is fatal even when an arm64 slice rides alongside.
pub(crate) fn dylib_loadable(host: HostArch, slices: &[Slice]) -> bool {
    match host {
        HostArch::Arm64 => {
            let arm64e = slices
                .iter()
                .any(|s| s.cputype == CPU_TYPE_ARM64 && is_arm64e(s.cpusubtype));
            let arm64 = slices
                .iter()
                .any(|s| s.cputype == CPU_TYPE_ARM64 && !is_arm64e(s.cpusubtype));
            !arm64e && arm64
        }
        HostArch::X86_64 => slices.iter().any(|s| s.cputype == CPU_TYPE_X86_64),
    }
}

// ---------------------------------------------------------------------
// The Mach-O header parse (pure bytes; unit-tested below)
// ---------------------------------------------------------------------

fn u32_at(buf: &[u8], off: usize, be: bool) -> Option<u32> {
    let b: [u8; 4] = buf.get(off..off + 4)?.try_into().ok()?;
    Some(if be {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    })
}

/// Parse the slice table of the Mach-O file whose header page is `buf`:
/// thin 64-bit Mach-O (either endianness) or fat (either endianness;
/// the fat_arch table itself carries the per-slice types — the slice
/// sub-headers are never sought). None = not a Mach-O / truncated /
/// implausible — the caller keeps insertion (today's behavior).
pub(crate) fn parse_slices(buf: &[u8]) -> Option<Vec<Slice>> {
    match buf.get(..4)? {
        // MH_MAGIC_64 / MH_CIGAM_64 as raw byte patterns (no integer
        // endianness confusion: the pattern selects the field order).
        [0xcf, 0xfa, 0xed, 0xfe] => parse_thin(buf, false),
        [0xfe, 0xed, 0xfa, 0xcf] => parse_thin(buf, true),
        // FAT_MAGIC / FAT_CIGAM.
        [0xca, 0xfe, 0xba, 0xbe] => parse_fat(buf, true),
        [0xbe, 0xba, 0xfe, 0xca] => parse_fat(buf, false),
        _ => None,
    }
}

fn parse_thin(buf: &[u8], be: bool) -> Option<Vec<Slice>> {
    Some(vec![Slice {
        cputype: u32_at(buf, 4, be)?,
        cpusubtype: u32_at(buf, 8, be)?,
    }])
}

fn parse_fat(buf: &[u8], be: bool) -> Option<Vec<Slice>> {
    let nfat = u32_at(buf, 4, be)? as usize;
    // The whole table must fit in the probed page (a garbage nfat just
    // fails here → not-a-parse → keep insertion).
    let mut out = Vec::with_capacity(nfat.min(16));
    for i in 0..nfat {
        let off = 8 + i * 20; // sizeof(struct fat_arch)
        out.push(Slice {
            cputype: u32_at(buf, off, be)?,
            cpusubtype: u32_at(buf, off + 4, be)?,
        });
    }
    Some(out)
}

/// Read one exec target's slice table: the first page carries every
/// realistic header (thin or fat). Real-libc IO ONLY (`plat::real_*`,
/// never the interposed symbols): the probe must not re-enter the engine,
/// and a host exec is not a jail-gated IO route, so the jail must not
/// gate it either.
pub(crate) fn probe_slices(path: &str) -> Option<Vec<Slice>> {
    let c = CString::new(path).ok()?;
    // SAFETY: plain libc through the interpose tuple's original; `c` is
    // NUL-terminated and outlives the call.
    let fd = unsafe { super::plat::real_open()(c.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }
    let mut buf = [0u8; 4096];
    // SAFETY: fd is open; buf is writable for its length.
    let n = unsafe { super::plat::real_read()(fd, buf.as_mut_ptr().cast(), buf.len()) };
    // SAFETY: plain libc; fd is ours.
    unsafe { super::plat::real_close()(fd) };
    if n < 0 {
        return None;
    }
    parse_slices(&buf[..n as usize])
}

// ---------------------------------------------------------------------
// The envp rebuild (the strip) + spawnp PATH resolution
// ---------------------------------------------------------------------

/// The rebuilt environment for one exec/spawn call: every entry except
/// `DYLD_INSERT_LIBRARIES` (REMOVED, not emptied). Owns its bytes; the
/// caller binds it to a local that outlives the real call (execve never
/// returns on success — the kernel reclaims the address space;
/// posix_spawn copies the envp before it returns, so the drop after the
/// call is correct there).
pub(crate) struct StrippedEnv {
    /// Held for lifetime only — the pointer vector indexes these bytes.
    _strings: Vec<CString>,
    /// The NULL-terminated envp vector to forward.
    ptrs: Vec<*mut c_char>,
}

impl StrippedEnv {
    pub(crate) fn as_envp(&self) -> *const *mut c_char {
        self.ptrs.as_ptr()
    }
}

/// Walk envp's entries. `None` for a NULL envp (execve's empty-env form —
/// nothing to strip, nothing to probe for).
unsafe fn for_each_env(envp: *const *mut c_char, mut f: impl FnMut(&CStr)) {
    if envp.is_null() {
        return;
    }
    let mut cur = envp;
    // SAFETY: envp is the intercepted call's envp — a NULL-terminated
    // vector of NUL-terminated strings, per the call contract.
    while !(unsafe { *cur }).is_null() {
        // SAFETY: per the contract above.
        f(unsafe { CStr::from_ptr(*cur) });
        // SAFETY: advancing within the vector; the NULL slot ends the loop.
        cur = unsafe { cur.add(1) };
    }
}

/// An env entry's name (the bytes before the first `=`).
fn entry_name(entry: &[u8]) -> &[u8] {
    match entry.iter().position(|&b| b == b'=') {
        Some(i) => &entry[..i],
        None => entry,
    }
}

/// Does envp carry the insertion variable? The cheap pre-check — every
/// gate consults this FIRST so a process exec'ing without insertion (the
/// common case) pays no filesystem IO at all.
unsafe fn envp_has_insert(envp: *const *mut c_char) -> bool {
    let mut found = false;
    unsafe { for_each_env(envp, |e| found |= entry_name(e.to_bytes()) == INSERT_VAR) };
    found
}

/// Build the stripped environment. None when envp is NULL or carries no
/// insertion variable (nothing to do — forward verbatim).
unsafe fn strip_insert(envp: *const *mut c_char) -> Option<StrippedEnv> {
    if envp.is_null() {
        return None;
    }
    let mut strings: Vec<CString> = Vec::new();
    let mut stripped = false;
    unsafe {
        for_each_env(envp, |e| {
            if entry_name(e.to_bytes()) == INSERT_VAR {
                stripped = true;
            } else {
                strings.push(e.to_owned());
            }
        })
    };
    if !stripped {
        return None;
    }
    let mut ptrs: Vec<*mut c_char> = strings.iter().map(|s| s.as_ptr().cast_mut()).collect();
    ptrs.push(std::ptr::null_mut());
    Some(StrippedEnv {
        _strings: strings,
        ptrs,
    })
}

/// envp's PATH value (None when the vector carries none — execvp's
/// default-path fallback is not replicated: an unresolvable name passes
/// through unchanged).
unsafe fn envp_path(envp: *const *mut c_char) -> Option<String> {
    let mut found = None;
    unsafe {
        for_each_env(envp, |e| {
            if found.is_none() {
                if let Some(rest) = e.to_bytes().strip_prefix(b"PATH=") {
                    found = Some(String::from_utf8_lossy(rest).into_owned());
                }
            }
        })
    };
    found
}

/// posix_spawnp/execvp bare-name resolution for the PROBE: walk envp's
/// PATH, first readable hit wins (an empty component is the cwd, per
/// execvp tradition). Unresolvable → None: the caller passes through
/// unchanged (the real spawnp's own search answers for the exec).
unsafe fn resolve_on_path(name: &str, envp: *const *mut c_char) -> Option<String> {
    let path = unsafe { envp_path(envp) }?;
    for dir in path.split(':') {
        let candidate = if dir.is_empty() {
            name.to_owned()
        } else {
            format!("{dir}/{name}")
        };
        let Ok(c) = CString::new(candidate.as_str()) else {
            continue;
        };
        // SAFETY: plain libc through the tuple's original; `c` outlives
        // the call. Readable = opens O_RDONLY (the probe only reads).
        let fd = unsafe { super::plat::real_open()(c.as_ptr(), libc::O_RDONLY) };
        if fd >= 0 {
            // SAFETY: plain libc; fd is ours.
            unsafe { super::plat::real_close()(fd) };
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------
// The gates (the shim call sites' entry points)
// ---------------------------------------------------------------------

/// The strip decision for an exec/spawn of the explicit host path
/// `target` (the materialized copy for a memfs route, the original path
/// otherwise): Some(stripped envp) when insertion must not reach this
/// target, None to forward envp verbatim.
pub(crate) unsafe fn strip_for_path_target(
    target: &str,
    envp: *const *mut c_char,
) -> Option<StrippedEnv> {
    if !unsafe { envp_has_insert(envp) } {
        return None;
    }
    let slices = probe_slices(target)?;
    if dylib_loadable(HOST_ARCH, &slices) {
        return None;
    }
    note_strip(target);
    unsafe { strip_insert(envp) }
}

/// The strip decision for a posix_spawnp target that may be a bare name:
/// bare names resolve through the caller's PATH first (first readable
/// hit); unresolvable → None (passthrough, today's behavior).
pub(crate) unsafe fn strip_for_spawnp_target(
    file: &str,
    envp: *const *mut c_char,
) -> Option<StrippedEnv> {
    if file.contains('/') {
        return unsafe { strip_for_path_target(file, envp) };
    }
    if !unsafe { envp_has_insert(envp) } {
        return None;
    }
    let resolved = unsafe { resolve_on_path(file, envp) }?;
    let slices = probe_slices(&resolved)?;
    if dylib_loadable(HOST_ARCH, &slices) {
        return None;
    }
    note_strip(&resolved);
    unsafe { strip_insert(envp) }
}

/// The strip note, on the crate's existing debug channel
/// (`TEBAKO_DEBUG_TFS` — the same flag the open/openat shims use).
fn note_strip(target: &str) {
    if std::env::var_os("TEBAKO_DEBUG_TFS").is_some() {
        eprintln!(
            "[preload] strip DYLD_INSERT_LIBRARIES for exec of {target}: \
             no loadable slice for this dylib (tebako#448)"
        );
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ARM64: u32 = CPU_TYPE_ARM64;
    const ARM64E: u32 = 0x8000_0002;
    const X86_64: u32 = CPU_TYPE_X86_64;

    /// A thin 64-bit Mach-O header page (little-endian MH_MAGIC_64).
    fn thin(cputype: u32, cpusubtype: u32) -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[0..4].copy_from_slice(&0xfeedfacfu32.to_le_bytes());
        b[4..8].copy_from_slice(&cputype.to_le_bytes());
        b[8..12].copy_from_slice(&cpusubtype.to_le_bytes());
        b
    }

    /// A thin 64-bit Mach-O header, big-endian (MH_CIGAM_64).
    fn thin_be(cputype: u32, cpusubtype: u32) -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[0..4].copy_from_slice(&0xfeedfacfu32.to_be_bytes());
        b[4..8].copy_from_slice(&cputype.to_be_bytes());
        b[8..12].copy_from_slice(&cpusubtype.to_be_bytes());
        b
    }

    /// A fat header page holding the given slice table (big-endian
    /// FAT_MAGIC, the spelling every universal binary uses).
    fn fat(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut b = vec![0u8; 8 + entries.len() * 20];
        b[0..4].copy_from_slice(&0xcafebabeu32.to_be_bytes());
        b[4..8].copy_from_slice(&(entries.len() as u32).to_be_bytes());
        for (i, &(cpu, sub)) in entries.iter().enumerate() {
            let off = 8 + i * 20;
            b[off..off + 4].copy_from_slice(&cpu.to_be_bytes());
            b[off + 4..off + 8].copy_from_slice(&sub.to_be_bytes());
        }
        b
    }

    /// A fat header, little-endian (FAT_CIGAM — the rare spelling).
    fn fat_le(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut b = vec![0u8; 8 + entries.len() * 20];
        b[0..4].copy_from_slice(&0xcafebabeu32.to_le_bytes());
        b[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        for (i, &(cpu, sub)) in entries.iter().enumerate() {
            let off = 8 + i * 20;
            b[off..off + 4].copy_from_slice(&cpu.to_le_bytes());
            b[off + 4..off + 8].copy_from_slice(&sub.to_le_bytes());
        }
        b
    }

    #[test]
    fn parses_thin_arm64() {
        let s = parse_slices(&thin(ARM64, 0)).expect("thin arm64 parses");
        assert_eq!(
            s,
            [Slice {
                cputype: ARM64,
                cpusubtype: 0
            }]
        );
    }

    #[test]
    fn parses_thin_arm64e() {
        let s = parse_slices(&thin(ARM64, ARM64E)).expect("thin arm64e parses");
        assert!(is_arm64e(s[0].cpusubtype));
        // …and the big-endian spelling of the same header.
        let s = parse_slices(&thin_be(ARM64, ARM64E)).expect("thin be parses");
        assert!(is_arm64e(s[0].cpusubtype));
    }

    #[test]
    fn parses_fat_arm64_arm64e() {
        let s = parse_slices(&fat(&[(ARM64, 0), (ARM64, ARM64E)])).expect("fat parses");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].cputype, ARM64);
        assert!(is_arm64e(s[1].cpusubtype));
        // The little-endian fat spelling parses identically.
        let s = parse_slices(&fat_le(&[(ARM64, 0), (ARM64, ARM64E)])).expect("fat le parses");
        assert!(is_arm64e(s[1].cpusubtype));
    }

    #[test]
    fn parses_fat_x86_64_arm64() {
        let s = parse_slices(&fat(&[(X86_64, 3), (ARM64, 0)])).expect("fat parses");
        assert_eq!(s[0].cputype, X86_64);
        assert_eq!(s[1].cputype, ARM64);
    }

    #[test]
    fn rejects_non_macho_and_garbage() {
        assert_eq!(parse_slices(b"#!/bin/sh\necho hi\n"), None);
        assert_eq!(parse_slices(&[0xde, 0xad, 0xbe, 0xef, 1, 2, 3]), None);
        assert_eq!(parse_slices(&[]), None);
        // A fat header whose table overruns the probed page.
        let mut b = vec![0u8; 64];
        b[0..4].copy_from_slice(&0xcafebabeu32.to_be_bytes());
        b[4..8].copy_from_slice(&1000u32.to_be_bytes());
        assert_eq!(parse_slices(&b), None);
        // A truncated thin header.
        assert_eq!(parse_slices(&0xfeedfacfu32.to_le_bytes()), None);
    }

    #[test]
    fn dylib_loadable_decision_matrix() {
        let arm64 = Slice {
            cputype: ARM64,
            cpusubtype: 0,
        };
        let arm64e = Slice {
            cputype: ARM64,
            cpusubtype: ARM64E,
        };
        let x64 = Slice {
            cputype: X86_64,
            cpusubtype: 3,
        };
        // arm64 host: keep iff an arm64 slice AND no arm64e slice.
        assert!(dylib_loadable(HostArch::Arm64, &[arm64]));
        assert!(!dylib_loadable(HostArch::Arm64, &[arm64e]));
        assert!(!dylib_loadable(HostArch::Arm64, &[arm64, arm64e]));
        assert!(!dylib_loadable(HostArch::Arm64, &[x64]));
        assert!(dylib_loadable(HostArch::Arm64, &[x64, arm64]));
        // A plain subtype-2 WITHOUT the capability bit is not arm64e (the
        // Xcode-emitted spelling carries the bit).
        assert!(dylib_loadable(
            HostArch::Arm64,
            &[Slice {
                cputype: ARM64,
                cpusubtype: 2
            }]
        ));
        // x86_64 host: keep iff an x86_64 slice.
        assert!(dylib_loadable(HostArch::X86_64, &[x64]));
        assert!(!dylib_loadable(HostArch::X86_64, &[arm64]));
        assert!(dylib_loadable(HostArch::X86_64, &[x64, arm64e]));
    }

    /// A fake envp vector owning its strings; `ptrs` is what the shim
    /// receives.
    struct FakeEnv {
        _strings: Vec<CString>,
        ptrs: Vec<*mut c_char>,
    }

    fn fake_env(entries: &[&str]) -> FakeEnv {
        let strings: Vec<CString> = entries.iter().map(|s| CString::new(*s).unwrap()).collect();
        let mut ptrs: Vec<*mut c_char> = strings.iter().map(|s| s.as_ptr().cast_mut()).collect();
        ptrs.push(std::ptr::null_mut());
        FakeEnv {
            _strings: strings,
            ptrs,
        }
    }

    /// Read an envp back into strings (test-side mirror of the walk).
    unsafe fn collect(envp: *const *mut c_char) -> Vec<String> {
        let mut out = Vec::new();
        unsafe { for_each_env(envp, |e| out.push(e.to_string_lossy().into_owned())) };
        out
    }

    #[test]
    fn strip_removes_the_insertion_variable() {
        let env = fake_env(&[
            "PATH=/usr/bin",
            "DYLD_INSERT_LIBRARIES=/x/libtfs_preload.dylib",
            "TEBAKO_TFS_MOUNTS=/i:/tfs",
        ]);
        let stripped = unsafe { strip_insert(env.ptrs.as_ptr()) }.expect("the var is stripped");
        let got = unsafe { collect(stripped.as_envp()) };
        assert_eq!(got, ["PATH=/usr/bin", "TEBAKO_TFS_MOUNTS=/i:/tfs"]);
    }

    #[test]
    fn strip_without_the_variable_is_a_passthrough() {
        let env = fake_env(&["PATH=/usr/bin", "FOO=1"]);
        assert!(unsafe { strip_insert(env.ptrs.as_ptr()) }.is_none());
        assert!(unsafe { strip_insert(std::ptr::null()) }.is_none());
        // …and the cheap pre-check agrees (no probe IO would follow).
        assert!(!unsafe { envp_has_insert(env.ptrs.as_ptr()) });
        assert!(!unsafe { envp_has_insert(std::ptr::null()) });
    }

    #[test]
    fn strip_only_the_named_variable() {
        // A same-prefix decoy and an empty value: the name match is exact.
        let env = fake_env(&["DYLD_INSERT_LIBRARIES_EXTRA=keep", "DYLD_INSERT_LIBRARIES="]);
        let stripped = unsafe { strip_insert(env.ptrs.as_ptr()) }.expect("stripped");
        let got = unsafe { collect(stripped.as_envp()) };
        assert_eq!(got, ["DYLD_INSERT_LIBRARIES_EXTRA=keep"]);
    }

    #[test]
    fn envp_path_and_spawnp_resolution() {
        let dir = std::env::temp_dir().join(format!("macho-arch-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("mytool");
        std::fs::write(&tool, b"#!/bin/sh\n").unwrap();
        let spec = format!("PATH=/nope:{}", dir.display());
        let env = fake_env(&[&spec, "OTHER=1"]);
        assert_eq!(
            unsafe { envp_path(env.ptrs.as_ptr()) },
            Some(format!("/nope:{}", dir.display()))
        );
        let hit =
            unsafe { resolve_on_path("mytool", env.ptrs.as_ptr()) }.expect("the readable hit");
        assert_eq!(hit, tool.to_string_lossy());
        // A name that is nowhere: unresolvable.
        assert!(unsafe { resolve_on_path("no-such-tool", env.ptrs.as_ptr()) }.is_none());
        // …and a missing PATH resolves nothing.
        let env2 = fake_env(&["OTHER=1"]);
        assert!(unsafe { resolve_on_path("mytool", env2.ptrs.as_ptr()) }.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

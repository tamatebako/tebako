//! In-process `dlopen`/`dlerror` rebinding (tebako#1080).
//!
//! The runtime exe ships hardened-runtime signed. On macOS 14 (darwin
//! 23) dyld silently drops `DYLD_INSERT_LIBRARIES` into a hardened
//! process — the entitlement plane does not cover insertion there — so
//! neither the boot-head micro interpose dylib nor the preload ever
//! enters the interpreter process, and a `dlopen` that originates in a
//! later-loaded bundle (ffi, fiddle) escapes to the host filesystem
//! with the raw dyld error (the yeptris LoadError class). macOS 15
//! honors the insertion; macOS 14 never did.
//!
//! This module removes the interpreter process's dependency on dyld
//! insertion: at boot it rewrites the `dlopen`/`dlerror` bindings of
//! exactly the images that can ever hold a VFS spelling — the
//! materialized natives under the exec cache (`tebako-dlcache` /
//! `tebako-dl-`) — directly in their indirect symbol tables, and
//! repeats for later arrivals via `_dyld_register_func_for_add_image`.
//! The exe's own bindings stay untouched (ruby's dln patch routes
//! in-process already), and the host/system images likewise (they never
//! see a VFS path).
//!
//! Scope note: every shipped bundle uses the classic `LC_DYLD_INFO`
//! layout (verified on the 0.17.14 line's own fiddle/ffi builds); a
//! chained-fixups image in scope carries no indirect tables and is
//! logged at debug and left untouched — the pre-existing behavior,
//! never a silent claim of coverage.
//!
//! The route semantics mirror `loader_interpose.c` exactly: coverage
//! check → `dlmap2file` materialize → the real `dlopen` of the host
//! copy; ENOENT passes the original spelling through (covered-but-not-
//! held, the dln precedent); any other failure carries the verdict on
//! the `dlerror` channel.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

type DlopenFn = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void;
type DlerrorFn = unsafe extern "C" fn() -> *mut c_char;

static REAL_DLOPEN: AtomicUsize = AtomicUsize::new(0);
static REAL_DLERROR: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// The materialization verdict a failed rebound `dlopen` leaves for
    /// the next `dlerror` on the same thread (loader_interpose.c's
    /// `tfs_dl_verdict` — same wording, same contract).
    static VERDICT: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

fn real_dlopen() -> DlopenFn {
    // Captured at install from THIS image's binding — the exe is never
    // in the rebind scope, so its binding still names libSystem's.
    unsafe { std::mem::transmute::<usize, DlopenFn>(REAL_DLOPEN.load(Ordering::Relaxed)) }
}

fn real_dlerror() -> DlerrorFn {
    unsafe { std::mem::transmute::<usize, DlerrorFn>(REAL_DLERROR.load(Ordering::Relaxed)) }
}

fn set_errno(err: c_int) {
    unsafe { *libc::__error() = err };
}

fn remember_verdict(msg: String) {
    VERDICT.with(|v| *v.borrow_mut() = CString::new(msg).ok());
}

extern "C" fn rebound_dlopen(path: *const c_char, mode: c_int) -> *mut c_void {
    let real = real_dlopen();
    if path.is_null() {
        return unsafe { real(path, mode) };
    }
    let path_s = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    let ctx = tfs::context::context();
    let embedded = ctx
        .read()
        .map(|c| c.path_is_embedded(&path_s))
        .unwrap_or(false);
    if !embedded {
        return unsafe { real(path, mode) };
    }
    let mapped = ctx
        .write()
        .map(|mut c| c.dlmap2file(&path_s))
        .unwrap_or(Err(libc::EIO));
    match mapped {
        Ok(host) => {
            // tebako#1083: the platform loader's async first-seen
            // assessment of a just-materialized copy is not
            // instantaneous — a dlopen inside the window fails
            // transiently and the same path loads tens of milliseconds
            // later (the preload's dlopen arm retries the same window).
            // Bounded; a persistent failure reports as before, slower
            // by the ladder.
            let mut handle = unsafe { real(host.as_ptr(), mode) };
            let mut attempts_left = 5u32;
            let mut delay_ms = 50u64;
            while handle.is_null() && attempts_left > 0 {
                attempts_left -= 1;
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                delay_ms += 50;
                handle = unsafe { real(host.as_ptr(), mode) };
            }
            handle
        }
        Err(e) => {
            if e == libc::ENOENT {
                // Covered but not held: a host path the mount happens to
                // cover — the raw call serves it from the host.
                return unsafe { real(path, mode) };
            }
            let mount = ctx
                .read()
                .ok()
                .and_then(|c| c.mount_point_of(&path_s))
                .unwrap_or_else(|| "(unknown)".to_string());
            remember_verdict(format!(
                "tebako: cannot materialize VFS-resident library '{}' (mount '{}'): {}",
                path_s,
                mount,
                std::io::Error::from_raw_os_error(e)
            ));
            set_errno(e);
            std::ptr::null_mut()
        }
    }
}

extern "C" fn rebound_dlerror() -> *mut c_char {
    let ours = VERDICT.with(|v| v.borrow_mut().take());
    if let Some(v) = ours {
        // dlerror's answer is caller-read, callee-owned; the buffer
        // outlives the call by POSIX's own discipline, so the CString
        // intentionally leaks (the C twin's __thread stash likewise
        // persists until the thread's next verdict).
        v.into_raw()
    } else {
        unsafe { real_dlerror()() }
    }
}

// ---------------------------------------------------------------------
// The classic indirect-table rebind (fishhook's algorithm, narrowed to
// two symbols and the exec-cache scope). libc carries the Mach-O header
// structs but not the section/symbol-table ones — the layouts below are
// the stable <mach-o/loader.h> / <mach-o/nlist.h> ABI.
// ---------------------------------------------------------------------

const MH_MAGIC_64: u32 = 0xfeed_facf;
const LC_SEGMENT_64: u32 = 0x19;
const LC_SYMTAB: u32 = 0x2;
const LC_DYSYMTAB: u32 = 0xb;

/// <mach-o/loader.h> mach_header_64 (declared locally — libc's mach
/// types are deprecation-marked in favor of an external crate we don't
/// need; these layouts are the stable ABI).
#[repr(C)]
struct MachHeader64 {
    magic: u32,
    cputype: i32,
    cpusubtype: i32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    flags: u32,
    reserved: u32,
}

/// <mach-o/loader.h> load_command.
#[repr(C)]
#[derive(Clone, Copy)]
struct MachLoadCommand {
    cmd: u32,
    cmdsize: u32,
}

/// <mach-o/loader.h> segment_command_64.
#[repr(C)]
struct MachSegmentCommand64 {
    cmd: u32,
    cmdsize: u32,
    segname: [c_char; 16],
    vmaddr: u64,
    vmsize: u64,
    fileoff: u64,
    filesize: u64,
    maxprot: i32,
    initprot: i32,
    nsects: u32,
    flags: u32,
}

/// <mach-o/loader.h> section_64.
#[repr(C)]
struct MachSection64 {
    sectname: [c_char; 16],
    segname: [c_char; 16],
    addr: u64,
    size: u64,
    offset: u32,
    align: u32,
    reloff: u32,
    nreloc: u32,
    flags: u32,
    reserved1: u32,
    reserved2: u32,
    reserved3: u32,
}

/// <mach-o/loader.h> symtab_command.
#[repr(C)]
#[derive(Clone, Copy)]
struct MachSymtabCommand {
    cmd: u32,
    cmdsize: u32,
    symoff: u32,
    nsyms: u32,
    stroff: u32,
    strsize: u32,
}

/// <mach-o/loader.h> dysymtab_command.
#[repr(C)]
#[derive(Clone, Copy)]
struct MachDysymtabCommand {
    cmd: u32,
    cmdsize: u32,
    ilocalsym: u32,
    nlocalsym: u32,
    iextdefsym: u32,
    nextdefsym: u32,
    iundefsym: u32,
    nundefsym: u32,
    tocoff: u32,
    ntoc: u32,
    modtaboff: u32,
    nmodtab: u32,
    extrefsymoff: u32,
    nextrefsyms: u32,
    indirectsymoff: u32,
    nindirectsyms: u32,
    extreloff: u32,
    nextrel: u32,
    locreloff: u32,
    nlocrel: u32,
}

/// <mach-o/nlist.h> nlist_64 (the n_un union flattened — layout-
/// identical, a bare u32 lead).
#[repr(C)]
struct MachNlist64 {
    n_strx: u32,
    n_type: u8,
    n_sect: u8,
    n_desc: u16,
    n_value: u64,
}

// The mach VM surface libc does not export (vm_region.h /
// vm_prot.h — stable kernel ABI).
const VM_PROT_READ: i32 = 0x1;
const VM_PROT_WRITE: i32 = 0x2;
const VM_PROT_COPY: i32 = 0x10;
const VM_REGION_BASIC_INFO_64: i32 = 9;
const KERN_SUCCESS: i32 = 0;

/// <mach/vm_region.h> vm_region_basic_info_data_64_t.
#[repr(C)]
#[derive(Default)]
struct VmRegionBasicInfo64 {
    protection: i32,
    max_protection: i32,
    inheritance: u32,
    shared: u32,
    reserved: u32,
    offset: u64,
    behavior: i32,
    user_wired_count: u16,
    user_tag: u16,
}

/// sizeof(VmRegionBasicInfo64)/sizeof(int) — the flavor's count arg.
const VM_REGION_BASIC_INFO_COUNT_64: u32 = 9;

unsafe extern "C" {
    // The dyld image surface, declared against the local header type —
    // libc's own declarations reference its deprecation-marked mach
    // types, which the workspace's -D warnings refuses.
    fn mach_task_self() -> u32;
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_header(i: u32) -> *const MachHeader64;
    fn _dyld_get_image_name(i: u32) -> *const c_char;
    fn _dyld_get_image_vmaddr_slide(i: u32) -> isize;
    fn vm_protect(task: u32, addr: u64, size: u64, set_maximum: i32, prot: i32) -> i32;
    fn vm_region_64(
        task: u32,
        addr: *mut u64,
        size: *mut u64,
        flavor: i32,
        info: *mut VmRegionBasicInfo64,
        count: *mut u32,
        object_name: *mut u32,
    ) -> i32;
    fn _dyld_register_func_for_add_image(func: unsafe extern "C" fn(*const MachHeader64, isize));
}

/// The exec-cache markers that name an in-scope image: materialized
/// natives live under `<exec-cache>/tebako-dlcache/<key>/…` (macOS) or
/// the per-process `<exec-cache>/tebako-dl-<hex>/…` (the linux form —
/// the scope text is shared so a cross-built driver stays honest).
const SCOPE_MARKERS: &[&str] = &["/tebako-dlcache/", "/tebako-dl-"];

/// Test-time scope additions (a fixture bundle built into a temp dir is
/// not under the exec cache). Production installs never write here.
static SCOPE_EXTRA: std::sync::RwLock<Vec<String>> = std::sync::RwLock::new(Vec::new());

fn image_in_scope(name: Option<&str>) -> bool {
    let Some(n) = name else { return false };
    if SCOPE_MARKERS.iter().any(|m| n.contains(m)) {
        return true;
    }
    SCOPE_EXTRA
        .read()
        .map(|extra| extra.iter().any(|m| n.contains(m)))
        .unwrap_or(false)
}

/// One indirect-pointer section's coordinates.
struct PtrSection {
    /// In-memory address of the first slot (slide applied).
    slots: *mut u64,
    /// Slot count.
    count: usize,
    /// The section's start index into the indirect symbol table.
    indirect_start: usize,
}

struct ImageLayout {
    symtab: MachSymtabCommand,
    dysymtab: MachDysymtabCommand,
    /// slide + (__LINKEDIT vmaddr − fileoff): the in-memory base the
    /// symtab/dysymtab file offsets index from.
    linkedit_base: usize,
    sections: Vec<PtrSection>,
}

/// Parse the load commands for the symbol tables and the indirect
/// pointer sections. None for a chained-fixups image (no indirect
/// tables — the scope note in the module docs) or a non-64-bit header.
unsafe fn parse_layout(header: *const MachHeader64, slide: isize) -> Option<ImageLayout> {
    if header.is_null() || unsafe { (*header).magic } != MH_MAGIC_64 {
        return None;
    }
    let mut symtab: Option<MachSymtabCommand> = None;
    let mut dysymtab: Option<MachDysymtabCommand> = None;
    let mut linkedit_base: Option<usize> = None;
    let mut sections: Vec<PtrSection> = Vec::new();
    let mut cmd = unsafe { header.add(1) } as *const MachLoadCommand;
    let ncmds = unsafe { (*header).ncmds };
    for _ in 0..ncmds {
        let c = unsafe { *cmd };
        match c.cmd {
            LC_SYMTAB => {
                symtab = Some(unsafe { *(cmd as *const MachSymtabCommand) });
            }
            LC_DYSYMTAB => {
                dysymtab = Some(unsafe { *(cmd as *const MachDysymtabCommand) });
            }
            LC_SEGMENT_64 => {
                let seg = unsafe { &*(cmd as *const MachSegmentCommand64) };
                let segname = c_name(&seg.segname);
                if segname == "__LINKEDIT" {
                    linkedit_base = Some(
                        (slide as usize)
                            .wrapping_add(seg.vmaddr as usize)
                            .wrapping_sub(seg.fileoff as usize),
                    );
                }
                if segname == "__DATA" || segname == "__DATA_CONST" {
                    // The sections trail the FULL segment_command_64
                    // header (72 bytes) — not a bare load_command.
                    let mut sect = unsafe { (cmd as *const MachSegmentCommand64).add(1) }
                        as *const MachSection64;
                    for _ in 0..seg.nsects as usize {
                        let s = unsafe { &*sect };
                        let sname = c_name(&s.sectname);
                        // 0x6 S_NON_LAZY_SYMBOL_POINTERS /
                        // 0x7 S_LAZY_SYMBOL_POINTERS by flags; __got is
                        // S_REGULAR on the modern toolchain and matches
                        // by name. reserved1 is the indirect-table start
                        // for all three.
                        if matches!(s.flags & 0xff, 0x6 | 0x7) || sname == "__got" {
                            sections.push(PtrSection {
                                slots: (slide as usize + s.addr as usize) as *mut u64,
                                count: s.size as usize / 8,
                                indirect_start: s.reserved1 as usize,
                            });
                        }
                        sect = unsafe { sect.add(1) };
                    }
                }
            }
            _ => {}
        }
        cmd = (cmd as usize + c.cmdsize as usize) as *const MachLoadCommand;
    }
    match (symtab, dysymtab, linkedit_base) {
        (Some(symtab), Some(dysymtab), Some(linkedit_base)) if !sections.is_empty() => {
            Some(ImageLayout {
                symtab,
                dysymtab,
                linkedit_base,
                sections,
            })
        }
        _ => None,
    }
}

fn c_name(buf: &[c_char; 16]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Rewrite one image's `dlopen`/`dlerror` slots to the rebound pair.
/// Returns (dlopen hits, dlerror hits).
unsafe fn rebind_image(header: *const MachHeader64, slide: isize) -> (usize, usize) {
    let Some(layout) = (unsafe { parse_layout(header, slide) }) else {
        return (0, 0);
    };
    let symtab = (layout.linkedit_base + layout.symtab.symoff as usize) as *const MachNlist64;
    let strtab = (layout.linkedit_base + layout.symtab.stroff as usize) as *const c_char;
    let indirect = (layout.linkedit_base + layout.dysymtab.indirectsymoff as usize) as *const u32;
    let mut hits = (0usize, 0usize);
    for sect in &layout.sections {
        for j in 0..sect.count {
            let idx = sect.indirect_start + j;
            if idx >= layout.dysymtab.nindirectsyms as usize {
                break;
            }
            let sym_idx = unsafe { *indirect.add(idx) };
            // INDIRECT_SYMBOL_LOCAL / ABSOLUTE entries name no import.
            if sym_idx & 0xe000_0000 != 0 {
                continue;
            }
            let sym = unsafe { &*symtab.add(sym_idx as usize) };
            let name = unsafe { CStr::from_ptr(strtab.add(sym.n_strx as usize)) }.to_string_lossy();
            let as_dlopen: DlopenFn = rebound_dlopen;
            let as_dlerror: DlerrorFn = rebound_dlerror;
            let replacement: Option<usize> = match name.as_ref() {
                "_dlopen" => Some(as_dlopen as usize),
                "_dlerror" => Some(as_dlerror as usize),
                _ => None,
            };
            let Some(repl) = replacement else { continue };
            let slot = unsafe { sect.slots.add(j) };
            if unsafe { write_slot(slot, repl as u64) } {
                if name == "_dlopen" {
                    hits.0 += 1;
                } else {
                    hits.1 += 1;
                }
            }
        }
    }
    hits
}

/// Write one pointer slot, flipping the page writable first when the
/// kernel says it is read-only (the `__DATA_CONST` case). The original
/// protection is restored exactly; a later lazy bind through dyld's own
/// machinery re-protects as it always has.
unsafe fn write_slot(slot: *mut u64, value: u64) -> bool {
    if unsafe { *slot } == value {
        return true;
    }
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let page = (slot as usize) & !(page_size - 1);
    let mut info = VmRegionBasicInfo64::default();
    let mut count = VM_REGION_BASIC_INFO_COUNT_64;
    let mut addr = page as u64;
    let mut size: u64 = page_size as u64;
    let mut object_name: u32 = 0;
    let kr = unsafe {
        vm_region_64(
            mach_task_self(),
            &mut addr,
            &mut size,
            VM_REGION_BASIC_INFO_64,
            &mut info,
            &mut count,
            &mut object_name,
        )
    };
    let original_prot = if kr == KERN_SUCCESS {
        info.protection
    } else {
        VM_PROT_READ
    };
    if original_prot & VM_PROT_WRITE == 0 {
        let ok = unsafe {
            vm_protect(
                mach_task_self(),
                page as u64,
                page_size as u64,
                0,
                VM_PROT_READ | VM_PROT_WRITE | VM_PROT_COPY,
            )
        };
        if ok != KERN_SUCCESS {
            return false;
        }
    }
    unsafe { *slot = value };
    if original_prot & VM_PROT_WRITE == 0 {
        unsafe {
            vm_protect(
                mach_task_self(),
                page as u64,
                page_size as u64,
                0,
                original_prot,
            )
        };
    }
    true
}

unsafe extern "C" fn on_add_image(header: *const MachHeader64, slide: isize) {
    let header64 = header;
    let idx = image_index_of(header64);
    let in_scope = idx.and_then(image_name).map(|n| image_in_scope(Some(&n)));
    if in_scope == Some(true) {
        let hits = unsafe { rebind_image(header64, slide) };
        if hits != (0, 0) {
            tebako_log::log!(
                tebako_log::Level::Debug,
                "tfs",
                "rebind: add-image dlopen={} dlerror={}",
                hits.0,
                hits.1
            );
        }
    }
}

fn image_count() -> u32 {
    unsafe { _dyld_image_count() }
}

fn image_name(i: u32) -> Option<String> {
    let p = unsafe { _dyld_get_image_name(i) };
    if p.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

fn image_index_of(header: *const MachHeader64) -> Option<u32> {
    (0..image_count()).find(|&i| unsafe { _dyld_get_image_header(i) } == header)
}

/// Install the rebinding: capture the real pair from this image's own
/// (never-rebound) bindings, sweep the already-loaded in-scope images,
/// then rebind each later arrival. Idempotent; the boot calls it once
/// at the head, after the self-insert attempt — the two mechanisms
/// share the route semantics and never double-apply within scope (a
/// rebound slot points at the rebound pair; the interpose dylib's
/// tuples match libSystem's originals only).
pub(crate) fn install() {
    install_inner();
}

#[cfg(test)]
fn install_for_test(extra_scope: &str) {
    SCOPE_EXTRA.write().unwrap().push(extra_scope.to_string());
    install_inner();
}

fn install_inner() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        REAL_DLOPEN.store(libc::dlopen as *const () as usize, Ordering::Relaxed);
        REAL_DLERROR.store(libc::dlerror as *const () as usize, Ordering::Relaxed);
        let mut total = (0usize, 0usize);
        for i in 0..image_count() {
            let Some(name) = image_name(i) else { continue };
            if !image_in_scope(Some(&name)) {
                continue;
            }
            let header = unsafe { _dyld_get_image_header(i) };
            let slide = unsafe { _dyld_get_image_vmaddr_slide(i) };
            let hits = unsafe { rebind_image(header, slide) };
            total.0 += hits.0;
            total.1 += hits.1;
        }
        if total != (0, 0) {
            tebako_log::log!(
                tebako_log::Level::Debug,
                "tfs",
                "rebind: sweep dlopen={} dlerror={}",
                total.0,
                total.1
            );
        }
        unsafe { _dyld_register_func_for_add_image(on_add_image) };
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Build the fixture bundle: one export, `fixture_call`, that calls
    /// the dlopen its own image bound (the ffi/fiddle call class).
    fn build_fixture(dir: &std::path::Path) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let src = dir.join("fixture.c");
        std::fs::write(
            &src,
            "#include <dlfcn.h>\nvoid *fixture_call(const char *path) { return dlopen(path, RTLD_NOW); }\n",
        )
        .unwrap();
        let bundle = dir.join("tebako_rebind_fixture.bundle");
        let status = Command::new("clang")
            .args(["-bundle", "-o"])
            .arg(&bundle)
            .arg(&src)
            .status()
            .unwrap();
        assert!(status.success());
        bundle
    }

    /// Count the image's indirect `_dlopen` slots pointing at `target`.
    fn count_dlopen_slots_pointing_at(image_suffix: &str, target: usize) -> usize {
        let mut found = 0;
        for i in 0..image_count() {
            let Some(name) = image_name(i) else { continue };
            if !name.ends_with(image_suffix) {
                continue;
            }
            let header = unsafe { _dyld_get_image_header(i) };
            let slide = unsafe { _dyld_get_image_vmaddr_slide(i) };
            let Some(layout) = (unsafe { parse_layout(header, slide) }) else {
                continue;
            };
            let symtab =
                (layout.linkedit_base + layout.symtab.symoff as usize) as *const MachNlist64;
            let strtab = (layout.linkedit_base + layout.symtab.stroff as usize) as *const c_char;
            let indirect =
                (layout.linkedit_base + layout.dysymtab.indirectsymoff as usize) as *const u32;
            for sect in &layout.sections {
                for j in 0..sect.count {
                    let idx = sect.indirect_start + j;
                    if idx >= layout.dysymtab.nindirectsyms as usize {
                        break;
                    }
                    let sym_idx = unsafe { *indirect.add(idx) };
                    if sym_idx & 0xe000_0000 != 0 {
                        continue;
                    }
                    let sym = unsafe { &*symtab.add(sym_idx as usize) };
                    let name = unsafe { CStr::from_ptr(strtab.add(sym.n_strx as usize)) };
                    if name.to_string_lossy() != "_dlopen" {
                        continue;
                    }
                    let slot = unsafe { sect.slots.add(j) };
                    if unsafe { *slot } == target as u64 {
                        found += 1;
                    }
                }
            }
        }
        found
    }

    fn rebound_dlopen_addr() -> usize {
        let f: DlopenFn = rebound_dlopen;
        f as usize
    }

    #[test]
    fn scope_matching_is_marker_based() {
        assert!(image_in_scope(Some(
            "/var/folders/x/tebako-exec-a/tebako-dlcache/k/__tfs__/lib/f.bundle"
        )));
        assert!(image_in_scope(Some(
            "/tmp/tebako-exec-k/tebako-dl-abc123/usr/lib/f.so"
        )));
        assert!(!image_in_scope(Some("/usr/lib/libSystem.B.dylib")));
        assert!(!image_in_scope(Some("/private/tmp/dlprobe/main")));
        assert!(!image_in_scope(None));
    }

    #[test]
    fn rebind_reroutes_a_late_loaded_bundles_dlopen() {
        // The full behavior in one test (install is Once): the sweep on
        // an already-loaded fixture, then the add-image callback on a
        // second fixture loaded after the install.
        let dir = std::env::temp_dir().join(format!("tebako-rebind-test-{}", std::process::id()));
        let bundle_a = build_fixture(&dir.join("a"));

        // Phase 1: fixture A loaded BEFORE install — the sweep covers it.
        let a_c = CString::new(bundle_a.to_string_lossy().as_ref()).unwrap();
        let ha = unsafe { libc::dlopen(a_c.as_ptr(), libc::RTLD_NOW) };
        assert!(!ha.is_null(), "fixture A loads");
        let before =
            count_dlopen_slots_pointing_at("a/tebako_rebind_fixture.bundle", rebound_dlopen_addr());
        assert_eq!(before, 0, "the sweep has not run yet");

        install_for_test(&dir.to_string_lossy());

        let after =
            count_dlopen_slots_pointing_at("a/tebako_rebind_fixture.bundle", rebound_dlopen_addr());
        assert!(after >= 1, "the sweep rebound fixture A's dlopen slot");

        // The rebound slot still reaches the real dlopen for a host path
        // (the not-embedded passthrough) — the fixture proves it by
        // successfully loading a system library through the rewrite.
        type CallFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
        let sym = unsafe { libc::dlsym(ha, c"fixture_call".as_ptr()) };
        assert!(!sym.is_null());
        let call: CallFn = unsafe { std::mem::transmute(sym) };
        let z = CString::new("/usr/lib/libz.dylib").unwrap();
        let got = unsafe { call(z.as_ptr()) };
        assert!(
            !got.is_null(),
            "the rebound route passes host loads through"
        );

        // Phase 2: fixture B loaded AFTER install — the add-image
        // callback covers it.
        let bundle_b = build_fixture(&dir.join("b"));
        let b_c = CString::new(bundle_b.to_string_lossy().as_ref()).unwrap();
        let hb = unsafe { libc::dlopen(b_c.as_ptr(), libc::RTLD_NOW) };
        assert!(!hb.is_null(), "fixture B loads");
        let late =
            count_dlopen_slots_pointing_at("b/tebako_rebind_fixture.bundle", rebound_dlopen_addr());
        assert!(late >= 1, "the add-image callback rebound fixture B");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! tebako-pkg engine: TPKG trailer surgery with the exact semantics of the
//! C++ `package.cpp` in libtfs (bundle/unbundle/reassemble/insert-image/
//! remove-image/set-runtime/info). Built on crates/tpkg for the trailer
//! format (byte-parity) and on crates/tfs only for the plain-archive
//! fallback of `info`.
//!
//! Error values are the exact message bodies the C++ tool prints after its
//! "Error: <cmd> failed: " prefix; `info` error strings match its own
//! distinct paths.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use tpkg::{Crc32, Manifest, Slot, TpkgError};

mod json;

pub use json::{escape as json_escape, parse as json_parse, Value as JsonValue};

/// Copy chunk size (1 MiB, like the C++ side).
const COPY_BUF: usize = 1 << 20;

/// Package-level options (trailer fields besides the slots).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageOptions {
    pub runtime_ref: String,
    pub package_flags: u32,
    pub launcher_abi: u32,
}

/// An image spec: path + optional explicit mount point + format override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageImage {
    pub path: PathBuf,
    pub mount_point: String,
    pub format_id: u32,
}

/// A byte range of a file to stream into a package (length None = to EOF).
#[derive(Debug, Clone)]
struct PartSource {
    path: PathBuf,
    offset: u64,
    length: Option<u64>,
}

#[derive(Debug, Clone)]
struct SlotSource {
    source: PartSource,
    format_id: u32,
    flags: u32,
    mount_point: String,
}

// ---------------------------------------------------------------------
// Small public helpers (exact C++ semantics)
// ---------------------------------------------------------------------

/// Default mount point for slot `index`
/// (`/__tebako_memfs__` for 0, `/__tebako_memfs_<N>__` for N).
pub fn default_mount(index: u32) -> String {
    if index == 0 {
        "/__tebako_memfs__".to_string()
    } else {
        format!("/__tebako_memfs_{index}__")
    }
}

/// Parse an `<img[:mountpoint]>` spec: split on the LAST ':' when it is
/// immediately followed by '/'; otherwise the whole spec is the path.
pub fn parse_image_spec(spec: &str) -> PackageImage {
    if let Some(colon) = spec.rfind(':') {
        if colon + 1 < spec.len() && spec.as_bytes()[colon + 1] == b'/' {
            return PackageImage {
                path: PathBuf::from(&spec[..colon]),
                mount_point: spec[colon + 1..].to_string(),
                format_id: tpkg::TPKG_FORMAT_AUTO,
            };
        }
    }
    PackageImage {
        path: PathBuf::from(spec),
        mount_point: String::new(),
        format_id: tpkg::TPKG_FORMAT_AUTO,
    }
}

/// Sniff the image format from magic bytes (dwarfs/squashfs/zip, else auto).
pub fn sniff_format(path: &Path) -> u32 {
    let Ok(mut f) = fs::File::open(path) else {
        return tpkg::TPKG_FORMAT_AUTO;
    };
    let mut magic = [0u8; 8];
    let Ok(n) = f.read(&mut magic) else {
        return tpkg::TPKG_FORMAT_AUTO;
    };
    let magic = &magic[..n];
    if magic.starts_with(b"DWARFS") {
        tpkg::TPKG_FORMAT_DWARFS
    } else if magic.starts_with(b"hsqs") {
        tpkg::TPKG_FORMAT_SQUASHFS
    } else if magic.starts_with(b"PK\x03\x04") || magic.starts_with(b"PK\x05\x06") {
        tpkg::TPKG_FORMAT_ZIP
    } else {
        tpkg::TPKG_FORMAT_AUTO
    }
}

/// Format name for the manifest.json `format` field.
fn format_name(format_id: u32) -> &'static str {
    match format_id {
        tpkg::TPKG_FORMAT_DWARFS => "dwarfs",
        tpkg::TPKG_FORMAT_SQUASHFS => "squashfs",
        tpkg::TPKG_FORMAT_ZIP => "zip",
        _ => "auto",
    }
}

// ---------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------

/// Stream `source` into `out`; on success returns (bytes_written, crc32).
fn stream_part(
    source: &PartSource,
    out: &mut fs::File,
    want_crc: bool,
) -> Result<(u64, u32), String> {
    let mut input = fs::File::open(&source.path)
        .map_err(|_| format!("cannot open part file: {}", source.path.display()))?;
    let file_size = input
        .metadata()
        .map_err(|_| format!("cannot open part file: {}", source.path.display()))?
        .len();
    if source.offset > file_size {
        return Err(format!(
            "part offset {} is beyond the end of file: {}",
            source.offset,
            source.path.display()
        ));
    }
    let available = file_size - source.offset;
    let n = source.length.map_or(available, |l| l.min(available));
    input
        .seek(SeekFrom::Start(source.offset))
        .map_err(|_| format!("read failed: {}", source.path.display()))?;

    let mut buf = vec![0u8; COPY_BUF];
    let mut crc = Crc32::new();
    let mut remaining = n;
    while remaining > 0 {
        let chunk = remaining.min(buf.len() as u64) as usize;
        input
            .read_exact(&mut buf[..chunk])
            .map_err(|_| format!("read failed: {}", source.path.display()))?;
        out.write_all(&buf[..chunk])
            .map_err(|_| "write failed while streaming part".to_string())?;
        if want_crc {
            crc.update(&buf[..chunk]);
        }
        remaining -= chunk as u64;
    }
    Ok((n, crc.finish()))
}

/// Copy `size` bytes at `offset` of `source.path` into `out`, computing
/// the SHA-256 of the copied bytes in the same pass (used for the v2
/// trailer's per-slot digests).
fn stream_part_sha(source: &PartSource, out: &mut fs::File) -> Result<(u64, [u8; 32]), String> {
    use sha2::Digest;

    let mut input = fs::File::open(&source.path)
        .map_err(|_| format!("cannot open part file: {}", source.path.display()))?;
    let file_size = input
        .metadata()
        .map_err(|_| format!("cannot open part file: {}", source.path.display()))?
        .len();
    if source.offset > file_size {
        return Err(format!(
            "part offset {} is beyond the end of file: {}",
            source.offset,
            source.path.display()
        ));
    }
    let available = file_size - source.offset;
    let n = source.length.map_or(available, |l| l.min(available));
    input
        .seek(SeekFrom::Start(source.offset))
        .map_err(|_| format!("read failed: {}", source.path.display()))?;

    let mut buf = vec![0u8; COPY_BUF];
    let mut h = sha2::Sha256::new();
    let mut remaining = n;
    while remaining > 0 {
        let chunk = remaining.min(buf.len() as u64) as usize;
        input
            .read_exact(&mut buf[..chunk])
            .map_err(|_| format!("read failed: {}", source.path.display()))?;
        out.write_all(&buf[..chunk])
            .map_err(|_| "write failed while streaming part".to_string())?;
        h.update(&buf[..chunk]);
        remaining -= chunk as u64;
    }
    let digest: [u8; 32] = h.finalize().into();
    Ok((n, digest))
}

// ---------------------------------------------------------------------
// trailer signing (item 29: every package is signed)
// ---------------------------------------------------------------------

/// Sign the trailer of the just-assembled package: compute the v2
/// extension (per-slot digests, press-local signer keyid, OpenPGP
/// signature over the canonical trailer bytes) and append the signed v2
/// trailer. `f` is the package file positioned anywhere (seeked inside).
/// `m` is updated to version 2 in place.
fn sign_and_write_trailer(
    f: &mut fs::File,
    m: &mut Manifest,
    digests: &[[u8; 32]],
) -> Result<(), String> {
    let home = tebako_signer::default_home().map_err(|e| e.to_string())?;

    // Press-local key (generated on first use) + auto-registration into
    // the local trusted keyring (item 29 point 7: dev iteration uses the
    // local press key, registered automatically — never unsigned).
    let press = tebako_signer::press_local_key(&home).map_err(|e| e.to_string())?;
    let _ = tebako_signer::register_trusted(&home, &press.public_key).map_err(|e| e.to_string())?;

    if digests.len() != m.slots.len() {
        return Err("internal error: digest count does not match slot count".into());
    }
    let mut v2 = tpkg::V2Extension::default();
    for (i, d) in digests.iter().enumerate() {
        v2.slot_digests[i] = *d;
    }
    v2.signer_keyid = press.keyid;
    v2.signature = vec![0u8]; // placeholder (excluded from the signed region)

    m.version = tpkg::TPKG_VERSION_2;
    m.v2 = Some(v2);

    let end = f
        .seek(SeekFrom::End(0))
        .map_err(|_| "cannot seek package for trailer signing".to_string())?;
    let trailer = tpkg::encode_trailer(m, end)
        .map_err(|e| format!("tpkg trailer encode failed: {}", tpkg::strerror(e.code())))?;
    let region = tpkg::v2_signed_region(&trailer)
        .map_err(|e| format!("tpkg trailer encode failed: {}", tpkg::strerror(e.code())))?;
    let signature = tebako_signer::sign_detached(region, &press.secret_key, &press.fingerprint)
        .map_err(|e| e.to_string())?;

    m.v2.as_mut().unwrap().signature = signature;
    tpkg::write_to(f, m)
        .map_err(|e| format!("tpkg trailer write failed: {}", tpkg::strerror(e.code())))
}

// ---------------------------------------------------------------------
// assemble (shared core)
// ---------------------------------------------------------------------

fn assemble(
    bootstrap: &PartSource,
    slots: &[SlotSource],
    output: &Path,
    options: &PackageOptions,
) -> Result<(), String> {
    if slots.is_empty() || slots.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(format!(
            "slot count out of range (1..{})",
            tpkg::TPKG_MAX_SLOTS
        ));
    }
    if options.runtime_ref.len() >= tpkg::TPKG_RUNTIME_REF_LEN {
        return Err(format!(
            "runtime_ref is too long (max {} characters)",
            tpkg::TPKG_RUNTIME_REF_LEN - 1
        ));
    }
    for s in slots {
        if s.mount_point.len() >= tpkg::TPKG_MOUNT_POINT_LEN {
            return Err(format!(
                "mount point is too long (max {} characters): {}",
                tpkg::TPKG_MOUNT_POINT_LEN - 1,
                s.mount_point
            ));
        }
    }

    // Refuse to clobber an input part (canonical-path comparison).
    let out_canon = fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf());
    let clashes =
        |p: &Path| -> bool { fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()) == out_canon };
    if clashes(&bootstrap.path) {
        return Err(format!(
            "output path must differ from the bootstrap file: {}",
            bootstrap.path.display()
        ));
    }
    for s in slots {
        if s.source.path != bootstrap.path && clashes(&s.source.path) {
            return Err(format!(
                "output path must differ from the image file: {}",
                s.source.path.display()
            ));
        }
    }

    let mut m = Manifest {
        package_flags: options.package_flags,
        launcher_abi: options.launcher_abi,
        ..Default::default()
    };
    m.set_runtime_ref(options.runtime_ref.as_bytes());

    let cleanup = |out_path: &Path| {
        let _ = fs::remove_file(out_path);
    };

    {
        let mut out = match fs::File::create(output) {
            Ok(f) => f,
            Err(_) => return Err(format!("cannot create output file: {}", output.display())),
        };
        let mut total = 0u64;
        match stream_part(bootstrap, &mut out, false) {
            Ok((written, _)) => total += written,
            Err(e) => {
                drop(out);
                cleanup(output);
                return Err(e);
            }
        }
        let mut digests: Vec<[u8; 32]> = Vec::with_capacity(slots.len());
        for s in slots {
            let (written, digest) = match stream_part_sha(&s.source, &mut out) {
                Ok(r) => r,
                Err(e) => {
                    drop(out);
                    cleanup(output);
                    return Err(e);
                }
            };
            let mut slot = Slot::new(total, written, s.format_id, &s.mount_point);
            slot.flags = s.flags;
            m.slots.push(slot);
            digests.push(digest);
            total += written;
        }
        if out.flush().is_err() {
            drop(out);
            cleanup(output);
            return Err(format!("write failed: {}", output.display()));
        }
        let _ = total;

        // Sign the trailer (item 29: every package is signed with the
        // press-local key) and append the v2 trailer.
        if let Err(e) = sign_and_write_trailer(&mut out, &mut m, &digests) {
            drop(out);
            cleanup(output);
            return Err(e);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------
// manifest reading + rewrite-in-place helpers
// ---------------------------------------------------------------------

fn require_manifest(binary: &Path) -> Result<Manifest, String> {
    let mut f =
        fs::File::open(binary).map_err(|_| format!("{}: cannot read file", binary.display()))?;
    match tpkg::read_from(&mut f) {
        Ok(m) => Ok(m),
        Err(TpkgError::NoTrailer) => Err(format!(
            "{}: no tpkg manifest trailer present (not a three-part package)",
            binary.display()
        )),
        Err(TpkgError::Io) => Err(format!("{}: cannot read file", binary.display())),
        Err(e) => Err(format!(
            "{}: {}",
            binary.display(),
            tpkg::strerror(e.code())
        )),
    }
}

fn slots_from_manifest(binary: &Path, m: &Manifest) -> Vec<SlotSource> {
    m.slots
        .iter()
        .map(|s| SlotSource {
            source: PartSource {
                path: binary.to_path_buf(),
                offset: s.offset,
                length: Some(s.size),
            },
            format_id: s.format_id,
            flags: s.flags,
            mount_point: s.mount_point_str().unwrap_or_default().to_string(),
        })
        .collect()
}

fn options_from_manifest(m: &Manifest) -> PackageOptions {
    PackageOptions {
        runtime_ref: m.runtime_ref_str().unwrap_or_default().to_string(),
        package_flags: m.package_flags,
        launcher_abi: m.launcher_abi,
    }
}

/// Rewrite `binary` from new part sources; the original is replaced
/// (keeping its permissions) only after the new file is fully written.
fn rewrite_in_place(
    binary: &Path,
    bootstrap: &PartSource,
    slots: &[SlotSource],
    options: &PackageOptions,
) -> Result<(), String> {
    let perms = fs::metadata(binary).ok().map(|md| md.permissions());

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = binary.with_file_name(format!(
        "{}.tpkg-tmp-{}-{n}",
        binary
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        std::process::id()
    ));

    assemble(bootstrap, slots, &tmp, options)?;
    if let Some(p) = perms {
        let _ = fs::set_permissions(&tmp, p);
    }
    if fs::rename(&tmp, binary).is_err() {
        // Windows cannot rename over an existing file.
        let _ = fs::remove_file(binary);
        if fs::rename(&tmp, binary).is_err() {
            let _ = fs::remove_file(&tmp);
            return Err(format!(
                "cannot replace {} with the rewritten package",
                binary.display()
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Public operations
// ---------------------------------------------------------------------

/// bundle: assemble a three-part package from a bootstrap file and images.
/// Empty mount points default per slot index (the C++ cmd_bundle
/// contract); use `bundle_exact` to keep them exactly as given.
pub fn bundle(
    bootstrap: &Path,
    images: &[PackageImage],
    output: &Path,
    options: &PackageOptions,
) -> Result<(), String> {
    bundle_impl(bootstrap, images, output, options, true)
}

/// bundle_exact: like `bundle`, but mount points are used exactly as
/// given — an empty mount point stays empty (the Ruby Stitcher's
/// semantics; a fat package's FORMAT_RUNTIME payload slot carries an
/// empty mount point).
pub fn bundle_exact(
    bootstrap: &Path,
    images: &[PackageImage],
    output: &Path,
    options: &PackageOptions,
) -> Result<(), String> {
    bundle_impl(bootstrap, images, output, options, false)
}

fn bundle_impl(
    bootstrap: &Path,
    images: &[PackageImage],
    output: &Path,
    options: &PackageOptions,
    default_mounts: bool,
) -> Result<(), String> {
    if images.is_empty() || images.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(format!(
            "image count out of range (1..{})",
            tpkg::TPKG_MAX_SLOTS
        ));
    }
    if !bootstrap.is_file() {
        return Err(format!("bootstrap file not found: {}", bootstrap.display()));
    }
    let boot = PartSource {
        path: bootstrap.to_path_buf(),
        offset: 0,
        length: None,
    };

    let mut slots = Vec::with_capacity(images.len());
    for (i, img) in images.iter().enumerate() {
        if !img.path.is_file() {
            return Err(format!("image file not found: {}", img.path.display()));
        }
        slots.push(SlotSource {
            source: PartSource {
                path: img.path.clone(),
                offset: 0,
                length: None,
            },
            format_id: if img.format_id == tpkg::TPKG_FORMAT_AUTO {
                sniff_format(&img.path)
            } else {
                img.format_id
            },
            flags: 0,
            mount_point: if img.mount_point.is_empty() && default_mounts {
                default_mount(i as u32)
            } else {
                img.mount_point.clone()
            },
        });
    }
    assemble(&boot, &slots, output, options)
}

/// unbundle: decompose a package into a directory of parts + manifest.json.
pub fn unbundle(binary: &Path, output_dir: &Path) -> Result<(), String> {
    let m = require_manifest(binary)?;

    let total_size = fs::metadata(binary)
        .map_err(|e| format!("cannot stat {}: {}", binary.display(), e))?
        .len();

    // Re-read the fixed-size header for slot_table_offset / header_crc32
    // (the manifest struct does not expose them).
    let mut hdr = [0u8; tpkg::TPKG_HEADER_SIZE];
    {
        let mut f =
            fs::File::open(binary).map_err(|_| format!("cannot open {}", binary.display()))?;
        f.seek(SeekFrom::Start(total_size - tpkg::TPKG_HEADER_SIZE as u64))
            .and_then(|_| f.read_exact(&mut hdr))
            .map_err(|_| format!("cannot read trailer header of {}", binary.display()))?;
    }
    let table_off = u64::from_le_bytes(hdr[22..30].try_into().unwrap());
    let header_crc = u32::from_le_bytes(hdr[162..166].try_into().unwrap());

    // Slot ranges must lie entirely before the slot table.
    for (i, s) in m.slots.iter().enumerate() {
        if s.offset > table_off || s.size > table_off - s.offset {
            return Err(format!(
                "slot {i} byte range is out of bounds (corrupt package)"
            ));
        }
    }
    let bootstrap_size = m.slots.first().map_or(0, |s| s.offset);

    // Warn about byte ranges covered by no slot (dropped on reassemble).
    let mut expected = bootstrap_size;
    for (i, s) in m.slots.iter().enumerate() {
        if s.offset > expected {
            eprintln!(
                "unbundle: warning: dropping {} gap byte(s) before slot {i}",
                s.offset - expected
            );
        }
        expected = s.offset + s.size;
    }
    if expected < table_off {
        eprintln!(
            "unbundle: warning: dropping {} trailing gap byte(s)",
            table_off - expected
        );
    }

    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "cannot create output directory {}: {}",
            output_dir.display(),
            e
        )
    })?;

    // Stream the parts out, computing per-part checksums.
    struct PartOut {
        file: String,
        size: u64,
        crc: u32,
    }
    let mut parts: Vec<PartOut> = Vec::new();
    let mut stream_out = |src: PartSource, name: &str| -> Result<(), String> {
        let dest = output_dir.join(name);
        let mut out = fs::File::create(&dest)
            .map_err(|_| format!("cannot create part file: {}", dest.display()))?;
        match stream_part(&src, &mut out, true) {
            Ok((written, crc)) => {
                drop(out);
                parts.push(PartOut {
                    file: name.to_string(),
                    size: written,
                    crc,
                });
                Ok(())
            }
            Err(e) => {
                drop(out);
                let _ = fs::remove_file(&dest);
                Err(e)
            }
        }
    };

    stream_out(
        PartSource {
            path: binary.to_path_buf(),
            offset: 0,
            length: Some(bootstrap_size),
        },
        "bootstrap.bin",
    )?;
    for (i, s) in m.slots.iter().enumerate() {
        stream_out(
            PartSource {
                path: binary.to_path_buf(),
                offset: s.offset,
                length: Some(s.size),
            },
            &format!("image-{i}.bin"),
        )?;
    }

    // manifest.json (deterministic formatting; checksums via the tpkg CRC-32).
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"format\": \"tpkg\",\n");
    json.push_str(&format!("  \"format_version\": {},\n", m.version));
    json.push_str(&format!("  \"package_flags\": {},\n", m.package_flags));
    json.push_str(&format!("  \"launcher_abi\": {},\n", m.launcher_abi));
    json.push_str(&format!(
        "  \"runtime_ref\": \"{}\",\n",
        json_escape(m.runtime_ref_str().unwrap_or_default())
    ));
    json.push_str(&format!("  \"slot_table_offset\": {table_off},\n"));
    json.push_str(&format!("  \"header_crc32\": {header_crc},\n"));
    json.push_str(&format!(
        "  \"bootstrap\": {{ \"file\": \"{}\", \"size\": {}, \"crc32\": {} }},\n",
        parts[0].file, parts[0].size, parts[0].crc
    ));
    json.push_str("  \"slots\": [\n");
    for (i, s) in m.slots.iter().enumerate() {
        let p = &parts[i + 1];
        json.push_str(&format!(
            "    {{ \"index\": {i}, \"file\": \"{}\", \"offset\": {}, \"size\": {}, \"format_id\": {}, \"format\": \"{}\", \"flags\": {}, \"mount_point\": \"{}\", \"crc32\": {} }}",
            p.file,
            s.offset,
            s.size,
            s.format_id,
            format_name(s.format_id),
            s.flags,
            json_escape(s.mount_point_str().unwrap_or_default()),
            p.crc
        ));
        json.push_str(if i + 1 < m.slots.len() { ",\n" } else { "\n" });
    }
    json.push_str("  ]\n");
    json.push_str("}\n");

    let manifest_path = output_dir.join("manifest.json");
    fs::write(&manifest_path, json)
        .map_err(|_| format!("cannot create {}", manifest_path.display()))?;
    Ok(())
}

/// Reject manifest member names that escape the unbundled directory.
fn safe_part_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && Path::new(name).file_name().is_some_and(|f| f == name)
}

/// reassemble: rebuild a binary from an unbundled directory.
pub fn reassemble(input_dir: &Path, output: &Path) -> Result<(), String> {
    let manifest_path = input_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(format!(
            "manifest.json not found in {} (not an unbundled package directory)",
            input_dir.display()
        ));
    }
    let text = fs::read_to_string(&manifest_path)
        .map_err(|_| format!("cannot read {}", manifest_path.display()))?;
    let root =
        json_parse(&text).map_err(|e| format!("cannot parse {}: {e}", manifest_path.display()))?;
    if !matches!(root, JsonValue::Object(_)) {
        return Err(format!(
            "{}: top-level value must be an object",
            manifest_path.display()
        ));
    }
    if let Some(fmt) = root.find("format") {
        if fmt.as_string().as_deref() != Some("tpkg") {
            return Err(format!(
                "{}: unsupported format (expected \"tpkg\")",
                manifest_path.display()
            ));
        }
    }

    let mut opts = PackageOptions::default();
    if let Some(f) = root.find("package_flags") {
        opts.package_flags = f.as_u64().ok_or_else(|| {
            format!(
                "{}: package_flags must be an unsigned integer",
                manifest_path.display()
            )
        })? as u32;
    }
    if let Some(a) = root.find("launcher_abi") {
        opts.launcher_abi = a.as_u64().ok_or_else(|| {
            format!(
                "{}: launcher_abi must be an unsigned integer",
                manifest_path.display()
            )
        })? as u32;
    }
    if let Some(r) = root.find("runtime_ref") {
        opts.runtime_ref = r
            .as_string()
            .ok_or_else(|| format!("{}: runtime_ref must be a string", manifest_path.display()))?;
    }

    let mut bootstrap_name = "bootstrap.bin".to_string();
    if let Some(b) = root.find("bootstrap") {
        if let Some(f) = b.find("file") {
            bootstrap_name = f.as_string().ok_or_else(|| {
                format!(
                    "{}: bootstrap.file must be a string",
                    manifest_path.display()
                )
            })?;
        }
    }
    if !safe_part_name(&bootstrap_name) {
        return Err(format!(
            "{}: unsafe bootstrap file name: {}",
            manifest_path.display(),
            bootstrap_name
        ));
    }
    let bootstrap_path = input_dir.join(&bootstrap_name);
    if !bootstrap_path.is_file() {
        return Err(format!(
            "bootstrap part not found: {}",
            bootstrap_path.display()
        ));
    }

    let slots_json = root.find("slots").ok_or_else(|| {
        format!(
            "{}: slots must be an array of 1..{} entries",
            manifest_path.display(),
            tpkg::TPKG_MAX_SLOTS
        )
    })?;
    let JsonValue::Array(items) = slots_json else {
        return Err(format!(
            "{}: slots must be an array of 1..{} entries",
            manifest_path.display(),
            tpkg::TPKG_MAX_SLOTS
        ));
    };
    if items.is_empty() || items.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(format!(
            "{}: slots must be an array of 1..{} entries",
            manifest_path.display(),
            tpkg::TPKG_MAX_SLOTS
        ));
    }

    let mut slots = Vec::with_capacity(items.len());
    for (i, sj) in items.iter().enumerate() {
        let file = sj
            .find("file")
            .and_then(|f| f.as_string())
            .ok_or_else(|| format!("{}: slots[{i}].file is required", manifest_path.display()))?;
        if !safe_part_name(&file) {
            return Err(format!(
                "{}: unsafe slot file name: {}",
                manifest_path.display(),
                file
            ));
        }
        let part_path = input_dir.join(&file);
        if !part_path.is_file() {
            return Err(format!("slot part not found: {}", part_path.display()));
        }

        let mut s = SlotSource {
            source: PartSource {
                path: part_path.clone(),
                offset: 0,
                length: None,
            },
            format_id: sniff_format(&part_path),
            flags: 0,
            mount_point: default_mount(i as u32),
        };
        if let Some(mp) = sj.find("mount_point") {
            s.mount_point = mp.as_string().ok_or_else(|| {
                format!(
                    "{}: slots[{i}].mount_point must be a string",
                    manifest_path.display()
                )
            })?;
        }
        if let Some(fi) = sj.find("format_id") {
            s.format_id = fi.as_u64().ok_or_else(|| {
                format!(
                    "{}: slots[{i}].format_id must be an unsigned integer",
                    manifest_path.display()
                )
            })? as u32;
        }
        if let Some(fl) = sj.find("flags") {
            s.flags = fl.as_u64().ok_or_else(|| {
                format!(
                    "{}: slots[{i}].flags must be an unsigned integer",
                    manifest_path.display()
                )
            })? as u32;
        }
        slots.push(s);
    }

    assemble(
        &PartSource {
            path: bootstrap_path,
            offset: 0,
            length: None,
        },
        &slots,
        output,
        &opts,
    )
}

/// insert-image: append an image slot to a package (rewritten in place).
pub fn insert_image(binary: &Path, image: &Path, mount_point: &str) -> Result<(), String> {
    let m = require_manifest(binary)?;
    if m.slots.len() >= tpkg::TPKG_MAX_SLOTS as usize {
        return Err(format!(
            "{}: package already has the maximum of {} image slots",
            binary.display(),
            tpkg::TPKG_MAX_SLOTS
        ));
    }
    if !image.is_file() {
        return Err(format!("image file not found: {}", image.display()));
    }
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    if canon(image) == canon(binary) {
        return Err("cannot insert the package into itself".to_string());
    }

    let mut slots = slots_from_manifest(binary, &m);
    slots.push(SlotSource {
        source: PartSource {
            path: image.to_path_buf(),
            offset: 0,
            length: None,
        },
        format_id: sniff_format(image),
        flags: 0,
        mount_point: if mount_point.is_empty() {
            default_mount(m.slots.len() as u32)
        } else {
            mount_point.to_string()
        },
    });

    rewrite_in_place(
        binary,
        &PartSource {
            path: binary.to_path_buf(),
            offset: 0,
            length: Some(m.slots[0].offset),
        },
        &slots,
        &options_from_manifest(&m),
    )
}

/// remove-image: remove an image slot from a package (rewritten in place).
pub fn remove_image(binary: &Path, slot_index: u32) -> Result<(), String> {
    let m = require_manifest(binary)?;
    if slot_index as usize >= m.slots.len() {
        return Err(format!(
            "{}: slot index {} out of range (package has {} slot(s))",
            binary.display(),
            slot_index,
            m.slots.len()
        ));
    }
    if m.slots.len() == 1 {
        return Err(format!(
            "{}: cannot remove the last image slot (a manifest requires at least one slot)",
            binary.display()
        ));
    }

    let mut slots = slots_from_manifest(binary, &m);
    slots.remove(slot_index as usize);

    rewrite_in_place(
        binary,
        &PartSource {
            path: binary.to_path_buf(),
            offset: 0,
            length: Some(m.slots[0].offset),
        },
        &slots,
        &options_from_manifest(&m),
    )
}

/// set-runtime: replace the bootstrap portion of a package (in place).
pub fn set_runtime(binary: &Path, runtime_file: &Path) -> Result<(), String> {
    let m = require_manifest(binary)?;
    if !runtime_file.is_file() {
        return Err(format!(
            "runtime file not found: {}",
            runtime_file.display()
        ));
    }
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    if canon(runtime_file) == canon(binary) {
        return Err("cannot use the package as its own runtime".to_string());
    }

    let slots = slots_from_manifest(binary, &m);
    rewrite_in_place(
        binary,
        &PartSource {
            path: runtime_file.to_path_buf(),
            offset: 0,
            length: None,
        },
        &slots,
        &options_from_manifest(&m),
    )
}

// ---------------------------------------------------------------------
// info
// ---------------------------------------------------------------------

/// Human-readable size ("%.1f <unit>", units B/KB/MB/GB/TB dividing by 1024).
pub fn format_size(size: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut unit = 0;
    let mut size_d = size as f64;
    while size_d >= 1024.0 && unit < 4 {
        size_d /= 1024.0;
        unit += 1;
    }
    format!("{size_d:.1} {}", UNITS[unit])
}

/// The `info` signature report (item 29): v2 → signer keyid + trust state
/// against the local trusted keyring; v1 → legacy unsigned notice.
fn signature_status(archive: &Path, m: &Manifest) -> String {
    let Some(v2) = &m.v2 else {
        return "Signature: none (v1 legacy trailer — unsigned)\n".to_string();
    };

    let keyid_hex = v2.signer_keyid_hex();
    let trust = (|| -> Result<String, String> {
        let home = tebako_signer::default_home().map_err(|e| e.to_string())?;
        let keyring = tebako_signer::trusted_keyring_bytes(&home).map_err(|e| e.to_string())?;

        let mut f = fs::File::open(archive).map_err(|_| "cannot re-read package".to_string())?;
        let tlen = tpkg::trailer_len(m);
        f.seek(std::io::SeekFrom::End(-(tlen as i64)))
            .map_err(|_| "cannot re-read trailer".to_string())?;
        let mut trailer = vec![0u8; tlen as usize];
        use std::io::Read;
        f.read_exact(&mut trailer)
            .map_err(|_| "cannot re-read trailer".to_string())?;
        let region =
            tpkg::v2_signed_region(&trailer).map_err(|e| tpkg::strerror(e.code()).to_string())?;

        let outcome =
            tebako_signer::verify_detached(&keyring, region, &v2.signature, &v2.signer_keyid)
                .map_err(|e| e.to_string())?;
        Ok(match outcome {
            tebako_signer::VerifyOutcome::Trusted(_) => "trusted".to_string(),
            tebako_signer::VerifyOutcome::Untrusted(_) => {
                "UNTRUSTED (signer key not in the local keyring)".to_string()
            }
            tebako_signer::VerifyOutcome::Invalid(_) => "INVALID SIGNATURE".to_string(),
        })
    })();

    match trust {
        Ok(t) => format!("Signature: OpenPGP v2, signer={keyid_hex} [{t}]\n"),
        Err(e) => format!("Signature: OpenPGP v2, signer={keyid_hex} [trust unknown: {e}]\n"),
    }
}

/// Dump a three-part package trailer (exact C++ `cmd_info` tpkg output),
/// or fall through to a plain-archive summary (via the tfs C ABI).
/// Errors match the C++ message bodies.
pub fn info(archive: &Path) -> Result<String, String> {
    let mut out = String::new();

    let mut f =
        fs::File::open(archive).map_err(|_| format!("{}: tpkg i/o error", archive.display()))?;
    let manifest = match tpkg::read_from(&mut f) {
        Ok(m) => Some(m),
        Err(TpkgError::NoTrailer) => None,
        Err(e) => {
            return Err(format!(
                "{}: {}",
                archive.display(),
                tpkg::strerror(e.code())
            ));
        }
    };

    if let Some(m) = manifest {
        let total_size = fs::metadata(archive).map(|md| md.len()).unwrap_or(0);
        out.push_str(&format!("Package: {}\n", archive.display()));
        out.push_str(&format!(
            "Format: tebako three-part package (tpkg v{})\n",
            m.version
        ));
        out.push_str(&format!(
            "Total size: {} ({} bytes)\n",
            format_size(total_size as i64),
            total_size
        ));
        out.push_str(&format!("Flags: 0x{:x}", m.package_flags));
        if m.is_lean() {
            out.push_str(" (LEAN)");
        }
        out.push('\n');
        out.push_str(&format!("Launcher ABI: {}\n", m.launcher_abi));
        let rr = m.runtime_ref_str().unwrap_or_default();
        out.push_str(&format!(
            "Runtime ref: {}\n",
            if rr.is_empty() {
                "(none — classic bundle)".to_string()
            } else {
                rr.to_string()
            }
        ));
        let bootstrap_size = m.slots.first().map_or(0, |s| s.offset);
        out.push_str(&format!("Bootstrap size: {bootstrap_size} bytes\n"));
        out.push_str(&format!("Slots: {}\n", m.slots.len()));
        for (i, s) in m.slots.iter().enumerate() {
            out.push_str(&format!(
                "  [{i}] offset={} size={} format={} flags={} mount={}\n",
                s.offset,
                s.size,
                format_name(s.format_id),
                s.flags,
                s.mount_point_str().unwrap_or_default()
            ));
        }
        out.push_str("Trailer: valid (magic and crc32 ok)\n");
        out.push_str(&signature_status(archive, &m));
        return Ok(out);
    }

    // Plain archive: mount through the tfs C ABI and summarize.
    out.push_str(&format!("Archive: {}\n", archive.display()));
    out.push_str(&plain_archive_summary(archive)?);
    Ok(out)
}

fn plain_archive_summary(archive: &Path) -> Result<String, String> {
    use tfs::c_api::*;

    let path = std::ffi::CString::new(archive.to_string_lossy().as_bytes())
        .map_err(|_| format!("Failed to open archive: {}", archive.display()))?;
    let mp = std::ffi::CString::new("/mnt").unwrap();

    let rc = unsafe { tebako_fs_init_from_file(path.as_ptr(), mp.as_ptr()) };
    if rc != 0 {
        return Err(format!(
            "Failed to open archive: {}\n       Unsupported format or file does not exist",
            archive.display()
        ));
    }
    struct Unmount;
    impl Drop for Unmount {
        fn drop(&mut self) {
            unsafe { tebako_fs_unmount() };
        }
    }
    let _guard = Unmount;

    // Archive type by extension (mirrors the C++ heuristics).
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ty = match ext.as_str() {
        "zip" => "ZIP",
        "sqfs" | "squashfs" => "SquashFS",
        // .tfs is the dwarfs-t-native (FlatBuffers metadata) extension
        // (postdates the C++ heuristics, which know .dwarfs only)
        "dwarfs" | "tfs" => "DwarFS",
        _ => "Unknown",
    };

    let mut file_count = 0i64;
    let mut dir_count = 0i64;
    let mut total_size = 0i64;
    count_recursive("/mnt", &mut file_count, &mut dir_count, &mut total_size);

    Ok(format!(
        "Type: {ty}\nFiles: {file_count}\nDirectories: {dir_count}\nTotal size: {} ({total_size} bytes)\n",
        format_size(total_size)
    ))
}

fn count_recursive(path: &str, files: &mut i64, dirs: &mut i64, total: &mut i64) {
    use tfs::c_api::*;
    let cpath = std::ffi::CString::new(path).unwrap();
    let dir = unsafe { tebako_fs_opendir(cpath.as_ptr()) };
    if dir.is_null() {
        return;
    }
    loop {
        let entry = unsafe { tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let is_dir = unsafe { (*entry).d_type } == tfs::DT_DIR;
        if is_dir {
            *dirs += 1;
            count_recursive(&format!("{path}/{name}"), files, dirs, total);
        } else {
            *files += 1;
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            let fpath = std::ffi::CString::new(format!("{path}/{name}")).unwrap();
            if unsafe { tebako_fs_stat(fpath.as_ptr(), &mut st) } == 0 {
                *total += st.st_size;
            }
        }
    }
    unsafe { tebako_fs_closedir(dir) };
}

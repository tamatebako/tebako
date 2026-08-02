//! tfs-cli engine: the generic VFS image operations (info/ls/tree/cat/
//! stat/extract/find/mkimage) with the exact output semantics of the C++
//! tebakofs CLI (libtfs `src/cli/tebakofs.cpp`).
//!
//! Everything goes through the tfs C ABI mounted at `/mnt` (like the C++
//! CLI). Error strings match the C++ message bodies; the CLI layer prints
//! them to stderr and returns exit code 1.

use std::ffi::CString;
use std::io::Write;
use std::path::{Path, PathBuf};

use tfs::c_api::*;

// The ENC verbs compile where tfs ships the `enc` feature (everywhere
// but windows — rnp's mingw build is unproven, TODO.v2-1/08); windows
// gets the named-ENOTSUP surface instead (TODO.v2-1/02). Same module API
// on both sides — keep enc.rs and enc_enotsup.rs in lockstep.
#[cfg(not(windows))]
pub mod enc;
#[cfg(windows)]
#[path = "enc_enotsup.rs"]
pub mod enc;

/// Options shared by the listing commands.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub recursive: bool,
    pub long_format: bool,
    pub verbose: bool,
    pub quiet: bool,
}

/// Options for extract.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub dest_dir: PathBuf,
    pub verbose: bool,
    pub quiet: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        ExtractOptions {
            dest_dir: PathBuf::from("."),
            verbose: false,
            quiet: false,
        }
    }
}

// ---------------------------------------------------------------------
// Mount guard
// ---------------------------------------------------------------------

/// Mount an image at /mnt via the C ABI; unmounts on drop.
pub struct MountGuard(());

impl MountGuard {
    /// Mount `image`; on failure returns the C++ `open_archive` error text.
    pub fn mount(image: &Path) -> Result<MountGuard, String> {
        let path = cstring(&image.to_string_lossy())?;
        let mp = cstring("/mnt")?;
        let rc = unsafe { tebako_fs_init_from_file(path.as_ptr(), mp.as_ptr()) };
        if rc != 0 {
            let err = unsafe { tebako_get_errno() };
            if err == libc::EIO {
                // The C++ factory succeeds on recognized magic and the mount
                // fails (e.g. corrupt image).
                return Err(format!("Failed to mount archive: {}", image.display()));
            }
            return Err(format!(
                "Failed to open archive: {}\n       Unsupported format or file does not exist",
                image.display()
            ));
        }
        Ok(MountGuard(()))
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        unsafe { tebako_fs_unmount() };
    }
}

fn cstring(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| "path contains interior NUL byte".to_string())
}

// ---------------------------------------------------------------------
// Path helpers (mirror the C++ full_path construction)
// ---------------------------------------------------------------------

/// "/mnt"-prefixed image path for a CLI path argument.
fn full_path(path: &str) -> String {
    if path == "/" || path.is_empty() {
        "/mnt".to_string()
    } else if path.starts_with('/') {
        format!("/mnt{path}")
    } else {
        format!("/mnt/{path}")
    }
}

/// stat helper (errno-valued). TebakoStat is the platform's `struct stat`
/// on unix and the pinned `__stat64`-layout struct on windows (the C ABI
/// authority — c_api.rs).
fn stat_raw(path: &str) -> Result<TebakoStat, i32> {
    let cpath = cstring(path).map_err(|_| libc::EINVAL)?;
    let mut st: TebakoStat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tebako_fs_stat(cpath.as_ptr(), &mut st) };
    if rc != 0 {
        return Err(unsafe { tebako_get_errno() });
    }
    Ok(st)
}

fn exists(path: &str) -> bool {
    stat_raw(path).is_ok()
}

// st_mode is mode_t on unix (u16 macOS / u32 Linux), u16 in the pinned
// windows tebako_stat, and the S_IF* constant widths follow the platform;
// the widening `as u32` is required on macOS/windows and an identity cast
// on Linux, so the platform-dependent unnecessary_cast lint is allowed
// here deliberately (the c_api::fill_stat pattern).
#[allow(clippy::unnecessary_cast)]
fn is_file(path: &str) -> bool {
    stat_raw(path).is_ok_and(|st| (st.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFREG as u32)
}

// The same st_mode/S_IF* width story as is_file above.
#[allow(clippy::unnecessary_cast)]
fn is_directory(path: &str) -> bool {
    stat_raw(path).is_ok_and(|st| (st.st_mode as u32 & libc::S_IFMT as u32) == libc::S_IFDIR as u32)
}

struct DirEntry {
    name: String,
    is_dir: bool,
}

fn read_dir(path: &str) -> Result<Vec<DirEntry>, i32> {
    let cpath = cstring(path).map_err(|_| libc::EINVAL)?;
    let dir = unsafe { tebako_fs_opendir(cpath.as_ptr()) };
    if dir.is_null() {
        return Err(unsafe { tebako_get_errno() });
    }
    let mut out = Vec::new();
    loop {
        let entry = unsafe { tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let is_dir = unsafe { (*entry).d_type } == tfs::DT_DIR;
        out.push(DirEntry { name, is_dir });
    }
    unsafe { tebako_fs_closedir(dir) };
    Ok(out)
}

// ---------------------------------------------------------------------
// Formatting helpers (exact C++ output)
// ---------------------------------------------------------------------

/// Human-readable size ("%.1f <unit>").
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

/// `ls -l` permission string from entry type (C++ print_entry passes a
/// bare 0755/0644 without the S_IFDIR bit, so the leading char is ALWAYS
/// '-' — the quirk is reproduced for output parity).
fn format_permissions(is_dir: bool) -> String {
    let mode: u32 = if is_dir { 0o755 } else { 0o644 };
    let mut s = String::with_capacity(10);
    s.push('-');
    for (bit, ch) in [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ] {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

/// "%Y-%m-%d %H:%M:%S" in local time (C++ format_time).
pub fn format_time(mtime: i64) -> String {
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        let t: libc::time_t = mtime as libc::time_t;
        // The reentrant localtime per platform: POSIX localtime_r
        // (time_t*, tm*) returns null on failure; the MS CRT localtime_s
        // takes the REVERSED argument order (tm*, time_t*) and returns an
        // errno_t (0 == success). Both fill `tm` in place on success.
        #[cfg(unix)]
        let failed = libc::localtime_r(&t, &mut tm).is_null();
        #[cfg(windows)]
        let failed = libc::localtime_s(&mut tm, &t) != 0;
        if failed {
            return "1970-01-01 00:00:00".to_string();
        }
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

// ---------------------------------------------------------------------
// ls
// ---------------------------------------------------------------------

/// `tfs ls` — single directory listing (or recursive with -r).
pub fn cmd_ls(image: &Path, path: &str, opts: &ListOptions) -> Result<String, (String, i32)> {
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;
    let full = full_path(path);

    if full != "/mnt" && !exists(&full) {
        return Err((format!("Error: Path does not exist: {path}"), 1));
    }

    let mut out = String::new();
    if is_file(&full) {
        let name = path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(path);
        let st = stat_raw(&full).map_err(|e| (format!("Error: stat failed ({e})"), 1))?;
        out.push_str(&print_entry(
            name,
            path,
            false,
            st.st_size,
            st.st_mtime,
            opts.long_format,
        ));
        return Ok(out);
    }

    if opts.recursive {
        list_recursive(&full, &mut out, opts.long_format)?;
    } else {
        let entries =
            read_dir(&full).map_err(|_| (format!("Error: Failed to list directory: {path}"), 1))?;
        for e in entries {
            let display = if path.ends_with('/') || path.is_empty() {
                format!("{path}{}", e.name)
            } else {
                format!("{path}/{}", e.name)
            };
            let (size, mtime) = entry_meta(&full, &e.name);
            out.push_str(&print_entry(
                &e.name,
                &display,
                e.is_dir,
                size,
                mtime,
                opts.long_format,
            ));
        }
    }
    Ok(out)
}

fn entry_meta(dir: &str, name: &str) -> (i64, i64) {
    stat_raw(&format!("{dir}/{name}")).map_or((0, 0), |st| (st.st_size, st.st_mtime))
}

fn list_recursive(dir: &str, out: &mut String, long_format: bool) -> Result<(), (String, i32)> {
    let entries =
        read_dir(dir).map_err(|_| (format!("Error: Failed to list directory: {dir}"), 1))?;
    for e in entries {
        let entry_path = format!("{dir}/{}", e.name);
        let display = entry_path
            .strip_prefix("/mnt")
            .unwrap_or(&entry_path)
            .to_string();
        let (size, mtime) = entry_meta(dir, &e.name);
        out.push_str(&print_entry(
            &e.name,
            &display,
            e.is_dir,
            size,
            mtime,
            long_format,
        ));
        if e.is_dir {
            list_recursive(&entry_path, out, long_format)?;
        }
    }
    Ok(())
}

fn print_entry(
    _name: &str,
    display_path: &str,
    is_dir: bool,
    size: i64,
    mtime: i64,
    long_format: bool,
) -> String {
    if long_format {
        format!(
            "{}  {:>10}  {}  {}\n",
            format_permissions(is_dir),
            format_size(size),
            format_time(mtime),
            display_path
        )
    } else {
        format!("{display_path}\n")
    }
}

// ---------------------------------------------------------------------
// tree
// ---------------------------------------------------------------------

/// `tfs tree` — the C++ tree output (├──/└──, trailing '/' on dirs).
pub fn cmd_tree(image: &Path, path: &str) -> Result<String, (String, i32)> {
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;
    let full = full_path(path);

    if full != "/mnt" && !exists(&full) {
        return Err((format!("Error: Path does not exist: {path}"), 1));
    }

    let mut out = format!("{}\n", if path.is_empty() { "/" } else { path });
    print_tree(&full, "", &mut out);
    Ok(out)
}

fn print_tree(dir: &str, prefix: &str, out: &mut String) {
    let Ok(entries) = read_dir(dir) else {
        return;
    };
    let last = entries.len().saturating_sub(1);
    for (i, e) in entries.iter().enumerate() {
        let is_last = i == last;
        out.push_str(prefix);
        out.push_str(if is_last { "└── " } else { "├── " });
        out.push_str(&e.name);
        if e.is_dir {
            out.push('/');
        }
        out.push('\n');
        if e.is_dir {
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            print_tree(&format!("{dir}/{}", e.name), &new_prefix, out);
        }
    }
}

// ---------------------------------------------------------------------
// cat
// ---------------------------------------------------------------------

/// `tfs cat` — stream a file to `out` (pread chunks; no full
/// materialization of large files in memory).
pub fn cmd_cat(image: &Path, file: &str, out: &mut dyn Write) -> Result<(), (String, i32)> {
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;
    let full = full_path(file);

    if !exists(&full) {
        return Err((format!("Error: File does not exist: {file}"), 1));
    }
    if is_directory(&full) {
        return Err((format!("Error: Path is a directory: {file}"), 1));
    }

    let cpath = cstring(&full).map_err(|_| ("Error: invalid path".to_string(), 1))?;
    let fd = unsafe { tebako_fs_open(cpath.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err((format!("Error: Failed to open file: {file}"), 1));
    }
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { tebako_fs_read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            break;
        }
        if out.write_all(&buf[..n as usize]).is_err() {
            unsafe { tebako_fs_close(fd) };
            return Err((format!("Error: write failed for: {file}"), 1));
        }
    }
    unsafe { tebako_fs_close(fd) };
    Ok(())
}

// ---------------------------------------------------------------------
// stat
// ---------------------------------------------------------------------

/// `tfs stat` — the C++ stat output.
pub fn cmd_stat(image: &Path, path: &str) -> Result<String, (String, i32)> {
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;
    let full = full_path(path);

    if full != "/mnt" && !exists(&full) {
        return Err((format!("Error: Path does not exist: {path}"), 1));
    }

    let mut out = format!("File: {path}\n");
    if is_directory(&full) {
        out.push_str("Type: directory\n");
    } else {
        out.push_str("Type: file\n");
        let st = stat_raw(&full).map_err(|e| (format!("Error: stat failed ({e})"), 1))?;
        out.push_str(&format!(
            "Size: {} ({} bytes)\n",
            format_size(st.st_size),
            st.st_size
        ));
        out.push_str(&format!("Modified: {}\n", format_time(st.st_mtime)));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// info
// ---------------------------------------------------------------------

/// `tfs info` — plain-archive summary (exact C++ plain-archive output).
///
/// Error parity with the C++ `cmd_info`: a file that cannot be READ at all
/// produces "tpkg i/o error" (the C++ probes the tpkg trailer first and
/// reports that path's error); an existing but unopenable/unformatted file
/// falls through to the mount failure text. tfs never dumps trailers —
/// that is tebako-pkg's info.
pub fn cmd_info(image: &Path) -> Result<String, (String, i32)> {
    if std::fs::File::open(image).is_err() {
        return Err((format!("Error: {}: tpkg i/o error", image.display()), 1));
    }
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;

    let ext = image
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ty = match ext.as_str() {
        "zip" => "ZIP",
        "sqfs" | "squashfs" => "SquashFS",
        // .tfs is the dwarfs-t-native (FlatBuffers metadata) extension
        "dwarfs" | "tfs" => "DwarFS",
        _ => "Unknown",
    };

    let mut file_count = 0i64;
    let mut dir_count = 0i64;
    let mut total_size = 0i64;
    count_recursive("/mnt", &mut file_count, &mut dir_count, &mut total_size);

    Ok(format!(
        "Archive: {}\nType: {ty}\nFiles: {file_count}\nDirectories: {dir_count}\nTotal size: {} ({total_size} bytes)\n",
        image.display(),
        format_size(total_size)
    ))
}

fn count_recursive(path: &str, files: &mut i64, dirs: &mut i64, total: &mut i64) {
    let Ok(entries) = read_dir(path) else {
        return;
    };
    for e in entries {
        if e.is_dir {
            *dirs += 1;
            count_recursive(&format!("{path}/{}", e.name), files, dirs, total);
        } else {
            *files += 1;
            if let Ok(st) = stat_raw(&format!("{path}/{}", e.name)) {
                *total += st.st_size;
            }
        }
    }
}

/// `tfs info --backend-json` (beyond-C++ flag): image-level metadata JSON
/// from the backend (dwarfs via item 24's image_info_json; ENOTSUP
/// elsewhere). This was `--json` before spec 15 made `--json` the full
/// info document; the behavior itself is unchanged.
pub fn cmd_info_json(image: &Path) -> Result<String, (String, i32)> {
    match tfs::image_info_json(&image.to_string_lossy()) {
        Ok(json) => Ok(format!("{json}\n")),
        Err(e) => Err((
            format!(
                "Error: image metadata JSON not available for {} (errno {e})",
                image.display()
            ),
            1,
        )),
    }
}

// ---------------------------------------------------------------------
// info: the spec-15 surface (additive flags, engine in tebako-info)
// ---------------------------------------------------------------------

/// The spec-15 `tfs info` flags (all additive; with none of them the
/// output is the legacy parity summary).
#[derive(Debug, Clone, Default)]
pub struct InfoOptions {
    /// `--manifest`: append the parsed manifest re-serialized as YAML.
    pub manifest: bool,
    /// `--provides`: the kind-specialized PROVIDES section.
    pub provides: bool,
    /// `--requires`: the DEPENDS edges.
    pub requires: bool,
    /// `--platforms`: expand the platform axis.
    pub platforms: bool,
    /// `--json`: everything as one JSON document (`"info_schema": 1`).
    pub json: bool,
    /// `--backend-json`: the backend metadata JSON (standalone, or folded
    /// into the `--json` document).
    pub backend_json: bool,
    /// `--verify`: spec 03 validation with strict exit codes.
    pub verify: bool,
    /// `--require-signed`: unsigned fails the signature check (71).
    pub require_signed: bool,
}

impl InfoOptions {
    /// True when any section flag is set.
    pub fn any_section(&self) -> bool {
        self.manifest || self.provides || self.requires || self.platforms
    }

    /// True when the rich (non-legacy) path handles the call.
    pub fn any_rich(&self) -> bool {
        self.any_section() || self.json || self.verify
    }
}

/// `tfs info <dir>` on a cache entry (spec 15 §4): a cache entry IS
/// artifacts + markers; the info surface reads the single `.tfs` payload
/// inside. Named errors when the entry holds none or several.
fn resolve_cache_entry(dir: &Path) -> Result<std::path::PathBuf, (String, i32)> {
    let mut images = Vec::new();
    let children = std::fs::read_dir(dir).map_err(|e| {
        (
            format!("Error: cannot read directory {}: {e}", dir.display()),
            1,
        )
    })?;
    for child in children.flatten() {
        let path = child.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("tfs") {
            images.push(path);
        }
    }
    match images.len() {
        1 => Ok(images.remove(0)),
        0 => Err((
            format!(
                "Error: {}: no .tfs payload in directory (not a cache entry)",
                dir.display()
            ),
            1,
        )),
        _ => Err((
            format!(
                "Error: {}: several .tfs payloads in directory (name one)",
                dir.display()
            ),
            1,
        )),
    }
}

/// The rich `tfs info` (spec 15 §2). Returns the output and the process
/// exit code (0 normally; the spec-15 §5 codes under `--verify`).
pub fn cmd_info_rich(image: &Path, opts: &InfoOptions) -> Result<(String, i32), (String, i32)> {
    let resolved;
    let image = if image.is_dir() {
        resolved = resolve_cache_entry(image)?;
        resolved.as_path()
    } else {
        image
    };

    if opts.verify {
        let checks = tebako_info::verify::verify_image(image, opts.require_signed)
            .map_err(|e| (format!("Error: {e}"), 1))?;
        let code = tebako_info::verify::exit_code_of(&checks);
        if opts.json {
            let p = tebako_info::payload::inspect_image(image)
                .map_err(|e| (format!("Error: {e}"), 1))?;
            let mut doc = tebako_info::payload::payload_json(&p, opts.backend_json);
            if let tebako_json::Value::Object(members) = &mut doc {
                members.push((
                    "checks".to_string(),
                    tebako_info::verify::checks_json(&checks),
                ));
            }
            return Ok((format!("{}\n", tebako_json::to_string(&doc)), code));
        }
        let mut out = String::new();
        if opts.any_section() {
            let p = tebako_info::payload::inspect_image(image)
                .map_err(|e| (format!("Error: {e}"), 1))?;
            out.push_str(&tebako_info::render::manifest_view(&p, sections_of(opts)));
        }
        out.push_str(&tebako_info::verify::render_report(
            &image.display().to_string(),
            &checks,
        ));
        return Ok((out, code));
    }

    let p = tebako_info::payload::inspect_image(image).map_err(|e| (format!("Error: {e}"), 1))?;
    if let Some(err) = &p.mount_error {
        return Err((format!("Error: {}: {err}", image.display()), 1));
    }
    if opts.json {
        let doc = tebako_info::payload::payload_json(&p, opts.backend_json);
        return Ok((format!("{}\n", tebako_json::to_string(&doc)), 0));
    }
    let mut out = tebako_info::render::manifest_view(&p, sections_of(opts));
    if opts.backend_json {
        if let Some(f) = &p.format {
            if let Some(json) = &f.backend_json {
                out.push_str(&format!("  backend: {json}\n"));
            }
        }
    }
    Ok((out, 0))
}

fn sections_of(opts: &InfoOptions) -> tebako_info::render::Sections {
    tebako_info::render::Sections {
        manifest: opts.manifest,
        provides: opts.provides,
        requires: opts.requires,
        platforms: opts.platforms,
    }
}

// ---------------------------------------------------------------------
// extract
// ---------------------------------------------------------------------

/// `tfs extract` — whole archive (C++ extract_all) or selected paths.
///
/// Returns (stdout, stderr, rc): verbose progress and "Extraction
/// complete" go to stdout; warnings/errors go to stderr (the C++ stream
/// split is part of the contract).
pub fn cmd_extract(image: &Path, files: &[String], opts: &ExtractOptions) -> (String, String, i32) {
    let _guard = match MountGuard::mount(image) {
        Ok(g) => g,
        Err(e) => return (String::new(), format!("Error: {e}\n"), 1),
    };
    let mut out = String::new();
    let mut err = String::new();
    let dest = &opts.dest_dir;

    let success = if files.is_empty() {
        if opts.verbose {
            out.push_str(&format!(
                "Extracting entire archive to: {}\n",
                dest.display()
            ));
        }
        let Ok(cdest) = cstring(&dest.to_string_lossy()) else {
            return (out, "Error: invalid dest\n".into(), 1);
        };
        let rc = unsafe { tebako_fs_extract_all(cdest.as_ptr()) };
        rc == 0
    } else {
        if opts.verbose {
            out.push_str(&format!(
                "Extracting {} item(s) to: {}\n",
                files.len(),
                dest.display()
            ));
        }
        extract_selected(files, dest, opts, &mut out, &mut err)
    };

    if success && !opts.quiet {
        out.push_str("Extraction complete\n");
    }
    let rc = if success { 0 } else { 1 };
    (out, err, rc)
}

fn extract_selected(
    files: &[String],
    dest_base: &Path,
    opts: &ExtractOptions,
    out: &mut String,
    err: &mut String,
) -> bool {
    let mut all_success = true;
    for file in files {
        let full = full_path(file);
        if !exists(&full) {
            // C++ prints this warning to stderr.
            err.push_str(&format!("Warning: Path does not exist: {file}\n"));
            all_success = false;
            continue;
        }
        if is_directory(&full) {
            if opts.verbose {
                out.push_str(&format!("Extracting directory: {file}\n"));
            }
            if extract_directory(&full, &dest_base.join(file)).is_err() {
                all_success = false;
            }
        } else {
            if opts.verbose {
                out.push_str(&format!("Extracting file: {file}\n"));
            }
            if extract_single_file(&full, &dest_base.join(file)).is_err() {
                all_success = false;
            }
        }
    }
    all_success
}

fn extract_directory(src: &str, dest: &Path) -> Result<(), ()> {
    std::fs::create_dir_all(dest).map_err(|_| ())?;
    let entries = read_dir(src).map_err(|_| ())?;
    let mut ok = true;
    for e in entries {
        let src_path = format!("{src}/{}", e.name);
        let dest_path = dest.join(&e.name);
        if e.is_dir {
            ok &= extract_directory(&src_path, &dest_path).is_ok();
        } else {
            ok &= extract_single_file(&src_path, &dest_path).is_ok();
        }
    }
    if ok {
        Ok(())
    } else {
        Err(())
    }
}

fn extract_single_file(src: &str, dest: &Path) -> Result<(), ()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| ())?;
    }
    let csrc = cstring(src).map_err(|_| ())?;
    let fd = unsafe { tebako_fs_open(csrc.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return Err(());
    }
    let mut out = std::fs::File::create(dest).map_err(|_| ())?;
    let mut buf = [0u8; 8192];
    loop {
        let n = unsafe { tebako_fs_read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            unsafe { tebako_fs_close(fd) };
            return Err(());
        }
        if n == 0 {
            break;
        }
        if out.write_all(&buf[..n as usize]).is_err() {
            unsafe { tebako_fs_close(fd) };
            return Err(());
        }
    }
    unsafe { tebako_fs_close(fd) };
    Ok(())
}

// ---------------------------------------------------------------------
// find
// ---------------------------------------------------------------------

/// `tfs find` — glob-match entry names (fnmatch semantics), printing
/// archive-relative paths.
pub fn cmd_find(image: &Path, pattern: &str) -> Result<String, (String, i32)> {
    let _guard = MountGuard::mount(image).map_err(|e| (format!("Error: {e}"), 1))?;
    let mut out = String::new();
    find_recursive("/mnt", pattern, &mut out);
    Ok(out)
}

fn find_recursive(dir: &str, pattern: &str, out: &mut String) {
    let Ok(entries) = read_dir(dir) else {
        return;
    };
    for e in entries {
        let entry_path = format!("{dir}/{}", e.name);
        let display = entry_path.strip_prefix("/mnt").unwrap_or(&entry_path);
        if name_matches(pattern, &e.name) {
            out.push_str(&format!("{display}\n"));
        }
        if e.is_dir {
            find_recursive(&entry_path, pattern, out);
        }
    }
}

/// fnmatch(pattern, name, 0) via libc.
#[cfg(unix)]
fn name_matches(pattern: &str, name: &str) -> bool {
    let Ok(p) = CString::new(pattern) else {
        return false;
    };
    let Ok(n) = CString::new(name) else {
        return false;
    };
    unsafe { libc::fnmatch(p.as_ptr(), n.as_ptr(), 0) == 0 }
}

/// fnmatch(pattern, name, 0) in pure Rust — the MS CRT has no fnmatch.
/// Flag-0 semantics: `*` matches any run INCLUDING '/', `?` any single
/// character, `[...]` character classes (ranges, a leading `!` negates, a
/// `]` first is a literal member), `\` quotes the next character (a
/// trailing `\` is a literal backslash), and an unterminated `[` is a
/// literal '['. The classic recursive matcher — find patterns are small.
/// The unit tests run on every platform, so libc's fnmatch stays the
/// oracle this matcher is held to.
#[cfg(windows)]
fn name_matches(pattern: &str, name: &str) -> bool {
    /// One class character at `*i` (`\x` unescaped); None at the end.
    fn class_char(p: &[char], i: &mut usize) -> Option<char> {
        let c = *p.get(*i)?;
        *i += 1;
        if c == '\\' {
            let e = *p.get(*i)?;
            *i += 1;
            Some(e)
        } else {
            Some(c)
        }
    }

    /// The "[...]" at p[0] against `c`: (matched, consumed pattern
    /// length), or None when the class is unterminated.
    fn class_match(p: &[char], c: char) -> Option<(bool, usize)> {
        let mut i = 1;
        let negated = p.get(i) == Some(&'!');
        if negated {
            i += 1;
        }
        let mut matched = false;
        let mut first = true;
        loop {
            match p.get(i) {
                None => return None,
                Some(&']') if !first => return Some((matched != negated, i + 1)),
                _ => {}
            }
            first = false;
            let lo = class_char(p, &mut i)?;
            // A range: '-' followed by a character other than ']'.
            let hi = if p.get(i) == Some(&'-') && p.get(i + 1).is_some_and(|&h| h != ']') {
                i += 1;
                class_char(p, &mut i)?
            } else {
                lo
            };
            if lo <= c && c <= hi {
                matched = true;
            }
        }
    }

    fn mat(p: &[char], n: &[char]) -> bool {
        let (mut pi, mut ni) = (0, 0);
        while pi < p.len() {
            match p[pi] {
                // A star run: try every rest (a trailing star matches the
                // empty rest too).
                '*' => {
                    while p.get(pi + 1) == Some(&'*') {
                        pi += 1;
                    }
                    return (ni..=n.len()).any(|k| mat(&p[pi + 1..], &n[k..]));
                }
                '?' => {
                    if ni == n.len() {
                        return false;
                    }
                    pi += 1;
                    ni += 1;
                }
                '[' => {
                    if ni == n.len() {
                        return false;
                    }
                    match class_match(&p[pi..], n[ni]) {
                        Some((true, len)) => {
                            pi += len;
                            ni += 1;
                        }
                        Some((false, _)) => return false,
                        // Unterminated class: a literal '['.
                        None if n[ni] == '[' => {
                            pi += 1;
                            ni += 1;
                        }
                        None => return false,
                    }
                }
                // `\x` quotes x.
                '\\' if pi + 1 < p.len() => {
                    if n.get(ni) != Some(&p[pi + 1]) {
                        return false;
                    }
                    pi += 2;
                    ni += 1;
                }
                c => {
                    if n.get(ni) != Some(&c) {
                        return false;
                    }
                    pi += 1;
                    ni += 1;
                }
            }
        }
        ni == n.len()
    }

    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    mat(&p, &n)
}

// ---------------------------------------------------------------------
// mkimage (in-process dwarfs-t Writer — no mkdwarfs binary anywhere)
// ---------------------------------------------------------------------

/// `tfs mkimage --format dwarfs <srcdir> -o <img>` — builds the image
/// in-process via the dwarfs-t Writer (the same C ABI the reader uses;
/// no mkdwarfs binary, no PATH lookup). An existing output is replaced
/// (the mkdwarfs --force parity). Images carry dwarfs-t-native
/// (FlatBuffers) metadata — name them `.tfs` (the `.dwarfs` extension
/// stays for upstream-compatible images).
pub fn cmd_mkimage(format: &str, source_dir: &Path, output: &Path) -> Result<(), (String, i32)> {
    let fmt = format.to_lowercase();
    if fmt == "zip" {
        return Err((
            "mkimage --format zip is not supported: the zip backend is read-only (only 'dwarfs' can be written)"
                .to_string(),
            1,
        ));
    }
    if fmt == "squashfs" {
        return Err((
            "mkimage --format squashfs is not supported (LGPL; opt-in source builds only)"
                .to_string(),
            1,
        ));
    }
    if fmt != "dwarfs" {
        return Err((
            format!("unsupported image format '{format}' (supported: dwarfs)"),
            1,
        ));
    }
    if !source_dir.is_dir() {
        return Err((
            format!("source directory not found: {}", source_dir.display()),
            1,
        ));
    }
    // Stamp the payload manifest's `tree_hash` when the source carries a
    // manifest (spec 03 §7 fixed-point rule: the hash excludes
    // `/__tpkg__/`, so the stamp is a fixed point). The stamped tree is
    // a hardlink staging mirror — the author's source is never mutated.
    let staged = stamp_tree_hash(source_dir)?;
    let staged_tree;
    let source = match &staged {
        Some((tmp, _)) => {
            staged_tree = tmp.path().join("tree");
            staged_tree.as_path()
        }
        None => source_dir,
    };
    // The Writer never overwrites; mkimage keeps the mkdwarfs --force
    // semantics by removing the target first.
    if output.exists() {
        std::fs::remove_file(output)
            .map_err(|e| (format!("cannot replace {}: {e}", output.display()), 1))?;
    }
    let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default())
        .map_err(|e| (format!("dwarfs writer: {e}"), 1))?;
    writer.add_tree(source, "/").map_err(|e| {
        (
            format!("dwarfs writer: scanning {}: {e}", source.display()),
            1,
        )
    })?;
    writer
        .write(output)
        .map_err(|e| (format!("dwarfs writer: {}: {e}", output.display()), 1))?;
    Ok(())
}

// ---------------------------------------------------------------------
// exec (spec 07 §8 tier 1: the preload interposition shim launcher)
// ---------------------------------------------------------------------

/// Options for `tfs exec`.
pub struct ExecOptions {
    /// `image[:mount]` tokens (the leading positional + `--image` repeats;
    /// default mount `/mnt`).
    pub images: Vec<String>,
    /// The `--jail` spec (the spec 08 env form shared with the shim), if
    /// given.
    pub jail: Option<String>,
    /// Command + args, verbatim (everything after `--`).
    pub cmd: Vec<String>,
}

/// The preload env var for this platform.
#[cfg(target_os = "macos")]
pub fn preload_var() -> &'static str {
    "DYLD_INSERT_LIBRARIES"
}

/// The preload env var for this platform.
#[cfg(target_os = "linux")]
pub fn preload_var() -> &'static str {
    "LD_PRELOAD"
}

/// The shim's library file name for this platform.
#[cfg(target_os = "macos")]
fn preload_lib_name() -> &'static str {
    "libtfs_preload.dylib"
}

/// The shim's library file name for this platform.
#[cfg(target_os = "linux")]
fn preload_lib_name() -> &'static str {
    "libtfs_preload.so"
}

/// Engine errno text for messages.
#[cfg(unix)]
fn errno_text(e: i32) -> String {
    String::from_utf8_lossy(tfs::errno::strerror(e)).into_owned()
}

/// Locate the preload shim: `TEBAKO_TFS_PRELOAD` wins, else the sibling
/// of this binary (`libtfs_preload.{dylib,so}` — same artifact directory).
#[cfg(unix)]
fn exec_shim_path() -> Result<PathBuf, (String, i32)> {
    if let Ok(p) = std::env::var("TEBAKO_TFS_PRELOAD") {
        if !p.is_empty() {
            let path = PathBuf::from(&p);
            if path.is_file() {
                return Ok(path);
            }
            return Err((
                format!("Error: TEBAKO_TFS_PRELOAD points at a missing file: {p}"),
                1,
            ));
        }
    }
    let exe = std::env::current_exe()
        .map_err(|e| (format!("Error: cannot locate the tfs binary: {e}"), 1))?;
    let cand = exe
        .parent()
        .map(|d| d.join(preload_lib_name()))
        .unwrap_or_else(|| PathBuf::from(preload_lib_name()));
    if cand.is_file() {
        return Ok(cand);
    }
    Err((
        format!(
            "Error: the preload shim is not available at {} \
             (build it with `cargo build -p libtfs-preload`, or set TEBAKO_TFS_PRELOAD)",
            cand.display()
        ),
        1,
    ))
}

/// `tfs exec` — launch a dynamic native command with the VFS injected
/// through the preload interposition shim (spec 07 §8 tier 1; spec 11 §6
/// access #5). On success this never returns (the process is replaced).
#[cfg(unix)]
pub fn cmd_exec(opts: &ExecOptions) -> Result<(), (String, i32)> {
    use tfs::policy::{HostPolicy, JailSpec};

    if opts.images.is_empty() || opts.cmd.is_empty() {
        return Err((
            "Error: wrong number of arguments\nusage: tfs exec <image>[:mount] [--image <image:mount>]... [--jail <spec>] -- <cmd> [args...]"
                .to_string(),
            1,
        ));
    }
    // Parse + canonicalize the mounts: the exec'd child's cwd is not
    // necessarily ours, so image paths must be absolute in the env.
    let mut decls: Vec<tfs::mount_spec::MountDecl> = Vec::with_capacity(opts.images.len());
    for token in &opts.images {
        let d = tfs::mount_spec::parse_cli_image_mount(token)
            .map_err(|e| (format!("Error: {e}"), 1))?;
        let canon = std::fs::canonicalize(&d.image)
            .map_err(|_| (format!("Error: Image not found: {}", d.image), 1))?;
        if decls.iter().any(|m| m.mount == d.mount) {
            return Err((format!("Error: duplicate mount point: {}", d.mount), 1));
        }
        decls.push(tfs::mount_spec::MountDecl {
            image: canon.to_string_lossy().into_owned(),
            mount: d.mount,
        });
    }
    // Validate the jail NOW (grant paths must exist at bind time —
    // failing here beats dying in the child's constructor) and carry the
    // canonical form into the child. The policy is NOT installed in this
    // process.
    let jail_env = match &opts.jail {
        Some(spec) => {
            let parsed = JailSpec::parse(spec).map_err(|e| (format!("Error: {e}"), 1))?;
            let policy = HostPolicy::bind(parsed.default_open, parsed.mounts, parsed.arg_files)
                .map_err(|e| {
                    (
                        format!("Error: --jail: cannot bind policy: {}", errno_text(e)),
                        1,
                    )
                })?;
            Some(policy.to_env_spec())
        }
        None => None,
    };
    let shim = exec_shim_path()?;
    let cmd0 = materialize_entrypoint(&decls, &opts.cmd[0])?;
    let mounts_env = tfs::mount_spec::to_env_spec(&decls);
    exec_child(
        &shim,
        &mounts_env,
        jail_env.as_deref(),
        &cmd0,
        &opts.cmd[1..],
    )
}

/// `tfs exec` is macOS/linux first (spec 07 §8 tier 1); windows is
/// roadmap 30 phase 2 (DLL injection).
#[cfg(not(unix))]
pub fn cmd_exec(_opts: &ExecOptions) -> Result<(), (String, i32)> {
    Err((
        "Error: tfs exec is not supported on this platform yet \
         (the preload shim targets macOS and linux-gnu first; windows is roadmap 30 phase 2)"
            .to_string(),
        1,
    ))
}

/// Mount the declared images TRANSIENTLY to check whether the command is
/// an in-image path; when it is, materialize it through the engine's
/// dlmap2file host cache (execve needs a host path — this is the spec 07
/// §8 tier-1 entrypoint mechanism; only the ENTRYPOINT is materialized,
/// everything the tool reads stays in the image) and hand that path back.
/// Unmounts before returning — the child's shim re-mounts from the env.
///
/// Note: the entrypoint copy lives in the per-process dl tmpdir, whose
/// removal is registered with atexit — exec bypasses atexit, so one small
/// copy leaks per in-image exec (gc is a later milestone; stated honestly
/// in the spec).
#[cfg(unix)]
fn materialize_entrypoint(
    decls: &[tfs::mount_spec::MountDecl],
    cmd: &str,
) -> Result<String, (String, i32)> {
    use tfs::context::context;

    // A command that is not an absolute path (bare name → PATH search, or
    // relative) is a host command by construction.
    if !cmd.starts_with('/') {
        return Ok(cmd.to_string());
    }
    for d in decls {
        let mount = tfs::mount::build_from_file(&d.image, &d.mount).map_err(|e| {
            (
                format!(
                    "Error: Failed to open archive: {}\n       {}",
                    d.image,
                    errno_text(e)
                ),
                1,
            )
        })?;
        context()
            .write()
            .unwrap()
            .mount_checked(mount)
            .map_err(|e| {
                (
                    format!(
                        "Error: cannot mount {} at {}: {}",
                        d.image,
                        d.mount,
                        errno_text(e)
                    ),
                    1,
                )
            })?;
    }
    let materialized = {
        let mut ctx = context().write().unwrap();
        if ctx.path_is_embedded(cmd) {
            Some(ctx.dlmap2file(cmd).map_err(|e| {
                (
                    format!("Error: cannot materialize {cmd}: {}", errno_text(e)),
                    1,
                )
            })?)
        } else {
            None
        }
    };
    context().write().unwrap().unmount();
    match materialized {
        Some(host) => {
            let path = host.to_string_lossy().into_owned();
            // The kernel needs the exec bit; zip-family backends honestly
            // report 0644, so OR 0111 in explicitly (dlmap2file preserves
            // the image's perms, which may already be fine).
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path)
                .map_err(|e| (format!("Error: cannot stat materialized {path}: {e}"), 1))?
                .permissions()
                .mode();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode | 0o111))
                .map_err(|e| (format!("Error: cannot chmod materialized {path}: {e}"), 1))?;
            Ok(path)
        }
        None => Ok(cmd.to_string()),
    }
}

/// Set the preload env on the CHILD ONLY (scrub any inherited values
/// first, then set exactly the shim contract) and exec with stdio
/// inherited. Grandchildren inherit the env naturally — the process tree
/// stays in the VFS (modulo SIP platform binaries stripping DYLD_*).
#[cfg(unix)]
fn exec_child(
    shim: &Path,
    mounts_env: &str,
    jail_env: Option<&str>,
    cmd: &str,
    args: &[String],
) -> Result<(), (String, i32)> {
    use std::os::unix::process::CommandExt as _;

    let mut command = std::process::Command::new(cmd);
    command
        .args(args)
        .env_remove(preload_var())
        .env_remove("TEBAKO_TFS_MOUNTS")
        .env_remove("TEBAKO_JAIL")
        .env_remove("TEBAKO_JAIL_SOURCE")
        .env(preload_var(), shim)
        .env("TEBAKO_TFS_MOUNTS", mounts_env);
    if let Some(j) = jail_env {
        command.env("TEBAKO_JAIL", j);
        // The audit-journal source label (spec 08 §2): this policy came
        // from the exec surface's --jail flag.
        command.env("TEBAKO_JAIL_SOURCE", "tfs exec --jail");
    }
    let err = command.exec(); // returns only on failure
    Err((format!("Error: cannot exec {cmd}: {err}"), 1))
}

/// When `source_dir` carries `__tpkg__/manifest.yaml`: compute the
/// payload tree hash, fill `identity.digest.tree_hash`, and stage a
/// hardlink mirror with the stamped manifest substituted in. Returns
/// the staging tempdir (its `tree` subdirectory is the image source)
/// and the rendered tree hash. `Ok(None)` for a manifest-less source —
/// plain images stay plain (spec 10's opt-in rule covers the whole
/// trust surface: nothing is ever stamped into existence).
///
/// Stamping is deliberately lenient: a manifest that does not parse or
/// validate goes into the image UNSTAMPED (no staging, the authored
/// bytes verbatim) — mkimage is the stamper, not the validator;
/// `tfs info --verify` grades malformed manifests (exit 65).
fn stamp_tree_hash(
    source_dir: &Path,
) -> Result<Option<(tempfile::TempDir, String)>, (String, i32)> {
    let manifest_path = source_dir
        .join(tpkg::merkle::MANIFEST_DIR)
        .join("manifest.yaml");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let digest = tpkg::tree_digest(&tpkg::merkle_host::HostTree::new(source_dir))
        .map_err(|e| (format!("cannot hash the source tree: {e}"), 1))?;
    let authored = std::fs::read_to_string(&manifest_path)
        .map_err(|e| (format!("cannot read {}: {e}", manifest_path.display()), 1))?;
    let Ok(filled) = tpkg::merkle_host::fill_tree_hash(&authored, &digest) else {
        return Ok(None); // malformed authored manifest: unstamped, verify grades it
    };
    let tmp = tempfile::tempdir().map_err(|e| (format!("cannot create a staging dir: {e}"), 1))?;
    let tree = tmp.path().join("tree");
    tpkg::merkle_host::stage_tree(source_dir, &tree, &filled)
        .map_err(|e| (format!("cannot stage the source tree: {e}"), 1))?;
    Ok(Some((tmp, tpkg::render_tree_hash(&digest))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(images: &[&str], jail: Option<&str>, cmd: &[&str]) -> ExecOptions {
        ExecOptions {
            images: images.iter().map(|s| s.to_string()).collect(),
            jail: jail.map(|s| s.to_string()),
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    #[cfg(unix)]
    fn exec_rejects_bad_arguments_before_exec() {
        // Missing image / command.
        assert!(cmd_exec(&opts(&[], None, &["true"])).is_err());
        assert!(cmd_exec(&opts(&["/x.zip"], None, &[])).is_err());
        // Relative image path (the CLI form defaults the mount to /mnt).
        let (msg, _) = cmd_exec(&opts(&["rel.zip"], None, &["true"])).unwrap_err();
        assert!(msg.contains("not absolute"), "{msg}");
        // A mount at "/" is legitimate (spec 17's app-payload mount —
        // covered-but-not-held falls through with the policy gate), so
        // the missing image dominates exactly like any other.
        let (msg, _) = cmd_exec(&opts(&["/x.zip:/"], None, &["true"])).unwrap_err();
        assert!(msg.contains("Image not found"), "{msg}");
        // A missing image.
        let (msg, _) = cmd_exec(&opts(&["/no/such/image.zip"], None, &["true"])).unwrap_err();
        assert!(msg.contains("Image not found"), "{msg}");
        // Duplicate mount points (images must exist to reach the check).
        let (msg, _) = cmd_exec(&opts(
            &["/etc/hosts:/tfs", "/etc/hosts:/tfs"],
            None,
            &["true"],
        ))
        .unwrap_err();
        assert!(msg.contains("duplicate mount point"), "{msg}");
        // A malformed jail spec (validated after the image resolves).
        let (msg, _) = cmd_exec(&opts(&["/etc/hosts"], Some("frob"), &["true"])).unwrap_err();
        assert!(msg.contains("invalid jail spec"), "{msg}");
        // A jail grant whose host path does not exist (bind-time check).
        let (msg, _) = cmd_exec(&opts(
            &["/etc/hosts"],
            Some("deny;/no/such/dir:/w:ro"),
            &["true"],
        ))
        .unwrap_err();
        assert!(msg.contains("cannot bind policy"), "{msg}");
    }

    #[test]
    #[cfg(all(unix, any(target_os = "macos", target_os = "linux")))]
    fn preload_var_matches_platform() {
        if cfg!(target_os = "macos") {
            assert_eq!(preload_var(), "DYLD_INSERT_LIBRARIES");
            assert_eq!(preload_lib_name(), "libtfs_preload.dylib");
        } else {
            assert_eq!(preload_var(), "LD_PRELOAD");
            assert_eq!(preload_lib_name(), "libtfs_preload.so");
        }
    }

    /// fnmatch flag-0 semantics. Runs on every platform: on unix it
    /// exercises libc::fnmatch (the oracle the windows pure-Rust matcher
    /// is held to), on windows the hand-rolled matcher itself.
    #[test]
    fn name_matches_fnmatch_semantics() {
        // '*' — any run, INCLUDING '/'.
        assert!(name_matches("*.txt", "readme.txt"));
        assert!(name_matches("*", "anything"));
        assert!(name_matches("a*c", "a/b/c"));
        assert!(!name_matches("*.txt", "readme.md"));
        // '?' — exactly one character.
        assert!(name_matches("a?c", "abc"));
        assert!(!name_matches("a?c", "ac"));
        assert!(!name_matches("a?c", "abbc"));
        // '[...]' — members, ranges, leading-'!' negation, literal ']'.
        assert!(name_matches("[abc]at", "bat"));
        assert!(name_matches("[a-z]at", "mat"));
        assert!(!name_matches("[a-z]at", "Mat"));
        assert!(name_matches("[!a-z]at", "9at"));
        assert!(!name_matches("[!abc]x", "bx"));
        assert!(name_matches("[]a]x", "]x"));
        // Backslash quoting.
        assert!(name_matches("\\*", "*"));
        assert!(!name_matches("\\*", "anything"));
        assert!(name_matches("a\\?b", "a?b"));
        // A trailing star matches the empty rest too.
        assert!(name_matches("a*", "a"));
        assert!(name_matches("prefix*", "prefix"));
        assert!(name_matches("a*", "a/b"));
        // (No unterminated-'[' case: the libcs diverge there — BSD
        // refuses the match, glibc and the windows matcher treat '[' as
        // a literal — so the shared oracle cannot pin it.)
    }
}

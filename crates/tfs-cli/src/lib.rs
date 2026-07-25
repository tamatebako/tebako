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

/// stat helper (errno-valued).
fn stat_raw(path: &str) -> Result<libc::stat, i32> {
    let cpath = cstring(path).map_err(|_| libc::EINVAL)?;
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { tebako_fs_stat(cpath.as_ptr(), &mut st) };
    if rc != 0 {
        return Err(unsafe { tebako_get_errno() });
    }
    Ok(st)
}

fn exists(path: &str) -> bool {
    stat_raw(path).is_ok()
}

fn is_file(path: &str) -> bool {
    stat_raw(path).is_ok_and(|st| (st.st_mode & libc::S_IFMT) == libc::S_IFREG as _)
}

fn is_directory(path: &str) -> bool {
    stat_raw(path).is_ok_and(|st| (st.st_mode & libc::S_IFMT) == libc::S_IFDIR as _)
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
        if libc::localtime_r(&t, &mut tm).is_null() {
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
        "dwarfs" => "DwarFS",
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

/// `tfs info --json` (beyond-C++ flag): image-level metadata JSON from the
/// backend (dwarfs via item 24's image_info_json; ENOTSUP elsewhere).
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
fn name_matches(pattern: &str, name: &str) -> bool {
    let Ok(p) = CString::new(pattern) else {
        return false;
    };
    let Ok(n) = CString::new(name) else {
        return false;
    };
    unsafe { libc::fnmatch(p.as_ptr(), n.as_ptr(), 0) == 0 }
}

// ---------------------------------------------------------------------
// mkimage (mkdwarfs shell-out — the writer stays external)
// ---------------------------------------------------------------------

/// Locate mkdwarfs: the TEBAKO_MKDWARFS env var, then PATH.
pub fn find_mkdwarfs(tool: Option<&str>) -> Option<PathBuf> {
    if let Some(t) = tool {
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    if let Ok(env) = std::env::var("TEBAKO_MKDWARFS") {
        if !env.is_empty() {
            return Some(PathBuf::from(env));
        }
    }
    find_on_path("mkdwarfs")
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_env = std::env::var("PATH").ok()?;
    for dir in path_env.split(':') {
        let cand = PathBuf::from(dir).join(name);
        if cand.is_file() && is_executable(&cand) {
            return Some(cand);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = CString::new(p.as_os_str().as_bytes()) else {
        return false;
    };
    unsafe { libc::access(c.as_ptr(), libc::X_OK) == 0 }
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// `tfs mkimage --format dwarfs <srcdir> -o <img>` — shells out to
/// mkdwarfs (the dwarfs WRITER is deliberately not bound; see README).
pub fn cmd_mkimage(
    format: &str,
    source_dir: &Path,
    output: &Path,
    tool: Option<&str>,
) -> Result<(), (String, i32)> {
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
    let Some(exe) = find_mkdwarfs(tool) else {
        return Err((
            "mkdwarfs not found on PATH (install dwarfs or set TEBAKO_MKDWARFS)".to_string(),
            1,
        ));
    };
    if !exe.is_file() {
        return Err((format!("mkdwarfs not found: {}", exe.display()), 1));
    }

    let status = std::process::Command::new(&exe)
        .arg("-i")
        .arg(source_dir)
        .arg("-o")
        .arg(output)
        .arg("--no-progress")
        .arg("--force")
        .status();
    let Ok(status) = status else {
        return Err((format!("mkdwarfs failed to start: {}", exe.display()), 1));
    };
    if !status.success() {
        return Err((
            format!(
                "mkdwarfs failed (exit code {}): \"{}\" -i \"{}\" -o \"{}\" --no-progress --force",
                status.code().unwrap_or(-1),
                exe.display(),
                source_dir.display(),
                output.display()
            ),
            1,
        ));
    }
    if !output.is_file() {
        return Err((
            format!(
                "mkdwarfs did not produce an output file: {}",
                output.display()
            ),
            1,
        ));
    }
    Ok(())
}

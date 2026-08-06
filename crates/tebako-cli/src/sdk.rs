//! Port of the gem's RuntimeSdk (lib/tebako/runtime_sdk.rb): the
//! native-build SDK of a prebuilt tebako runtime.
//!
//! Prebuilt runtime images are stripped for size: no bin/ruby and no ruby
//! headers, so mkmf-driven gem native extension builds cannot run against
//! them directly. The SDK closes the gap from the runtime's own provenance:
//! it fetches the pre-patched ruby source release the runtime was built
//! from (tamatebako/ruby, the same artifact tebako-runtime-ruby consumes)
//! and replays the configure arguments recorded in the runtime's rbconfig
//! (build-machine paths filtered out) to generate the matching header tree,
//! plus a symbol-stub archive re-declaring every ruby symbol the runtime
//! executable exports (mkmf's link probes get true yes/no resolution; the
//! shipped extension never links it). Provisioned once per
//! (ruby version, src release, press platform) into the packaging
//! environment (`<prefix>/deps/sdk/...` — never the runtime cache) and
//! reused afterwards.
//!
//! Image-era runtimes (item 30b): the rbconfig provenance is read from the
//! mounted runtime image in-process (the tfs C ABI — no `--tebako-extract`,
//! no `layout/` tree in the cache). v1-era runtimes keep the gem's
//! extracted-layout flow (golden parity).
//!
//! Deliberate deviations from the gem (documented in the crate README):
//! - the SDK cache key's host tag is the tebako platform id
//!   (`macos-arm64` & co, options::host_platform) — the gem uses the press
//!   host's ruby RbConfig (`darwin24-arm64` & co), which a hostless Rust
//!   CLI cannot reproduce; both are stable per-platform cache keys;
//! - the src tarball is extracted in-process (flate2 + tar, pure Rust)
//!   where the gem shells out to `tar`;
//! - nm/cc/ar stay system tools (a native build is impossible without a C
//!   toolchain — the same reliance the gem and the deploy toolchain
//!   fallback table already carry); `./configure` is the downloaded
//!   artifact's own script, spawned like the runtime itself.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{packaging_error, plain_error, TebakoError};
use crate::fetch::{fetch_bytes, fetch_text, FetchError};
use crate::options::host_platform;

pub const DEFAULT_SRC_RELEASE: &str = "v0.2.1";
pub const DEFAULT_MIRROR: &str = "https://github.com/tamatebako/ruby/releases/download";
const SUMS_FILE: &str = "SHA256SUMS";
const MARKER_FILE: &str = ".sdk-complete";
const LOCK_FILE: &str = ".sdk.lock";
const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Symbols the probe executable must not be offered (gem's
/// STUB_EXCLUDED_CRT): re-declaring them in the stub collides with the
/// probe's own startup/runtime objects.
const STUB_EXCLUDED_CRT: &[&str] = &[
    "main",
    "start",
    "init",
    "fini",
    "end",
    "edata",
    "bss_start",
    "data_start",
    "libc_start_main",
    "libc_csu_init",
    "libc_csu_fini",
    "dso_handle",
    "progname",
    "program_invocation_name",
    "IO_stdin_used",
    "TMC_END",
];
/// Only ruby-ABI symbols go into the stub (gem's STUB_INCLUDED_PREFIXES):
/// third-party symbols the runtime exports (its statically linked OpenSSL
/// & co) would duplicate the system libraries the probes link.
const STUB_INCLUDED_PREFIXES: &[&str] = &["rb_", "ruby_", "onig", "st_", "Init_", "iseq_"];

/// Where the runtime's rbconfig.rb (the configure-args provenance) is
/// read from.
#[derive(Debug, Clone)]
pub enum RbconfigSource {
    /// Image-era runtime: the cached `.tfs`, mounted in-process (no
    /// extraction anywhere).
    Image(PathBuf),
    /// v1-era runtime: the extracted layout next to the cached executable
    /// (the gem's flow).
    Layout(PathBuf),
}

/// The provisioned SDK tree, exposed to the deploy driver's RbConfig
/// overrides.
#[derive(Debug, Clone)]
pub struct SdkPaths {
    pub root: PathBuf,
    pub include: PathBuf,
    pub archhdr: PathBuf,
    pub stub: PathBuf,
}

pub struct RuntimeSdk {
    runtime_path: PathBuf,
    rbconfig_source: RbconfigSource,
    ruby_ver: String,
    src_release: String,
    mirror: String,
    sdk_root: PathBuf,
}

impl RuntimeSdk {
    /// SDK root for the runtime at `runtime_path`; provisions on first use.
    pub fn resolve(
        runtime_path: &Path,
        rbconfig_source: RbconfigSource,
        deps_dir: &Path,
        ruby_ver: &str,
    ) -> Result<SdkPaths, TebakoError> {
        let src_release = std::env::var("TEBAKO_SDK_SRC_RELEASE")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_SRC_RELEASE.to_string());
        let mirror = std::env::var("TEBAKO_SDK_SRC_MIRROR")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_MIRROR.to_string());
        let sdk = RuntimeSdk {
            runtime_path: runtime_path.to_path_buf(),
            rbconfig_source,
            ruby_ver: ruby_ver.to_string(),
            sdk_root: deps_dir
                .join("sdk")
                .join(format!("{ruby_ver}-{src_release}-{}", host_tag()?)),
            src_release,
            mirror: mirror.trim_end_matches('/').to_string(),
        };
        sdk.resolve_locked()
    }

    fn paths(&self) -> SdkPaths {
        SdkPaths {
            root: self.sdk_root.clone(),
            include: self.sdk_root.join("include"),
            archhdr: self.sdk_root.join("archhdr"),
            stub: self.sdk_root.join("lib").join("libruby-stub.a"),
        }
    }

    fn resolve_locked(&self) -> Result<SdkPaths, TebakoError> {
        if self.complete() {
            return Ok(self.paths());
        }
        fs::create_dir_all(&self.sdk_root)
            .map_err(|e| plain_error(format!("{e} creating {}", self.sdk_root.display())))?;
        let lock_path = self.sdk_root.join(LOCK_FILE);
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| plain_error(format!("{e} opening {}", lock_path.display())))?;
        self.acquire_lock(&lock)?;
        let result = if self.complete() {
            Ok(())
        } else {
            self.provision()
        };
        crate::resolve::flock(&lock, crate::resolve::LOCK_UN);
        result?;
        Ok(self.paths())
    }

    fn complete(&self) -> bool {
        self.sdk_root.join(MARKER_FILE).is_file()
            && self.sdk_root.join("include").join("ruby.h").is_file()
            && self
                .sdk_root
                .join("archhdr")
                .join("ruby")
                .join("config.h")
                .is_file()
    }

    fn acquire_lock(&self, lock: &fs::File) -> Result<(), TebakoError> {
        let deadline = std::time::Instant::now() + LOCK_TIMEOUT;
        loop {
            if crate::resolve::flock(lock, crate::resolve::LOCK_EX | crate::resolve::LOCK_NB) {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err(packaging_error(
                    125,
                    Some(&format!("runtime SDK {}", self.sdk_root.display())),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn provision(&self) -> Result<(), TebakoError> {
        self.check_platform_match()?;
        println!("-- Provisioning the runtime SDK (ruby headers for native extension builds)");
        let tmp = self.sdk_root.join(format!("tmp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp)
            .map_err(|e| plain_error(format!("{e} creating {}", tmp.display())))?;
        let result = (|| {
            let tarball = self.download_source(&tmp)?;
            self.configure(&tmp, &tarball)?;
            self.install_headers(&tmp)?;
            self.generate_symbol_stub(&tmp)
        })();
        if result.is_ok() {
            let marker = format!(
                "ruby {} {} {}\n",
                self.ruby_ver,
                self.src_release,
                host_tag()?
            );
            fs::write(self.sdk_root.join(MARKER_FILE), marker)
                .map_err(|e| plain_error(format!("{e} writing the SDK marker")))?;
        }
        let _ = fs::remove_dir_all(&tmp);
        result
    }

    /// The SDK replays the runtime's own configure arguments on the press
    /// host: a runtime built for another platform would generate a header
    /// tree that compiles extensions the runtime cannot load. The runtime
    /// resolver only ever picks the host platform's package, so a mismatch
    /// here means a hand-placed or foreign cache entry — fail loudly.
    fn check_platform_match(&self) -> Result<(), TebakoError> {
        let platform = host_platform()?;
        let name = self
            .runtime_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = name.strip_suffix(".exe").unwrap_or(&name);
        if stem.ends_with(&format!("-{platform}")) {
            Ok(())
        } else {
            Err(packaging_error(
                135,
                Some(&format!(
                    "runtime platform mismatch: {} is not a {platform} runtime",
                    self.runtime_path.display()
                )),
            ))
        }
    }

    // ---- source download (gem's download_source / source_sha256) ---------

    fn release_url(&self) -> String {
        format!("{}/{}", self.mirror, self.src_release)
    }

    fn download_source(&self, tmp: &Path) -> Result<PathBuf, TebakoError> {
        let filename = format!("tfs-ruby-{}-src.tar.gz", self.ruby_ver);
        let url = format!("{}/{filename}", self.release_url());
        let expected = self.source_sha256(&filename)?;
        let body = fetch_bytes(&url).map_err(|e| fetch_error(e, &url))?;
        let got = sha256_hex(&body);
        if got != expected {
            return Err(packaging_error(
                135,
                Some(&format!("{filename}: expected {expected}, got {got}")),
            ));
        }
        let tarball = tmp.join(&filename);
        fs::write(&tarball, &body)
            .map_err(|e| plain_error(format!("{e} writing {}", tarball.display())))?;
        println!("   ... {filename} (SHA256 verified)");
        Ok(tarball)
    }

    fn source_sha256(&self, filename: &str) -> Result<String, TebakoError> {
        let url = format!("{}/{SUMS_FILE}", self.release_url());
        let sums = fetch_text(&url).map_err(|e| fetch_error(e, &url))?;
        for line in sums.lines() {
            let trimmed = line.trim();
            if trimmed.ends_with(&format!(" {filename}"))
                || trimmed.ends_with(&format!(" *{filename}"))
            {
                if let Some(sha) = trimmed.split_whitespace().next() {
                    return Ok(sha.to_ascii_lowercase());
                }
            }
        }
        Err(packaging_error(
            135,
            Some(&format!(
                "{filename} not found in {} {SUMS_FILE}",
                self.src_release
            )),
        ))
    }

    // ---- configure replay (gem's configure / filtered_configure_args) ----

    fn configure(&self, tmp: &Path, tarball: &Path) -> Result<PathBuf, TebakoError> {
        let src_dir = extract_src_tarball(tarball, tmp)?;
        let mut args = self.filtered_configure_args()?;
        args.push(format!("--prefix={}", tmp.join("install").display()));
        let output = std::process::Command::new("./configure")
            .args(&args)
            .current_dir(&src_dir)
            .output()
            .map_err(|e| {
                packaging_error(135, Some(&format!("failed to run ruby configure: {e}")))
            })?;
        if !output.status.success() {
            let out = String::from_utf8_lossy(&output.stdout).into_owned()
                + &String::from_utf8_lossy(&output.stderr);
            let tail: Vec<&str> = out.lines().collect();
            let start = tail.len().saturating_sub(10);
            return Err(packaging_error(
                135,
                Some(&format!(
                    "ruby configure failed:\n{}",
                    tail[start..].join("\n")
                )),
            ));
        }
        Ok(src_dir)
    }

    /// The runtime's own configure arguments, replayed from its rbconfig
    /// with the build machine's paths and compiler assignments filtered out
    /// (the press host supplies those); feature flags
    /// (--with/--without/--disable) are kept verbatim so the generated
    /// config.h matches the runtime. rbconfig normalizes '--with-out-ext'
    /// to '--without-ext', which only the original spelling configures.
    fn filtered_configure_args(&self) -> Result<Vec<String>, TebakoError> {
        Ok(filter_quoted_args(&self.configure_args()?))
    }

    /// `CONFIG["configure_args"]` of the runtime's rbconfig.rb — from the
    /// mounted runtime image (image-era) or the extracted layout (v1).
    fn configure_args(&self) -> Result<String, TebakoError> {
        let (content, origin) = match &self.rbconfig_source {
            RbconfigSource::Image(image) => {
                let content = crate::packager::read_image_rbconfig(image)?.ok_or_else(|| {
                    packaging_error(
                        135,
                        Some(&format!("no rbconfig.rb found in {}", image.display())),
                    )
                })?;
                (content, image.display().to_string())
            }
            RbconfigSource::Layout(layout) => {
                let rbconfig = find_rbconfig_in_layout(layout).ok_or_else(|| {
                    packaging_error(
                        135,
                        Some(&format!("no rbconfig.rb found in {}", layout.display())),
                    )
                })?;
                let content = fs::read_to_string(&rbconfig).map_err(|e| {
                    packaging_error(135, Some(&format!("{e} reading {}", rbconfig.display())))
                })?;
                (content, rbconfig.display().to_string())
            }
        };
        parse_configure_args(&content).ok_or_else(|| {
            packaging_error(
                135,
                Some(&format!("no configure_args recorded in {origin}")),
            )
        })
    }

    // ---- header install (gem's install_headers) ---------------------------

    fn install_headers(&self, tmp: &Path) -> Result<(), TebakoError> {
        let src_dir = find_src_dir(tmp)?;
        let paths = self.paths();
        let _ = fs::remove_dir_all(&paths.include);
        let _ = fs::remove_dir_all(&paths.archhdr);
        copy_dir(&src_dir.join("include"), &paths.include)?;
        let config_h = find_config_h(&src_dir)
            .ok_or_else(|| packaging_error(135, Some("configure produced no ruby/config.h")))?;
        let arch_ruby = paths.archhdr.join("ruby");
        fs::create_dir_all(&arch_ruby)
            .map_err(|e| plain_error(format!("{e} creating {}", arch_ruby.display())))?;
        fs::copy(&config_h, arch_ruby.join("config.h"))
            .map_err(|e| plain_error(format!("{e} copying {}", config_h.display())))?;
        Ok(())
    }

    // ---- symbol stub (gem's generate_symbol_stub) --------------------------

    /// The runtime ships no libruby archive, so mkmf's link probes have
    /// nothing true to resolve against (linking with undefined-symbol
    /// lookup makes every probe succeed, which mis-detects features). The
    /// stub is an archive re-declaring every symbol the runtime executable
    /// exports — the exact, provenance-true symbol table. Only throwaway
    /// probe binaries link it; shipped extensions stay on dynamic lookup
    /// against the executable (which exports these symbols).
    fn generate_symbol_stub(&self, tmp: &Path) -> Result<(), TebakoError> {
        let out = run_quiet("nm", &nm_args(&self.runtime_path), None).map_err(|e| {
            packaging_error(
                135,
                Some(&format!(
                    "nm failed on {}: {e}",
                    self.runtime_path.display()
                )),
            )
        })?;
        let symbols = parse_nm_defined(&out);
        if symbols.is_empty() {
            return Err(packaging_error(
                135,
                Some(&format!(
                    "no exported symbols found in {}",
                    self.runtime_path.display()
                )),
            ));
        }
        let asm = tmp.join("symbols.s");
        fs::write(&asm, symbol_stub_asm(&symbols))
            .map_err(|e| plain_error(format!("{e} writing {}", asm.display())))?;
        let object = tmp.join("symbols.o");
        run_quiet(
            "cc",
            &[
                "-c".to_string(),
                asm.to_string_lossy().into_owned(),
                "-o".to_string(),
                object.to_string_lossy().into_owned(),
            ],
            None,
        )
        .map_err(|e| packaging_error(135, Some(&format!("stub compile failed: {e}"))))?;
        let stub = self.paths().stub;
        if let Some(dir) = stub.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| plain_error(format!("{e} creating {}", dir.display())))?;
        }
        run_quiet(
            "ar",
            &[
                "rcs".to_string(),
                stub.to_string_lossy().into_owned(),
                object.to_string_lossy().into_owned(),
            ],
            None,
        )
        .map_err(|e| packaging_error(135, Some(&format!("ar failed: {e}"))))?;
        Ok(())
    }
}

/// The SDK cache key's host tag (see the module docs for the deviation
/// from the gem's RbConfig-derived tag).
fn host_tag() -> Result<String, TebakoError> {
    host_platform()
}

/// gem's nm_command: `nm -gU` on darwin, `nm -g --defined-only` elsewhere.
fn nm_args(runtime_path: &Path) -> Vec<String> {
    let mut args = if cfg!(target_os = "macos") {
        vec!["-gU".to_string()]
    } else {
        vec!["-g".to_string(), "--defined-only".to_string()]
    };
    args.push(runtime_path.to_string_lossy().into_owned());
    args
}

/// Run a provisioning tool quietly (no "   ... @ ..." announcement — the
/// gem's Open3.capture2e prints nothing, and the golden side-by-side
/// filters only the two SDK lines both sides share); on failure the
/// combined output rides the error (the extconf/configure tail surfaces).
fn run_quiet(program: &str, args: &[String], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("spawn failed: {e}"))?;
    let out = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(out)
    } else {
        Err(out)
    }
}

/// gem's fetch error mapping: downloads are error 122.
fn fetch_error(e: FetchError, url: &str) -> TebakoError {
    match e {
        FetchError::IndexUnavailable(msg) | FetchError::DownloadFailed(msg) => {
            packaging_error(122, Some(&format!("{msg} fetching {url}")))
        }
        e @ FetchError::Throttled { .. } => {
            packaging_error(122, Some(&format!("{e} fetching {url}")))
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The quoted configure arguments of an rbconfig `configure_args` line,
/// filtered for replay (gem's filtered_configure_args).
fn filter_quoted_args(configure_args: &str) -> Vec<String> {
    quoted_fields(configure_args)
        .into_iter()
        .filter(|arg| !arg.starts_with("--prefix=") && !is_assignment(arg))
        .map(|arg| {
            arg.strip_prefix("--without-ext=")
                .map(|rest| format!("--with-out-ext={rest}"))
                .unwrap_or(arg)
        })
        .collect()
}

/// `'([^']*)'` fields, in order (the gem's String#scan).
fn quoted_fields(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\'' {
            let mut field = String::new();
            for c2 in chars.by_ref() {
                if c2 == '\'' {
                    break;
                }
                field.push(c2);
            }
            out.push(field);
        }
    }
    out
}

/// `[A-Z_]+=` (case-insensitive like the gem's /\A[A-Z_]+=/i): the build
/// machine's compiler/flag assignments (CC=..., cflags=..., LDFLAGS=...).
fn is_assignment(arg: &str) -> bool {
    let Some(eq) = arg.find('=') else {
        return false;
    };
    !arg[..eq].is_empty()
        && arg[..eq]
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '_')
}

/// Extract `CONFIG["configure_args"] = "..."` from rbconfig.rb content,
/// undoing the \" and \' escapes (the gem's match + gsub; the line is
/// indented in real rbconfigs and the gem's regexp is unanchored).
fn parse_configure_args(rbconfig_content: &str) -> Option<String> {
    const KEY: &str = "CONFIG[\"configure_args\"] = \"";
    for line in rbconfig_content.lines() {
        if let Some(rest) = line.trim_start().strip_prefix(KEY) {
            let raw = rest.strip_suffix('"').unwrap_or(rest);
            return Some(raw.replace("\\\"", "\"").replace("\\'", "'"));
        }
    }
    None
}

/// The gem's `Dir.glob(File.join(layout, "lib", "ruby", "*", "*", "rbconfig.rb")).first`.
fn find_rbconfig_in_layout(layout: &Path) -> Option<PathBuf> {
    let lib_ruby = layout.join("lib").join("ruby");
    for ver in sorted_dirs(&lib_ruby) {
        for arch in sorted_dirs(&ver) {
            let candidate = arch.join("rbconfig.rb");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Extract `tfs-ruby-*-src.tar.gz` in-process (flate2 + tar — no tar
/// binary anywhere) and return the `tfs-ruby-*-src` root.
fn extract_src_tarball(tarball: &Path, dest: &Path) -> Result<PathBuf, TebakoError> {
    let result = (|| -> Result<(), String> {
        let file = fs::File::open(tarball).map_err(|e| format!("{e}"))?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.set_preserve_permissions(true);
        archive.set_preserve_mtime(true);
        archive.unpack(dest).map_err(|e| format!("{e}"))
    })();
    match result {
        Ok(()) => find_src_dir(dest),
        Err(e) => Err(packaging_error(
            135,
            Some(&format!("failed to extract {}: {e}", tarball.display())),
        )),
    }
}

/// The `tfs-ruby-*-src` directory the src release unpacks to.
fn find_src_dir(dir: &Path) -> Result<PathBuf, TebakoError> {
    let mut candidates: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .map(|n| {
                                let n = n.to_string_lossy();
                                n.starts_with("tfs-ruby-") && n.ends_with("-src")
                            })
                            .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    candidates.sort();
    candidates.into_iter().next().ok_or_else(|| {
        packaging_error(
            135,
            Some(&format!("no tfs-ruby-*-src directory in {}", dir.display())),
        )
    })
}

/// `.ext/include/<arch>/ruby/config.h` under the configured source tree
/// (the gem's Dir.glob(...).first).
fn find_config_h(src_dir: &Path) -> Option<PathBuf> {
    let ext_include = src_dir.join(".ext").join("include");
    for arch in sorted_dirs(&ext_include) {
        let candidate = arch.join("ruby").join("config.h");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// cp -r `src` `dest` (the whole directory, like FileUtils.cp_r of the
/// ruby include tree).
fn copy_dir(src: &Path, dest: &Path) -> Result<(), TebakoError> {
    if !src.is_dir() {
        return Err(packaging_error(
            135,
            Some(&format!("no include tree at {}", src.display())),
        ));
    }
    fs::create_dir_all(dest).map_err(|e| plain_error(format!("{e}")))?;
    for child in fs::read_dir(src).map_err(|e| plain_error(format!("{e}")))? {
        let child = child.map_err(|e| plain_error(format!("{e}")))?;
        let target = dest.join(child.file_name());
        if child.path().is_dir() {
            copy_dir(&child.path(), &target)?;
        } else {
            fs::copy(child.path(), &target).map_err(|e| plain_error(format!("{e}")))?;
        }
    }
    Ok(())
}

/// Defined global symbols from nm output: `<hex> <type letter> <name>`
/// lines (the gem's /^\h+ [A-Za-z] (\S+)$/), deduped, with the stub
/// exclusions applied.
fn parse_nm_defined(output: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let (Some(addr), Some(ty), Some(name)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if !addr.bytes().all(|b| b.is_ascii_hexdigit()) || addr.is_empty() {
            continue;
        }
        if ty.len() != 1 || !ty.bytes().next().is_some_and(|b| b.is_ascii_alphabetic()) {
            continue;
        }
        if !stub_excluded(name) && seen.insert(name.to_string()) {
            out.push(name.to_string());
        }
    }
    out
}

/// gem's stub_excluded?: Mach-O header pseudo-symbols, the CRT boundary
/// symbols, and everything outside the ruby-ABI prefixes.
fn stub_excluded(symbol: &str) -> bool {
    let name = symbol.strip_prefix('_').unwrap_or(symbol);
    if name.starts_with("__mh_") {
        return true;
    }
    if STUB_EXCLUDED_CRT.contains(&name) {
        return true;
    }
    !STUB_INCLUDED_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// The stub translation unit: one returning thunk per symbol (the gem's
/// `.text` + `.globl s` / `s: ret` per symbol).
fn symbol_stub_asm(symbols: &[String]) -> String {
    let mut out = String::from(".text\n");
    for s in symbols {
        out.push_str(&format!(".globl {s}\n{s}: ret\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_args_parsing_matches_the_gem() {
        // Real rbconfigs indent the CONFIG assignments (the gem's regexp
        // is unanchored).
        let rbconfig = "  CONFIG[\"configure_args\"] = \" '--with-openssl-dir=/opt/homebrew/opt/openssl@3' '--without-gmp' '--disable-shared' '--without-ext=dbm,win32,win32ole,-test-/*' '--prefix=/build/machine/o/s' 'cflags=-fPIC -I/build/machine/include' 'LDFLAGS=-L/build/machine/lib' 'LIBS=' 'CC=clang' 'CXX=clang++'\"\n";
        let parsed = parse_configure_args(rbconfig).unwrap();
        let filtered = filter_quoted_args(&parsed);
        assert_eq!(
            filtered,
            vec![
                "--with-openssl-dir=/opt/homebrew/opt/openssl@3",
                "--without-gmp",
                "--disable-shared",
                // '--without-ext' is rbconfig's normalization of the
                // original '--with-out-ext'; only the original configures
                "--with-out-ext=dbm,win32,win32ole,-test-/*",
            ]
        );
    }

    #[test]
    fn configure_args_unescapes_quotes() {
        // \" inside a single-quoted field survives (the gem gsubs before
        // scanning — \' would break the field there identically).
        let rbconfig = "CONFIG[\"configure_args\"] = \" '--x=\\\"y\\\"' '--without-gmp'\"\n";
        let parsed = parse_configure_args(rbconfig).unwrap();
        assert_eq!(parsed, " '--x=\"y\"' '--without-gmp'");
        assert_eq!(
            filter_quoted_args(&parsed),
            vec!["--x=\"y\"", "--without-gmp"]
        );
    }

    #[test]
    fn configure_args_missing_is_none() {
        assert!(parse_configure_args("CONFIG[\"MAJOR\"] = \"3\"\n").is_none());
    }

    #[test]
    fn assignments_are_filtered() {
        assert!(is_assignment("CC=clang"));
        assert!(is_assignment("cflags=-fPIC -I/x"));
        assert!(is_assignment("LDFLAGS=-L/x -ly"));
        assert!(!is_assignment("--with-out-ext=dbm"));
        assert!(!is_assignment("--prefix=/x"));
        assert!(!is_assignment("--without-gmp"));
    }

    #[test]
    fn nm_parsing_keeps_ruby_abi_symbols() {
        let nm = "0000000100001234 T _rb_intern\n\
                  0000000100002345 T _ruby_init\n\
                  0000000100003456 T _onig_new\n\
                  0000000100004567 T _st_init_numtable\n\
                  0000000100005678 T _Init_stringio\n\
                  0000000100006789 T _iseq_compile\n\
                  0000000100007890 T _SSL_new\n\
                  0000000100008901 T __mh_execute_header\n\
                  0000000100009012 T _main\n\
                  0000000100010123 D _progname\n\
                  not-an-address line\n\
                  0000000100011234 U _undef\n";
        let symbols = parse_nm_defined(nm);
        assert_eq!(
            symbols,
            vec![
                "_rb_intern",
                "_ruby_init",
                "_onig_new",
                "_st_init_numtable",
                "_Init_stringio",
                "_iseq_compile",
            ]
        );
    }

    #[test]
    fn nm_parsing_dedupes() {
        let nm = "0000000100001234 T _rb_intern\n0000000100001234 T _rb_intern\n";
        assert_eq!(parse_nm_defined(nm), vec!["_rb_intern"]);
    }

    #[test]
    fn stub_exclusion_rules() {
        assert!(stub_excluded("__mh_execute_header"));
        assert!(stub_excluded("_main"));
        assert!(stub_excluded("start"));
        assert!(stub_excluded("_IO_stdin_used"));
        assert!(stub_excluded("_dso_handle"));
        assert!(stub_excluded("SSL_new"));
        assert!(stub_excluded("rbconfig_foo")); // 'rbconfig' is not 'rb_'
        assert!(!stub_excluded("_rb_define_method"));
        assert!(!stub_excluded("ruby_xmalloc"));
        assert!(!stub_excluded("onig_region_new"));
        assert!(!stub_excluded("st_lookup"));
        assert!(!stub_excluded("Init_foo"));
        assert!(!stub_excluded("iseq_new"));
    }

    #[test]
    fn stub_asm_shape() {
        let asm = symbol_stub_asm(&["_rb_intern".to_string(), "_ruby_init".to_string()]);
        assert_eq!(
            asm,
            ".text\n.globl _rb_intern\n_rb_intern: ret\n.globl _ruby_init\n_ruby_init: ret\n"
        );
    }

    #[test]
    fn nm_args_per_platform() {
        let args = nm_args(Path::new("/tmp/runtime"));
        if cfg!(target_os = "macos") {
            assert_eq!(args, vec!["-gU", "/tmp/runtime"]);
        } else {
            assert_eq!(args, vec!["-g", "--defined-only", "/tmp/runtime"]);
        }
    }

    #[test]
    fn sdk_root_naming() {
        // The gem's <ruby>-<src_release>-<host> cache key shape.
        let deps = Path::new("/tmp/prefix/deps");
        let root = deps.join("sdk").join(format!(
            "3.3.7-{}-{}",
            DEFAULT_SRC_RELEASE,
            host_tag().unwrap()
        ));
        assert!(root.starts_with(deps));
        assert!(root.to_string_lossy().contains("3.3.7-v0.2.1-"));
    }

    #[test]
    fn platform_mismatch_is_named_135() {
        let sdk = RuntimeSdk {
            runtime_path: PathBuf::from("/tmp/cache/tebako-runtime-0.15.9-3.3.7-linux-gnu-x86_64"),
            rbconfig_source: RbconfigSource::Layout(PathBuf::from("/tmp")),
            ruby_ver: "3.3.7".to_string(),
            src_release: DEFAULT_SRC_RELEASE.to_string(),
            mirror: DEFAULT_MIRROR.to_string(),
            sdk_root: PathBuf::from("/tmp/sdk"),
        };
        if host_platform().unwrap() != "linux-gnu-x86_64" {
            let err = sdk.check_platform_match().unwrap_err();
            assert_eq!(err.code, 135);
            assert!(err.message.contains("platform mismatch"), "{err}");
        }
        let matching = RuntimeSdk {
            runtime_path: PathBuf::from(format!(
                "/tmp/cache/tebako-runtime-0.15.9-3.3.7-{}",
                host_platform().unwrap()
            )),
            ..sdk
        };
        assert!(matching.check_platform_match().is_ok());
    }

    #[test]
    fn sums_lookup_accepts_both_marker_spellings() {
        // source_sha256's line rule: "<sha>  <file>" and "<sha> *<file>".
        let lines = [
            "abc123  tfs-ruby-3.3.7-src.tar.gz",
            "def456 *tfs-ruby-3.3.7-src.tar.gz",
        ];
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let filename = "tfs-ruby-3.3.7-src.tar.gz";
            assert!(
                trimmed.ends_with(&format!(" {filename}"))
                    || trimmed.ends_with(&format!(" *{filename}")),
                "line {i} must match"
            );
        }
    }

    #[test]
    fn find_rbconfig_in_layout_glob() {
        let dir = std::env::temp_dir().join(format!("tebako-sdk-test-{}", std::process::id()));
        let arch = dir.join("lib/ruby/3.3.0/arm64-darwin24");
        fs::create_dir_all(&arch).unwrap();
        fs::write(arch.join("rbconfig.rb"), "CONFIG[\"MAJOR\"] = \"3\"\n").unwrap();
        assert_eq!(
            find_rbconfig_in_layout(&dir),
            Some(dir.join("lib/ruby/3.3.0/arm64-darwin24/rbconfig.rb"))
        );
        let _ = fs::remove_dir_all(&dir);
        assert!(find_rbconfig_in_layout(&dir).is_none());
    }
}

//! Acquisition (spec 27 §1/§3/§5): everything the run engine needs before
//! the matrix starts — the downloaded product tools, the v1 packed-mn
//! executable, the v2-managed store contents, the v2-press fat package, and
//! the workload source documents — plus the per-mode cache wiping.
//!
//! **The dogfood rule (task-level decision, recorded here and in the
//! commit):** anything the PRODUCT already does with its own verification
//! rides the product — `tebako add-registry` / `tebako install` populate
//! the bench home's store by SPAWNING the downloaded CLI (in-process
//! re-implementation would duplicate the spec 05 store + registry-pin
//! verification logic), and the runtime download rides the shim/bootstrap
//! dispatch (release-index manifest.json / SHA256SUMS.txt verification is
//! the product's own download path, tebako-shim/src/runtime.rs). The
//! harness itself verifies exactly the bytes IT downloads: the tebako
//! release assets against the release `SHA256SUMS`, and the packed-mn
//! asset against its bare-hash `.sha256.txt` sidecar. The fat package's
//! runtime slot is the store's verified exe, pinned byte-for-byte by the
//! trailer's `;sha256=` (re-verified by the bootstrap at every run).
//!
//! **No shell-outs to platform tools** (spec 27 §0): downloads are
//! tebako-http, archives are flate2/tar in-process. The ONE exception is
//! macOS ad-hoc re-signing of the assembled fat package — `codesign` has
//! no in-process form (it is a Mach-O load-command rewrite) and the
//! product's own press does exactly the same, best-effort and loud on
//! failure (tebako-cli::resign_if_needed).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::Digest;

use crate::error::BenchError;
use crate::platforms::PlatformFile;
use crate::result::ImageFormat;
use crate::suite::{SourceKind, Target, TargetKind, Workload};

/// The triplet spellings are the release vocabulary (spec 27 §3) — the
/// Windows legs carry the `.exe` suffix in their release asset names.
pub fn exe_suffix(triplet: &str) -> &'static str {
    if triplet.starts_with("windows") {
        ".exe"
    } else {
        ""
    }
}

/// Canonicalize a path that will be handed to a spawned child. On
/// Windows `canonicalize` yields verbatim `\\?\C:\…` paths, and several
/// child toolchains reject the prefix (java's `-cp` parser produced
/// ClassNotFoundException for every classpath workload on the windows
/// leg) — so the prefix is simplified away when the path is a plain
/// drive-letter (or UNC) form. Every path the harness puts into a
/// child's argv/cwd/env flows through here, never a raw canonicalize.
pub fn canonicalize_for_children(path: &Path) -> Result<PathBuf, BenchError> {
    let c = path.canonicalize().map_err(|e| {
        BenchError::operational(format!("acquire: cannot resolve {}: {e}", path.display()))
    })?;
    Ok(simplify_verbatim(c))
}

/// `\\?\C:\…` → `C:\…`, `\\?\UNC\server\share\…` → `\\server\share\…`.
/// Identity elsewhere (and for the exotic verbatim forms there is no
/// plain spelling for — the volume-guid form stays verbatim).
fn simplify_verbatim(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            let b = rest.as_bytes();
            if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
                return PathBuf::from(rest);
            }
        }
    }
    p
}

/// The `--out` directory's internal layout (spec 27 §5's hermetic bench
/// home lives here — a cold run never touches the host's real caches).
#[derive(Debug, Clone)]
pub struct BenchLayout {
    /// `<out>` itself.
    pub root: PathBuf,
    /// `<out>/bin` — the downloaded product tools under their BARE names
    /// (the CLI locates the dispatcher next to its own current_exe).
    pub bin: PathBuf,
    /// `<out>/assets` — raw downloads (kept for audit).
    pub assets: PathBuf,
    /// `<out>/home` — the hermetic bench home (the child's HOME; its
    /// `.tebako` is the v2 store; `.metanorma`/`.relaton` the payload
    /// caches).
    pub home: PathBuf,
    /// `<out>/tmp/<target>` — the per-target TMPDIR (the v1 stack's
    /// extraction root is its TMPDIR, spec 27 §9 spike c).
    pub tmp: PathBuf,
    /// `<out>/sources/<workload>` — materialized workload source trees.
    pub sources: PathBuf,
    /// `<out>/targets/<target>` — the acquired executables (v1 exe, fat
    /// package).
    pub targets: PathBuf,
    /// `<out>/scratch/<workload>/<target>/<mode>-<iteration>` — per-run
    /// scratch (the child's cwd; `{doc}` and expectations live here).
    pub scratch: PathBuf,
    /// `<out>/logs` — one log per run plus the acquisition logs.
    pub logs: PathBuf,
}

impl BenchLayout {
    pub fn new(out: &Path) -> Result<Self, BenchError> {
        // The layout is ALWAYS absolute: the measured child's cwd is its
        // scratch cell and admin spawns run in the bench home, so a
        // relative --out would spawn ENOENT every staged program (the
        // 2026-08-31 run's 0-measured-cells bug — "cannot spawn
        // out/bin/tebako" from inside the scratch/home cwd). The path
        // spelling is child-safe (no windows verbatim prefix).
        std::fs::create_dir_all(out).map_err(|e| {
            BenchError::operational(format!("acquire: cannot create {}: {e}", out.display()))
        })?;
        let out = canonicalize_for_children(out)?;
        let layout = BenchLayout {
            root: out.to_path_buf(),
            bin: out.join("bin"),
            assets: out.join("assets"),
            home: out.join("home"),
            tmp: out.join("tmp"),
            sources: out.join("sources"),
            targets: out.join("targets"),
            scratch: out.join("scratch"),
            logs: out.join("logs"),
        };
        for dir in [
            &layout.root,
            &layout.bin,
            &layout.assets,
            &layout.home,
            &layout.tmp,
            &layout.sources,
            &layout.targets,
            &layout.scratch,
            &layout.logs,
        ] {
            std::fs::create_dir_all(dir).map_err(|e| {
                BenchError::operational(format!("acquire: cannot create {}: {e}", dir.display()))
            })?;
        }
        Ok(layout)
    }

    /// The bench home's store root (`<out>/home/.tebako`, TEBAKO_HOME).
    pub fn store(&self) -> PathBuf {
        self.home.join(".tebako")
    }

    /// The hermetic environment every spawned child rides (spec 27 §5):
    /// HOME + TEBAKO_HOME under `<out>/home`, TMPDIR per target. On
    /// Windows the USERPROFILE/TEMP/TMP equivalents come along (the leg's
    /// platform IS the host — `cfg!(windows)` is the triplet's truth).
    pub fn child_env(&self, target: &str) -> Vec<(String, String)> {
        let tmp = self.tmp.join(target);
        let mut env = vec![
            ("HOME".to_string(), self.home.to_string_lossy().into_owned()),
            (
                "TEBAKO_HOME".to_string(),
                self.store().to_string_lossy().into_owned(),
            ),
            ("TMPDIR".to_string(), tmp.to_string_lossy().into_owned()),
        ];
        if cfg!(windows) {
            env.push((
                "USERPROFILE".to_string(),
                self.home.to_string_lossy().into_owned(),
            ));
            env.push(("TEMP".to_string(), tmp.to_string_lossy().into_owned()));
            env.push(("TMP".to_string(), tmp.to_string_lossy().into_owned()));
        }
        env
    }

    /// The spec 27 §5 cold-run wipe for one target: the payload caches
    /// (`~/.metanorma`, `~/.relaton` — both stacks, parity holds) always;
    /// then per arm:
    ///
    /// - v1-exe: the per-target TMPDIR (the v1 memfs extraction root) —
    ///   first-boot means re-extraction inside the measured span.
    /// - v2-managed: the whole store — the payload re-installs
    ///   (UNMEASURED, spec 27 §5's cold flow) and the runtime download
    ///   lands inside the measured span. The unmeasured re-install also
    ///   restores any spawned-dependency runtimes the payload declared.
    /// - v2-press: the fat package's OWN runtime entry
    ///   (`runtimes/<entry>`, named by `runtime_dir`) — the package
    ///   carries the runtime EXE but NOT the env image (§9 spike a), so
    ///   the env-image download lands inside the measured span.
    ///   Spawned-dependency runtime entries (a payload spawning a second
    ///   interpreter, e.g. metanorma's jing validation spawning java)
    ///   stay: a spawn never downloads, so wiping them would make the
    ///   cold run un-runnable by construction.
    /// - runtime-exe (spec 27 §10.3): the whole store + the per-target
    ///   TMPDIR — the driver's/exec-cache's first-boot state. The runtime
    ///   pair itself stays staged (its download+verify is acquisition in
    ///   this suite; the measured span is the first mount).
    /// - runtime-exe-lazy (spec 27 §10.5): the same, plus the entry
    ///   RESEEDS — the sealed image and block cache go, the pristine
    ///   seed descriptor returns, so every cold cell re-measures the
    ///   on-demand stream from an empty entry.
    /// - on-system never reaches here: its cold cell is a declared gap.
    pub fn wipe_cold_caches(
        &self,
        target: &str,
        kind: TargetKind,
        runtime_dir: Option<&Path>,
    ) -> Result<(), BenchError> {
        let mut wipes = vec![self.home.join(".metanorma"), self.home.join(".relaton")];
        match kind {
            TargetKind::V1Exe => wipes.push(self.tmp.join(target)),
            TargetKind::V2Managed => wipes.push(self.store()),
            TargetKind::V2Press => wipes.push(
                runtime_dir
                    .ok_or_else(|| {
                        BenchError::operational(format!(
                            "acquire: the v2-press cold wipe for '{target}' needs the package's runtime entry dir (harness bug)"
                        ))
                    })?
                    .to_path_buf(),
            ),
            TargetKind::RuntimeExe => {
                wipes.push(self.store());
                wipes.push(self.tmp.join(target));
            }
            TargetKind::RuntimeExeLazy => {
                wipes.push(self.store());
                wipes.push(self.tmp.join(target));
            }
            TargetKind::OnSystem => {}
        }
        for dir in &wipes {
            remove_tree(dir)?;
        }
        if kind == TargetKind::RuntimeExeLazy {
            // The lazy arm's wipe RESEEDS the entry (spec 27 §10.5):
            // the sealed image + block cache go, the pristine seed
            // descriptor returns — the next cold cell re-measures the
            // on-demand stream from an empty entry.
            reseed_lazy_entry(
                runtime_dir
                    .ok_or_else(|| {
                        BenchError::operational(format!(
                            "acquire: the runtime-exe-lazy cold wipe for '{target}' needs the entry dir (harness bug)"
                        ))
                    })?,
            )?;
        }
        // The wiped TMPDIR must exist again for the next child.
        std::fs::create_dir_all(self.tmp.join(target)).map_err(|e| {
            BenchError::operational(format!(
                "acquire: cannot recreate {}: {e}",
                self.tmp.join(target).display()
            ))
        })?;
        Ok(())
    }
}

fn remove_tree(dir: &Path) -> Result<(), BenchError> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(BenchError::operational(format!(
            "acquire: cannot wipe {}: {e}",
            dir.display()
        ))),
    }
}

// ---------------------------------------------------------------------
// downloads + sha256 verification (the bytes the HARNESS downloads)
// ---------------------------------------------------------------------

/// sha256 of a file's bytes, lowercase hex.
pub fn sha256_file_hex(path: &Path) -> Result<String, BenchError> {
    let mut f = std::fs::File::open(path).map_err(|e| {
        BenchError::operational(format!("acquire: cannot open {}: {e}", path.display()))
    })?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut f, &mut hasher).map_err(|e| {
        BenchError::operational(format!("acquire: cannot read {}: {e}", path.display()))
    })?;
    Ok(hex_lower(&hasher.finalize()))
}

pub fn sha256_bytes_hex(bytes: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hex_lower(&hasher.finalize())
}

fn hex_lower(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// One `SHA256SUMS` line: `<64-hex><space(s)>[*]<name>` (the coreutils
/// format). Returns the lowercase digest for `asset`, if listed.
pub fn parse_sha256sums(text: &str, asset: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hex = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        if name == asset && is_hex64(hex) {
            Some(hex.to_lowercase())
        } else {
            None
        }
    })
}

/// The packed-mn `.sha256.txt` sidecar is a BARE hash (spec 27 §9 spike
/// c): one 64-hex token, optionally with trailing whitespace/filename.
pub fn parse_bare_hash(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    is_hex64(token).then(|| token.to_lowercase())
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// GET `url` (tebako-http — in-process, HTTPS-only) → BenchError with the
/// URL named.
fn get(url: &str) -> Result<Vec<u8>, BenchError> {
    tebako_http::get(url)
        .map_err(|e| BenchError::operational(format!("acquire: download failed for {url}: {e}")))
}

/// Download `url` to `dest` (tmp + rename — a partial download is
/// invisible), verifying against the expected lowercase hex digest.
pub fn download_verified(url: &str, dest: &Path, expected_sha256: &str) -> Result<(), BenchError> {
    let bytes = get(url)?;
    let actual = sha256_bytes_hex(&bytes);
    if actual != expected_sha256.to_lowercase() {
        return Err(BenchError::operational(format!(
            "acquire: SHA256 mismatch for {url}\n  expected: {}\n  actual:   {actual}\n  the download was NOT written (the trust anchor is the checksum)",
            expected_sha256.to_lowercase()
        )));
    }
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(|e| {
        BenchError::operational(format!("acquire: cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, dest).map_err(|e| {
        BenchError::operational(format!("acquire: cannot publish {}: {e}", dest.display()))
    })?;
    Ok(())
}

// ---------------------------------------------------------------------
// the product tools (tebako release assets, SHA256SUMS-verified)
// ---------------------------------------------------------------------

/// The downloaded product trio under their bare names, plus the resolved
/// tebako version (the binary's own report — never the requested tag).
pub struct TebakoTools {
    pub cli: PathBuf,
    pub shim: PathBuf,
    pub bootstrap: PathBuf,
    /// The RESOLVED tebako version (e.g. "0.2.5"), from the binary itself.
    pub version: String,
}

/// The tools the harness drives: `tebako` (add-registry/install), its
/// sibling dispatcher `tebako-shim`, and `tebako-bootstrap` (the fat
/// package's part A). `release`: a tag ("v0.2.5") or None for the latest
/// release (`releases/latest/download/...` — the version is learned from
/// the release's own SHA256SUMS, never guessed).
pub fn fetch_tebako_tools(
    layout: &BenchLayout,
    release: Option<&str>,
    triplet: &str,
) -> Result<TebakoTools, BenchError> {
    let base = match release {
        Some(tag) => format!("https://github.com/tamatebako/tebako/releases/download/{tag}"),
        None => "https://github.com/tamatebako/tebako/releases/latest/download".to_string(),
    };
    let sums_url = format!("{base}/SHA256SUMS");
    let sums_bytes = get(&sums_url)?;
    let sums = String::from_utf8(sums_bytes)
        .map_err(|e| BenchError::operational(format!("acquire: {sums_url} is not UTF-8: {e}")))?;

    let suffix = exe_suffix(triplet);
    let version = match release {
        Some(tag) => tag.strip_prefix('v').unwrap_or(tag).to_string(),
        None => version_from_sums(&sums, triplet).ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: {sums_url} lists no tebako-<ver>-{triplet}{suffix} asset — cannot learn the release version"
            ))
        })?,
    };

    let mut tools = TebakoTools {
        cli: PathBuf::new(),
        shim: PathBuf::new(),
        bootstrap: PathBuf::new(),
        version: version.clone(),
    };
    for (tool, slot) in [
        ("tebako", &mut tools.cli),
        ("tebako-shim", &mut tools.shim),
        ("tebako-bootstrap", &mut tools.bootstrap),
    ] {
        let asset = format!("{tool}-{version}-{triplet}{suffix}");
        let expected = parse_sha256sums(&sums, &asset).ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: {asset} is not in {sums_url} (typo in the triplet or the release is incomplete)"
            ))
        })?;
        let dest = layout.assets.join(&asset);
        download_verified(&format!("{base}/{asset}"), &dest, &expected)?;
        let bare = layout.bin.join(format!("{tool}{suffix}"));
        std::fs::copy(&dest, &bare).map_err(|e| {
            BenchError::operational(format!(
                "acquire: cannot stage {} as {}: {e}",
                dest.display(),
                bare.display()
            ))
        })?;
        chmod_0755(&bare)?;
        *slot = bare;
    }

    // The resolved version is the binary's own report (spec 27 §6:
    // resolved, never requested). This doubles as an exec smoke test of
    // the downloaded CLI on this triplet.
    tools.version = tebako_cli_version(&tools.cli).unwrap_or(version);
    Ok(tools)
}

/// The `tebako-<ver>-<triplet>` line in a SHA256SUMS reveals the version
/// of a `latest` download: the middle must be a bare version (digits and
/// dots — "shim-0.2.5" / "bootstrap-0.2.5" / "pkg-0.2.5" are rejected by
/// construction).
///
/// The scan itself stays out of the public surface; the unit tests pin
/// its grammar through this wrapper.
#[doc(hidden)]
pub fn testonly_version_from_sums(sums: &str, triplet: &str) -> Option<String> {
    version_from_sums(sums, triplet)
}

fn version_from_sums(sums: &str, triplet: &str) -> Option<String> {
    let suffix = format!("-{triplet}{}", exe_suffix(triplet));
    sums.lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .find_map(|name| {
            let name = name.trim_start_matches('*');
            let rest = name.strip_prefix("tebako-")?.strip_suffix(&suffix)?;
            if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                Some(rest.to_string())
            } else {
                None
            }
        })
}

/// `tebako --version` → "Tebako executable packager version 0.2.5" → the
/// trailing version token. None on any surprise (the caller keeps the
/// SHA256SUMS-learned version).
fn tebako_cli_version(cli: &Path) -> Option<String> {
    let out = std::process::Command::new(cli)
        .arg("--version")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let token = text.split_whitespace().last()?;
    if token.bytes().next()?.is_ascii_digit() {
        Some(token.to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------
// the v1 arm: the packed-mn release executable
// ---------------------------------------------------------------------

/// Download + verify + (for POSIX) extract the packed-mn asset named by
/// the platforms document. Returns the runnable executable's path.
pub fn acquire_v1_exe(
    layout: &BenchLayout,
    platforms: &PlatformFile,
    triplet: &str,
) -> Result<PathBuf, BenchError> {
    let entry = platforms.triplets.get(triplet).ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: platforms.yaml has no triplet '{triplet}'"
        ))
    })?;
    let asset = entry.v1_asset.as_deref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: v1-exe on {triplet} is a named gap (v1_asset: null) — the caller must not attempt acquisition"
        ))
    })?;
    let (repo, tag) = (&platforms.packed_mn.repo, &platforms.packed_mn.tag);
    let base = format!("https://github.com/{repo}/releases/download/{tag}");
    let sidecar = get(&format!("{base}/{asset}.sha256.txt"))?;
    let expected = parse_bare_hash(&String::from_utf8_lossy(&sidecar)).ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: {base}/{asset}.sha256.txt is not a bare 64-hex sha256"
        ))
    })?;
    let dest = layout.assets.join(asset);
    download_verified(&format!("{base}/{asset}"), &dest, &expected)?;

    let target_dir = layout.targets.join("v1-packed-mn");
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot create {}: {e}",
            target_dir.display()
        ))
    })?;
    if asset.ends_with(".tgz") {
        // The single-member rule (spec 27 §3/§9 spike c): decompress,
        // take the ONE member, mark it executable.
        let bytes = std::fs::read(&dest).map_err(|e| {
            BenchError::operational(format!("acquire: cannot read {}: {e}", dest.display()))
        })?;
        let exe = extract_single_member_tgz(&bytes, &target_dir)?;
        chmod_0755(&exe)?;
        Ok(exe)
    } else if asset.ends_with(".exe") {
        let exe = target_dir.join(asset);
        std::fs::copy(&dest, &exe).map_err(|e| {
            BenchError::operational(format!("acquire: cannot stage {}: {e}", exe.display()))
        })?;
        chmod_0755(&exe)?;
        Ok(exe)
    } else {
        Err(BenchError::operational(format!(
            "acquire: unsupported packed-mn asset form '{asset}' (.tgz single-member or .exe expected)"
        )))
    }
}

/// The packed-mn POSIX layout (spec 27 §9 spike c): a gzipped tar with
/// exactly ONE regular-file member (the executable). Anything else —
/// zero members, several members, a path that would escape `dest_dir` —
/// is a named error, never a guess.
pub fn extract_single_member_tgz(bytes: &[u8], dest_dir: &Path) -> Result<PathBuf, BenchError> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    let mut member: Option<(String, Vec<u8>)> = None;
    let entries = archive
        .entries()
        .map_err(|e| BenchError::operational(format!("acquire: tgz read failed: {e}")))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|e| BenchError::operational(format!("acquire: tgz entry: {e}")))?;
        if !entry.header().entry_type().is_file() {
            continue; // pax headers, dirs — not the payload
        }
        let name = entry
            .path()
            .map_err(|e| BenchError::operational(format!("acquire: tgz entry path: {e}")))?
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or_else(|| {
                BenchError::operational("acquire: tgz member has no file name".to_string())
            })?;
        if member.is_some() {
            return Err(BenchError::operational(format!(
                "acquire: the packed-mn tgz carries more than one file member (at least '{name}' and one other) — the single-member rule is violated"
            )));
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| BenchError::operational(format!("acquire: tgz member read: {e}")))?;
        member = Some((name, buf));
    }
    let (name, buf) = member.ok_or_else(|| {
        BenchError::operational(
            "acquire: the packed-mn tgz carries NO file member — single-member rule violated"
                .to_string(),
        )
    })?;
    let dest = dest_dir.join(&name);
    std::fs::write(&dest, &buf).map_err(|e| {
        BenchError::operational(format!("acquire: cannot write {}: {e}", dest.display()))
    })?;
    Ok(dest)
}

// ---------------------------------------------------------------------
// the v2 arms: store population via the downloaded CLI (the dogfood path)
// ---------------------------------------------------------------------

/// The payload records read back from the bench home's store after
/// `tebako install` (the store is the SSOT — the harness never parses the
/// image itself).
pub struct PayloadHome {
    pub name: String,
    pub version: String,
    /// The resolved feedstock release tag (e.g. "1.16.9-3"), from the
    /// store's registry cache (the L3 mirror the resolution used).
    pub release_tag: String,
    /// The store's payload image (byte-identical with the registry
    /// artifact — the CLI sha256-verified it at install).
    pub image: PathBuf,
    /// The dispatcher-visible manifest mirror (the embedded manifest,
    /// authoritative).
    pub mirror: tpkg::PayloadManifest,
    /// The image backend (sniffed from the payload's magic bytes — the
    /// tebako-pkg sniff rule) for `versions.image_format`.
    pub image_format: ImageFormat,
}

/// `tebako add-registry <ref>...` + `tebako install <name@version>` in
/// the bench home, then read the store records back. Payload bytes are
/// sha256-verified by the PRODUCT against the registry pin (spec 05) —
/// that verification is deliberately not re-implemented here.
pub fn install_payload(
    layout: &BenchLayout,
    tools: &TebakoTools,
    target: &Target,
) -> Result<PayloadHome, BenchError> {
    let payload = target.payload.as_deref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: v2 target '{}' carries no payload reference",
            target.id
        ))
    })?;
    let (name, version) = payload.split_once('@').ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: payload reference '{payload}' is not name@version"
        ))
    })?;
    let registries = target.registries.as_deref().unwrap_or(&[]);
    if registries.is_empty() {
        return Err(BenchError::operational(format!(
            "acquire: v2 target '{}' carries no registries",
            target.id
        )));
    }
    for r in registries {
        run_admin(
            layout,
            &tools.cli,
            &["add-registry", r],
            "acquire-add-registry.log",
        )?;
    }
    run_admin(
        layout,
        &tools.cli,
        &["install", payload],
        "acquire-install.log",
    )?;

    let record_dir = layout.store().join("payloads").join(name);
    let image = record_dir.join(format!("{version}.tfs"));
    if !image.is_file() {
        return Err(BenchError::operational(format!(
            "acquire: `tebako install {payload}` left no {} in the store — the install log names why",
            image.display()
        )));
    }
    if !record_dir.join(format!("{version}.tfs.sha256")).is_file() {
        return Err(BenchError::operational(format!(
            "acquire: the store's {} has no .sha256 trust anchor — the payload record is incomplete",
            image.display()
        )));
    }
    let mirror_path = record_dir.join(format!("{version}.manifest.yaml"));
    let mirror_text = std::fs::read_to_string(&mirror_path).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot read the manifest mirror {}: {e}",
            mirror_path.display()
        ))
    })?;
    let mirror = tpkg::PayloadManifest::from_yaml(&mirror_text).map_err(|e| {
        BenchError::operational(format!(
            "acquire: the manifest mirror {} does not parse: {e}",
            mirror_path.display()
        ))
    })?;
    let release_tag = registry_release_tag(layout, name, version)?;
    let image_format = sniff_image_format(&image)?;
    Ok(PayloadHome {
        name: name.to_string(),
        version: version.to_string(),
        release_tag,
        image,
        mirror,
        image_format,
    })
}

/// The feedstock release tag the installed payload came from, read from
/// the store's registry cache (`registries/*.yaml` — the verbatim fetched
/// L3 mirror): `release.ref`'s trailing `:tag`.
fn registry_release_tag(
    layout: &BenchLayout,
    name: &str,
    version: &str,
) -> Result<String, BenchError> {
    #[derive(serde::Deserialize)]
    struct Registry {
        payloads: Vec<RegPayload>,
    }
    #[derive(serde::Deserialize)]
    struct RegPayload {
        name: String,
        versions: Vec<RegVersion>,
    }
    #[derive(serde::Deserialize)]
    struct RegVersion {
        version: serde_yml::Value,
        release: Option<RegRelease>,
    }
    #[derive(serde::Deserialize)]
    struct RegRelease {
        #[serde(rename = "ref")]
        reference: String,
    }

    let dir = layout.store().join("registries");
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot list the registry cache {}: {e}",
            dir.display()
        ))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let Ok(registry) = serde_yml::from_str::<Registry>(&text) else {
            continue;
        };
        for payload in &registry.payloads {
            if payload.name != name {
                continue;
            }
            for v in &payload.versions {
                let vtext = match &v.version {
                    serde_yml::Value::String(s) => s.clone(),
                    other => format!("{other:?}").trim_matches('"').to_string(),
                };
                if vtext == version {
                    if let Some(release) = &v.release {
                        if let Some(tag) = release.reference.rsplit(':').next() {
                            return Ok(tag.to_string());
                        }
                    }
                }
            }
        }
    }
    Err(BenchError::operational(format!(
        "acquire: no registry in the store cache names {name} {version} with a release ref — cannot record versions.payload (resolved, never requested)"
    )))
}

/// The image backend from the payload's magic bytes (the tebako-pkg
/// `sniff_format` rule — the bench reads bytes, never links a backend).
fn sniff_image_format(image: &Path) -> Result<ImageFormat, BenchError> {
    let mut f = std::fs::File::open(image).map_err(|e| {
        BenchError::operational(format!("acquire: cannot open {}: {e}", image.display()))
    })?;
    let mut magic = [0u8; 8];
    let n = f.read(&mut magic).map_err(|e| {
        BenchError::operational(format!("acquire: cannot read {}: {e}", image.display()))
    })?;
    let magic = &magic[..n];
    if magic.starts_with(b"DWARFS") {
        Ok(ImageFormat::Dwarfs)
    } else if magic.starts_with(b"LMFS") {
        Ok(ImageFormat::Limnifs)
    } else {
        Err(BenchError::operational(format!(
            "acquire: {} is neither dwarfs- nor limnifs-format (versions.image_format has no spelling for it)",
            image.display()
        )))
    }
}

// ---------------------------------------------------------------------
// the runtime: resolved by the PRODUCT, read back from the store
// ---------------------------------------------------------------------

/// The runtime cache entry the priming dispatch resolved (dir name +
/// trust marker parse — resolution logic is the product's, never
/// duplicated here).
pub struct RuntimeEntry {
    pub engine: String,
    /// The language version (e.g. "3.3.7").
    pub lang_version: String,
    /// The tebako runtime release (e.g. "0.16.9").
    pub tebako_version: String,
    /// The cache entry directory (`runtimes/<engine>-<lv>-<ver>-<triplet>`)
    /// — the v2-press cold wipe's scoped target.
    pub dir: PathBuf,
    /// The cached interpreter exe (the fat package's runtime slot).
    pub exe: PathBuf,
    /// The exe's verified digest (the store's `sha256` marker) — pinned
    /// into the fat package's runtime_ref.
    pub exe_sha256: String,
}

/// The dispatch version pin (spec 27 §1): `TEBAKO_<TOOL>_VERSION` from
/// the suite's `name@version` — the suite's declared version wins over a
/// variant-suffixed registry default through the version chain's env
/// tier, so the dispatch resolves the payload the suite asked for.
pub fn version_pin_env(name: &str, version: &str) -> (String, String) {
    (
        format!("TEBAKO_{}_VERSION", name.to_uppercase().replace('-', "_")),
        version.to_string(),
    )
}

/// Force the runtime resolution once (UNMEASURED — acquisition, not a
/// benchmark run): dispatch the installed payload's shim with `--version`
/// and read the resolved runtime back from the store. The child's exit
/// code is irrelevant — the runtime download happens before the payload's
/// argv matters — so the check is the cache entry's existence, never the
/// exit status.
pub fn prime_runtime(
    layout: &BenchLayout,
    triplet: &str,
    payload: &PayloadHome,
) -> Result<RuntimeEntry, BenchError> {
    let entrypoint = app_entrypoints(&payload.mirror)
        .first()
        .map(|e| e.name.clone())
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: the {} {} manifest declares no entrypoints — nothing to prime with",
                payload.name, payload.version
            ))
        })?;
    let shim = layout
        .store()
        .join("shims")
        .join(format!("{entrypoint}{}", exe_suffix(triplet)));
    // Best-effort: a dispatch failure is fine as long as the runtime
    // landed in the store (the read-back below is the real check).
    let pin = [version_pin_env(&payload.name, &payload.version)];
    let _ = run_admin_with_env(
        layout,
        &shim,
        &["--version"],
        &pin,
        "acquire-prime-runtime.log",
    );
    read_runtime_entry(layout, triplet, &payload.mirror)
}

/// The dispatcher path for the payload's first entrypoint — the managed
/// arm's measured program (the engine re-runs it after `prime_runtime`
/// staged the store).
pub fn shim_path(
    layout: &BenchLayout,
    triplet: &str,
    payload: &PayloadHome,
) -> Result<PathBuf, BenchError> {
    let entrypoint = app_entrypoints(&payload.mirror)
        .first()
        .map(|e| e.name.clone())
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: the {} {} manifest declares no entrypoints — nothing to dispatch",
                payload.name, payload.version
            ))
        })?;
    let shim = layout
        .store()
        .join("shims")
        .join(format!("{entrypoint}{}", exe_suffix(triplet)));
    if !shim.is_file() {
        return Err(BenchError::operational(format!(
            "acquire: the install left no dispatcher {} in the store",
            shim.display()
        )));
    }
    Ok(shim)
}

/// Scan `runtimes/<engine>-<lv>-<ver>-<triplet>/` for THE cached runtime.
/// The scan is scoped to entries matching THIS payload's engine and THIS
/// leg's triplet (a sibling suite's runtime — say a java pair from an
/// earlier run of the same bench home — is not ambiguity, it is noise).
/// Zero matching entries (the priming failed) or several (ambiguity) are
/// named errors — never a guess (invariant 9).
fn read_runtime_entry(
    layout: &BenchLayout,
    triplet: &str,
    mirror: &tpkg::PayloadManifest,
) -> Result<RuntimeEntry, BenchError> {
    let engine = app_entrypoints(mirror)
        .first()
        .and_then(|e| e.runtime_requirement.as_ref())
        .map(|r| r.engine().to_string())
        .unwrap_or_else(|| "ruby".to_string());
    let dir = layout.store().join("runtimes");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&format!("{engine}-")) && name.ends_with(&format!("-{triplet}")) {
                found.push(p);
            }
        }
    }
    let entry_dir = match found.len() {
        1 => found.remove(0),
        0 => {
            return Err(BenchError::operational(format!(
                "acquire: the priming dispatch left no {engine} runtime for {triplet} in {} — see logs/acquire-prime-runtime.log",
                dir.display()
            )))
        }
        n => {
            return Err(BenchError::operational(format!(
                "acquire: {n} {engine} runtime entries for {triplet} in {} — the bench home is shared or stale; refusing to guess",
                dir.display()
            )))
        }
    };
    let dir_name = entry_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // "<engine>-<lv>-<ver>-<triplet>": strip the known prefix/suffix, then
    // split the remainder at its first '-' (neither version carries '-').
    let middle = dir_name
        .strip_prefix(&format!("{engine}-"))
        .and_then(|s| s.strip_suffix(&format!("-{triplet}")))
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: runtime cache entry '{dir_name}' is not {engine}-<lang-ver>-<tebako-ver>-{triplet}"
            ))
        })?;
    let (lang_version, tebako_version) = middle.split_once('-').ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: runtime cache entry '{dir_name}' does not carry <lang-ver>-<tebako-ver>"
        ))
    })?;
    // The store's exe spelling follows the factory's asset spelling
    // (spec 27 §10.1): openjdk's windows exe is `.exe`-suffixed, the
    // ruby/python factories' is bare — probe both, never assume.
    // tebako#716's dual-era grammar (era law rule 5): the new-era
    // engine-carrying stem probes first, the immutable old-era
    // (≤ v0.16.32) stem follows — by file existence.
    let stem_new = format!("tebako-runtime-{tebako_version}-{engine}-{lang_version}-{triplet}");
    let stem_old = format!("tebako-runtime-{tebako_version}-{lang_version}-{triplet}");
    let exe = [
        format!("{stem_new}{}", exe_suffix(triplet)),
        stem_new,
        format!("{stem_old}{}", exe_suffix(triplet)),
        stem_old,
    ]
    .into_iter()
    .map(|name| entry_dir.join(name))
    .find(|p| p.is_file())
    .ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: the runtime cache entry {} has no interpreter exe (neither era's spelling was found, suffixed or bare)",
            entry_dir.display()
        ))
    })?;
    let marker = std::fs::read_to_string(entry_dir.join("sha256")).map_err(|e| {
        BenchError::operational(format!(
            "acquire: the runtime cache entry {} has no readable sha256 marker: {e}",
            entry_dir.display()
        ))
    })?;
    let exe_sha256 = parse_bare_hash(&marker).ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: the sha256 marker of {} is not a 64-hex digest",
            entry_dir.display()
        ))
    })?;
    Ok(RuntimeEntry {
        engine,
        lang_version: lang_version.to_string(),
        tebako_version: tebako_version.to_string(),
        dir: entry_dir,
        exe,
        exe_sha256,
    })
}

// ---------------------------------------------------------------------
// the v2-press arm: the fat package, assembled in-process through tpkg
// ---------------------------------------------------------------------

/// Assemble the fat tpkg (spec 27 §1: bootstrap + payload image slot +
/// runtime slot) from the verified published artifacts. The wire format
/// is written through `tpkg` — the L0 owner crate — mirroring
/// tebako-cli's `stitch` byte-for-byte in shape: TPKG_FLAG_LEAN set
/// (every press writes it; fatness is the runtime slot's presence, the
/// bootstrap never branches on the flag), launcher ABI 1 (spec 17), the
/// type-2 package manifest naming entries[0] + the union mount at "/"
/// (the shim's mount rule, tebako-shim/src/dispatch.rs).
///
/// `;sha256=` in the runtime_ref pins the runtime EXE's verified digest
/// (the store's trust marker) — the bootstrap re-verifies the slot
/// against it at every run, so the package's runtime half is anchored to
/// the same bytes the managed arm resolved.
pub fn assemble_fat_package(
    layout: &BenchLayout,
    tools: &TebakoTools,
    payload: &PayloadHome,
    runtime: &RuntimeEntry,
    target: &Target,
) -> Result<PathBuf, BenchError> {
    let entrypoint = app_entrypoints(&payload.mirror).first().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: the {} {} manifest declares no entrypoints",
            payload.name, payload.version
        ))
    })?;
    let runtime_ref = format!(
        "{}@{};tebako={};image;sha256={}",
        runtime.engine, runtime.lang_version, runtime.tebako_version, runtime.exe_sha256
    );

    let target_dir = layout.targets.join(&target.id);
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot create {}: {e}",
            target_dir.display()
        ))
    })?;
    // The package rides the leg's platform; cfg! is that truth in-leg.
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let package = target_dir.join(format!("{}-fat{suffix}", payload.name));

    let stem = format!("{}-fat", payload.name);
    let manifest = tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: stem,
            version: "0.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-bench".to_string(),
                tool_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            created: rfc3339_now(),
        },
        entries: vec![tpkg::PackageEntry {
            name: entrypoint.name.clone(),
            slot: Some(0),
            entrypoint: entrypoint.path.clone(),
            runtime_ref: runtime_ref.clone(),
        }],
        jail: None,
        env: Default::default(),
        lock: None,
        mounts: vec![tpkg::PackageMount {
            slot: 0,
            point: "/".to_string(),
            mode: tpkg::MountMode::Union,
            precedence: Some(tpkg::Precedence::AfterEnv),
        }],
    };

    let slots: [(&Path, &str, u32); 2] = [
        // The payload slot auto-detects (format_id 0 = auto): the wire
        // field answers "how do I read these bytes" and the magic says
        // it (the orthogonality law, spec 00 §4).
        (&payload.image, "/", tpkg::TPKG_FORMAT_AUTO),
        // The runtime exe rides as a role slot (never mounted).
        (&runtime.exe, "", tpkg::TPKG_FORMAT_RUNTIME),
    ];
    write_package(&tools.bootstrap, &slots, &runtime_ref, &manifest, &package)?;
    chmod_0755(&package)?;
    resign_ad_hoc_if_macos(&package);

    // Read-back gate: the assembled package must parse and carry exactly
    // what was written (the wire owner validates its own bytes).
    let mut f = std::fs::File::open(&package).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot re-open the assembled package {}: {e}",
            package.display()
        ))
    })?;
    let m = tpkg::read_from(&mut f).map_err(|e| {
        BenchError::operational(format!(
            "acquire: the assembled package {} failed the trailer read-back: {}",
            package.display(),
            tpkg::strerror(e.code())
        ))
    })?;
    if m.slots.len() != 2 || m.slots[1].format_id != tpkg::TPKG_FORMAT_RUNTIME {
        return Err(BenchError::operational(format!(
            "acquire: the assembled package {} read back with {} slots / runtime slot format {} (expected 2 / {})",
            package.display(),
            m.slots.len(),
            m.slots.get(1).map(|s| s.format_id).unwrap_or(0),
            tpkg::TPKG_FORMAT_RUNTIME
        )));
    }
    Ok(package)
}

/// The package writer, mirroring tebako-pkg's `assemble` on the unsigned
/// path: bootstrap bytes, then each slot's bytes, then the ext blocks +
/// trailer through tpkg (the L0 owner). `;sha256=` digests ride the
/// runtime_ref, not a signing block (signing stays opt-in, spec 00 §7 —
/// the benchmark assembles unsigned packages exactly like the dogfood
/// press).
fn write_package(
    bootstrap: &Path,
    slots: &[(&Path, &str, u32)],
    runtime_ref: &str,
    manifest: &tpkg::PackageManifest,
    output: &Path,
) -> Result<(), BenchError> {
    if runtime_ref.len() >= tpkg::TPKG_RUNTIME_REF_LEN {
        return Err(BenchError::operational(format!(
            "acquire: runtime_ref exceeds {} bytes: {runtime_ref}",
            tpkg::TPKG_RUNTIME_REF_LEN - 1
        )));
    }
    let mut m = tpkg::Manifest {
        package_flags: tpkg::TPKG_FLAG_LEAN,
        launcher_abi: 1, // spec 17's launcher ABI version 1
        ..Default::default()
    };
    m.set_runtime_ref(runtime_ref.as_bytes());
    m.set_package_manifest(manifest)
        .map_err(|e| BenchError::operational(format!("acquire: invalid package manifest: {e}")))?;

    let tmp = output.with_extension("part");
    let result = (|| -> Result<(), BenchError> {
        let mut out = std::fs::File::create(&tmp).map_err(|e| {
            BenchError::operational(format!("acquire: cannot create {}: {e}", tmp.display()))
        })?;
        let mut total = stream_into(&mut out, bootstrap)?;
        for (path, mount, format_id) in slots {
            let written = stream_into(&mut out, path)?;
            m.slots
                .push(tpkg::Slot::new(total, written, *format_id, mount));
            total += written;
        }
        out.flush().map_err(|e| {
            BenchError::operational(format!("acquire: write failed for {}: {e}", tmp.display()))
        })?;
        tpkg::write_to(&mut out, &m).map_err(|e| {
            BenchError::operational(format!(
                "acquire: tpkg trailer write failed: {}",
                tpkg::strerror(e.code())
            ))
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result?;
    std::fs::rename(&tmp, output).map_err(|e| {
        BenchError::operational(format!("acquire: cannot publish {}: {e}", output.display()))
    })?;
    Ok(())
}

fn stream_into(out: &mut std::fs::File, path: &Path) -> Result<u64, BenchError> {
    let mut f = std::fs::File::open(path).map_err(|e| {
        BenchError::operational(format!("acquire: cannot open {}: {e}", path.display()))
    })?;
    std::io::copy(&mut f, out).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot stream {} into the package: {e}",
            path.display()
        ))
    })
}

/// macOS ad-hoc re-sign after appending the trailer (the product's own
/// press does exactly this — tebako-cli::resign_if_needed — best-effort,
/// loud on failure, the package kept either way). The ONE platform-tool
/// spawn in the harness: codesign has no in-process form, and the
/// no-shell-out uniformity argument (one implementation serving all
/// triplets) does not apply to a macOS-only Mach-O post-step.
#[cfg(target_os = "macos")]
fn resign_ad_hoc_if_macos(package: &Path) {
    match std::process::Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(package)
        .output()
    {
        Ok(out) if out.status.success() => {}
        other => {
            eprintln!(
                "tebako-bench: warning: ad-hoc re-sign of {} failed ({:?}); keeping the package (tebako-cli parity: it still executes on macOS)",
                package.display(),
                other.map(|o| o.status)
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn resign_ad_hoc_if_macos(_package: &Path) {}

/// The app PROVIDES entrypoints of a payload manifest (empty for non-app
/// kinds — tebako-shim's manifest.rs dispatchables rule).
fn app_entrypoints(mirror: &tpkg::PayloadManifest) -> &[tpkg::Entrypoint] {
    match &mirror.provides {
        tpkg::Provides::App(app) => &app.entrypoints,
        _ => &[],
    }
}

// ---------------------------------------------------------------------
// the runtime suite (spec 27 §10): runtime pairs, parity probes,
// the ioread fixture, the in-leg java compile
// ---------------------------------------------------------------------

/// Fetch one more product tool beyond the trio: the `tfs` image tool.
/// The runtime suite builds its fixture image through it (the dogfood
/// rule — the harness never re-implements imaging). Returns the staged
/// binary and the resolved tebako version.
pub fn fetch_tfs_tool(
    layout: &BenchLayout,
    release: Option<&str>,
    triplet: &str,
) -> Result<(PathBuf, String), BenchError> {
    let base = match release {
        Some(tag) => format!("https://github.com/tamatebako/tebako/releases/download/{tag}"),
        None => "https://github.com/tamatebako/tebako/releases/latest/download".to_string(),
    };
    let sums_url = format!("{base}/SHA256SUMS");
    let sums_bytes = get(&sums_url)?;
    let sums = String::from_utf8(sums_bytes)
        .map_err(|e| BenchError::operational(format!("acquire: {sums_url} is not UTF-8: {e}")))?;
    let suffix = exe_suffix(triplet);
    let version = match release {
        Some(tag) => tag.strip_prefix('v').unwrap_or(tag).to_string(),
        None => version_from_sums(&sums, triplet).ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: {sums_url} lists no tebako-<ver>-{triplet}{suffix} asset — cannot learn the release version"
            ))
        })?,
    };
    let asset = format!("tfs-{version}-{triplet}{suffix}");
    let expected = parse_sha256sums(&sums, &asset).ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: {asset} is not in {sums_url} (the release is incomplete)"
        ))
    })?;
    let dest = layout.assets.join(&asset);
    download_verified(&format!("{base}/{asset}"), &dest, &expected)?;
    let bare = layout.bin.join(format!("tfs{suffix}"));
    std::fs::copy(&dest, &bare).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot stage {} as {}: {e}",
            dest.display(),
            bare.display()
        ))
    })?;
    chmod_0755(&bare)?;
    Ok((bare, version))
}

/// A staged tebako runtime pair (runtime-exe targets, spec 27 §10.1):
/// the interpreter exe + the env image, both sha256-verified against the
/// factory release's per-asset `.sha256` sidecars.
pub struct RuntimePair {
    pub exe: PathBuf,
    pub image: PathBuf,
    /// The factory's tebako version (the tag's version segment: the
    /// monolithic tag sans `v`, or the leading semver of a spec 36 §6
    /// per-line tag — see [`tag_tebako_version`]).
    pub tebako_version: String,
    /// The interpreter version (the authored pin — the parity PROBE, not
    /// this field, is the fair-comparison assertion).
    pub lang_version: String,
}

/// The tag's tebako-version segment. The monolithic convention spells
/// `v<tb>` (v0.16.32 → `0.16.32`); the spec 36 §6 per-line shards spell
/// `v<tb>-<lang><line>(-<platform>)?` (v0.17.0-ruby3.3-macos → `0.17.0`)
/// — the line/platform suffix never joins the asset stems. A suffix
/// that is no factory line spelling (a prerelease ride like `-rc1`)
/// stays part of the version — exactly the pre-shard behavior.
fn tag_tebako_version(tag: &str) -> &str {
    let bare = tag.strip_prefix('v').unwrap_or(tag);
    let semver = |head: &str| {
        let mut parts = head.split('.');
        matches!(
            (parts.next(), parts.next(), parts.next(), parts.next()),
            (Some(a), Some(b), Some(c), None)
                if [a, b, c]
                    .iter()
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        )
    };
    for lang in [
        "ruby",
        "python",
        "java",
        "jruby",
        "truffleruby",
        "node",
        "bun",
    ] {
        if let Some((head, tail)) = bare.split_once(&format!("-{lang}")) {
            if semver(head) && tail.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
                return head;
            }
        }
    }
    bare
}

/// The candidate download bases of a runtime pin, shard-era aware
/// (spec 36 §6; tpkg owns the tag grammar):
/// - a pin already carrying a platform segment (`…-windows`) substitutes
///   the leg's segment, verbatim as the fallback;
/// - a shard-era LINE pin (`v0.17.1-ruby3.3`) appends the leg's segment
///   — an unparsable leg triplet is the named error, never a guess;
/// - anything else (a monolithic pin, a ≤ 0.16 line tag) rides verbatim.
fn candidate_bases(
    repo: &str,
    tag: &str,
    tebako_version: &str,
    line_pin: bool,
    leg: Option<tpkg::Platform>,
) -> Result<Vec<String>, BenchError> {
    let raw_base = |tag: &str| format!("https://github.com/{repo}/releases/download/{tag}");
    Ok(match tpkg::shard_tag_segment_of(tag) {
        Some(_) => match leg.and_then(|p| tpkg::shard_tag_substitute(tag, p)) {
            Some(sub) if sub != tag => vec![raw_base(&sub), raw_base(tag)],
            _ => vec![raw_base(tag)],
        },
        None if line_pin && tpkg::versions::sharded_release_era(tebako_version) => {
            let platform = leg.ok_or_else(|| {
                BenchError::operational(format!(
                    "acquire: cannot derive the per-platform shard of '{tag}' for the leg's triplet — the leg names no known platform"
                ))
            })?;
            vec![
                raw_base(&tpkg::shard_tag_append(tag, platform)),
                raw_base(tag),
            ]
        }
        None => vec![raw_base(tag)],
    })
}

/// The candidate base whose first sidecar probe answers — the spec 36
/// §6 per-platform derivation is probe-verified, never assumed. Per
/// candidate in order: each stem's bundle sidecar and exe sidecar
/// spellings. A 404 (IndexUnavailable) moves on; any other failure is
/// named. When no candidate answers, the FIRST candidate stands — the
/// downstream flow's named error then names the base the era most
/// expects.
fn select_base(bases: &[String], stems: &[String], suffix: &str) -> Result<String, BenchError> {
    for base in bases {
        for stem in stems {
            let mut probes = vec![format!("{stem}.tar.gz.sha256")];
            if !suffix.is_empty() {
                probes.push(format!("{stem}{suffix}.sha256"));
            }
            probes.push(format!("{stem}.sha256"));
            for name in probes {
                match tebako_http::get(&format!("{base}/{name}")) {
                    Ok(_) => return Ok(base.clone()),
                    Err(tebako_http::FetchError::IndexUnavailable(_)) => {}
                    Err(e) => {
                        return Err(BenchError::operational(format!(
                            "acquire: cannot probe {base}/{name}: {e}"
                        )))
                    }
                }
            }
        }
    }
    Ok(bases[0].clone())
}

/// Download + verify + stage the runtime pair named by the target's
/// `runtime` ref. The asset grammar is the factories', dual-era
/// (tebako#716): the new era spells
/// `tebako-runtime-<tebako-ver>-<engine>-<lang-ver>-<triplet>[.exe]`,
/// the immutable ≤ v0.16.32 lines keep the engine-less
/// `tebako-runtime-<tebako-ver>-<lang-ver>-<triplet>` — plus the `.tfs`
/// env image. On Windows a sibling `.dll` rides along WHEN the
/// factory ships one (ruby/python do, openjdk does not — its sidecar
/// 404 is the absence proof, never a guess). The release's grammar era
/// is unknowable before a sidecar answers: when the ref's repo parses
/// as a first-party factory (`<owner>/tebako-runtime-<engine>`) the
/// new-era stem probes FIRST and the old-era stem follows (one extra
/// sidecar probe round, era law rule 3); any other repo name keeps the
/// old-era-only behavior.
///
/// Since the spec 36 bundle era the factory ships ONE `<stem>.tar.gz`
/// (the pair + a closing SHA256SUMS) instead of the bare pair; the
/// bundle sidecar's existence selects the grammar (never a rename
/// guess), and the bundle rides [`stage_bundle_pair`]. The tag the
/// URLs ride derives per leg when the pin is a shard-era line pin
/// (spec 36 §6's per-platform grammar, tpkg-owned): the derived shard
/// base probes first, the authored tag verbatim second.
pub fn acquire_runtime_pair(
    layout: &BenchLayout,
    triplet: &str,
    target: &Target,
) -> Result<RuntimePair, BenchError> {
    let rr = target.runtime.as_ref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: runtime-exe target '{}' carries no runtime ref",
            target.id
        ))
    })?;
    // The version segment of the tag (monolithic v<tb>, the spec 36 §6
    // per-line v<tb>-<lang><line>, or the 0.17.1-era per-platform shard
    // v<tb>-<lang><line>-<platform>) — it alone joins the asset stems;
    // the v-prefix stays an authored-pin validation.
    if !rr.tag.starts_with('v') {
        return Err(BenchError::operational(format!(
            "acquire: runtime tag '{}' is not v-prefixed",
            rr.tag
        )));
    }
    let tebako_version = tag_tebako_version(&rr.tag).to_string();
    // tebako#716: the engine derives from the factory repo's basename
    // (`tamatebako/tebako-runtime-<engine>`); a repo name that does not
    // parse as a factory keeps the old-era-only stem list.
    let engine = rr
        .repo
        .rsplit('/')
        .next()
        .and_then(|basename| basename.strip_prefix("tebako-runtime-"))
        .filter(|e| !e.is_empty());
    let stems: Vec<String> = match engine {
        Some(engine) => vec![
            format!(
                "tebako-runtime-{tebako_version}-{engine}-{}-{triplet}",
                rr.lang_version
            ),
            format!(
                "tebako-runtime-{tebako_version}-{}-{triplet}",
                rr.lang_version
            ),
        ],
        None => vec![format!(
            "tebako-runtime-{tebako_version}-{}-{triplet}",
            rr.lang_version
        )],
    };
    let suffix = exe_suffix(triplet);
    // The 0.17.1-era per-platform shards (spec 36 §6): an authored pin
    // names a line (`v<tb>-<engine><line>`) or one published platform's
    // shard (`…-<platform>`) — the leg's own shard base DERIVES (tpkg
    // owns the grammar) and probes first, the authored tag verbatim
    // second (era law rule 3: never a rename guess without a probe).
    let leg = tpkg::Platform::from_release_asset_name(triplet);
    let line_pin = tag_tebako_version(&rr.tag) != rr.tag.strip_prefix('v').unwrap_or(&rr.tag);
    let bases = candidate_bases(&rr.repo, &rr.tag, &tebako_version, line_pin, leg)?;
    let base = select_base(&bases, &stems, suffix)?;
    let target_dir = layout.targets.join(&target.id);
    std::fs::create_dir_all(&target_dir).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot create {}: {e}",
            target_dir.display()
        ))
    })?;

    // The spec 36 bundle era: `<stem>.tar.gz` carries the pair (+
    // a closing SHA256SUMS) under one verified download. Its sidecar's
    // existence selects the grammar — the bare-pair probes below never
    // run against a bundle-era release. The stems probe in era order
    // (tebako#716: new-era first).
    for stem in &stems {
        match tebako_http::get(&format!("{base}/{stem}.tar.gz.sha256")) {
            Ok(sidecar) => {
                let expected =
                    parse_bare_hash(&String::from_utf8_lossy(&sidecar)).ok_or_else(|| {
                        BenchError::operational(format!(
                            "acquire: {base}/{stem}.tar.gz.sha256 is not a bare 64-hex sha256"
                        ))
                    })?;
                return stage_bundle_pair(
                    layout,
                    &base,
                    stem,
                    &expected,
                    &target_dir,
                    tebako_version,
                    &rr.lang_version,
                );
            }
            Err(tebako_http::FetchError::IndexUnavailable(_)) => {}
            Err(e) => {
                return Err(BenchError::operational(format!(
                    "acquire: cannot probe {base}/{stem}.tar.gz.sha256: {e}"
                )))
            }
        }
    }

    // One helper: download <name> + its bare-hash <name>.sha256 sidecar,
    // verify, stage under the target dir.
    let stage = |name: &str, executable: bool| -> Result<PathBuf, BenchError> {
        let sidecar = get(&format!("{base}/{name}.sha256"))?;
        let expected = parse_bare_hash(&String::from_utf8_lossy(&sidecar)).ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: {base}/{name}.sha256 is not a bare 64-hex sha256"
            ))
        })?;
        let asset = layout.assets.join(name);
        download_verified(&format!("{base}/{name}"), &asset, &expected)?;
        let staged = target_dir.join(name);
        std::fs::copy(&asset, &staged).map_err(|e| {
            BenchError::operational(format!(
                "acquire: cannot stage {} as {}: {e}",
                asset.display(),
                staged.display()
            ))
        })?;
        if executable {
            chmod_0755(&staged)?;
        }
        Ok(staged)
    };

    // The factories disagree on the windows interpreter's spelling
    // (spec 27 §10.1): openjdk ships `<stem>.exe`, the ruby/python
    // factories ship the bare `<stem>` beside their `.dll`. The sidecar
    // that EXISTS names the asset — probe each era's stem in order
    // (tebako#716: new-era first), the suffixed sidecar before the bare
    // one; a 404 (IndexUnavailable) moves to the next spelling. Never a
    // rename guess.
    let mut selected: Option<(String, String)> = None;
    for stem in &stems {
        if !suffix.is_empty() {
            match tebako_http::get(&format!("{base}/{stem}{suffix}.sha256")) {
                Ok(_) => {
                    selected = Some((stem.clone(), format!("{stem}{suffix}")));
                    break;
                }
                Err(tebako_http::FetchError::IndexUnavailable(_)) => {}
                Err(e) => {
                    return Err(BenchError::operational(format!(
                        "acquire: cannot probe {base}/{stem}{suffix}.sha256: {e}"
                    )))
                }
            }
        }
        match tebako_http::get(&format!("{base}/{stem}.sha256")) {
            Ok(_) => {
                selected = Some((stem.clone(), stem.clone()));
                break;
            }
            Err(tebako_http::FetchError::IndexUnavailable(_)) => {}
            Err(e) => {
                return Err(BenchError::operational(format!(
                    "acquire: cannot probe {base}/{stem}.sha256: {e}"
                )))
            }
        }
    }
    let (stem, exe_name) = selected.ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: {base} serves no interpreter exe sidecar for {tebako_version}-{}-{triplet} under either era's spelling",
            rr.lang_version
        ))
    })?;
    let exe = stage(&exe_name, true)?;
    let image = stage(&format!("{stem}.tfs"), false)?;
    // The windows runtimes that need a sibling dylib ship one; probe its
    // sidecar — 404 (IndexUnavailable) is the factory saying "no dll".
    if cfg!(windows) {
        let dll = format!("{stem}.dll");
        match tebako_http::get(&format!("{base}/{dll}.sha256")) {
            Ok(sidecar) => {
                let expected =
                    parse_bare_hash(&String::from_utf8_lossy(&sidecar)).ok_or_else(|| {
                        BenchError::operational(format!(
                            "acquire: {base}/{dll}.sha256 is not a bare 64-hex sha256"
                        ))
                    })?;
                let asset = layout.assets.join(&dll);
                download_verified(&format!("{base}/{dll}"), &asset, &expected)?;
                let staged = target_dir.join(&dll);
                std::fs::copy(&asset, &staged).map_err(|e| {
                    BenchError::operational(format!(
                        "acquire: cannot stage {} as {}: {e}",
                        asset.display(),
                        staged.display()
                    ))
                })?;
            }
            Err(tebako_http::FetchError::IndexUnavailable(_)) => {}
            Err(e) => {
                return Err(BenchError::operational(format!(
                    "acquire: cannot probe {base}/{dll}.sha256: {e}"
                )))
            }
        }
    }
    Ok(RuntimePair {
        exe,
        image,
        tebako_version,
        lang_version: rr.lang_version.clone(),
    })
}

/// The spec 36 bundle manifest's member declarations (the fields the
/// staging reads — the rest of the manifest is the product's
/// resolution surface, not the harness's).
#[derive(serde::Deserialize)]
struct BundleManifestMember {
    filename: String,
    sha256: String,
}

#[derive(serde::Deserialize)]
struct BundleManifest {
    filename: String,
    sha256: String,
    image: BundleManifestMember,
    /// The windows dylib, when the factory ships one (same absence
    /// rule as the bare era: the manifest not naming it is the proof).
    dll: Option<BundleManifestMember>,
}

/// The bundle-era staging (spec 36 §2/§5): the release `.sha256`
/// sidecar anchors the `<stem>.tar.gz` bytes; the manifest declares
/// the member set + per-member pins; the unpack enforces the §2
/// grammar (regular files, bare basenames, the closing SHA256SUMS
/// last) and every member verifies against BOTH the manifest's pin
/// and the SHA256SUMS entry. A disagreement is a named error — never
/// a best-effort unpack.
fn stage_bundle_pair(
    layout: &BenchLayout,
    base: &str,
    stem: &str,
    bundle_sha: &str,
    target_dir: &Path,
    tebako_version: String,
    lang_version: &str,
) -> Result<RuntimePair, BenchError> {
    let manifest_body = get(&format!("{base}/{stem}.manifest.json"))?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_body).map_err(|e| {
        BenchError::operational(format!(
            "acquire: {base}/{stem}.manifest.json does not parse: {e}"
        ))
    })?;
    let mut members = vec![
        (manifest.filename.clone(), manifest.sha256.clone()),
        (
            manifest.image.filename.clone(),
            manifest.image.sha256.clone(),
        ),
    ];
    if let Some(dll) = &manifest.dll {
        members.push((dll.filename.clone(), dll.sha256.clone()));
    }

    let bundle_name = format!("{stem}.tar.gz");
    let bundle = layout.assets.join(&bundle_name);
    download_verified(&format!("{base}/{bundle_name}"), &bundle, bundle_sha)?;
    let mut staged = unpack_runtime_bundle(&bundle, &bundle_name, &members, target_dir)?;

    let take = |staged: &mut std::collections::BTreeMap<String, PathBuf>, name: &str| {
        staged.remove(name).ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: bundle {bundle_name} carries no member '{name}' (harness bug — the set was verified)"
            ))
        })
    };
    let exe = take(&mut staged, &manifest.filename)?;
    chmod_0755(&exe)?;
    let image = take(&mut staged, &manifest.image.filename)?;
    Ok(RuntimePair {
        exe,
        image,
        tebako_version,
        lang_version: lang_version.to_string(),
    })
}

/// Unpack a verified `<stem>.tar.gz` runtime bundle under the spec 36
/// §2 grammar, returning the staged members by name. The member stream
/// hashes as it writes; the set must be exactly `members` plus the
/// closing SHA256SUMS, and every member's bytes must agree with its
/// manifest pin AND its SHA256SUMS line.
fn unpack_runtime_bundle(
    bundle: &Path,
    bundle_name: &str,
    members: &[(String, String)],
    target_dir: &Path,
) -> Result<std::collections::BTreeMap<String, PathBuf>, BenchError> {
    let invalid = |detail: String| {
        BenchError::operational(format!(
            "acquire: invalid release bundle {bundle_name}: {detail} — the release and the bundle disagree"
        ))
    };
    let file = std::fs::File::open(bundle).map_err(|e| {
        BenchError::operational(format!("acquire: cannot open {}: {e}", bundle.display()))
    })?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut staged = std::collections::BTreeMap::new();
    let mut digests: Vec<(String, String)> = Vec::new();
    let mut sums: Option<String> = None;
    let entries = archive
        .entries()
        .map_err(|e| invalid(format!("not a tar stream: {e}")))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| invalid(format!("a member does not parse: {e}")))?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            return Err(invalid(
                "a non-regular member (the grammar is regular files only)".to_string(),
            ));
        }
        let path = entry
            .path()
            .map_err(|e| invalid(format!("a member path does not parse: {e}")))?
            .into_owned();
        let name = match path.components().collect::<Vec<_>>().as_slice() {
            [std::path::Component::Normal(s)] => s.to_string_lossy().into_owned(),
            [std::path::Component::CurDir, std::path::Component::Normal(s)] => {
                s.to_string_lossy().into_owned()
            }
            _ => {
                return Err(invalid(format!(
                    "member '{}' is not a bare basename",
                    path.display()
                )))
            }
        };
        if name == "SHA256SUMS" {
            let mut text = String::new();
            entry
                .take(1 << 20)
                .read_to_string(&mut text)
                .map_err(|e| invalid(format!("SHA256SUMS does not read: {e}")))?;
            sums = Some(text);
            continue;
        }
        if sums.is_some() {
            return Err(invalid(format!(
                "member '{name}' follows the closing SHA256SUMS"
            )));
        }
        let dest = target_dir.join(&name);
        let mut out = std::fs::File::create(&dest).map_err(|e| {
            BenchError::operational(format!("acquire: cannot stage {}: {e}", dest.display()))
        })?;
        let mut hasher = sha2::Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = entry
                .read(&mut buf)
                .map_err(|e| invalid(format!("member '{name}' does not read: {e}")))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| {
                BenchError::operational(format!("acquire: cannot write {}: {e}", dest.display()))
            })?;
        }
        digests.push((name.clone(), format!("{:x}", hasher.finalize())));
        staged.insert(name, dest);
    }
    let sums =
        sums.ok_or_else(|| invalid("the closing SHA256SUMS member is absent".to_string()))?;
    let declared: std::collections::BTreeMap<&str, &str> = members
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    if staged.len() != declared.len() || staged.keys().any(|n| !declared.contains_key(n.as_str())) {
        return Err(invalid(
            "the member set is not the manifest's declaration (never a skipped member, never an extra one)"
                .to_string(),
        ));
    }
    for (name, hex) in &digests {
        if declared.get(name.as_str()) != Some(&hex.as_str()) {
            return Err(invalid(format!(
                "member '{name}' disagrees with the manifest's pin"
            )));
        }
        if parse_sha256sums(&sums, name).as_deref() != Some(hex.as_str()) {
            return Err(invalid(format!(
                "member '{name}' disagrees with the closing SHA256SUMS"
            )));
        }
    }
    Ok(staged)
}

/// A staged LAZY_SEEDING entry (runtime-exe-lazy targets, spec 27/// §10.5): the same verified pair as [`acquire_runtime_pair`], restaged
/// as a spec 39 §4 store entry — the exe present, the env image ABSENT,
/// the seed descriptor's `source` naming the leg's loopback fixture.
pub struct LazyPair {
    /// `entry/<exe name>` — the measured program.
    pub exe: PathBuf,
    /// `entry/<image name>` — the absent image TEBAKO_RUNTIME_IMAGE
    /// names (the driver state-detects the seeding entry beside it).
    pub image: PathBuf,
    /// The entry dir — the cold wipe's reseed target.
    pub entry_dir: PathBuf,
    /// The serving loopback fixture (lives for the whole leg).
    pub server: crate::lazy_server::LazyServer,
}

/// Acquire the pair, then build the seeding entry: the exe copied in,
/// the blksum sidecar authored in-process from the verified image bytes
/// (spec 39 §3's publisher rule — the harness IS this fixture's
/// publisher), the descriptor written against the fixture's port, and a
/// pristine descriptor copy kept for the cold reseed.
pub fn acquire_lazy_runtime_pair(
    layout: &BenchLayout,
    triplet: &str,
    target: &Target,
) -> Result<LazyPair, BenchError> {
    let pair = acquire_runtime_pair(layout, triplet, target)?;
    let exe_name = pair
        .exe
        .file_name()
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: the staged exe {} has no file name",
                pair.exe.display()
            ))
        })?
        .to_string_lossy()
        .into_owned();
    let image_base = pair
        .image
        .file_name()
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: the staged image {} has no file name",
                pair.image.display()
            ))
        })?
        .to_string_lossy()
        .into_owned();
    let image_bytes = std::fs::read(&pair.image).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot read the staged image {}: {e}",
            pair.image.display()
        ))
    })?;
    let blksum = tpkg::lazy::Blksum::from_image_bytes(&image_bytes).render();
    let server =
        crate::lazy_server::LazyServer::start(pair.image.clone(), blksum.clone().into_bytes())?;
    let seed = tpkg::lazy::LazySeed {
        source: format!("http://127.0.0.1:{}/{image_base}", server.port),
        sha256: tpkg::lazy::sha256_hex(&image_bytes),
        blksum_sha256: tpkg::lazy::sha256_hex(blksum.as_bytes()),
        size_bytes: image_bytes.len() as u64,
        group_count: tpkg::lazy::group_count_for(image_bytes.len() as u64),
    };
    let entry = layout.targets.join(&target.id).join("entry");
    std::fs::create_dir_all(&entry).map_err(|e| {
        BenchError::operational(format!("acquire: cannot create {}: {e}", entry.display()))
    })?;
    let exe = entry.join(&exe_name);
    std::fs::copy(&pair.exe, &exe).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot stage {} as {}: {e}",
            pair.exe.display(),
            exe.display()
        ))
    })?;
    chmod_0755(&exe)?;
    tpkg::lazy::write_descriptor(&entry, &image_base, &seed).map_err(|e| {
        BenchError::operational(format!("acquire: cannot write the seed descriptor: {e}"))
    })?;
    // The cold reseed's pristine copy: `pristine/<image>.lazy.json` —
    // the wipe restores it verbatim after removing the sealed state.
    let pristine = entry.join("pristine");
    std::fs::create_dir_all(&pristine).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot create {}: {e}",
            pristine.display()
        ))
    })?;
    std::fs::write(
        pristine.join(tpkg::lazy::descriptor_name(&image_base)),
        seed.render(),
    )
    .map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot stage the pristine descriptor copy: {e}"
        ))
    })?;
    Ok(LazyPair {
        exe,
        image: entry.join(&image_base),
        entry_dir: entry,
        server,
    })
}

/// The lazy arm's cold reseed (spec 27 §10.5): remove the sealed image
/// and the block cache, restore the pristine LAZY_SEEDING descriptor —
/// every cold cell pays the on-demand seeding boot from an empty entry.
pub fn reseed_lazy_entry(entry: &Path) -> Result<(), BenchError> {
    let pristine = entry.join("pristine");
    let copies = std::fs::read_dir(&pristine).map_err(|e| {
        BenchError::operational(format!(
            "acquire: the lazy entry {} carries no pristine descriptor copy: {e}",
            entry.display()
        ))
    })?;
    for copy in copies {
        let copy = copy.map_err(|e| {
            BenchError::operational(format!("acquire: cannot scan {}: {e}", pristine.display()))
        })?;
        let name = copy.file_name().to_string_lossy().into_owned();
        let Some(image_base) = name.strip_suffix(".lazy.json") else {
            continue;
        };
        match std::fs::remove_file(entry.join(image_base)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(BenchError::operational(format!(
                    "acquire: cannot remove the sealed image {}: {e}",
                    entry.join(image_base).display()
                )))
            }
        }
        remove_tree(&entry.join(tpkg::lazy::blocks_dir_name(image_base)))?;
        std::fs::copy(copy.path(), entry.join(&name)).map_err(|e| {
            BenchError::operational(format!(
                "acquire: cannot restore the pristine descriptor {}: {e}",
                copy.path().display()
            ))
        })?;
    }
    Ok(())
}

/// Spawn `program args` and capture stdout+stderr (the version probes —
/// the parity assertion reads both streams; java's `-version` writes
/// stderr). The combined output is appended to `logs/<log_name>` and
/// returned. A nonzero exit is a named error carrying the output tail.
pub fn run_capture(
    layout: &BenchLayout,
    program: &Path,
    args: &[String],
    extra_env: &[(String, String)],
    log_name: &str,
) -> Result<String, BenchError> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .current_dir(&layout.home)
        .envs(layout.child_env("admin"))
        .envs(extra_env.iter().cloned())
        .stdin(std::process::Stdio::null());
    let out = cmd.output().map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot spawn {} for the version probe: {e}",
            program.display()
        ))
    })?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    let log_path = layout.logs.join(log_name);
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| {
            BenchError::operational(format!("acquire: cannot open {}: {e}", log_path.display()))
        })?;
    use std::io::Write as _;
    writeln!(log, "$ {} {}", program.display(), args.join(" "))
        .and_then(|()| log.write_all(text.as_bytes()))
        .map_err(|e| {
            BenchError::operational(format!("acquire: cannot write {}: {e}", log_path.display()))
        })?;
    if !out.status.success() {
        return Err(BenchError::operational(format!(
            "acquire: the version probe `{} {}` failed ({}) — see {}",
            program.display(),
            args.join(" "),
            out.status
                .code()
                .map(|c| format!("exit {c}"))
                .unwrap_or_else(|| "signal".to_string()),
            log_path.display()
        )));
    }
    Ok(text)
}

/// The ioread fixture (spec 27 §10.2): 64 MiB of fixed-seed xorshift64*
/// bytes — high-entropy so the dwarfs zstd blocks do real work,
/// deterministic so every leg reads the same bytes.
pub const FIXTURE_LEN: u64 = 64 * 1024 * 1024;

/// The fixture's in-image path under the harness's fixed mount point.
pub const FIXTURE_VFS_PATH: &str = "/bench-fixture/fixture.bin";

pub fn generate_fixture(dest: &Path) -> Result<(), BenchError> {
    let mut f = std::fs::File::create(dest).map_err(|e| {
        BenchError::operational(format!("acquire: cannot create {}: {e}", dest.display()))
    })?;
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut written: u64 = 0;
    let mut buf = Vec::with_capacity(1 << 20);
    while written < FIXTURE_LEN {
        buf.clear();
        for _ in 0..((1 << 20) / 8) {
            // xorshift64* (Marsaglia/Vigna) — fixed seed, platform-neutral.
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            buf.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
        }
        f.write_all(&buf).map_err(|e| {
            BenchError::operational(format!("acquire: cannot write {}: {e}", dest.display()))
        })?;
        written += buf.len() as u64;
    }
    Ok(())
}

/// Build the fixture image through the downloaded `tfs` tool (the dogfood
/// rule): `tfs mkimage --format dwarfs <dir> --output <img>`, unmeasured.
pub fn build_fixture_image(
    layout: &BenchLayout,
    tfs: &Path,
    fixture_dir: &Path,
    out: &Path,
) -> Result<(), BenchError> {
    let dir = fixture_dir.to_string_lossy().into_owned();
    let img = out.to_string_lossy().into_owned();
    run_admin(
        layout,
        tfs,
        &["mkimage", "--format", "dwarfs", &dir, "--output", &img],
        "acquire-fixture-image.log",
    )
}

/// Compile the vendored java classes ONCE in-leg with the on-system
/// javac (spec 27 §10.2's compile-once rule — both JVMs run the same
/// .class files). Returns the classes directory (`{classes}` resolves
/// to it). javac is the workflow-provisioned toolchain binary — the
/// harness is CI tooling and never ships.
pub fn compile_java_classes(
    layout: &BenchLayout,
    repo_root: &Path,
    src_rel: &str,
) -> Result<PathBuf, BenchError> {
    let src_dir = repo_root.join(src_rel);
    let mut sources: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(&src_dir).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot list the java fixture dir {}: {e}",
            src_dir.display()
        ))
    })?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("java") {
            sources.push(p);
        }
    }
    sources.sort();
    if sources.is_empty() {
        return Err(BenchError::operational(format!(
            "acquire: the java fixture dir {} holds no .java sources",
            src_dir.display()
        )));
    }
    let out_dir = layout.sources.join("_classes");
    remove_tree(&out_dir)?;
    std::fs::create_dir_all(&out_dir).map_err(|e| {
        BenchError::operational(format!("acquire: cannot create {}: {e}", out_dir.display()))
    })?;
    let mut args: Vec<String> = vec!["-d".to_string(), out_dir.to_string_lossy().into_owned()];
    args.extend(sources.iter().map(|p| p.to_string_lossy().into_owned()));
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_admin(layout, Path::new("javac"), &arg_refs, "acquire-javac.log")?;
    Ok(out_dir)
}

// ---------------------------------------------------------------------
// workload sources
// ---------------------------------------------------------------------
/// A materialized workload source: `root` is copied into each run's
/// scratch (the whole tree — the document's relative includes must
/// resolve), `doc_rel` selects the document inside it.
pub struct MaterializedSource {
    pub root: PathBuf,
    pub doc_rel: PathBuf,
}

/// Materialize a workload's source document (spec 27 §2), when it has one
/// — interpreter workloads (the runtime suite) are source-less and get
/// None (their scratch cell stays empty). Vendored: copy
/// the repo file. Git: fetch the host's archive-of-commit over HTTPS
/// in-process (never a `git` shell-out — invariant 1) and extract the
/// whole tree in-process.
pub fn materialize_source(
    workload: &Workload,
    layout: &BenchLayout,
    repo_root: &Path,
) -> Result<Option<MaterializedSource>, BenchError> {
    let Some(source) = &workload.source else {
        return Ok(None);
    };
    let dest = layout.sources.join(&workload.id);
    remove_tree(&dest)?;
    std::fs::create_dir_all(&dest).map_err(|e| {
        BenchError::operational(format!("acquire: cannot create {}: {e}", dest.display()))
    })?;
    match source.kind {
        SourceKind::Vendored => {
            let src = repo_root.join(&source.path);
            let name = Path::new(&source.path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .ok_or_else(|| {
                    BenchError::operational(format!(
                        "acquire: vendored source '{}' has no file name",
                        source.path
                    ))
                })?;
            std::fs::copy(&src, dest.join(&name)).map_err(|e| {
                BenchError::operational(format!(
                    "acquire: cannot vendor {} (run from the repo root): {e}",
                    src.display()
                ))
            })?;
            Ok(Some(MaterializedSource {
                root: dest,
                doc_rel: PathBuf::from(name),
            }))
        }
        SourceKind::Git => fetch_git_source(workload, source, dest).map(Some),
    }
}

/// GitHub's archive-of-commit (`codeload .../tar.gz/<40-hex>`), extracted
/// in-process. The suite's semantic gate already pinned a 40-hex ref; a
/// non-github host is a named error here (MECE reference syntax — never
/// a guessed host grammar).
fn fetch_git_source(
    workload: &Workload,
    source: &crate::suite::Source,
    dest: PathBuf,
) -> Result<MaterializedSource, BenchError> {
    let url = source.url.as_deref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: git workload '{}' carries no url",
            workload.id
        ))
    })?;
    let git_ref = source.git_ref.as_deref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: git workload '{}' carries no pinned ref",
            workload.id
        ))
    })?;
    let path = url
        .strip_prefix("https://github.com/")
        .ok_or_else(|| {
            BenchError::operational(format!(
                "acquire: workload '{}' url '{url}' is not a GitHub repo URL (the archive-of-commit fetch speaks codeload only)",
                workload.id
            ))
        })?
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string();
    let archive_url = format!("https://codeload.github.com/{path}/tar.gz/{git_ref}");
    let bytes = get(&archive_url)?;
    let gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&dest).map_err(|e| {
        BenchError::operational(format!(
            "acquire: cannot extract the {archive_url} archive: {e}"
        ))
    })?;
    // The archive's single top-level dir is "<repo>-<sha>/".
    let mut tops: Vec<PathBuf> = std::fs::read_dir(&dest)
        .map_err(|e| {
            BenchError::operational(format!("acquire: cannot list {}: {e}", dest.display()))
        })?
        .flatten()
        .map(|e| e.path())
        .collect();
    if tops.len() != 1 {
        return Err(BenchError::operational(format!(
            "acquire: the {archive_url} archive holds {} top-level entries (one expected)",
            tops.len()
        )));
    }
    let root = tops.remove(0);
    let doc_rel = PathBuf::from(&source.path);
    if doc_rel.is_absolute() || doc_rel.to_string_lossy().contains("..") {
        return Err(BenchError::operational(format!(
            "acquire: workload '{}' path '{}' must be tree-relative without '..'",
            workload.id, source.path
        )));
    }
    if !root.join(&doc_rel).is_file() {
        return Err(BenchError::operational(format!(
            "acquire: the pinned tree holds no '{}' for workload '{}'",
            source.path, workload.id
        )));
    }
    Ok(MaterializedSource { root, doc_rel })
}

// ---------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------

/// Run an UNMEASURED product command in the bench home (the dogfood
/// install path): the hermetic env, output appended to logs/<log_name>,
/// nonzero exit → named error pointing at the log. This spawns the
/// downloaded product binaries — never platform tools (spec 27 §0).
pub fn run_admin(
    layout: &BenchLayout,
    program: &Path,
    args: &[&str],
    log_name: &str,
) -> Result<(), BenchError> {
    run_admin_with_env(layout, program, args, &[], log_name)
}

/// `run_admin` plus extra environment (the dispatch version pin, spec
/// 27 §1 — the suite's declared payload version wins over the registry
/// default through the version chain's env tier).
pub fn run_admin_with_env(
    layout: &BenchLayout,
    program: &Path,
    args: &[&str],
    extra_env: &[(String, String)],
    log_name: &str,
) -> Result<(), BenchError> {
    let log_path = layout.logs.join(log_name);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| {
            BenchError::operational(format!("acquire: cannot open {}: {e}", log_path.display()))
        })?;
    let log_err = log.try_clone().map_err(|e| {
        BenchError::operational(format!("acquire: cannot clone the log handle: {e}"))
    })?;
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .current_dir(&layout.home)
        // The store location for admin commands is target-independent —
        // the tmp override uses a fixed "admin" slot so v1's TMPDIR rules
        // never leak into v2 store population.
        .envs(layout.child_env("admin"))
        .envs(extra_env.iter().cloned())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(log))
        .stderr(std::process::Stdio::from(log_err));
    let status = cmd.status().map_err(|e| {
        BenchError::operational(format!("acquire: cannot spawn {}: {e}", program.display()))
    })?;
    if !status.success() {
        return Err(BenchError::operational(format!(
            "acquire: `{} {}` failed ({}) — see {}",
            program.display(),
            args.join(" "),
            status
                .code()
                .map(|c| format!("exit {c}"))
                .unwrap_or_else(|| "signal".to_string()),
            log_path.display()
        )));
    }
    Ok(())
}

fn chmod_0755(path: &Path) -> Result<(), BenchError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| {
                BenchError::operational(format!("acquire: cannot stat {}: {e}", path.display()))
            })?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).map_err(|e| {
            BenchError::operational(format!("acquire: cannot chmod {}: {e}", path.display()))
        })?;
    }
    let _ = path;
    Ok(())
}

/// RFC 3339 UTC now, for the package manifest's `created` (the metadata
/// convention — no chrono in the tree; the civil-from-days algorithm is
/// Howard Hinnant's, same as tebako-cli's).
fn rfc3339_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 3) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::{candidate_bases, tag_tebako_version};

    #[test]
    fn monolithic_tags_keep_the_bare_version() {
        assert_eq!(tag_tebako_version("v0.16.32"), "0.16.32");
        assert_eq!(tag_tebako_version("v0.2.0"), "0.2.0");
    }

    #[test]
    fn per_line_tags_yield_the_leading_semver() {
        assert_eq!(tag_tebako_version("v0.17.0-ruby3.3"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-ruby3.3-macos"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-java21"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-jruby10.0.2"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-truffleruby33"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-node22"), "0.17.0");
        assert_eq!(tag_tebako_version("v0.17.0-bun1"), "0.17.0");
    }

    #[test]
    fn non_line_suffixes_stay_part_of_the_version() {
        assert_eq!(tag_tebako_version("v0.17.0-rc1"), "0.17.0-rc1");
        assert_eq!(tag_tebako_version("0.17.0"), "0.17.0");
    }

    fn bases(repo: &str, tag: &str, triplet: &str) -> Vec<String> {
        let tebako_version = tag_tebako_version(tag).to_string();
        let line_pin = tebako_version != tag.strip_prefix('v').unwrap_or(tag);
        let leg = tpkg::Platform::from_release_asset_name(triplet);
        candidate_bases(repo, tag, &tebako_version, line_pin, leg).unwrap()
    }

    #[test]
    fn a_shard_era_line_pin_derives_the_legs_shard_first() {
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3",
            "linux-gnu-x86_64",
        );
        assert_eq!(
            b,
            vec![
                "https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu",
                "https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3",
            ]
        );
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3",
            "macos-arm64",
        );
        assert!(b[0].ends_with("/v0.17.1-ruby3.3-macos"), "{}", b[0]);
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3",
            "windows-ucrt64",
        );
        assert!(b[0].ends_with("/v0.17.1-ruby3.3-windows"), "{}", b[0]);
        // the two-segment musl form
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3",
            "linux-musl-arm64",
        );
        assert!(b[0].ends_with("/v0.17.1-ruby3.3-linux-musl"), "{}", b[0]);
    }

    #[test]
    fn a_suffixed_pin_substitutes_the_legs_segment() {
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3-windows",
            "macos-x86_64",
        );
        assert_eq!(
            b,
            vec![
                "https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos",
                "https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-windows",
            ]
        );
        // already the leg's own shard: verbatim only
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3-macos",
            "macos-arm64",
        );
        assert_eq!(b.len(), 1);
        assert!(b[0].ends_with("/v0.17.1-ruby3.3-macos"));
    }

    #[test]
    fn pre_shard_era_pins_ride_verbatim() {
        // a ≤ 0.16 line pin (the v0.16.28 recovery shape): no shard exists
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.16.28-ruby3.4",
            "macos-arm64",
        );
        assert_eq!(b.len(), 1);
        assert!(b[0].ends_with("/v0.16.28-ruby3.4"));
        // a monolithic pin
        let b = bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.16.32",
            "linux-gnu-x86_64",
        );
        assert_eq!(b.len(), 1);
        assert!(b[0].ends_with("/v0.16.32"));
    }

    #[test]
    fn a_shard_era_line_pin_on_an_unknown_leg_is_the_named_error() {
        let tebako_version = "0.17.1".to_string();
        let err = candidate_bases(
            "tamatebako/tebako-runtime-ruby",
            "v0.17.1-ruby3.3",
            &tebako_version,
            true,
            None,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot derive the per-platform shard"),
            "{err}"
        );
    }
}

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
    ///   lands inside the measured span.
    /// - v2-press: the store's `runtimes/` — the fat package carries the
    ///   runtime EXE but NOT the env image (§9 spike a), so the env-image
    ///   download lands inside the measured span. The package never
    ///   touches the store's payload side, so the payload record survives
    ///   (and the next v2-managed cold rep re-installs anyway).
    pub fn wipe_cold_caches(&self, target: &str, kind: TargetKind) -> Result<(), BenchError> {
        let mut wipes = vec![self.home.join(".metanorma"), self.home.join(".relaton")];
        match kind {
            TargetKind::V1Exe => wipes.push(self.tmp.join(target)),
            TargetKind::V2Managed => wipes.push(self.store()),
            TargetKind::V2Press => wipes.push(self.store().join("runtimes")),
        }
        for dir in &wipes {
            remove_tree(dir)?;
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
            "acquire: SHA256 mismatch for {url}\n  expected: {}\n  actual:   {actual}\n  the download was NOT written (the trust anchor is the checksum, spec 00 §8)",
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
            "acquire: {base}/{asset}.sha256.txt is not a bare 64-hex sha256 (spec 27 §9 spike c)"
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
            "acquire: unsupported packed-mn asset form '{asset}' (.tgz single-member or .exe expected — spec 27 §3)"
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
                "acquire: the packed-mn tgz carries more than one file member (at least '{name}' and one other) — the single-member rule (spec 27 §3) is violated"
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
    /// The cached interpreter exe (the fat package's runtime slot).
    pub exe: PathBuf,
    /// The exe's verified digest (the store's `sha256` marker) — pinned
    /// into the fat package's runtime_ref.
    pub exe_sha256: String,
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
    let _ = run_admin(layout, &shim, &["--version"], "acquire-prime-runtime.log");
    read_runtime_entry(layout, triplet, &payload.mirror)
}

/// Scan `runtimes/<engine>-<lv>-<ver>-<triplet>/` for THE cached runtime.
/// Zero entries (the priming failed) or several (ambiguity) are named
/// errors — never a guess (invariant 9).
fn read_runtime_entry(
    layout: &BenchLayout,
    triplet: &str,
    mirror: &tpkg::PayloadManifest,
) -> Result<RuntimeEntry, BenchError> {
    let engine = app_entrypoints(mirror)
        .first()
        .and_then(|e| e.runtime_requirement.as_ref())
        .map(|r| r.engine.clone())
        .unwrap_or_else(|| "ruby".to_string());
    let dir = layout.store().join("runtimes");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                found.push(p);
            }
        }
    }
    let entry_dir = match found.len() {
        1 => found.remove(0),
        0 => {
            return Err(BenchError::operational(format!(
                "acquire: the priming dispatch left no runtime in {} — see logs/acquire-prime-runtime.log",
                dir.display()
            )))
        }
        n => {
            return Err(BenchError::operational(format!(
                "acquire: {} runtime entries in {} — the bench home is shared or stale; refusing to guess",
                n,
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
    let asset = format!(
        "tebako-runtime-{tebako_version}-{lang_version}-{triplet}{}",
        exe_suffix(triplet)
    );
    let exe = entry_dir.join(&asset);
    if !exe.is_file() {
        return Err(BenchError::operational(format!(
            "acquire: the runtime cache entry {} has no {asset}",
            entry_dir.display()
        )));
    }
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
            slot: 0,
            entrypoint: entrypoint.path.clone(),
            runtime_ref: runtime_ref.clone(),
        }],
        jail: None,
        env: Default::default(),
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
// workload sources
// ---------------------------------------------------------------------

/// A materialized workload source: `root` is copied into each run's
/// scratch (the whole tree — the document's relative includes must
/// resolve), `doc_rel` selects the document inside it.
pub struct MaterializedSource {
    pub root: PathBuf,
    pub doc_rel: PathBuf,
}

/// Materialize a workload's source document (spec 27 §2). Vendored: copy
/// the repo file. Git: fetch the host's archive-of-commit over HTTPS
/// in-process (never a `git` shell-out — invariant 1) and extract the
/// whole tree in-process.
pub fn materialize_source(
    workload: &Workload,
    layout: &BenchLayout,
    repo_root: &Path,
) -> Result<MaterializedSource, BenchError> {
    let dest = layout.sources.join(&workload.id);
    remove_tree(&dest)?;
    std::fs::create_dir_all(&dest).map_err(|e| {
        BenchError::operational(format!("acquire: cannot create {}: {e}", dest.display()))
    })?;
    match workload.source.kind {
        SourceKind::Vendored => {
            let src = repo_root.join(&workload.source.path);
            let name = Path::new(&workload.source.path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .ok_or_else(|| {
                    BenchError::operational(format!(
                        "acquire: vendored source '{}' has no file name",
                        workload.source.path
                    ))
                })?;
            std::fs::copy(&src, dest.join(&name)).map_err(|e| {
                BenchError::operational(format!(
                    "acquire: cannot vendor {} (run from the repo root): {e}",
                    src.display()
                ))
            })?;
            Ok(MaterializedSource {
                root: dest,
                doc_rel: PathBuf::from(name),
            })
        }
        SourceKind::Git => fetch_git_source(workload, dest),
    }
}

/// GitHub's archive-of-commit (`codeload .../tar.gz/<40-hex>`), extracted
/// in-process. The suite's semantic gate already pinned a 40-hex ref; a
/// non-github host is a named error here (MECE reference syntax — never
/// a guessed host grammar).
fn fetch_git_source(workload: &Workload, dest: PathBuf) -> Result<MaterializedSource, BenchError> {
    let url = workload.source.url.as_deref().ok_or_else(|| {
        BenchError::operational(format!(
            "acquire: git workload '{}' carries no url",
            workload.id
        ))
    })?;
    let git_ref = workload.source.git_ref.as_deref().ok_or_else(|| {
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
    let doc_rel = PathBuf::from(&workload.source.path);
    if doc_rel.is_absolute() || doc_rel.to_string_lossy().contains("..") {
        return Err(BenchError::operational(format!(
            "acquire: workload '{}' path '{}' must be tree-relative without '..'",
            workload.id, workload.source.path
        )));
    }
    if !root.join(&doc_rel).is_file() {
        return Err(BenchError::operational(format!(
            "acquire: the pinned tree holds no '{}' for workload '{}'",
            workload.source.path, workload.id
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

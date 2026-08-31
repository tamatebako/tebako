//! Shared fixtures: temp homes, installed payload records, cached
//! runtimes, file:// runtime mirrors.
//!
//! Each integration test compiles this module separately and uses its own
//! subset — dead-code warnings are expected and allowed.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tebako_shim::Ctx;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "tebako-shim-test-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub fn ctx(home: &Path, cwd: &Path) -> Ctx {
    Ctx {
        home: home.to_path_buf(),
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// An installed payload record (image + trust anchor + manifest mirror).
pub fn write_payload(home: &Path, name: &str, version: &str, manifest_yaml: &str) -> PathBuf {
    let dir = home.join("payloads").join(name);
    std::fs::create_dir_all(&dir).expect("payload dir");
    let image = dir.join(format!("{version}.tfs"));
    let bytes = format!("fake tfs image {name} {version}\n");
    std::fs::write(&image, &bytes).expect("image");
    std::fs::write(
        dir.join(format!("{version}.tfs.sha256")),
        format!("{}  {version}.tfs\n", sha256_hex(bytes.as_bytes())),
    )
    .expect("sha marker");
    std::fs::write(dir.join(format!("{version}.manifest.yaml")), manifest_yaml)
        .expect("manifest mirror");
    image
}

/// The unified payload manifest (spec 03 — the mirror's shape after the
/// item 40 manifest unify): kind app, with `entrypoints_block` as the
/// provides.entrypoints body ("  entrypoints:\n    - name: …" at the
/// provides-level indent).
pub fn app_manifest(name: &str, version: &str, entrypoints_block: &str) -> String {
    app_manifest_full(name, version, entrypoints_block, "")
}

/// An app manifest carrying DEPENDS edges (`requires_block` at the
/// top-level indent, "requires:\n  - kind: …").
pub fn app_manifest_requires(
    name: &str,
    version: &str,
    entrypoints_block: &str,
    requires_block: &str,
) -> String {
    app_manifest_full(name, version, entrypoints_block, requires_block)
}

fn app_manifest_full(
    name: &str,
    version: &str,
    entrypoints_block: &str,
    requires_block: &str,
) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n{entrypoints_block}  platforms: universal\n  capabilities: {{exec: true, read: true}}\n{requires_block}",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    )
}

/// A data payload's manifest (dependency fixtures): no entrypoints.
pub fn data_manifest(name: &str, version: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: data\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  mount_semantics: {{suggested: /usr/share/{name}}}\n  capabilities: {{exec: false, read: true}}\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    )
}

pub const RUBY_ENTRY: &str = "  entrypoints:\n    - name: TOOL\n      path: /app/bin/TOOL\n      runtime_requirement: {engine: ruby, constraint: \">= 3.3, < 5.0\"}\n";

pub const NATIVE_ENTRY: &str = "  entrypoints:\n    - name: TOOL\n      path: /app/bin/TOOL\n";

pub fn entrypoint_yaml(template: &str, tool: &str) -> String {
    template.replace("TOOL", tool)
}

pub fn platform() -> &'static str {
    tebako_shim::runtime::platform_string()
}

/// A cached runtime entry
/// `runtimes/ruby-<lv>-<ver>-<triplet>/tebako-runtime-<ver>-<lv>-<triplet>[.exe]`
/// — the resolver looks the asset up by its platform name, suffix
/// included (spec: `exe_suffix()` on Windows).
pub fn write_runtime(home: &Path, lv: &str, ver: &str, with_image: bool) -> PathBuf {
    write_runtime_engine(home, "ruby", lv, ver, with_image)
}

/// `write_runtime` generalized over the engine (spec 30 fixtures: a
/// cached java runtime for the spawned-edge tests).
pub fn write_runtime_engine(
    home: &Path,
    engine: &str,
    lv: &str,
    ver: &str,
    with_image: bool,
) -> PathBuf {
    let platform = platform();
    let suffix = tebako_shim::runtime::exe_suffix();
    let dir = home
        .join("runtimes")
        .join(format!("{engine}-{lv}-{ver}-{platform}"));
    std::fs::create_dir_all(&dir).expect("runtime dir");
    let exe = dir.join(format!("tebako-runtime-{ver}-{lv}-{platform}{suffix}"));
    let exe_bytes = b"fake runtime exe\n";
    std::fs::write(&exe, exe_bytes).expect("exe");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    std::fs::write(
        dir.join("sha256"),
        format!(
            "{}  tebako-runtime-{ver}-{lv}-{platform}{suffix}\n",
            sha256_hex(exe_bytes)
        ),
    )
    .ok();
    if with_image {
        let image = dir.join(format!("tebako-runtime-{ver}-{lv}-{platform}.tfs"));
        let bytes = b"fake runtime image\n";
        std::fs::write(&image, bytes).expect("image");
        std::fs::write(
            dir.join(format!("tebako-runtime-{ver}-{lv}-{platform}.tfs.sha256")),
            format!(
                "{}  tebako-runtime-{ver}-{lv}-{platform}.tfs\n",
                sha256_hex(bytes)
            ),
        )
        .expect("image marker");
    }
    exe
}

/// A file:// runtime mirror holding one release (`v<ver>`) with exe,
/// image, and a manifest.json index in the factory's shape (spec 13):
/// one entry per package, the image nested under the additive `image`
/// key.
pub fn write_mirror(root: &Path, lv: &str, ver: &str, tamper: bool) -> PathBuf {
    let platform = platform();
    let dir = root.join(format!("v{ver}"));
    std::fs::create_dir_all(&dir).expect("mirror dir");
    // The release asset names the resolver derives: the exe carries the
    // platform suffix (.exe on Windows); the IMAGE is named off the
    // suffix-free asset base (<base>.tfs — never <base>.exe.tfs).
    let asset_base = format!("tebako-runtime-{ver}-{lv}-{platform}");
    let exe_name = format!("{asset_base}{}", tebako_shim::runtime::exe_suffix());
    let image_name = format!("{asset_base}.tfs");
    let exe_bytes = b"mirrored runtime exe\n";
    let image_bytes = b"mirrored runtime image\n";
    std::fs::write(dir.join(&exe_name), exe_bytes).expect("exe");
    std::fs::write(dir.join(&image_name), image_bytes).expect("image");
    let exe_sha = sha256_hex(exe_bytes);
    let image_sha = if tamper {
        "f".repeat(64)
    } else {
        sha256_hex(image_bytes)
    };
    std::fs::write(
        dir.join("manifest.json"),
        // The era-2 factory shape (spec 18 C2): the contract set is
        // declared per entry — the pre-download gate refuses anything
        // less.
        format!(
            "[{{\"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"filename\": \"{exe_name}\", \"sha256\": \"{exe_sha}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{image_sha}\"}}}}]\n"
        ),
    )
    .expect("manifest.json");
    root.to_path_buf()
}

/// `write_mirror` plus the windows ruby DLL facet
/// (tebako-runtime-ruby#40): the release also carries `<asset_base>.dll`
/// and the manifest entry declares the additive `dll` key with the PE
/// name (`install_as`) the exe imports. `tamper` poisons the DLL's
/// declared sha256. Returns (mirror root, install_as).
pub fn write_mirror_dll(root: &Path, lv: &str, ver: &str, tamper: bool) -> (PathBuf, String) {
    let platform = platform();
    let dir = root.join(format!("v{ver}"));
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let asset_base = format!("tebako-runtime-{ver}-{lv}-{platform}");
    let exe_name = format!("{asset_base}{}", tebako_shim::runtime::exe_suffix());
    let image_name = format!("{asset_base}.tfs");
    let dll_name = format!("{asset_base}.dll");
    let install_as = "x64-ucrt-ruby330.dll";
    let exe_bytes = b"mirrored runtime exe\n";
    let image_bytes = b"mirrored runtime image\n";
    let dll_bytes = b"mirrored ruby dll\n";
    std::fs::write(dir.join(&exe_name), exe_bytes).expect("exe");
    std::fs::write(dir.join(&image_name), image_bytes).expect("image");
    std::fs::write(dir.join(&dll_name), dll_bytes).expect("dll");
    let exe_sha = sha256_hex(exe_bytes);
    let image_sha = sha256_hex(image_bytes);
    let dll_sha = if tamper {
        "f".repeat(64)
    } else {
        sha256_hex(dll_bytes)
    };
    std::fs::write(
        dir.join("manifest.json"),
        // The era-2 factory shape with the additive `dll` key
        // (tebako-runtime-ruby#40) alongside the `image` key.
        format!(
            "[{{\"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"filename\": \"{exe_name}\", \"sha256\": \"{exe_sha}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{image_sha}\"}}, \"dll\": {{\"filename\": \"{dll_name}\", \"install_as\": \"{install_as}\", \"sha256\": \"{dll_sha}\"}}}}]\n"
        ),
    )
    .expect("manifest.json");
    (root.to_path_buf(), install_as.to_string())
}

pub fn write_config(home: &Path, yaml: &str) {
    std::fs::create_dir_all(home).expect("home");
    std::fs::write(home.join("config.yaml"), yaml).expect("config");
}

/// A file:// runtime mirror holding one release (`v<ver>`) whose
/// manifest.json is a MULTI-entry release index in the factory's locked
/// shape (spec 13 §2a): every entry declares `tebako_version`,
/// `ruby_version`, `platform`, and the era-2 contract set. Exe + image
/// assets are written for every listed version, so any index-selected
/// download target resolves against real, verifiable bytes.
pub fn write_release_index(root: &Path, ver: &str, lvs: &[&str]) -> PathBuf {
    let platform = platform();
    let dir = root.join(format!("v{ver}"));
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let mut entries = Vec::new();
    for lv in lvs {
        let asset_base = format!("tebako-runtime-{ver}-{lv}-{platform}");
        let exe_name = format!("{asset_base}{}", tebako_shim::runtime::exe_suffix());
        let image_name = format!("{asset_base}.tfs");
        let exe_bytes = format!("mirrored runtime exe {lv}\n");
        let image_bytes = format!("mirrored runtime image {lv}\n");
        std::fs::write(dir.join(&exe_name), &exe_bytes).expect("exe");
        std::fs::write(dir.join(&image_name), &image_bytes).expect("image");
        entries.push(format!(
            "{{\"tebako_version\": \"{ver}\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"ruby_version\": \"{lv}\", \"platform\": \"{platform}\", \"filename\": \"{exe_name}\", \"sha256\": \"{}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{}\"}}}}",
            sha256_hex(exe_bytes.as_bytes()),
            sha256_hex(image_bytes.as_bytes())
        ));
    }
    std::fs::write(
        dir.join("manifest.json"),
        format!("[{}]\n", entries.join(", ")),
    )
    .expect("manifest.json");
    root.to_path_buf()
}

/// `write_release_index` with an explicit exe asset spelling (spec 05 §2
/// SSOT; tebako#456): the release index's `filename` is the ONLY
/// authoritative asset spelling — the factory publishes windows exe
/// assets SUFFIX-LESS, so the fixture must carry any spelling the index
/// declares. The image keeps the exe's stem plus `.tfs`.
pub fn write_release_index_renamed(root: &Path, ver: &str, lv: &str, exe_name: &str) -> PathBuf {
    let platform = platform();
    let dir = root.join(format!("v{ver}"));
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let stem = exe_name
        .strip_suffix(tebako_shim::runtime::exe_suffix())
        .unwrap_or(exe_name);
    let image_name = format!("{stem}.tfs");
    let exe_bytes = format!("mirrored runtime exe {lv}\n");
    let image_bytes = format!("mirrored runtime image {lv}\n");
    std::fs::write(dir.join(exe_name), &exe_bytes).expect("exe");
    std::fs::write(dir.join(&image_name), &image_bytes).expect("image");
    std::fs::write(
        dir.join("manifest.json"),
        // The era-2 factory shape (spec 18 C2) with the identity triple
        // the matcher reads (spec 05 §2) — `filename` verbatim.
        format!(
            "[{{\"tebako_version\": \"{ver}\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"ruby_version\": \"{lv}\", \"platform\": \"{platform}\", \"filename\": \"{exe_name}\", \"sha256\": \"{}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{}\"}}}}]\n",
            sha256_hex(exe_bytes.as_bytes()),
            sha256_hex(image_bytes.as_bytes())
        ),
    )
    .expect("manifest.json");
    root.to_path_buf()
}

/// `write_runtime_abi` plus a release index manifest carrying the runtime's
/// own `abi` string (spec 13's per-package `abi` key) — the field the
/// abi-line filter reads.
pub fn write_runtime_abi(home: &Path, lv: &str, ver: &str, abi: Option<&str>) -> PathBuf {
    let exe = write_runtime(home, lv, ver, false);
    if let Some(abi) = abi {
        let dir = exe.parent().expect("runtime entry dir").to_path_buf();
        let exe_name = exe.file_name().expect("exe name").to_string_lossy();
        std::fs::write(
            dir.join("manifest.json"),
            format!("[{{\"filename\": \"{exe_name}\", \"abi\": \"{abi}\"}}]\n"),
        )
        .expect("manifest.json");
    }
    exe
}

/// `write_runtime_engine` (with the env image — spawn edges require it)
/// plus a release-index mirror carrying the per-entry `abi` /
/// `implementation` keys the spawned-edge filters read (spec 30 §1).
pub fn write_runtime_engine_meta(
    home: &Path,
    engine: &str,
    lv: &str,
    ver: &str,
    abi: Option<&str>,
    implementation: Option<&str>,
) -> PathBuf {
    let exe = write_runtime_engine(home, engine, lv, ver, true);
    let dir = exe.parent().expect("runtime entry dir").to_path_buf();
    let exe_name = exe.file_name().expect("exe name").to_string_lossy();
    let mut keys = String::new();
    if let Some(abi) = abi {
        keys.push_str(&format!(", \"abi\": \"{abi}\""));
    }
    if let Some(imp) = implementation {
        keys.push_str(&format!(", \"implementation\": \"{imp}\""));
    }
    std::fs::write(
        dir.join("manifest.json"),
        format!("[{{\"filename\": \"{exe_name}\"{keys}}}]\n"),
    )
    .expect("manifest.json");
    exe
}

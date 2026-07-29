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
    let platform = platform();
    let suffix = tebako_shim::runtime::exe_suffix();
    let dir = home
        .join("runtimes")
        .join(format!("ruby-{lv}-{ver}-{platform}"));
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
/// image, and a manifest.json index.
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
        format!(
            "[{{\"filename\": \"{exe_name}\", \"sha256\": \"{exe_sha}\"}},\n {{\"filename\": \"{image_name}\", \"sha256\": \"{image_sha}\"}}]\n"
        ),
    )
    .expect("manifest.json");
    root.to_path_buf()
}

pub fn write_config(home: &Path, yaml: &str) {
    std::fs::create_dir_all(home).expect("home");
    std::fs::write(home.join("config.yaml"), yaml).expect("config");
}

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
/// included (spec: `exe_suffix()` on Windows). The fixture stages the
/// pre-tebako#716 (engine-less) spelling deliberately: that is the
/// immutable old-era shape every ≤ v0.16.32 line keeps forever, and the
/// install path under test flows asset names from index entries,
/// era-agnostic (tebako#716's era law — a new-era entry's
/// `<ver>-<engine>-<lv>` spelling rides the exact same code).
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
    // The pre-tebako#716 (old-era) asset spelling — deliberate: see
    // `write_runtime`'s doc comment (the era law's era-agnostic flow).
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

/// `write_runtime_engine` plus a release-index shard whose entry carries
/// verbatim extra JSON keys (spec 33 fixtures: the `on_runtime` mirror on
/// the depending runtime; `contract_version` + `implementation` on the
/// owner). `extra_json` is spliced after `"filename": "<exe>"` — lead
/// with a comma (`, "on_runtime": {…}`).
pub fn write_runtime_engine_shard(
    home: &Path,
    engine: &str,
    lv: &str,
    ver: &str,
    with_image: bool,
    extra_json: &str,
) -> PathBuf {
    let exe = write_runtime_engine(home, engine, lv, ver, with_image);
    let dir = exe.parent().expect("runtime entry dir").to_path_buf();
    let exe_name = exe.file_name().expect("exe name").to_string_lossy();
    std::fs::write(
        dir.join("manifest.json"),
        format!("[{{\"filename\": \"{exe_name}\"{extra_json}}}]\n"),
    )
    .expect("manifest.json");
    exe
}

/// A shard-era (roadmap 85, spec 05 §2) file:// release mirror: the
/// per-package shard `<stem>.manifest.json` (ONE manifest-entry object,
/// never an array) + per-asset `<asset>.sha256` sidecars, and NO
/// monoliths (no manifest.json, no SHA256SUMS.txt — the post-85 line).
/// When `key` is Some the shard and both assets carry their detached
/// ascs and the shard entry declares the per-asset `signature` blocks
/// (spec 13 §2a's shape, one-entry deep). `tag` is the release directory
/// — decoupled from the row's tebako line (the openjdk v2.5.1 shape).
/// Returns the release directory.
#[allow(clippy::too_many_arguments)]
pub fn write_shard_release(
    root: &Path,
    engine: &str,
    lv: &str,
    tebako: &str,
    tag: &str,
    key: Option<&tebako_signer::PressKey>,
    declared_keyid: Option<&str>,
) -> PathBuf {
    let platform = platform();
    let dir = root.join(tag);
    std::fs::create_dir_all(&dir).expect("mirror dir");
    let asset_base = format!("tebako-runtime-{tebako}-{lv}-{platform}");
    let exe_name = format!("{asset_base}{}", tebako_shim::runtime::exe_suffix());
    let image_name = format!("{asset_base}.tfs");
    let exe_bytes = format!("shard-era runtime exe {lv}\n");
    let image_bytes = format!("shard-era runtime image {lv}\n");
    let exe_sha = sha256_hex(exe_bytes.as_bytes());
    let image_sha = sha256_hex(image_bytes.as_bytes());
    std::fs::write(dir.join(&exe_name), &exe_bytes).expect("exe");
    std::fs::write(dir.join(&image_name), &image_bytes).expect("image");
    std::fs::write(
        dir.join(format!("{exe_name}.sha256")),
        format!("{exe_sha}  {exe_name}\n"),
    )
    .expect("exe sidecar");
    std::fs::write(
        dir.join(format!("{image_name}.sha256")),
        format!("{image_sha}  {image_name}\n"),
    )
    .expect("image sidecar");
    let declared = key.map(|k| {
        declared_keyid
            .map(str::to_string)
            .unwrap_or_else(|| tebako_signer::hex_lower(&k.keyid))
    });
    let signature_block = |asc: &str| match &declared {
        Some(keyid) => format!(", \"signature\": {{\"keyid\": \"{keyid}\", \"asc\": \"{asc}\"}}"),
        None => String::new(),
    };
    let shard = format!(
        "{{\"tebako_version\": \"{tebako}\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"{engine}_version\": \"{lv}\", \"platform\": \"{platform}\", \"filename\": \"{exe_name}\", \"sha256\": \"{exe_sha}\"{}, \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{image_sha}\"{}}}}}\n",
        signature_block(&format!("{exe_name}.asc")),
        signature_block(&format!("{image_name}.asc")),
    );
    std::fs::write(dir.join(format!("{asset_base}.manifest.json")), &shard).expect("shard");
    if let Some(k) = key {
        let sign = |data: &[u8]| {
            tebako_signer::sign_detached(data, &k.secret_key, &k.fingerprint).expect("sign")
        };
        std::fs::write(
            dir.join(format!("{asset_base}.manifest.json.asc")),
            sign(shard.as_bytes()),
        )
        .expect("shard asc");
        std::fs::write(
            dir.join(format!("{exe_name}.asc")),
            sign(exe_bytes.as_bytes()),
        )
        .expect("exe asc");
        std::fs::write(
            dir.join(format!("{image_name}.asc")),
            sign(image_bytes.as_bytes()),
        )
        .expect("image asc");
    }
    dir
}

// ---------------------------------------------------------------------
// The OCI distribution mock (spec 38 §5) — URL-keyed answers over the
// manifest/blob verbs, every request logged. Serves both the registry
// index (the RegistryIndex class) and the runtime bundle (RuntimeBundle),
// so one endpoint can play a whole OCI-hosted registry. Arc-shared: the
// priming fetcher and the runtime fetch ride clones of the one mock.
// ---------------------------------------------------------------------

/// One canned distribution answer.
#[derive(Clone)]
pub struct OciAnswer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

struct OciDistInner {
    answers: std::sync::Mutex<std::collections::HashMap<String, OciAnswer>>,
    seen: std::sync::Mutex<Vec<String>>,
}

/// The mock distribution endpoint: URL-keyed answers, every request
/// logged (the never-fetch assertions read the log). Unknown URLs answer
/// 404 with a distribution error body — the same classification input a
/// real registry serves.
#[derive(Clone)]
pub struct OciDist {
    inner: std::sync::Arc<OciDistInner>,
}

impl OciDist {
    pub fn new() -> Self {
        OciDist {
            inner: std::sync::Arc::new(OciDistInner {
                answers: std::sync::Mutex::new(std::collections::HashMap::new()),
                seen: std::sync::Mutex::new(Vec::new()),
            }),
        }
    }

    /// Add one 200 answer (answers are immutable content once served —
    /// re-keying an existing URL overwrites, never appends).
    pub fn ok(&self, url: &str, headers: &[(&str, String)], body: &[u8]) {
        self.inner.answers.lock().unwrap().insert(
            url.to_string(),
            OciAnswer {
                status: 200,
                headers: headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: body.to_vec(),
            },
        );
    }

    pub fn requested(&self, needle: &str) -> bool {
        self.inner
            .seen
            .lock()
            .unwrap()
            .iter()
            .any(|u| u.contains(needle))
    }

    /// Pull one answer — the broken-arm fixture (the URL falls through to
    /// the 404 default).
    pub fn drop_answer(&self, url: &str) {
        self.inner.answers.lock().unwrap().remove(url);
    }
}

impl tebako_resolve::Transport for OciDist {
    fn get(&self, url: &str) -> Result<Vec<u8>, tebako_http::FetchError> {
        // The plan pipeline's OCI stream buffers through `get`.
        self.inner.seen.lock().unwrap().push(url.to_string());
        match self.inner.answers.lock().unwrap().get(url) {
            Some(a) if a.status == 200 => Ok(a.body.clone()),
            _ => Err(tebako_http::FetchError::IndexUnavailable(format!(
                "{url} (the OCI mock serves mapped 200 answers only)"
            ))),
        }
    }

    fn get_raw(
        &self,
        url: &str,
        _accept: Option<&str>,
        _header: Option<(&str, &str)>,
    ) -> Result<tebako_http::RawResponse, tebako_http::FetchError> {
        self.inner.seen.lock().unwrap().push(url.to_string());
        match self.inner.answers.lock().unwrap().get(url) {
            Some(a) => Ok(tebako_http::RawResponse::new(
                a.status,
                a.headers.clone(),
                a.body.clone(),
            )),
            None => Ok(tebako_http::RawResponse::new(
                404,
                Vec::new(),
                br#"{"errors":[{"code":"MANIFEST_UNKNOWN","message":"mock: unknown url"}]}"#
                    .to_vec(),
            )),
        }
    }
}

/// The OCI credential chain's tier 3 reads `$DOCKER_CONFIG/config.json` —
/// the developer machine's own config must not leak in (a shared empty
/// dir; the absent file is the anonymous ride).
pub fn isolate_docker_config() {
    let dir = std::env::temp_dir().join(format!(
        "tebako-shim-common-oci-docker-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("DOCKER_CONFIG", &dir);
}

/// The spec 38 §3 wire constants — the fixtures pin the exact shape the
/// adapter parses.
pub const OCI_MANIFEST_MT: &str = "application/vnd.oci.image.manifest.v1+json";
pub const OCI_EMPTY_CONFIG_MT: &str = "application/vnd.oci.empty.v1+json";
pub const OCI_EMPTY_CONFIG_DIGEST: &str =
    "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";
pub const OCI_ANNOTATION_TITLE: &str = "org.opencontainers.image.title";
pub const OCI_ANNOTATION_SHARD: &str = "org.tebako.runtime.shard";

fn oci_json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// One OCI manifest body (spec 38 §3's shape): the canonical empty
/// config, exactly one layer, the title + extra annotations. Returns
/// (body, manifest hex digest).
pub fn oci_manifest(
    class: tebako_resolve::ArtifactClass,
    title: &str,
    blob: &[u8],
    extra: &[(&str, &str)],
) -> (Vec<u8>, String) {
    let layer_hex = sha256_hex(blob);
    let extras = extra
        .iter()
        .map(|(k, v)| format!(r#", "{k}": "{}""#, oci_json_escape(v)))
        .collect::<String>();
    let body = format!(
        r#"{{"schemaVersion": 2, "mediaType": "{OCI_MANIFEST_MT}",
  "artifactType": "{}",
  "config": {{"mediaType": "{OCI_EMPTY_CONFIG_MT}", "digest": "{OCI_EMPTY_CONFIG_DIGEST}", "size": 2}},
  "layers": [{{"mediaType": "{}", "digest": "sha256:{layer_hex}", "size": {}}}],
  "annotations": {{"{OCI_ANNOTATION_TITLE}": "{title}"{extras}}}}}"#,
        class.artifact_type(),
        class.layer_media_type(),
        blob.len(),
    );
    let hex = sha256_hex(body.as_bytes());
    (body.into_bytes(), hex)
}

/// Serve a registry index over the OCI verbs (spec 38 §4): the `latest`
/// tag's manifest, the digest re-resolve, and the YAML blob. Returns the
/// index's `tfs+oci:` ref for the book entry.
pub fn serve_oci_registry_index(mock: &OciDist, host: &str, ns: &str, yaml: &str) -> String {
    let (manifest, manifest_hex) = oci_manifest(
        tebako_resolve::ArtifactClass::RegistryIndex,
        "tpkg-registry.yaml",
        yaml.as_bytes(),
        &[],
    );
    let blob_hex = sha256_hex(yaml.as_bytes());
    let base = format!("https://{host}/v2/{ns}");
    mock.ok(
        &format!("{base}/manifests/latest"),
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    mock.ok(
        &format!("{base}/manifests/sha256:{manifest_hex}"),
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    mock.ok(
        &format!("{base}/blobs/sha256:{blob_hex}"),
        &[],
        yaml.as_bytes(),
    );
    format!("tfs+oci://{host}/{ns}:latest")
}

/// What [`serve_oci_runtime`] published — the card the test asserts on.
pub struct OciRuntimePublish {
    /// The bundle stem (`tebako-runtime-<ver>-<impl>-<lv>-<platform>`),
    /// also the manifest tag.
    pub stem: String,
    pub manifest_hex: String,
    pub bundle_hex: String,
    pub exe: Vec<u8>,
    pub image: Vec<u8>,
}

/// A spec 38 §5.4 runtime-bundle publish over the mock: the bundle bytes
/// (exe + image members + the closing SHA256SUMS, the spec 36 §2 order),
/// the stem-tag manifest whose shard annotation carries the spec 36 §3
/// card, the digest re-resolve, the blob. The stem spells the
/// post-tebako#716 grammar — pre-#716 lines never publish to OCI
/// (spec 38 §7).
pub fn serve_oci_runtime(
    mock: &OciDist,
    host: &str,
    repo: &str,
    engine: &str,
    implementation: &str,
    lv: &str,
    tebako: &str,
) -> OciRuntimePublish {
    let platform = platform();
    let stem = format!("tebako-runtime-{tebako}-{implementation}-{lv}-{platform}");
    let bundle_name = format!("{stem}.tar.gz");
    let image_name = format!("{stem}.tfs");
    let exe = format!("the OCI {engine} interpreter {lv}\n").into_bytes();
    let image = format!("the OCI {engine} env image {lv}\n").into_bytes();
    let mut sums = String::new();
    let mut members: Vec<(String, Vec<u8>, u32)> = Vec::new();
    for (name, bytes, mode) in [
        (stem.clone(), exe.clone(), 0o755u32),
        (image_name.clone(), image.clone(), 0o444u32),
    ] {
        sums.push_str(&format!("{}  {}\n", sha256_hex(&bytes), name));
        members.push((name, bytes, mode));
    }
    members.push(("SHA256SUMS".to_string(), sums.into_bytes(), 0o444));
    let mut bundle: Vec<u8> = Vec::new();
    {
        let gz = flate2::write::GzEncoder::new(&mut bundle, flate2::Compression::none());
        let mut builder = tar::Builder::new(gz);
        for (name, bytes, mode) in &members {
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(tar::EntryType::Regular);
            h.set_size(bytes.len() as u64);
            h.set_mode(*mode);
            builder.append_data(&mut h, name, bytes.as_slice()).unwrap();
        }
        builder.finish().unwrap();
    }
    let bundle_hex = sha256_hex(&bundle);
    // The shard the manifest's annotation carries (spec 36 §3): the
    // identity triple, the per-member pins, the bundle block.
    let shard = format!(
        r#"{{"tebako_version": "{tebako}", "contract_era": 2, "contract_version": 2, "mount_root": "/__tfs__", "implementation": "{implementation}", "{engine}_version": "{lv}", "platform": "{platform}", "filename": "{stem}", "sha256": "{}", "image": {{"filename": "{image_name}", "sha256": "{}"}}, "bundle": {{"filename": "{bundle_name}", "sha256": "{bundle_hex}"}}}}"#,
        sha256_hex(&exe),
        sha256_hex(&image),
    );
    let (manifest, manifest_hex) = oci_manifest(
        tebako_resolve::ArtifactClass::RuntimeBundle,
        &bundle_name,
        &bundle,
        &[(OCI_ANNOTATION_SHARD, &shard)],
    );
    let base = format!("https://{host}/v2/{repo}");
    mock.ok(
        &format!("{base}/manifests/{stem}"),
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    mock.ok(
        &format!("{base}/manifests/sha256:{manifest_hex}"),
        &[("Docker-Content-Digest", format!("sha256:{manifest_hex}"))],
        &manifest,
    );
    mock.ok(
        &format!("{base}/blobs/sha256:{bundle_hex}"),
        &[],
        &bundle,
    );
    OciRuntimePublish {
        stem,
        manifest_hex,
        bundle_hex,
        exe,
        image,
    }
}

/// The `channel: oci` runtime registry YAML (spec 38 §11): one runtime
/// row whose per-triplet `oci:` mirror is THE locator (the git-host
/// release.ref rides along — dual publish, the migration-era shape). The
/// row's sha256 pin and the mirror's `?sha256=` bind the same bundle
/// bytes (§11's "the pin binds both").
pub fn channel_oci_registry_yaml(
    host: &str,
    repo: &str,
    engine: &str,
    implementation: &str,
    lv: &str,
    tebako: &str,
    publish: &OciRuntimePublish,
) -> String {
    let host_platform = tpkg::Platform::host();
    let triplet = host_platform.as_triplet();
    let asset = host_platform.release_asset_name();
    format!(
        "schema_version: 1\npayloads:\n  - name: tebako-runtime-{implementation}\n    kind: runtime\n    engine: {engine}\n    implementation: {implementation}\n    versions:\n      - version: '{lv}'\n        platforms:\n          {triplet}: {{artifact: tebako-runtime-{tebako}-{implementation}-{lv}-{asset}.tar.gz, sha256: '{}', oci: 'tfs+oci://{host}/{repo}:{}?sha256={}'}}\n        release: {{ref: 'tfs:github:acme/tebako-runtime-{implementation}:v{tebako}'}}\n",
        publish.bundle_hex, publish.stem, publish.bundle_hex,
    )
}

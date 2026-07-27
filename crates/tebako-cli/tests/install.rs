//! Install/registry surface tests (spec 04 §2, spec 16 §3.3): the
//! add-registry flow, the nickname resolution matrix, the ref form,
//! signature trust, the embedded-manifest preference, and uninstall.
//! Everything runs against `file://` mirrors and temp TEBAKO_HOMEs — no
//! network, no env mutation (the env-mutating legs live in
//! install_env.rs, a separate test binary).

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use tebako_cli::install;
use tebako_resolve::{sha256_hex, Fetcher, Transport};
use tpkg::Platform;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-install-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A temp TEBAKO_HOME plus a mirror dir holding payload/registry files.
struct Fixture {
    dir: PathBuf,
    home: PathBuf,
    mirror: PathBuf,
    shim_binary: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        // a stand-in dispatcher binary the shims link to
        let shim_binary = dir.join("tebako-shim");
        fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
        Fixture {
            dir,
            home,
            mirror,
            shim_binary,
        }
    }

    fn payload(&self, file: &str, bytes: &[u8]) -> String {
        fs::write(self.mirror.join(file), bytes).unwrap();
        format!("file://{}/{}", self.mirror.display(), file)
    }

    fn registry(&self, file: &str, yaml: &str) -> String {
        fs::write(self.mirror.join(file), yaml).unwrap();
        format!("file://{}/{}", self.mirror.display(), file)
    }

    fn payloads_dir(&self) -> PathBuf {
        self.home.join("payloads")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn sha(byte: u8) -> String {
    String::from(char::from(byte)).repeat(64)
}

/// The spec example registry, pointed at file:// mirrors.
fn registry_yaml(name: &str, version: &str, payload_ref: &str, default: Option<&str>) -> String {
    let mut yaml = format!(
        "schema_version: 1\npayloads:\n  - name: {name}\n    kind: app\n    versions:\n      - version: {version}\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        entrypoints: [{name}]\n"
    );
    if let Some(d) = default {
        yaml.push_str(&format!("    default: {d}\n"));
    }
    yaml
}

// ---------------------------------------------------------------------
// add-registry / list-registries
// ---------------------------------------------------------------------

#[test]
fn add_registry_registers_and_preserves_config_keys() {
    let fx = Fixture::new("addreg");
    fs::write(
        fx.home.join("config.yaml"),
        "defaults:\n  metanorma: 1.2.3\n",
    )
    .unwrap();
    let payload_ref = fx.payload("app-1.0.tfs", b"app-bytes");
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );

    let (outcome, registry) = install::add_registry(&fx.home, &reg_ref).unwrap();
    assert_eq!(outcome, tebako_shim::config::AddRegistryOutcome::Added);
    assert_eq!(registry.payloads.len(), 1);

    // existing keys survive; the ref is registered once
    let cfg = tebako_shim::config::load_config(&fx.home).unwrap();
    assert_eq!(cfg.defaults.get("metanorma").unwrap(), "1.2.3");
    assert_eq!(cfg.registries, vec![reg_ref.clone()]);
    assert_eq!(
        install::list_registries(&fx.home).unwrap(),
        vec![reg_ref.clone()]
    );

    let (outcome, _) = install::add_registry(&fx.home, &reg_ref).unwrap();
    assert_eq!(
        outcome,
        tebako_shim::config::AddRegistryOutcome::AlreadyPresent
    );
    assert_eq!(install::list_registries(&fx.home).unwrap().len(), 1);
}

#[test]
fn add_registry_rejects_bad_refs_and_unparsable_registries() {
    let fx = Fixture::new("addregbad");
    let err = install::add_registry(&fx.home, "metanorma").unwrap_err();
    assert!(
        err.message.contains("invalid registry reference"),
        "{err:?}"
    );

    let bad = fx.registry("bad.yaml", "schema_version: 99\n");
    let err = install::add_registry(&fx.home, &bad).unwrap_err();
    assert!(err.message.contains("schema_version 99"), "{err:?}");

    let missing = format!("file://{}/missing.yaml", fx.mirror.display());
    assert!(install::add_registry(&fx.home, &missing).is_err());
    // nothing was registered
    assert!(install::list_registries(&fx.home).unwrap().is_empty());
}

// ---------------------------------------------------------------------
// the nickname resolution matrix (spec 16 §3.3)
// ---------------------------------------------------------------------

#[test]
fn install_nickname_without_registries_is_a_named_error_with_the_hint() {
    let fx = Fixture::new("nick0");
    let err = install::install(&fx.home, "metanorma", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(
        err.message
            .contains("no registered registry carries a payload named 'metanorma'"),
        "{err:?}"
    );
    assert!(err.message.contains("(none)"), "{err:?}");
    assert!(err.message.contains("tebako add-registry <ref>"), "{err:?}");
}

#[test]
fn install_nickname_unknown_in_registered_registries_lists_them() {
    let fx = Fixture::new("nick1");
    let payload_ref = fx.payload("app-1.0.tfs", b"app-bytes");
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "metanorma", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(
        err.message
            .contains("no registered registry carries a payload named 'metanorma'"),
        "{err:?}"
    );
    assert!(err.message.contains(&reg_ref), "{err:?}");
}

#[test]
fn install_nickname_ambiguous_is_a_named_error_listing_the_registries() {
    let fx = Fixture::new("nick2");
    let payload_ref = fx.payload("app-1.0.tfs", b"app-bytes");
    let reg_a = fx.registry(
        "a.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    let reg_b = fx.registry(
        "b.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_a).unwrap();
    install::add_registry(&fx.home, &reg_b).unwrap();

    let err = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("AmbiguousRegistries"), "{err:?}");
    assert!(
        err.message.contains(&reg_a) && err.message.contains(&reg_b),
        "{err:?}"
    );
}

#[test]
fn install_nickname_resolves_default_and_explicit_versions() {
    let fx = Fixture::new("nick3");
    let p10 = fx.payload("app-1.0.tfs", b"v1.0-bytes");
    let p11 = fx.payload("app-1.1.tfs", b"v1.1-bytes");
    let yaml = format!(
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {p10}}}\n        entrypoints: [app]\n      - version: 1.1\n        platforms: universal\n        release: {{ref: {p11}}}\n        entrypoints: [app]\n    default: 1.1\n"
    );
    install::add_registry(&fx.home, &fx.registry("tpkg-registry.yaml", &yaml)).unwrap();

    // no @ver → the registry default
    let out = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.version, "1.1");
    assert_eq!(out.status, tebako_resolve::InstallStatus::Installed);
    assert_eq!(fs::read(&out.path).unwrap(), b"v1.1-bytes");
    assert_eq!(out.commands, vec!["app"]);
    assert_eq!(out.shims.len(), 1);
    assert!(out.shims[0].is_symlink() || out.shims[0].is_file());
    // the mirror record + trust anchor exist
    assert!(fx.payloads_dir().join("app/1.1.tfs.sha256").is_file());
    assert!(fx.payloads_dir().join("app/1.1.manifest.yaml").is_file());

    // explicit @ver
    let out = install::install(&fx.home, "app@1.0", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.version, "1.0");
    assert_eq!(fs::read(&out.path).unwrap(), b"v1.0-bytes");

    // reinstall → cache hit, no re-verify
    let out = install::install(&fx.home, "app@1.0", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.status, tebako_resolve::InstallStatus::Hit);

    // unknown version → named error listing the available ones
    let err = install::install(&fx.home, "app@9.9", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(
        err.message.contains("has no version '9.9' of 'app'"),
        "{err:?}"
    );
    assert!(
        err.message.contains("1.0") && err.message.contains("1.1"),
        "{err:?}"
    );
}

#[test]
fn install_nickname_without_default_needs_an_explicit_version() {
    let fx = Fixture::new("nick4");
    let payload_ref = fx.payload("app-1.0.tfs", b"app-bytes");
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, None),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("pins no default for 'app'"), "{err:?}");
    assert!(
        err.message.contains("tebako install app@<version>"),
        "{err:?}"
    );

    let out = install::install(&fx.home, "app@1.0", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.version, "1.0");
}

// ---------------------------------------------------------------------
// host-triplet selection (declarative; mock transports for the service leg)
// ---------------------------------------------------------------------

struct MockTransport {
    answers: HashMap<String, Vec<u8>>,
}
impl MockTransport {
    fn new() -> MockTransport {
        MockTransport {
            answers: HashMap::new(),
        }
    }
    fn with_file(mut self, url_path: &str) -> MockTransport {
        let bytes = fs::read(url_path).unwrap();
        self.answers.insert(format!("file://{url_path}"), bytes);
        self
    }
    fn with(mut self, url: &str, body: &[u8]) -> MockTransport {
        self.answers.insert(url.to_string(), body.to_vec());
        self
    }
}
impl Transport for MockTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, tebako_http::FetchError> {
        self.answers
            .get(url)
            .cloned()
            .ok_or_else(|| tebako_http::FetchError::IndexUnavailable(url.to_string()))
    }
}

#[test]
fn per_triplet_selection_fetches_the_host_artifact_with_the_registry_pin() {
    let fx = Fixture::new("triplet");
    let mac_bytes = b"mac-payload";
    let registry_path = fx.mirror.join("tpkg-registry.yaml");
    fs::write(
        &registry_path,
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms:\n          aarch64-macos:\n            artifact: app-1.0-macos.tfs\n            sha256: {}\n          x86_64-linux-gnu:\n            artifact: app-1.0-linux.tfs\n            sha256: {}\n        release: {{ref: tfs:github:acme/app:1.0}}\n        entrypoints: [app]\n    default: 1.0\n",
            sha256_hex(mac_bytes),
            sha(b'0')
        ),
    )
    .unwrap();
    let api = "https://api.github.com/repos/acme/app/releases/tags/1.0";
    let release = r#"{"assets":[
        {"name":"app-1.0-macos.tfs","browser_download_url":"https://dl/app-1.0-macos.tfs"},
        {"name":"app-1.0-linux.tfs","browser_download_url":"https://dl/app-1.0-linux.tfs"}]}"#;
    let t = MockTransport::new()
        .with_file(registry_path.to_str().unwrap())
        .with(api, release.as_bytes())
        .with("https://dl/app-1.0-macos.tfs", mac_bytes)
        // served bytes whose digest is NOT the registry's 000…0 pin
        .with("https://dl/app-1.0-linux.tfs", b"linux-payload");
    let fetcher = Fetcher::with_transport(t);

    let reg_ref = format!("file://{}", registry_path.display());
    install::add_registry_with(&fx.home, &reg_ref, &fetcher).unwrap();

    // a triplet whose registry pin does not match the bytes → sha error,
    // nothing cached (do this first: the version key is host-implicit and
    // the happy-path install below legitimately fills it)
    let err = install::install_with(
        &fx.home,
        "app",
        Some(Platform::X86_64LinuxGnu),
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap_err();
    assert_eq!(err.code, 70, "{err:?}");
    assert!(!fx.payloads_dir().join("app/1.0.tfs").exists());

    // the host triplet picks ITS artifact — declaratively, no adapter magic
    let out = install::install_with(
        &fx.home,
        "app",
        Some(Platform::Aarch64Macos),
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap();
    assert_eq!(fs::read(&out.path).unwrap(), b"mac-payload");
    assert_eq!(out.sha256, sha256_hex(mac_bytes));

    // a triplet the registry does not publish → the named error
    let err = install::install_with(
        &fx.home,
        "app",
        Some(Platform::X86_64WindowsUcrt),
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap_err();
    assert!(
        err.message
            .contains("is not published for the host triplet x86_64-windows-ucrt"),
        "{err:?}"
    );
    assert!(err.message.contains("aarch64-macos"), "{err:?}");
}

#[test]
fn universal_selection_uses_the_single_tfs_rule() {
    let fx = Fixture::new("universal");
    let registry_path = fx.mirror.join("tpkg-registry.yaml");
    fs::write(
        &registry_path,
        "schema_version: 1\npayloads:\n  - name: tool\n    kind: app\n    versions:\n      - version: 2.0\n        platforms: universal\n        release: {ref: tfs:github:acme/tool:2.0}\n        entrypoints: [tool]\n    default: 2.0\n",
    )
    .unwrap();
    let api = "https://api.github.com/repos/acme/tool/releases/tags/2.0";
    let release = r#"{"assets":[
        {"name":"tool-2.0.tfs","browser_download_url":"https://dl/tool-2.0.tfs"},
        {"name":"notes.txt","browser_download_url":"https://dl/notes.txt"}]}"#;
    let t = MockTransport::new()
        .with_file(registry_path.to_str().unwrap())
        .with(api, release.as_bytes())
        .with("https://dl/tool-2.0.tfs", b"tool-bytes");
    let fetcher = Fetcher::with_transport(t);
    install::add_registry_with(
        &fx.home,
        &format!("file://{}", registry_path.display()),
        &fetcher,
    )
    .unwrap();

    let out = install::install_with(
        &fx.home,
        "tool",
        Some(Platform::Aarch64Macos),
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap();
    assert_eq!(fs::read(&out.path).unwrap(), b"tool-bytes");
}

// ---------------------------------------------------------------------
// the ref form
// ---------------------------------------------------------------------

#[test]
fn install_pinned_file_reference_is_content_addressed() {
    let fx = Fixture::new("reffile");
    let payload_ref = fx.payload("app.tfs", b"app-bytes");
    let pin = sha256_hex(b"app-bytes");
    let out = install::install(
        &fx.home,
        &format!("{payload_ref}?sha256={pin}"),
        None,
        Some(&fx.shim_binary),
    )
    .unwrap();
    assert_eq!(out.name, "app");
    assert_eq!(out.version, pin);
    assert!(out.path.ends_with(format!("payloads/app/{pin}.tfs")));
    // the fallback synthesized a single shim named after the payload
    assert_eq!(out.commands, vec!["app"]);

    // a wrong pin → the named sha error, nothing cached
    let err = install::install(
        &fx.home,
        &format!("{payload_ref}?sha256={}", sha(b'0')),
        None,
        Some(&fx.shim_binary),
    )
    .unwrap_err();
    assert_eq!(err.code, 70, "{err:?}");

    // unpinned verbatim refs carry no version — a named error, never a guess
    let err = install::install(&fx.home, &payload_ref, None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("carries no version"), "{err:?}");
}

#[test]
fn install_service_reference_via_the_release_api() {
    let fx = Fixture::new("refsvc");
    let api = "https://api.github.com/repos/acme/tool/releases/tags/1.0";
    let release = r#"{"assets":[
        {"name":"tool-1.0-linux.tfs","browser_download_url":"https://dl/linux.tfs"},
        {"name":"tool-1.0-macos.tfs","browser_download_url":"https://dl/macos.tfs"}]}"#;
    let t = MockTransport::new()
        .with(api, release.as_bytes())
        .with("https://dl/macos.tfs", b"mac-img");
    let fetcher = Fetcher::with_transport(t);

    // multi-artifact without # → AmbiguousAssets naming every candidate
    let err = install::install_with(
        &fx.home,
        "tfs:github:acme/tool:1.0",
        None,
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap_err();
    assert!(
        err.message.contains("tool-1.0-linux.tfs") && err.message.contains("tool-1.0-macos.tfs"),
        "{err:?}"
    );

    // with # → exactly that asset
    let out = install::install_with(
        &fx.home,
        "tfs:github:acme/tool:1.0#tool-1.0-macos.tfs",
        None,
        Some(&fx.shim_binary),
        &fetcher,
    )
    .unwrap();
    assert_eq!(out.name, "tool");
    assert_eq!(out.version, "1.0");
    assert_eq!(fs::read(&out.path).unwrap(), b"mac-img");
}

// ---------------------------------------------------------------------
// signatures (spec 09)
// ---------------------------------------------------------------------

fn signed_fixture(tag: &str) -> (Fixture, String, String, Vec<u8>, String) {
    let fx = Fixture::new(tag);
    let key = tebako_signer::press_local_key(&fx.home).unwrap();
    let payload_ref = fx.payload("app-1.0.tfs", b"signed-bytes");
    let asc =
        tebako_signer::sign_detached(b"signed-bytes", &key.secret_key, &key.fingerprint).unwrap();
    let asc_ref = fx.payload("app-1.0.tfs.asc", &asc);
    let keyid = tebako_signer::hex_lower(&key.keyid);
    (fx, payload_ref, asc_ref, key.public_key.clone(), keyid)
}

fn signed_registry(payload_ref: &str, asc_ref: &str, keyid: &str) -> String {
    format!(
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        signature: {{keyid: \"{keyid}\", asc: \"{asc_ref}\"}}\n        entrypoints: [app]\n    default: 1.0\n"
    )
}

#[test]
fn signed_entry_verifies_against_the_trusted_keyring() {
    let (fx, payload_ref, asc_ref, public_key, keyid) = signed_fixture("sig1");
    tebako_signer::register_trusted(&fx.home, &public_key).unwrap();
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &signed_registry(&payload_ref, &asc_ref, &keyid),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let out = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.signer.as_deref(), Some(keyid.as_str()));
    let journal = fs::read_to_string(fx.home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=payload-signature-trusted"),
        "{journal}"
    );
}

#[test]
fn untrusted_signer_is_the_named_trust_error_and_caches_nothing() {
    let (fx, payload_ref, asc_ref, _public_key, keyid) = signed_fixture("sig2");
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &signed_registry(&payload_ref, &asc_ref, &keyid),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(err.code, 72, "{err:?}");
    assert!(
        err.message.contains("not in the trusted keyring"),
        "{err:?}"
    );
    assert!(!fx.payloads_dir().join("app/1.0.tfs").exists());
}

#[test]
fn invalid_signature_is_the_named_signature_error() {
    let (fx, payload_ref, _asc_ref, public_key, keyid) = signed_fixture("sig3");
    tebako_signer::register_trusted(&fx.home, &public_key).unwrap();
    // sign DIFFERENT bytes than the payload carries
    let key = tebako_signer::press_local_key(&fx.home).unwrap();
    let bad_asc =
        tebako_signer::sign_detached(b"other-bytes", &key.secret_key, &key.fingerprint).unwrap();
    let bad_asc_ref = fx.payload("bad.asc", &bad_asc);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &signed_registry(&payload_ref, &bad_asc_ref, &keyid),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap_err();
    assert_eq!(err.code, 71, "{err:?}");
    assert!(!fx.payloads_dir().join("app/1.0.tfs").exists());
}

#[test]
fn unsigned_entry_installs_with_the_legacy_warn_and_journal_line() {
    let fx = Fixture::new("sig4");
    let payload_ref = fx.payload("app-1.0.tfs", b"plain-bytes");
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let out = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.signer, None);
    let journal = fs::read_to_string(fx.home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=legacy-unsigned-accepted"),
        "{journal}"
    );
    assert!(journal.contains("event=payload-installed"), "{journal}");
}

// ---------------------------------------------------------------------
// the embedded manifest (tier 1, authoritative)
// ---------------------------------------------------------------------

fn zip_image_with_manifest(manifest_yaml: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("__tpkg__/manifest.yaml", options)
        .unwrap();
    writer.write_all(manifest_yaml.as_bytes()).unwrap();
    writer.start_file("app/bin/app", options).unwrap();
    writer.write_all(b"#!/bin/sh\n").unwrap();
    writer.finish().unwrap().into_inner()
}

fn embedded_manifest_yaml(name: &str, version: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {name}\n  version: {version}\n  producer: {{tool: tebako, tool_version: 0.15.9}}\n  created: \"2026-07-26T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {name}\n      path: /app/bin/{name}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
        sha(b'a'),
        sha(b'b')
    )
}

#[test]
fn embedded_manifest_drives_the_mirror_when_present() {
    let fx = Fixture::new("embedded");
    let image = zip_image_with_manifest(&embedded_manifest_yaml("app", "1.0"));
    let payload_ref = fx.payload("app-1.0.tfs", &image);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let out = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap();
    assert!(out.notes.is_empty(), "{:?}", out.notes);
    assert_eq!(out.commands, vec!["app"]);

    // the mirror comes from the embedded manifest (path + runtime), not
    // the /<command> fallback convention
    let mirror =
        tebako_shim::manifest::Manifest::load(&fx.payloads_dir().join("app/1.0.manifest.yaml"))
            .unwrap();
    let ep = mirror.entrypoint("app").unwrap();
    assert_eq!(ep.path, "/app/bin/app");
    let req = ep.runtime_requirement.as_ref().unwrap();
    assert_eq!(req.engine, "ruby");
    assert_eq!(req.constraint, ">= 3.3, < 5.0");
}

#[test]
fn embedded_manifest_mismatch_with_the_registry_is_a_named_error() {
    let fx = Fixture::new("embeddedbad");
    let image = zip_image_with_manifest(&embedded_manifest_yaml("app", "9.9"));
    let payload_ref = fx.payload("app-1.0.tfs", &image);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("app", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("inconsistent"), "{err:?}");
    assert!(err.message.contains("9.9"), "{err:?}");
}

#[test]
fn plain_bytes_fall_back_to_the_synthesized_mirror_with_a_note() {
    let fx = Fixture::new("fallback");
    let payload_ref = fx.payload("app-1.0.tfs", b"not-an-image");
    let yaml = format!(
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        runtime_requirement: {{engine: ruby, constraint: \"~> 3.3.0\"}}\n        entrypoints: [app, app-helper]\n    default: 1.0\n"
    );
    let reg_ref = fx.registry("tpkg-registry.yaml", &yaml);
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let out = install::install(&fx.home, "app", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.commands, vec!["app", "app-helper"]);
    assert_eq!(out.shims.len(), 2);
    assert_eq!(out.notes.len(), 1);
    assert!(
        out.notes[0].contains("no embedded manifest"),
        "{:?}",
        out.notes
    );

    let mirror =
        tebako_shim::manifest::Manifest::load(&fx.payloads_dir().join("app/1.0.manifest.yaml"))
            .unwrap();
    let ep = mirror.entrypoint("app-helper").unwrap();
    assert_eq!(ep.path, "/app-helper");
    assert_eq!(
        ep.runtime_requirement.as_ref().unwrap().constraint,
        "~> 3.3.0"
    );
}

// ---------------------------------------------------------------------
// uninstall
// ---------------------------------------------------------------------

#[test]
fn uninstall_removes_shims_and_cache_and_journals_the_anchors() {
    let fx = Fixture::new("uninstall");
    let p10 = fx.payload("app-1.0.tfs", b"v1.0-bytes");
    let p11 = fx.payload("app-1.1.tfs", b"v1.1-bytes");
    let yaml = format!(
        "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {p10}}}\n        entrypoints: [app]\n      - version: 1.1\n        platforms: universal\n        release: {{ref: {p11}}}\n        entrypoints: [app]\n    default: 1.1\n"
    );
    install::add_registry(&fx.home, &fx.registry("tpkg-registry.yaml", &yaml)).unwrap();
    install::install(&fx.home, "app@1.0", None, Some(&fx.shim_binary)).unwrap();
    install::install(&fx.home, "app@1.1", None, Some(&fx.shim_binary)).unwrap();
    assert!(fx.home.join("shims/app").exists());

    let out = install::uninstall(&fx.home, "app").unwrap();
    assert_eq!(out.versions, vec!["1.0", "1.1"]);
    assert_eq!(out.shims_removed.len(), 1);
    assert!(!fx.home.join("shims/app").exists());
    assert!(!fx.payloads_dir().join("app").exists());

    // the trust anchors survived in the audit journal
    let journal = fs::read_to_string(fx.home.join("journal.log")).unwrap();
    assert!(
        journal.contains("event=payload-uninstalled name=app version=1.0"),
        "{journal}"
    );
    assert!(journal.contains(&sha256_hex(b"v1.0-bytes")), "{journal}");
    assert!(
        journal.contains("event=payload-uninstalled name=app version=1.1"),
        "{journal}"
    );

    // uninstall is not idempotent: the named error
    let err = install::uninstall(&fx.home, "app").unwrap_err();
    assert!(err.message.contains("is not installed"), "{err:?}");
}

// ---------------------------------------------------------------------
// suite install (spec 03 §6, spec 07 §2.0): ONE package, N entries →
// N shims, each with its own slot and its own runtime requirement
// ---------------------------------------------------------------------

/// A suite tpkg: fake bootstrap bytes + N fake image slots + the type-2
/// package manifest pinning each entry to its own runtime.
fn suite_tpkg(name: &str, version: &str, entries: &[(&str, u32, &str)]) -> Vec<u8> {
    let bootstrap = b"fake suite bootstrap";
    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 0,
        ..Default::default()
    };
    // The v1 trailer field carries entries[0]'s ref (v1-era loaders).
    m.set_runtime_ref(entries[0].2.as_bytes());
    let mut pos = bootstrap.len() as u64;
    let mut images: Vec<Vec<u8>> = Vec::new();
    for (i, _) in entries.iter().enumerate() {
        let bytes = format!("fake suite image {i}").into_bytes();
        m.slots.push(tpkg::Slot::new(
            pos,
            bytes.len() as u64,
            tpkg::TPKG_FORMAT_DWARFS,
            "/__tebako_memfs__",
        ));
        pos += bytes.len() as u64;
        images.push(bytes);
    }
    m.set_package_manifest(&tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: name.to_string(),
            version: version.to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.15.9".to_string(),
            },
            created: "2026-07-27T00:00:00Z".to_string(),
        },
        entries: entries
            .iter()
            .map(|&(name, slot, runtime_ref)| tpkg::PackageEntry {
                name: name.to_string(),
                slot,
                entrypoint: name.to_string(),
                runtime_ref: runtime_ref.to_string(),
            })
            .collect(),
        jail: None,
        env: Default::default(),
    })
    .unwrap();
    let mut out = bootstrap.to_vec();
    for img in &images {
        out.extend_from_slice(img);
    }
    let mut cursor = std::io::Cursor::new(&mut out);
    tpkg::write_to(&mut cursor, &m).unwrap();
    out
}

/// A cached runtime entry (the dispatcher's resolution target), the shim
/// tests' fixture shape.
fn write_cached_runtime(home: &Path, lv: &str, ver: &str) -> PathBuf {
    let platform = tebako_shim::runtime::platform_string();
    let dir = home
        .join("runtimes")
        .join(format!("ruby-{lv}-{ver}-{platform}"));
    fs::create_dir_all(&dir).unwrap();
    let exe = dir.join(format!("tebako-runtime-{ver}-{lv}-{platform}"));
    fs::write(&exe, b"fake runtime exe\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }
    exe
}

#[test]
fn suite_install_registers_every_entry_shim_and_dispatches_per_entry() {
    let fx = Fixture::new("suite");
    let pkg = suite_tpkg(
        "hellosuite",
        "1.0",
        &[
            ("hello34", 0, "ruby@3.4.2;tebako=0.15.9"),
            ("hello33", 1, "ruby@3.3.7;tebako=0.15.9"),
        ],
    );
    let payload_ref = fx.payload("hellosuite-1.0.tpkg", &pkg);
    let yaml = format!(
        "schema_version: 1\npayloads:\n  - name: hellosuite\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n        entrypoints: [hello34, hello33]\n    default: 1.0\n"
    );
    let reg_ref = fx.registry("tpkg-registry.yaml", &yaml);
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let out = install::install(&fx.home, "hellosuite", None, Some(&fx.shim_binary)).unwrap();
    assert_eq!(out.commands, vec!["hello34", "hello33"]);
    assert_eq!(out.shims.len(), 2);
    assert!(fx.home.join("shims/hello34").exists());
    assert!(fx.home.join("shims/hello33").exists());

    // The mirror carries each entry's own slot and exact runtime pin.
    let mirror = tebako_shim::manifest::Manifest::load(
        &fx.payloads_dir().join("hellosuite/1.0.manifest.yaml"),
    )
    .unwrap();
    let e34 = mirror.entrypoint("hello34").unwrap();
    assert_eq!(e34.slot, 0);
    assert_eq!(e34.path, "hello34");
    let req34 = e34.runtime_requirement.as_ref().unwrap();
    assert_eq!(
        (req34.engine.as_str(), req34.constraint.as_str()),
        ("ruby", "3.4.2")
    );
    let e33 = mirror.entrypoint("hello33").unwrap();
    assert_eq!(e33.slot, 1);
    let req33 = e33.runtime_requirement.as_ref().unwrap();
    assert_eq!(
        (req33.engine.as_str(), req33.constraint.as_str()),
        ("ruby", "3.3.7")
    );

    // The vertical end (spec 07 §2.0): both registered commands dispatch
    // to their own slot of the ONE installed package against their own
    // cached runtimes — differing versions, simultaneously usable.
    let exe34 = write_cached_runtime(&fx.home, "3.4.2", "0.15.9");
    let exe33 = write_cached_runtime(&fx.home, "3.3.7", "0.15.9");
    let mut ctx = tebako_shim::Ctx {
        home: fx.home.clone(),
        cwd: fx.dir.clone(),
        env: std::collections::BTreeMap::new(),
    };
    ctx.env
        .insert("TEBAKO_HELLO34_VERSION".into(), "1.0".into());
    ctx.env
        .insert("TEBAKO_HELLO33_VERSION".into(), "1.0".into());
    let plan34 = tebako_shim::dispatch::dispatch("hello34", &[], &ctx).unwrap();
    let plan33 = tebako_shim::dispatch::dispatch("hello33", &[], &ctx).unwrap();
    assert_eq!(plan34.program, exe34);
    assert_eq!(plan33.program, exe33);
    let image = fx.payloads_dir().join("hellosuite/1.0.tfs");
    assert_eq!(
        plan34.mounts[0].triple(),
        format!("{}:0:/", image.display())
    );
    assert_eq!(
        plan33.mounts[0].triple(),
        format!("{}:1:/", image.display())
    );

    // uninstall removes every shim the suite registered.
    let out = install::uninstall(&fx.home, "hellosuite").unwrap();
    assert_eq!(out.shims_removed.len(), 2);
    assert!(!fx.home.join("shims/hello34").exists());
    assert!(!fx.home.join("shims/hello33").exists());
}

#[test]
fn suite_install_identity_mismatch_with_the_registry_is_a_named_error() {
    let fx = Fixture::new("suitebad");
    let pkg = suite_tpkg(
        "othersuite",
        "9.9",
        &[("hello34", 0, "ruby@3.4.2;tebako=0.15.9")],
    );
    let payload_ref = fx.payload("hellosuite-1.0.tpkg", &pkg);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("hellosuite", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "hellosuite", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("inconsistent"), "{err:?}");
    assert!(err.message.contains("othersuite 9.9"), "{err:?}");
}

#[test]
fn suite_install_corrupt_slot_reference_is_a_named_error() {
    let fx = Fixture::new("suiteghost");
    // entry "ghost" names slot 7; the package carries one slot.
    let pkg = suite_tpkg(
        "hellosuite",
        "1.0",
        &[
            ("hello34", 0, "ruby@3.4.2;tebako=0.15.9"),
            ("ghost", 7, "ruby@3.4.2;tebako=0.15.9"),
        ],
    );
    let payload_ref = fx.payload("hellosuite-1.0.tpkg", &pkg);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("hellosuite", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "hellosuite", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(err.message.contains("slot 7"), "{err:?}");
}

#[test]
fn suite_install_rejects_a_malformed_runtime_ref() {
    let fx = Fixture::new("suitebadref");
    let pkg = suite_tpkg("hellosuite", "1.0", &[("hello34", 0, "not-a-ref")]);
    let payload_ref = fx.payload("hellosuite-1.0.tpkg", &pkg);
    let reg_ref = fx.registry(
        "tpkg-registry.yaml",
        &registry_yaml("hellosuite", "1.0", &payload_ref, Some("1.0")),
    );
    install::add_registry(&fx.home, &reg_ref).unwrap();

    let err = install::install(&fx.home, "hellosuite", None, Some(&fx.shim_binary)).unwrap_err();
    assert!(
        err.message.contains("invalid entries[].runtime_ref"),
        "{err:?}"
    );
}

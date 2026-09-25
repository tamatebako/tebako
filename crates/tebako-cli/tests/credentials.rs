//! Spec 37 §5's credential book at the install surface — a separate
//! test binary because the credential book and the journal home are
//! process-global (installed by the config load) and the token env var
//! is process-global too; every test here takes the one lock.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use tebako_cli::install;
use tebako_http::FetchError;
use tebako_resolve::{Fetcher, Transport};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TOKEN_VAR: &str = "TEBAKO_CLI_TEST_NIST_TOKEN";
const CONTENTS_URL: &str = "https://api.github.com/repos/acme/priv/contents/tpkg-registry.yaml";

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tebako-cli-credentials-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The home the GitHub-hosted private registry resolves in: the payload
/// itself is a local `file://` artifact (only the REGISTRY fetch is
/// credentialed in these legs).
struct Env {
    dir: PathBuf,
    home: PathBuf,
    shim_binary: PathBuf,
    registry_json: Vec<u8>,
}

impl Env {
    fn new(tag: &str) -> Env {
        let dir = scratch(tag);
        let home = dir.join("home");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&mirror).unwrap();
        fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
        let app_url = tebako_http::file_url(&mirror.join("app-1.0.tfs"));
        let registry_yaml = format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {app_url}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n"
        );
        let registry_json = format!(
            r#"{{"name":"tpkg-registry.yaml","encoding":"base64","content":"{}"}}"#,
            tebako_resolve::credentials::base64_encode(registry_yaml.as_bytes())
        )
        .into_bytes();
        fs::write(
            home.join("config.yaml"),
            format!(
                "registries:\n  - ref: tfs:github:acme/priv\n    name: nist\ncredentials:\n  - registry: nist\n    token_env: {TOKEN_VAR}\n"
            ),
        )
        .unwrap();
        let shim_binary = dir.join("tebako-shim");
        fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
        Env {
            dir,
            home,
            shim_binary,
            registry_json,
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Records the credential header the wrapper decided for every URL and
/// plays the private endpoint: the contents API 401s unless the
/// expected Bearer rides.
type SeenHeaders = Mutex<Vec<(String, Option<(String, String)>)>>;

struct GuardedTransport {
    registry_json: Vec<u8>,
    demand: Option<String>,
    seen: SeenHeaders,
}

impl Transport for GuardedTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        // The plan executor's stream path buffers through here (the
        // payload is a local `file://` artifact).
        tebako_http::get(url)
    }
    fn get_with_header(
        &self,
        url: &str,
        header: Option<(&str, &str)>,
    ) -> Result<Vec<u8>, FetchError> {
        self.seen.lock().unwrap().push((
            url.to_string(),
            header.map(|(n, v)| (n.to_string(), v.to_string())),
        ));
        if url == CONTENTS_URL {
            match (&self.demand, header) {
                (Some(want), Some((_, got))) if want == got => {
                    return Ok(self.registry_json.clone())
                }
                (Some(_), _) => {
                    return Err(FetchError::AuthRejected {
                        url: url.to_string(),
                        status: 401,
                    })
                }
                (None, _) => return Ok(self.registry_json.clone()),
            }
        }
        Err(FetchError::IndexUnavailable(url.to_string()))
    }
}

#[test]
fn a_matched_entry_without_its_env_fails_closed_naming_registry_and_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::remove_var(TOKEN_VAR);
    let env = Env::new("unset");
    let t = GuardedTransport {
        registry_json: env.registry_json.clone(),
        demand: Some("Bearer sekrit".to_string()),
        seen: Mutex::new(Vec::new()),
    };
    let err = install::install_with(
        &env.home,
        "app",
        None,
        Some(&env.shim_binary),
        &Fetcher::with_transport(t),
    )
    .unwrap_err();
    assert_eq!(err.code, 69, "{err:?}");
    assert!(err.message.contains("(CredentialRequired)"), "{err:?}");
    assert!(err.message.contains("registry 'nist'"), "{err:?}");
    assert!(err.message.contains(TOKEN_VAR), "{err:?}");
    assert!(err.message.contains("credentials:"), "{err:?}");
    assert!(
        !env.home.join("payloads/app/1.0.tfs").exists(),
        "nothing was cached"
    );
}

#[test]
fn the_decided_bearer_installs_and_journals_its_class() {
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var(TOKEN_VAR, "sekrit");
    let env = Env::new("set");
    let t = GuardedTransport {
        registry_json: env.registry_json.clone(),
        demand: Some("Bearer sekrit".to_string()),
        seen: Mutex::new(Vec::new()),
    };
    let fetcher = Fetcher::with_transport(t);
    install::install_with(&env.home, "app", None, Some(&env.shim_binary), &fetcher).unwrap();
    std::env::remove_var(TOKEN_VAR);
    assert!(env.home.join("payloads/app/1.0.tfs").exists());
    let journal = fs::read_to_string(env.home.join("journal.log")).unwrap();
    assert!(
        journal.contains(&format!(
            "event=fetch host=api.github.com credential=token:env:{TOKEN_VAR}"
        )),
        "{journal}"
    );
    assert!(!journal.contains("sekrit"), "{journal}");
}

#[test]
fn a_bookless_home_installs_anonymously_and_journals_no_fetch_lines() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = scratch("bookless");
    let home = dir.join("home");
    let mirror = dir.join("mirror");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("app-1.0.tfs"), b"app-bytes").unwrap();
    let shim_binary = dir.join("tebako-shim");
    fs::write(&shim_binary, b"#!/bin/sh\n").unwrap();
    fs::write(
        mirror.join("tpkg-registry.yaml"),
        format!(
            "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app]\n    default: 1.0\n",
            tebako_http::file_url(&mirror.join("app-1.0.tfs"))
        ),
    )
    .unwrap();
    install::add_registry(
        &home,
        &tebako_http::file_url(&mirror.join("tpkg-registry.yaml")),
    )
    .unwrap();
    install::install(&home, "app", None, Some(&shim_binary)).unwrap();
    assert!(home.join("payloads/app/1.0.tfs").exists());
    let journal = home.join("journal.log");
    if journal.exists() {
        let text = fs::read_to_string(&journal).unwrap();
        assert!(!text.contains("event=fetch"), "{text}");
    }
    let _ = fs::remove_dir_all(&dir);
}

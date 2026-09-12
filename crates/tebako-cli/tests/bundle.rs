//! `tebako bundle` tests (spec 16 §6, roadmap 83): the offline bundle
//! stages the platform's tool set, the payload closure, the warmed
//! primary runtime, a from-reality pinned config.yaml, relocatable
//! shims, and the BUNDLE.yaml descriptor — staged under a tmp dir and
//! renamed into place (a failed bundle never leaves a tree behind).
//! Everything runs against file:// mirrors and temp homes — no network,
//! no process-env mutation (TEBAKO_RUNTIME_MIRROR rides the injected
//! env map, never std::env).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tebako_cli::bundle::{self, ArchiveFormat, BundleOutcome, BundleRequest};
use tebako_cli::error::TebakoError;
use tebako_resolve::{sha256_hex, Fetcher};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tebako-cli-bundle-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A bundle fixture: the builder's TEBAKO_HOME (its config names the
/// registry + the ruby line to stage), a tools dir with the four CLI
/// binaries, and a mirror dir holding the payload, its registry, and a
/// ruby runtime release index.
struct Fixture {
    dir: PathBuf,
    builder_home: PathBuf,
    tools: PathBuf,
    mirror: PathBuf,
    registry_url: String,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let dir = scratch(tag);
        let builder_home = dir.join("builder-home");
        let tools = dir.join("tools");
        let mirror = dir.join("mirror");
        fs::create_dir_all(&builder_home).unwrap();
        fs::create_dir_all(&tools).unwrap();
        fs::create_dir_all(&mirror).unwrap();

        // The platform's tool set (fake bytes; the bundle copies, never
        // executes them).
        for tool in ["tebako", "tebako-shim", "tfs", "tebako-pkg"] {
            let name = format!("{tool}{}", exe_suffix());
            let path = tools.join(&name);
            fs::write(&path, b"fake tool\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        // The app payload (fake bytes — install stores them verbatim and
        // synthesizes the manifest mirror from the registry's tier-3
        // fields) and its registry.
        let payload_path = mirror.join("app-1.0.tfs");
        fs::write(&payload_path, b"app-bytes").unwrap();
        let registry_path = mirror.join("tpkg-registry.yaml");
        fs::write(
            &registry_path,
            format!(
                "schema_version: 1\npayloads:\n  - name: app\n    kind: app\n    versions:\n      - version: 1.0\n        platforms: universal\n        release: {{ref: {}}}\n        runtime_requirement: {{engine: ruby, constraint: \">= 3.1\"}}\n        entrypoints: [app, app-helper]\n    default: 1.0\n",
                tebako_http::file_url(&payload_path)
            ),
        )
        .unwrap();
        let registry_url = tebako_http::file_url(&registry_path);

        // The builder's config registers the registry and pins the ruby
        // line to stage (the operator flow: the bundle stages the lines
        // the config names; the warm's index probe reads {mirror}/v0.0.1).
        fs::write(
            builder_home.join("config.yaml"),
            format!(
                "registries:\n  - {registry_url}\nruntimes:\n  ruby:\n    version: \"3.3.5\"\n    tebako: \"0.0.1\"\n"
            ),
        )
        .unwrap();

        // The ruby runtime release mirror (the factory's locked index
        // shape, spec 13 §2a): one version, real sha256 anchors.
        let platform = tebako_shim::runtime::platform_string();
        let rt_dir = mirror.join("v0.0.1");
        fs::create_dir_all(&rt_dir).unwrap();
        let exe_name = format!("tebako-runtime-0.0.1-3.3.5-{platform}{}", exe_suffix());
        let image_name = format!("tebako-runtime-0.0.1-3.3.5-{platform}.tfs");
        fs::write(rt_dir.join(&exe_name), b"fake runtime exe\n").unwrap();
        fs::write(rt_dir.join(&image_name), b"fake runtime image\n").unwrap();
        fs::write(
            rt_dir.join("manifest.json"),
            format!(
                "[{{\"tebako_version\": \"0.0.1\", \"contract_era\": 2, \"contract_version\": 2, \"mount_root\": \"/__tfs__\", \"ruby_version\": \"3.3.5\", \"platform\": \"{platform}\", \"filename\": \"{exe_name}\", \"sha256\": \"{}\", \"image\": {{\"filename\": \"{image_name}\", \"sha256\": \"{}\"}}}}]\n",
                sha256_hex(b"fake runtime exe\n"),
                sha256_hex(b"fake runtime image\n")
            ),
        )
        .unwrap();

        Fixture {
            dir,
            builder_home,
            tools,
            mirror,
            registry_url,
        }
    }

    /// Bundle `app` against this fixture. TEBAKO_RUNTIME_MIRROR rides
    /// the injected env map (the warm's channel-2 download).
    fn run(
        &self,
        out: &Path,
        overlay: Option<&Path>,
        archive: Option<ArchiveFormat>,
    ) -> Result<BundleOutcome, TebakoError> {
        let mut env = BTreeMap::new();
        env.insert(
            "TEBAKO_RUNTIME_MIRROR".to_string(),
            self.mirror.to_string_lossy().to_string(),
        );
        bundle::bundle_with(
            &BundleRequest {
                builder_home: &self.builder_home,
                tools_dir: &self.tools,
                target: "app",
                output: out,
                overlay,
                archive,
                env: &env,
            },
            &Fetcher::new(),
        )
    }

    fn output(&self) -> PathBuf {
        self.dir.join("out")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn exe_suffix() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

#[test]
fn bundle_stages_a_self_consistent_offline_tree() {
    let fx = Fixture::new("happy");
    let out = fx.output();
    let outcome = fx.run(&out, None, None).unwrap();

    assert_eq!(outcome.payload, ("app".to_string(), "1.0".to_string()));

    // bin/: the whole tool set.
    for tool in ["tebako", "tebako-shim", "tfs", "tebako-pkg"] {
        assert!(
            out.join("bin")
                .join(format!("{tool}{}", exe_suffix()))
                .is_file(),
            "missing tool {tool}"
        );
    }

    // home/payloads: the payload image + its trust anchor.
    let image = out.join("home/payloads/app/1.0.tfs");
    assert!(image.is_file(), "payload image staged");
    assert_eq!(fs::read(&image).unwrap(), b"app-bytes");
    assert!(out.join("home/payloads/app/1.0.tfs.sha256").is_file());

    // home/runtimes: the primary runtime WARMED (install alone leaves it
    // to first dispatch — the bundle is offline, so it must be staged).
    let runtimes_dir = out.join("home/runtimes");
    let entries: Vec<_> = fs::read_dir(&runtimes_dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(entries.len(), 1, "exactly the primary runtime: {entries:?}");
    let expected = format!(
        "ruby-3.3.5-0.0.1-{}",
        tebako_shim::runtime::platform_string()
    );
    assert_eq!(entries[0], expected);

    // home/config.yaml: the registries survive, and runtimes: is pinned
    // FROM REALITY (the staged store, not the builder's hope).
    let cfg = tebako_shim::config::load_config(&out.join("home")).unwrap();
    assert_eq!(cfg.registries, vec![fx.registry_url.clone()]);
    let ruby = cfg.runtimes.get("ruby").expect("ruby pin");
    assert_eq!(ruby.version, "3.3.5");
    assert_eq!(ruby.tebako, "0.0.1");

    // BUNDLE.yaml: the installer's descriptor — and version round-trips
    // as a STRING ("1.0" must not re-type as a float).
    let text = fs::read_to_string(out.join("BUNDLE.yaml")).unwrap();
    let doc: serde_yml::Value = serde_yml::from_str(&text).unwrap();
    assert_eq!(doc["payload"].as_str(), Some("app"));
    assert_eq!(doc["version"].as_str(), Some("1.0"), "{text}");
    assert_eq!(
        doc["runtimes"][0]["engine"].as_str(),
        Some("ruby"),
        "{text}"
    );
    let commands: Vec<&str> = doc["commands"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|c| c.as_str())
        .collect();
    assert!(commands.contains(&"app"), "{text}");

    // The shims exist and (unix) are RELATIVE — the tree relocates as
    // one piece.
    let shim = out
        .join("home/shims")
        .join(tebako_shim::manage::shim_file_name("app"));
    assert!(fs::symlink_metadata(&shim).is_ok(), "shim staged");
    #[cfg(unix)]
    {
        let target = fs::read_link(&shim).unwrap();
        assert!(
            !target.is_absolute(),
            "the shim link is relative: {target:?}"
        );
        assert_eq!(
            target,
            Path::new("../../bin").join(format!("tebako-shim{}", exe_suffix()))
        );
        // And it resolves inside the bundle.
        let resolved = shim.parent().unwrap().join(&target);
        assert!(
            resolved
                .canonicalize()
                .unwrap()
                .starts_with(out.canonicalize().unwrap()),
            "the shim resolves inside the bundle"
        );
    }
}

#[test]
fn bundle_overlay_layers_the_org_config() {
    let fx = Fixture::new("overlay");
    let overlay = fx.dir.join("org.yaml");
    fs::write(
        &overlay,
        "defaults:\n  app: \"1.0\"\nnetwork:\n  proxy: http://corp:3128\n",
    )
    .unwrap();
    let out = fx.output();
    fx.run(&out, Some(&overlay), None).unwrap();
    let cfg = tebako_shim::config::load_config(&out.join("home")).unwrap();
    assert_eq!(cfg.defaults.get("app").unwrap(), "1.0");
    assert_eq!(cfg.network.proxy.as_deref(), Some("http://corp:3128"));
    // The from-reality pin still lands over the overlay.
    assert_eq!(cfg.runtimes.get("ruby").unwrap().version, "3.3.5");
}

#[test]
fn bundle_refuses_an_occupied_output_and_a_partial_tool_set() {
    let fx = Fixture::new("guards");
    // Occupied output: refused by name, nothing staged.
    let out = fx.output();
    fs::create_dir_all(&out).unwrap();
    let err = fx.run(&out, None, None).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("already exists"), "{err:?}");

    // A missing tool: named, not silently skipped.
    fs::remove_file(fx.tools.join(format!("tfs{}", exe_suffix()))).unwrap();
    let out2 = fx.dir.join("out2");
    let err = fx.run(&out2, None, None).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(err.message.contains("incomplete"), "{err:?}");
    assert!(!out2.exists(), "a failed bundle leaves no tree");
}

#[test]
fn bundle_packs_a_relocatable_tarball() {
    let fx = Fixture::new("archive");
    let out = fx.output();
    let outcome = fx.run(&out, None, Some(ArchiveFormat::TarGz)).unwrap();
    let archive = outcome.archive.expect("the archive path");
    assert!(archive.is_file(), "the tarball landed");
    assert_eq!(archive.extension().and_then(|e| e.to_str()), Some("gz"));

    // The tree packed under the bundle's own directory name, symlinks
    // preserved (the relocatable shims ARE the dispatch surface).
    let file = fs::File::open(&archive).unwrap();
    let dec = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(dec);
    let mut names = Vec::new();
    let mut link_names = Vec::new();
    for entry in tar.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        if entry.header().entry_type() == tar::EntryType::Symlink {
            link_names.push(path.clone());
        }
        names.push(path);
    }
    let base = out.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        names
            .iter()
            .any(|n| *n == format!("{base}/home/config.yaml")),
        "config packed: {names:?}"
    );
    #[cfg(unix)]
    assert!(
        link_names.iter().any(|n| n.contains("shims/app")),
        "the shim link survived the pack: {link_names:?}"
    );
}

#[test]
fn bundle_zip_is_a_named_error_until_pr2() {
    let fx = Fixture::new("zipstub");
    let out = fx.output();
    let err = fx.run(&out, None, Some(ArchiveFormat::Zip)).unwrap_err();
    assert_eq!(err.code, 65, "{err:?}");
    assert!(
        err.message.contains("zip bundle leg is not written yet"),
        "{err:?}"
    );
    // The tree still staged and renamed — the pack failure post-dates it.
    assert!(out.join("BUNDLE.yaml").is_file());
}

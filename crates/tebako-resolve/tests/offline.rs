//! TEBAKO_OFFLINE registry discipline (spec 05 §4: cache hit or hard
//! error). Lives in its own test binary: TEBAKO_OFFLINE is process-global
//! state, and a separate process cannot race the other suites.

use std::fs;

use tebako_resolve::{Fetcher, RegistryRef, ResolveError};

const REGISTRY_YAML: &str = "schema_version: 1\npayloads:\n  - name: tool\n    kind: app\n    versions:\n      - {version: 1.0, platforms: universal, release: {ref: tfs:github:o/tool:1.0}, entrypoints: [tool]}\n";

#[test]
fn offline_resolves_file_mirrors_only() {
    let dir = std::env::temp_dir().join(format!("tebako-resolve-offline-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let mirror = dir.join("mirror");
    fs::create_dir_all(&mirror).unwrap();
    fs::write(mirror.join("tpkg-registry.yaml"), REGISTRY_YAML).unwrap();

    std::env::set_var("TEBAKO_OFFLINE", "1");
    let local =
        RegistryRef::parse(&format!("file://{}/tpkg-registry.yaml", mirror.display())).unwrap();
    assert!(Fetcher::new().resolve_registry(&local).is_ok());

    let remote = RegistryRef::parse("tfs:github:o/r").unwrap();
    let err = Fetcher::new().resolve_registry(&remote).unwrap_err();
    assert!(matches!(err, ResolveError::Offline { .. }));
    assert!(err.to_string().contains("registry tfs:github:o/r"));

    let pinned = RegistryRef::parse("tfs:github:o/r:v9#tpkg-registry.yaml").unwrap();
    assert!(matches!(
        Fetcher::new().resolve_registry(&pinned).unwrap_err(),
        ResolveError::Offline { .. }
    ));
    let git = RegistryRef::parse("tfs+git://h/registry.git@main#tpkg-registry.yaml").unwrap();
    assert!(matches!(
        Fetcher::new().resolve_registry(&git).unwrap_err(),
        ResolveError::Offline { .. }
    ));
    std::env::remove_var("TEBAKO_OFFLINE");
    let _ = fs::remove_dir_all(&dir);
}

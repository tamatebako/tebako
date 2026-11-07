//! Runtime-side layout grants (spec 03 §2.8's amendment): a handoff
//! triple whose mount is `role:<name>` resolves against the ENV image's
//! declared `layout_grants` — the slice mounts below the grant's
//! `slices_at`, named by its own manifest, and the grant's `search_env`
//! gains the mount (appended after any inherited entries).
//!
//! The dispatch split is deliberate: the SHIM selects the slice (engine
//! match + the version constraint) and emits the role form; the DRIVER —
//! the one place the runtime's own manifest is in hand (captured from
//! the mounted env image) — resolves role → path. Neither spells the
//! other's knowledge.

use std::fmt::Write as _;

use crate::driver::{errno_text, qualify_mount};
use crate::handoff::{Handoff, ImageSource, SlotRef};
use crate::{
    driver::Env, driver::MountedMember, DriverError, EX_TEBAKO_LAYOUT, EX_TEBAKO_MANIFEST,
};

/// The mount-position marker: `role+<grant>` (the shim's role form).
/// Distinguishable from every path form by construction (an absolute
/// mount starts with `/`; a windows drive mount carries `:`; `+` is
/// neither's second char) — and `:` would break the triple's
/// right-split, so the marker carries none. The owner is
/// `tpkg::AUGMENT_ROLE_PREFIX` (one wire, both sides).
pub const ROLE_PREFIX: &str = tpkg::AUGMENT_ROLE_PREFIX;

/// True when a declared mount is the role form.
pub fn is_role_mount(mount: &str) -> bool {
    mount.starts_with(ROLE_PREFIX)
}

/// Resolve every role-form triple in the handoff against the env image's
/// grants: rewrite its mount to `<slices_at>/<slice-name>` (drive-
/// qualified), and extend each touched grant's `search_env` with the
/// mount (appended after the inherited value, in triple order).
///
/// Runs after the env image mounts (the grant set is its manifest's) and
/// before the payload mount loop — a role triple never reaches the
/// table unrewritten.
pub fn resolve(
    h: &mut Handoff,
    mounted: &[MountedMember],
    env: &dyn Env,
    runtime_root: &str,
) -> Result<(), DriverError> {
    if !h.images.iter().any(|s| is_role_mount(&s.mount)) {
        return Ok(());
    }
    // The grant set's owner: the env image's own manifest (L1 — never a
    // registry mirror). NO env manifest at all is the runtime-absent
    // case (the composition mounts no runtime for the slice to augment —
    // LayoutGrantRuntimeAbsent); a manifest granting nothing makes every
    // role unknown (UnknownLayoutGrant).
    let env_manifest = mounted
        .iter()
        .find(|m| m.is_env)
        .and_then(|m| m.manifest.as_ref());
    let grants: &[tpkg::LayoutGrant] = match env_manifest.and_then(|m| match &m.provides {
        tpkg::Provides::Runtime(p) => Some(p.layout_grants.as_slice()),
        _ => None,
    }) {
        Some(g) => g,
        None => &[],
    };
    if grants.is_empty() {
        let first_role = h
            .images
            .iter()
            .find_map(|s| s.mount.strip_prefix(ROLE_PREFIX))
            .unwrap_or("?")
            .to_string();
        return Err(DriverError::new(
            EX_TEBAKO_LAYOUT,
            if env_manifest.is_none() {
                format!(
                    "a role-form mount (role+{first_role}) arrived but the composition carries no runtime manifest for it to augment (spec 03 §2.8; LayoutGrantRuntimeAbsent)"
                )
            } else {
                format!(
                    "the runtime declares no layout grants at all — the slice's runtime edge names role {first_role:?} the runtime must open (spec 03 §2.8; UnknownLayoutGrant, granted: none)"
                )
            },
        ));
    }

    let mut extensions: Vec<(String, String, String)> = Vec::new(); // (search_env, mount, journal line fields)
    for spec in &mut h.images {
        let Some(role) = spec.mount.strip_prefix(ROLE_PREFIX) else {
            continue;
        };
        let Some(grant) = grants.iter().find(|g| g.role == role) else {
            let granted: Vec<&str> = grants.iter().map(|g| g.role.as_str()).collect();
            return Err(DriverError::new(
                EX_TEBAKO_LAYOUT,
                format!(
                    "the runtime declares no layout grant {role:?} (granted: {}) — the slice's runtime edge names a role the runtime must open (spec 03 §2.8; UnknownLayoutGrant)",
                    if granted.is_empty() {
                        "none".to_string()
                    } else {
                        granted.join(", ")
                    }
                ),
            ));
        };

        // The slice's name is its manifest's — read from the image's own
        // bytes BEFORE the mount (the mount point depends on it; the
        // tebako#825 rule: never through the union view).
        let name = slice_name(spec)?;
        let base = grant.slices_at.trim_end_matches('/');
        let mount = qualify_mount(&format!("{base}/{name}"), runtime_root);
        extensions.push((
            grant.search_env.clone(),
            mount.clone(),
            format!("slice={name} role={role} path={mount}"),
        ));
        spec.mount = mount;
    }

    // Extend each touched search env: the inherited value stays FIRST
    // (the runtime's own entries win by position), each slice's mount
    // appends in triple order. The separator is the platform's list
    // separator.
    let mut per_var: Vec<(String, Vec<String>)> = Vec::new();
    for (var, mount, _line) in &extensions {
        match per_var.iter_mut().find(|(v, _)| v == var) {
            Some((_, mounts)) => mounts.push(mount.clone()),
            None => per_var.push((var.clone(), vec![mount.clone()])),
        }
    }
    for (var, mounts) in per_var {
        let sep = if cfg!(windows) { ';' } else { ':' };
        let mut value = crate::driver::env_var(env, &var).unwrap_or_default();
        for m in mounts {
            if !value.is_empty() {
                value.push(sep);
            }
            let _ = write!(value, "{m}");
        }
        env.set_var(&var, &value);
    }

    // The audit record: one line per resolved grant, best-effort (the
    // journal's own discipline — a write failure never gates the boot).
    if let Some(journal) = tfs::journal::open_journal() {
        for (_var, _mount, line) in &extensions {
            // line carries "slice=<n> role=<r> path=<m>".
            let fields: Vec<&str> = line.split_whitespace().collect();
            let slice = fields
                .first()
                .and_then(|f| f.strip_prefix("slice="))
                .unwrap_or("?");
            let role = fields
                .get(1)
                .and_then(|f| f.strip_prefix("role="))
                .unwrap_or("?");
            let path = fields
                .get(2)
                .and_then(|f| f.strip_prefix("path="))
                .unwrap_or("?");
            tfs::journal::journal_layout_grant(&journal, slice, role, path);
        }
    }
    Ok(())
}

/// The slice's identity name, read from its image's own manifest
/// pre-mount (the tebako#825 rule: from its own bytes, never the union
/// view). A slot region reads exactly the bytes the mount will.
fn slice_name(spec: &crate::handoff::ImageSpec) -> Result<String, DriverError> {
    let mount = match &spec.source {
        ImageSource::File(path, slot) => {
            let display = path.display().to_string();
            let built = match slot {
                SlotRef::Whole => tfs::mount::build_from_file(&display, "/__layout-probe"),
                SlotRef::Slot(_) => {
                    let resolved =
                        crate::driver::resolve_image(path, *slot, &display, &spec.mount)?;
                    match resolved.region {
                        crate::driver::ResolvedRegion::Whole => {
                            tfs::mount::build_from_file(&display, "/__layout-probe")
                        }
                        crate::driver::ResolvedRegion::Region(offset, size) => {
                            tfs::mount::build_from_file_at(
                                &display,
                                offset,
                                size,
                                "/__layout-probe",
                            )
                        }
                    }
                }
            };
            built.map_err(|e| {
                DriverError::new(
                    EX_TEBAKO_LAYOUT,
                    format!(
                        "the slice image {display} does not read ({}) — the layout grant cannot name it",
                        errno_text(e)
                    ),
                )
            })?
        }
        ImageSource::OwnSlot(n) => {
            return Err(DriverError::new(
                EX_TEBAKO_MANIFEST,
                format!("a role-form mount on the package's own slot {n} is not supported in v1 — the slice arrives as its own image file (the loader emits it so)"),
            ));
        }
    };
    let bytes = mount
        .read_file(tpkg::PAYLOAD_MANIFEST_PATH, crate::layout::LAYOUT_MAX_BYTES)
        .map_err(|e| {
            DriverError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "the slice image carries no readable manifest ({}) — a layout-grant slice names itself",
                    errno_text(e)
                ),
            )
        })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            "the slice image's manifest is not UTF-8".to_string(),
        )
    })?;
    let manifest = tpkg::PayloadManifest::from_yaml(&text).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            format!("the slice image's manifest does not parse ({e})"),
        )
    })?;
    Ok(manifest.identity.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{Env, MountedMember};
    use crate::handoff::{ImageSource, ImageSpec, SlotRef};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Write as _;

    struct MapEnv(RefCell<HashMap<String, String>>);
    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }
        fn names_with_prefix(&self, prefix: &str) -> Vec<String> {
            self.0
                .borrow()
                .keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect()
        }
    }

    fn runtime_manifest(with_grants: bool) -> tpkg::PayloadManifest {
        let grants = if with_grants {
            "  layout_grants:\n    - {role: site-packages, slices_at: /tebako/slices.d, layout: python-site, search_env: PYTHONPATH}\n"
        } else {
            ""
        };
        let text = format!(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-python\n  version: \"3.13.5\"\n\
             \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-11-07T00:00:00Z\"\n\
             \x20 digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  provides: {{engine: python, version: \"3.13.5\", language_version: \"3.13\", abi_line: \"3.13\", platform: aarch64-macos}}\n\
             \x20 built_from: {{src_sha256: {}, patch_set: v1}}\n{}\
             \x20 capabilities: {{exec: true, read: true, runtime: true}}\n",
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64),
            grants
        );
        tpkg::PayloadManifest::from_yaml(&text).unwrap()
    }

    fn env_member(with_grants: bool) -> MountedMember {
        MountedMember {
            point: "/__tfs__".to_string(),
            desc: "env image".to_string(),
            manifest: Some(runtime_manifest(with_grants)),
            is_env: true,
        }
    }

    /// A slice image on disk: a zip carrying its manifest.
    fn write_slice(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let p = dir.join(format!("{name}.zip"));
        let file = std::fs::File::create(&p).unwrap();
        let mut w = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        w.add_directory("__tpkg__", opts).unwrap();
        w.start_file("__tpkg__/manifest.yaml", opts).unwrap();
        w.write_all(
            format!(
                "identity:\n  schema_version: 1\n  kind: data\n  name: {name}\n  version: 1.0.0\n\
                 \x20 producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-11-07T00:00:00Z\"\n\
                 \x20 digest: {{tree_hash: \"sha256:{}\", blob_sha256: {}}}\n\
                 \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
                 provides:\n  mount_semantics: {{suggested: /x}}\n  capabilities: {{exec: false, read: true}}\n",
                "a".repeat(64),
                "b".repeat(64)
            )
            .as_bytes(),
        )
        .unwrap();
        w.finish().unwrap();
        p
    }

    fn role_handoff(image: &std::path::Path, role: &str) -> Handoff {
        Handoff {
            images: vec![ImageSpec {
                source: ImageSource::File(image.to_path_buf(), SlotRef::Whole),
                mount: format!("role+{role}"),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_role_mount_resolves_and_extends_the_search_env() {
        let dir = std::env::temp_dir().join(format!("layout-grants-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let slice = write_slice(&dir, "numpy-extras");
        let mut h = role_handoff(&slice, "site-packages");
        let env = MapEnv(RefCell::new(HashMap::from([(
            "PYTHONPATH".to_string(),
            "/runtime/lib".to_string(),
        )])));
        let mounted = vec![env_member(true)];
        resolve(&mut h, &mounted, &env, "/__tfs__").unwrap();
        assert_eq!(h.images[0].mount, "/tebako/slices.d/numpy-extras");
        // The list separator is the platform's (`;` on windows).
        let sep = if cfg!(windows) { ";" } else { ":" };
        let expected = format!("/runtime/lib{sep}/tebako/slices.d/numpy-extras");
        assert_eq!(
            env.var("PYTHONPATH").as_deref(),
            Some(expected.as_str()),
            "the runtime's own entries stay first; the slice appends"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_role_is_a_named_error() {
        let dir = std::env::temp_dir().join(format!("layout-grants-err-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let slice = write_slice(&dir, "numpy-extras");
        let mut h = role_handoff(&slice, "node-modules");
        let env = MapEnv(RefCell::new(HashMap::new()));
        let mounted = vec![env_member(true)];
        let err = resolve(&mut h, &mounted, &env, "/__tfs__").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_LAYOUT);
        assert!(err.message.contains("node-modules"), "{err}");
        assert!(err.message.contains("site-packages"), "{err}");
        // The untouched grant set is named in the error.
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_grantless_runtime_names_every_role_unknown() {
        let dir = std::env::temp_dir().join(format!("layout-grants-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let slice = write_slice(&dir, "numpy-extras");
        let mut h = role_handoff(&slice, "site-packages");
        let env = MapEnv(RefCell::new(HashMap::new()));
        let mounted = vec![env_member(false)];
        let err = resolve(&mut h, &mounted, &env, "/__tfs__").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_LAYOUT);
        assert!(err.message.contains("UnknownLayoutGrant"), "{err}");
        assert!(err.message.contains("site-packages"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_runtime_manifest_at_all_is_layout_grant_runtime_absent() {
        let dir = std::env::temp_dir().join(format!("layout-grants-absent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let slice = write_slice(&dir, "numpy-extras");
        let mut h = role_handoff(&slice, "site-packages");
        let env = MapEnv(RefCell::new(HashMap::new()));
        // No env member at all (TEBAKO_RUNTIME_IMAGE unset at boot).
        let err = resolve(&mut h, &[], &env, "/__tfs__").unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_LAYOUT);
        assert!(err.message.contains("LayoutGrantRuntimeAbsent"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_role_mounts_is_a_noop() {
        let mut h = Handoff::default();
        let env = MapEnv(RefCell::new(HashMap::new()));
        resolve(&mut h, &[], &env, "/__tfs__").unwrap();
        assert!(h.images.is_empty());
        assert!(env.var("PYTHONPATH").is_none());
    }
}

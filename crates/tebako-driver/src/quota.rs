//! The per-mount materialization ceiling's boot-time resolve
//! (tebako#1102; TODO.sota2/03's mount arm).
//!
//! The ceiling is config-driven: `mounts.quota` in
//! `~/.tebako/config.yaml` (store-config schema minor 6), parsed through
//! the grammar's single owner (`tpkg::mount_quota`). The boot resolves
//! it once — after the exec-cache export, before any mount can
//! materialize — and arms [`tfs::quota`]: the class-R resources tier
//! (materialize.rs), the windows tree tier (materialize/tree.rs), and
//! the persistent dlcache tier (tfs) then write against it. Absent
//! config binds the documented default
//! ([`tpkg::mount_quota::DEFAULT_BYTES`]); `quota: off` disarms (the
//! pre-quota behavior); a malformed value is a named 65 — the boot
//! never guesses around a torn config.
//!
//! The home discovery is the store's own
//! ([`tpkg::runtime_store::tebako_home`], spec 05 §3.1) through the
//! boot env — the same read the materialize tier's trust-store arm
//! performs; an undiscoverable home binds the default (the ceiling
//! needs no home to hold — the journal stays best-effort).

use std::path::PathBuf;

use crate::driver::{env_var, DriverError, Env};
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

/// Resolve the ceiling from the boot env's config and arm the engine.
/// Called once per boot, after [`crate::exec_cache::export`] named this
/// boot's exec-cache root — the surface is contractual, never assumed.
pub fn resolve_and_arm(env: &dyn Env) -> Result<(), DriverError> {
    let quota = resolve(env)?;
    let Some(cache) = env_var(env, crate::exec_cache::VAR) else {
        return Err(DriverError::new(
            EX_TEBAKO_IO,
            format!(
                "{} is not exported — the exec-cache export runs before the mount-quota arm at boot",
                crate::exec_cache::VAR
            ),
        ));
    };
    tfs::quota::arm(quota, PathBuf::from(cache));
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "mount quota armed: {}",
        match quota.ceiling() {
            Some(bytes) => format!("{bytes} bytes per mount"),
            None => "off (unbounded)".to_string(),
        }
    );
    Ok(())
}

/// The config read: home discovery → `config.yaml` → the `mounts`
/// section → the grammar's owner. Every malformed shape is a named 65
/// (fail closed); an absent home or file binds the default.
fn resolve(env: &dyn Env) -> Result<tpkg::mount_quota::MountQuota, DriverError> {
    use tpkg::mount_quota::{MountQuota, CONFIG_KEY};
    let home = match tpkg::runtime_store::tebako_home(|k| env_var(env, k)) {
        Ok(home) => home,
        Err(e) => {
            tebako_log::log!(
                tebako_log::Level::Debug,
                "driver",
                "tebako home undiscoverable ({e}) — the default {CONFIG_KEY} binds"
            );
            return Ok(MountQuota::default());
        }
    };
    let path = home.join("config.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MountQuota::default()),
        Err(e) => {
            return Err(DriverError::new(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", path.display()),
            ))
        }
    };
    let doc: serde_yml::Value = serde_yml::from_str(&text).map_err(|e| {
        DriverError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse {} ({e}) — fix or remove it", path.display()),
        )
    })?;
    let mounts = doc.get("mounts");
    let quota = match mounts {
        None => return Ok(MountQuota::default()),
        Some(serde_yml::Value::Mapping(mapping)) => mapping.get("quota"),
        Some(_) => {
            return Err(DriverError::new(
                EX_TEBAKO_MANIFEST,
                format!("{}: `mounts` must be a mapping", path.display()),
            ))
        }
    };
    let value = match quota {
        None | Some(serde_yml::Value::Null) => return Ok(MountQuota::default()),
        Some(value) => value,
    };
    let err =
        |why: String| DriverError::new(EX_TEBAKO_MANIFEST, format!("{}: {why}", path.display()));
    match value {
        serde_yml::Value::Number(n) => {
            let Some(v) = n.as_u64() else {
                return Err(err(format!(
                    "{CONFIG_KEY} value {n} is not a byte count (want a positive integer, a K|M|G-suffixed string, or `off`)"
                )));
            };
            tpkg::mount_quota::from_count(v).map_err(err)
        }
        serde_yml::Value::String(s) => tpkg::mount_quota::parse(s).map_err(err),
        other => Err(err(format!(
            "{CONFIG_KEY} value {other:?} is not a byte count (want a positive integer, a K|M|G-suffixed string, or `off`)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn temp(tag: &str) -> PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tebako-driver-quota-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

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

    fn env_with(pairs: &[(&str, &str)]) -> MapEnv {
        MapEnv(RefCell::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
    }

    fn write_config(home: &PathBuf, text: &str) {
        std::fs::create_dir_all(home).unwrap();
        std::fs::write(home.join("config.yaml"), text).unwrap();
    }

    #[test]
    fn an_absent_home_or_file_binds_the_default() {
        // An undiscoverable home (no TEBAKO_HOME, no usable HOME/HOMEDRIVE
        // chain — the lookup sees only the empty env): default.
        let env = env_with(&[]);
        assert_eq!(
            resolve(&env).unwrap().ceiling(),
            Some(tpkg::mount_quota::DEFAULT_BYTES)
        );
        // A home with no config.yaml: default.
        let home = temp("default");
        let env = env_with(&[("TEBAKO_HOME", home.to_str().unwrap())]);
        assert_eq!(
            resolve(&env).unwrap().ceiling(),
            Some(tpkg::mount_quota::DEFAULT_BYTES)
        );
        // A config with no mounts section: default.
        write_config(&home, "defaults:\n  app: \"1.0\"\n");
        assert_eq!(
            resolve(&env).unwrap().ceiling(),
            Some(tpkg::mount_quota::DEFAULT_BYTES)
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_section_parses_through_the_grammar_owner() {
        let home = temp("parse");
        let env = env_with(&[("TEBAKO_HOME", home.to_str().unwrap())]);
        write_config(&home, "mounts:\n  quota: 512M\n");
        assert_eq!(resolve(&env).unwrap().ceiling(), Some(512 << 20));
        write_config(&home, "mounts:\n  quota: 1048576\n");
        assert_eq!(resolve(&env).unwrap().ceiling(), Some(1 << 20));
        write_config(&home, "mounts:\n  quota: off\n");
        assert_eq!(resolve(&env).unwrap().ceiling(), None);
        // Unknown keys in the section are ignored (the schema's
        // forward-compat rule), a null quota binds the default.
        write_config(&home, "mounts:\n  quota:\n  future_key: whatever\n");
        assert_eq!(
            resolve(&env).unwrap().ceiling(),
            Some(tpkg::mount_quota::DEFAULT_BYTES)
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn malformed_values_are_named_65s() {
        let home = temp("malformed");
        let env = env_with(&[("TEBAKO_HOME", home.to_str().unwrap())]);
        for bad in ["lots", "1T", "-5", "0"] {
            write_config(&home, &format!("mounts:\n  quota: {bad}\n"));
            let err = resolve(&env).unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{bad}: {}", err.message);
            assert!(err.message.contains("config.yaml"), "{}", err.message);
        }
        // A non-mapping section is the same family.
        write_config(&home, "mounts: [nope]\n");
        let err = resolve(&env).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(
            err.message.contains("`mounts` must be a mapping"),
            "{}",
            err.message
        );
        // An unparseable document: the dispatcher's own wording (fix or
        // remove), never a guessed default.
        write_config(&home, "mounts:\n  quota: [unclosed\n");
        let err = resolve(&env).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(err.message.contains("fix or remove it"), "{}", err.message);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn resolve_and_arm_needs_the_exec_cache_export() {
        let env = env_with(&[]);
        let err = resolve_and_arm(&env).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_IO, "{}", err.message);
        assert!(
            err.message.contains(crate::exec_cache::VAR),
            "{}",
            err.message
        );
    }
}

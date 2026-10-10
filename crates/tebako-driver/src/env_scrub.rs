//! The packager-credential scrub (tebako#737, spec 03 §2.10): the
//! interpreter's environment must not carry the packager's (or the CI
//! host's) `TEBAKO_*` variables past the handoff — `TEBAKO_GITHUB_TOKEN`
//! above all. A packaged app must not exfiltrate the packager's
//! credentials by default.
//!
//! The CONTRACT surface — the variables the driver, the tfs layer, and
//! the documented handoff exports consume — stays; everything else
//! `TEBAKO_`-prefixed is blanked at boot's end, after every contract
//! read (blanking, never removal: the env-var filter treats empty as
//! absent, and the value is never logged — only the name).
//!
//! A payload opts IN to carrying named variables through its
//! manifest's `env_pass` list (spec 03 §2.10): the packager declares
//! exactly what the packaged run should see.

use crate::driver::Env;

/// The variables the boot contract itself consumes (plus the
/// `TEBAKO_MOUNT_` discovery prefix). Anything else `TEBAKO_`-prefixed
/// is packager-side state and gets scrubbed.
const CONTRACT_VARS: &[&str] = &[
    // spec 17 §1/§7 — the handoff and the root ceremony
    "TEBAKO_MOUNT_ROOT",
    "TEBAKO_RUNTIME_IMAGE",
    "TEBAKO_RUNTIME_DLL",
    "TEBAKO_MATERIALIZE_BOOT",
    // spec 08 §2 — the jail
    "TEBAKO_JAIL",
    "TEBAKO_JAIL_SOURCE",
    "TEBAKO_JAIL_JOURNAL",
    // resolution + lazy behavior (spec 39)
    "TEBAKO_OFFLINE",
    "TEBAKO_LAZY_SEAL",
    "TEBAKO_SPAWN_LOCK",
    // the exec cache + trace (spec 22 §6, spec 25 §2)
    "TEBAKO_EXEC_CACHE",
    "TEBAKO_TRACE",
    // the child-injection surface (spec 22 §3) and the preload tier
    "TEBAKO_TFS_MOUNTS",
    "TEBAKO_PRELOAD_SHIM",
    "TEBAKO_LOADER_INTERPOSED",
    "TEBAKO_OVERLAYS",
    "TEBAKO_DECRYPT", // TODO(#737-followup): feed keys at mount, not lazily
    // trust material the runtime's own fetches consume (spec 81's plane)
    "TEBAKO_EXTRA_CA",
    "TEBAKO_TLS_PLATFORM_ROOTS",
    // the invoked program name (tebako#237, spec 17 §7) — the ruby
    // patch reads it at interpreter start, after this scrub
    "TEBAKO_PROGRAM_NAME",
    // the store + toolchain surface an embedded tebako CLI needs
    "TEBAKO_HOME",
    "TEBAKO_DEPLOY_BINDIR",
    "TEBAKO_RUNTIME_MIRROR",
    "TEBAKO_CONTRACT_VERSION",
    "TEBAKO_DEBUG_TFS",
];

fn is_contract(name: &str) -> bool {
    CONTRACT_VARS.contains(&name) || name.starts_with("TEBAKO_MOUNT_")
}

/// Blank every non-contract `TEBAKO_*` variable except the names the
/// payload's `env_pass` declared. Called at boot's end, after every
/// contract read; the scrub journals each NAME at debug (never a
/// value).
pub fn scrub(env: &dyn Env, pass: &[String]) {
    for name in env.names_with_prefix("TEBAKO_") {
        if is_contract(&name) || pass.iter().any(|p| p == &name) {
            continue;
        }
        env.set_var(&name, "");
        tebako_log::log!(
            tebako_log::Level::Debug,
            "driver",
            "scrubbed packager env var {name}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

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

    #[test]
    fn the_packager_token_is_scrubbed_and_the_contract_kept() {
        let env = env_with(&[
            ("TEBAKO_GITHUB_TOKEN", "ghp_secret"),
            ("TEBAKO_CI_INTERNAL_FLAG", "1"),
            ("TEBAKO_RUNTIME_IMAGE", "/store/img.tfs"),
            ("TEBAKO_JAIL", "ro:/"),
            ("TEBAKO_MOUNT_TOOLS", "/opt/tools"),
            ("HOME", "/home/u"),
        ]);
        scrub(&env, &[]);
        let map = env.0.into_inner();
        assert_eq!(map.get("TEBAKO_GITHUB_TOKEN").map(String::as_str), Some(""));
        assert_eq!(
            map.get("TEBAKO_CI_INTERNAL_FLAG").map(String::as_str),
            Some("")
        );
        assert_eq!(
            map.get("TEBAKO_RUNTIME_IMAGE").map(String::as_str),
            Some("/store/img.tfs")
        );
        assert_eq!(map.get("TEBAKO_JAIL").map(String::as_str), Some("ro:/"));
        assert_eq!(
            map.get("TEBAKO_MOUNT_TOOLS").map(String::as_str),
            Some("/opt/tools")
        );
        assert_eq!(map.get("HOME").map(String::as_str), Some("/home/u"));
    }

    #[test]
    fn the_env_pass_opt_in_carries_its_declared_names() {
        let env = env_with(&[
            ("TEBAKO_GITHUB_TOKEN", "ghp_secret"),
            ("TEBAKO_FLAVOR_KEY", "carried"),
            ("TEBAKO_OTHER_SECRET", "x"),
        ]);
        let pass = vec!["TEBAKO_FLAVOR_KEY".to_string()];
        scrub(&env, &pass);
        let map = env.0.into_inner();
        assert_eq!(
            map.get("TEBAKO_FLAVOR_KEY").map(String::as_str),
            Some("carried")
        );
        assert_eq!(map.get("TEBAKO_GITHUB_TOKEN").map(String::as_str), Some(""));
        assert_eq!(map.get("TEBAKO_OTHER_SECRET").map(String::as_str), Some(""));
    }
}

//! The runtime-own command surface (spec 07 §10) — the shim side.
//!
//! A runtime's own tools (the interpreter, its package manager, its
//! loaders) are first-class commands: `tebako install <engine>[:<command>]`
//! registers `~/.tebako/shims/<command>`, and the argv0 dispatch of such a
//! name boots the engine's runtime directly — the spec-17 wire exactly
//! (the wrapper exe + `TEBAKO_RUNTIME_IMAGE` + `--tebako-entry <token>` +
//! the user args verbatim), with NO payload resolution, mount composition,
//! spawn lock, or jail computation in between.
//!
//! The claims live in `~/.tebako/shims/.runtime-own.yaml` (a dotfile —
//! doctor's shim-dir scan skips dotfiles, the links it sits beside are the
//! surface), a YAML map `command → {engine, entry}` where `entry` is the
//! wire token recorded at install: `self` (the spec-17 reserved
//! interpreter keyword) for the bare form, the declared command name for
//! the `:command` form. The metadata is what lets `list`/`disable` answer
//! the engine class of a runtime-own name.
//!
//! The collision rule (MECE, spec 07 §10): a PAYLOAD-provided name wins
//! over a runtime-own name of the same spelling — every dispatch of a
//! claimed name first asks the provider scan; only `BareProvider::None`
//! lets the runtime-own claim drive. A later payload claim displaces the
//! runtime-own one loudly (`manage::link_shims`, the single registration
//! chokepoint).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dispatch::ExecPlan;
use crate::runtime::{self, RuntimeResolution};
use crate::{config, fail, resolve, Ctx, ShimError};
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_TEBAKO_UNAVAILABLE};

/// One runtime-own claim: the engine class plus the wire token the
/// dispatch hands to `--tebako-entry` (`self` for the bare form — the
/// spec-17 reserved interpreter keyword; the declared command name for
/// the `:command` form).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimeOwnClaim {
    pub engine: String,
    pub entry: String,
}

/// The on-disk claims map: command → claim.
pub type Claims = BTreeMap<String, RuntimeOwnClaim>;

/// The claims file — a dotfile beside the links (`manage::shims_dir`),
/// never a shim name itself.
pub fn claims_path(home: &Path) -> PathBuf {
    home.join("shims").join(".runtime-own.yaml")
}

/// Load the claims. A missing file is an empty map; an unparseable or
/// grammar-violating one is a NAMED error naming the file and entry
/// (spec 00 invariant 9 — never a silently ignored record).
pub fn load_claims(home: &Path) -> Result<Claims, ShimError> {
    let path = claims_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Claims::new()),
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let claims: Claims = serde_yaml::from_str(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse {} ({e})", path.display()),
        )
    })?;
    for (command, claim) in &claims {
        crate::manifest::check_path_component("command name", command).map_err(|e| {
            ShimError::new(EX_TEBAKO_MANIFEST, format!("{}: {}", path.display(), e.message))
        })?;
        if !valid_engine_name(&claim.engine) {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{}: \"{command}\": invalid engine name \"{}\" — the grammar is [a-z][a-z0-9-]* (spec 04 §2)",
                    path.display(),
                    claim.engine
                ),
            );
        }
        if claim.entry != "self" {
            crate::manifest::check_path_component("entry token", &claim.entry).map_err(|e| {
                ShimError::new(EX_TEBAKO_MANIFEST, format!("{}: {}", path.display(), e.message))
            })?;
        }
    }
    Ok(claims)
}

/// Write the claims (tmp + rename — a partial write is never visible, the
/// disabled-state file's discipline).
pub fn save_claims(home: &Path, claims: &Claims) -> Result<(), ShimError> {
    let dir = home.join("shims");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let path = claims_path(home);
    let tmp = dir.join(format!(".runtime-own.yaml.{}.tmp", std::process::id()));
    let text = serde_yaml::to_string(claims).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize the runtime-own claims: {e}"),
        )
    })?;
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })
}

/// The claim `command` carries, if any.
pub fn claim_for(home: &Path, command: &str) -> Result<Option<RuntimeOwnClaim>, ShimError> {
    Ok(load_claims(home)?.get(command).cloned())
}

/// Record (or replace) the claim `command → {engine, entry}`.
pub fn record(home: &Path, command: &str, engine: &str, entry: &str) -> Result<(), ShimError> {
    let mut claims = load_claims(home)?;
    claims.insert(
        command.to_string(),
        RuntimeOwnClaim {
            engine: engine.to_string(),
            entry: entry.to_string(),
        },
    );
    save_claims(home, &claims)
}

/// Drop the claim on `command`; returns it when one stood (the
/// displacement notice names the engine class it recorded).
pub fn remove(home: &Path, command: &str) -> Result<Option<RuntimeOwnClaim>, ShimError> {
    let mut claims = load_claims(home)?;
    let gone = claims.remove(command);
    if gone.is_some() {
        save_claims(home, &claims)?;
    }
    Ok(gone)
}

/// The engine name grammar (spec 04 §2 MINOR 1 — the registry entry's
/// `engine` key): a bare lowercase name, `[a-z][a-z0-9-]*`.
pub fn valid_engine_name(engine: &str) -> bool {
    let mut chars = engine.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The engine's version chain (spec 07 §10: `TEBAKO_<ENGINE>_VERSION` env
/// → the nearest `.tebako-tools.yaml` → the config `defaults:` entry →
/// the registry's newest), rendered as the resolution constraint. A
/// payload-qualified pin (`payload@version`) is meaningless on the engine
/// axis — the named grammar error, naming the link and value (the §7
/// chain-parse rule); no pin anywhere is the open constraint (the
/// registry's/store's newest answers).
pub fn engine_constraint(
    engine: &str,
    ctx: &Ctx,
    cfg: &config::UserConfig,
) -> Result<tpkg::Constraint, ShimError> {
    let raw = match resolve::chain_pick(engine, ctx, cfg)? {
        Some((pin, source)) => {
            if pin.payload.is_some() {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "{source}: invalid engine pin \"{pin}\" — the runtime-own chain takes a bare version, never payload@version (spec 07 §10)"
                    ),
                );
            }
            Some((pin.version, source.to_string()))
        }
        None => None,
    };
    let (version, source) = raw.unwrap_or_else(|| (">= 0".to_string(), "open".to_string()));
    tpkg::Constraint::new(&version).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{source}: invalid engine version \"{version}\": {e}"),
        )
    })
}

/// The resolution half of a runtime-own dispatch: the chain constraint
/// ([`engine_constraint`]) against the store first, then the fetch path
/// every payload's runtime requirement rides (`runtime::resolve_runtime_edge`
/// — the config `runtimes:` pref, `TEBAKO_RUNTIME_MIRROR`, the registered
/// registries' `kind: runtime` entries, the ruby-only product default).
pub fn engine_resolve(
    engine: &str,
    ctx: &Ctx,
    allow_download: bool,
) -> Result<runtime::CachedRuntime, ShimError> {
    let cfg = config::load_config(&ctx.home)?;
    let constraint = engine_constraint(engine, ctx, &cfg)?;
    runtime::resolve_runtime_edge(engine, None, &constraint, None, None, allow_download, ctx)
}

/// The spec-17 wire plan of a runtime-own dispatch: [`engine_resolve`]'s
/// pick with `TEBAKO_RUNTIME_IMAGE` + `--tebako-entry <token>` + the user
/// args verbatim. No payload resolution, no mount composition, no spawn
/// lock, no jail computation — a runtime-own command is the user's own
/// process.
pub fn engine_plan(
    engine: &str,
    entry: &str,
    user_args: &[String],
    ctx: &Ctx,
    allow_download: bool,
) -> Result<ExecPlan, ShimError> {
    let rt = engine_resolve(engine, ctx, allow_download)?;
    let image = rt
        .image_handoff_path()
        .expect("resolve_runtime_edge post-asserts the env image");
    let mut argv = vec![
        rt.exe.to_string_lossy().into_owned(),
        "--tebako-entry".to_string(),
        entry.to_string(),
    ];
    argv.extend(user_args.iter().cloned());
    let mut plan = ExecPlan {
        program: rt.exe.clone(),
        argv,
        env: vec![(
            "TEBAKO_RUNTIME_IMAGE".to_string(),
            image.to_string_lossy().into_owned(),
        )],
        mounts: Vec::new(),
        runtime: RuntimeResolution::Ready(Box::new(rt)),
    };
    if allow_download {
        // The trust bridge (spec 17 §2.3) rides every dispatch shape —
        // `which`-style callers pass allow_download=false and never exec.
        crate::trust::apply(&mut plan, ctx)?;
    }
    Ok(plan)
}

/// The argv0 fallthrough (spec 07 §10): `Some(plan)` when `tool` carries
/// a runtime-own claim AND no payload provides or exposes the name. The
/// payload claim wins otherwise (the collision rule) — including a sole
/// DISABLED payload claim, whose named refusal the payload path owns.
/// A disabled runtime-own claim (selectors `all` / `<engine>@all`) is the
/// named refusal; the version chain and resolution are [`engine_plan`]'s.
pub fn maybe_dispatch(
    tool: &str,
    user_args: &[String],
    ctx: &Ctx,
) -> Result<Option<ExecPlan>, ShimError> {
    let Some(claim) = claim_for(&ctx.home, tool)? else {
        return Ok(None);
    };
    if !matches!(
        resolve::provider_for_bare_default(&ctx.home, tool)?,
        resolve::BareProvider::None
    ) {
        return Ok(None);
    }
    let disabled = config::load_disabled(&ctx.home)?;
    if config::claim_disabled(&disabled, tool, &claim.engine) {
        return fail(
            EX_TEBAKO_UNAVAILABLE,
            format!(
                "the runtime-own claim on \"{tool}\" (engine {}) is disabled — re-enable it with `tebako-shim enable {tool} --of {}`",
                claim.engine, claim.engine
            ),
        );
    }
    engine_plan(&claim.engine, &claim.entry, user_args, ctx, true).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-runtime-own-test-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_claims_round_trip_and_remove() {
        let home = fresh_home("roundtrip");
        assert!(claim_for(&home, "java").unwrap().is_none());
        record(&home, "java", "java", "self").unwrap();
        record(&home, "keytool", "java", "keytool").unwrap();
        assert_eq!(
            claim_for(&home, "java").unwrap(),
            Some(RuntimeOwnClaim {
                engine: "java".to_string(),
                entry: "self".to_string()
            })
        );
        assert_eq!(
            claim_for(&home, "keytool").unwrap().unwrap().entry,
            "keytool".to_string()
        );
        // re-record replaces
        record(&home, "java", "java", "java").unwrap();
        assert_eq!(
            claim_for(&home, "java").unwrap().unwrap().entry,
            "java".to_string()
        );
        let gone = remove(&home, "java").unwrap();
        assert_eq!(gone.unwrap().engine, "java");
        assert!(claim_for(&home, "java").unwrap().is_none());
        assert!(claim_for(&home, "keytool").unwrap().is_some());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_malformed_claims_file_is_a_named_error() {
        let home = fresh_home("malformed");
        let dir = home.join("shims");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(claims_path(&home), "java: [not, a, map]\n").unwrap();
        let err = load_claims(&home).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(err.message.contains(".runtime-own.yaml"), "{err:?}");
        // a grammar-violating engine name is named too
        std::fs::write(claims_path(&home), "java:\n  engine: Java\n  entry: self\n").unwrap();
        let err = load_claims(&home).unwrap_err();
        assert!(err.message.contains("invalid engine name"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_engine_name_grammar() {
        for good in ["ruby", "node", "java", "python", "bun", "openjdk21"] {
            assert!(valid_engine_name(good), "{good}");
        }
        for bad in ["", "Ruby", "1node", "no_de", "no/slash", "no:colon", "-lead"] {
            assert!(!valid_engine_name(bad), "{bad}");
        }
    }

    #[test]
    fn a_payload_qualified_engine_pin_is_the_named_grammar_error() {
        let home = fresh_home("pin");
        std::fs::write(
            home.join("config.yaml"),
            "defaults:\n  java: metanorma@1.2.3\n",
        )
        .unwrap();
        let cfg = config::load_config(&home).unwrap();
        let ctx = Ctx {
            home: home.clone(),
            cwd: home.clone(),
            env: BTreeMap::new(),
        };
        let err = engine_constraint("java", &ctx, &cfg).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(err.message.contains("payload@version"), "{err:?}");
        // a bare version pins exactly
        std::fs::write(home.join("config.yaml"), "defaults:\n  java: \"21.0.12\"\n").unwrap();
        let cfg = config::load_config(&home).unwrap();
        let c = engine_constraint("java", &ctx, &cfg).unwrap();
        assert_eq!(c.as_str(), "21.0.12");
        // no pin anywhere is the open constraint
        std::fs::write(home.join("config.yaml"), "defaults: {}\n").unwrap();
        let cfg = config::load_config(&home).unwrap();
        let c = engine_constraint("java", &ctx, &cfg).unwrap();
        assert_eq!(c.as_str(), ">= 0");
        let _ = std::fs::remove_dir_all(&home);
    }
}

//! The trust bridge, dispatcher half (spec 17 §2.3): the loader plane's
//! resolved netconfig verdict rides the handoff env to the driver — env
//! inheritance is the wire — and the java plane gets its dispatcher-side
//! bridge, because the JVM is a spawned child (spec 30), never a driver
//! boot.
//!
//! Two moves on the composed [`ExecPlan`]:
//!
//! - **Conveyance** (every plane): the resolved verdict exports as
//!   `TEBAKO_TLS_PLATFORM_ROOTS` / `TEBAKO_EXTRA_CA` so the driver
//!   materializes its merged cert bundle (the env layer alone would lose
//!   a config-file-only verdict — `network:` in `~/.tebako/config.yaml`).
//!   The spellings have one owner: tebako-http's netconfig.
//! - **The java plane**: platform mode on windows additively appends
//!   `-Djavax.net.ssl.trustStoreType=Windows-ROOT` to
//!   `JAVA_TOOL_OPTIONS` (the JVM then trusts the OS store natively) —
//!   an additive merge, never a stomp: an existing trustStore setting is
//!   the user's own and wins. `extra_ca` with java in force is the open
//!   sub-item of roadmap 81: a loud journal line names the gap, never a
//!   silent partial-trust boot.

use tebako_http::netconfig::{NetworkConfig, TlsRoots};

use crate::dispatch::ExecPlan;
use crate::runtime::RuntimeResolution;
use crate::{Ctx, ShimError, EX_TEBAKO_MANIFEST};

/// The JVM's option channel (read by every JVM at startup; the driver
/// never touches it — spec 17 §2.2's interp_env surface). `pub` for the
/// CLI's package-run surface, which composes the same append.
pub const JAVA_TOOL_OPTIONS: &str = "JAVA_TOOL_OPTIONS";

/// The windows platform-mode bridge option: the JVM trusts the OS
/// (Windows-ROOT) store — where the GPO/MDM-pushed enterprise CA lives.
const JAVA_WINDOWS_ROOTS: &str = "-Djavax.net.ssl.trustStoreType=Windows-ROOT";

/// Apply the trust bridge to a composed plan (spec 17 §2.3). Called by
/// `dispatch` after [`crate::dispatch::plan`] — the one place every
/// dispatch shape (plain, exposed runtime edge, exposed executable edge)
/// passes through; `which` resolves without the bridge (it never execs).
pub fn apply(plan: &mut ExecPlan, ctx: &Ctx) -> Result<(), ShimError> {
    apply_with(plan, ctx, &tebako_http::netconfig::global())
}

/// [`apply`] on an explicit config (the pure, hermetic form): the
/// process-global read is the caller's.
pub fn apply_with(plan: &mut ExecPlan, ctx: &Ctx, cfg: &NetworkConfig) -> Result<(), ShimError> {
    plan.env.extend(
        tebako_http::netconfig::trust_bridge_env_from(cfg)
            .map_err(|e| ShimError::new(EX_TEBAKO_MANIFEST, e.to_string()))?,
    );
    if !plan_has_java(plan) {
        return Ok(());
    }
    // The java plane. The append is windows + platform mode exactly (the
    // JVM reads the OS store natively only there); the extra_ca gap is
    // journaled on every platform — the JVM's own truststore never sees
    // the added CAs either way.
    if cfg!(windows) && cfg.tls_roots == TlsRoots::Platform {
        let existing = plan
            .env
            .iter()
            .find(|(k, _)| k == JAVA_TOOL_OPTIONS)
            .map(|(_, v)| v.clone())
            .or_else(|| ctx.env_get(JAVA_TOOL_OPTIONS).map(str::to_string));
        if let Some(merged) = java_tool_options_merge(existing.as_deref()) {
            tebako_log::log!(
                tebako_log::Level::Debug,
                "shim",
                "java trust bridge: {JAVA_TOOL_OPTIONS} += {JAVA_WINDOWS_ROOTS}"
            );
            match plan.env.iter_mut().find(|(k, _)| k == JAVA_TOOL_OPTIONS) {
                Some(slot) => slot.1 = merged,
                None => plan.env.push((JAVA_TOOL_OPTIONS.to_string(), merged)),
            }
        }
    }
    if !cfg.extra_ca.is_empty() {
        journal_java_gap(Some(&ctx.home), "tebako-shim");
    }
    Ok(())
}

/// The extra_ca + java gap, made loud (spec 17 §2.3 — roadmap 81's open
/// sub-item): a stderr warning always, a journal line when the home is
/// known — never a silent partial-trust boot. `pub` so the CLI's
/// package-run surface names the same gap with its own prefix.
pub fn journal_java_gap(home: Option<&std::path::Path>, prefix: &str) {
    // roadmap 81's open sub-item: the materialized-PKCS12 bridge is
    // not built — the gap is loud (stderr + journal), never a silent
    // partial-trust boot.
    eprintln!(
        "{prefix}: warning: TEBAKO_EXTRA_CA / network.extra_ca does not reach the JVM plane — this dispatch involves a java runtime, whose truststore stays its own (the merged-bundle bridge serves openssl-fashioned stacks); use tls_roots: platform to bridge the JVM via the OS store"
    );
    if let Some(home) = home {
        crate::runtime::journal(
            home,
            "event=trust-bridge-java-gap mode=extra-ca detail=extra-ca-not-bridged-to-jvm",
        );
    }
}

/// Does this plan run a JVM — as the primary runtime or through a
/// spawned edge (the spawn-lock rows, spec 30 §3 / spec 32 §5)? Either
/// way the JVM reads `JAVA_TOOL_OPTIONS` from the environ every layer
/// inherits.
fn plan_has_java(plan: &ExecPlan) -> bool {
    if let RuntimeResolution::Ready(rt) = &plan.runtime {
        if rt.engine == "java" {
            return true;
        }
    }
    plan.env
        .iter()
        .find(|(k, _)| k == tpkg::runtime_store::SPAWN_LOCK_VAR)
        .and_then(|(_, v)| tpkg::runtime_store::parse_spawn_lock(v).ok())
        .is_some_and(|rows| rows.iter().any(|row| row.engine == "java"))
}

/// The java question over the package lock's `spawned[]` rows (spec 23
/// §13.6) — the CLI's package-run surface, which composes the lock
/// rather than the spawn-lock env. A row names a JVM when its runtime
/// pair's engine is `java` (directly for a runtime row, nested for a
/// payload row).
pub fn spawned_rows_have_java(rows: &[tpkg::LockedSpawned]) -> bool {
    rows.iter().any(|row| {
        let runtime = match row {
            tpkg::LockedSpawned::Runtime(r) => r,
            tpkg::LockedSpawned::Payload(p) => &p.runtime,
        };
        runtime.engine == "java"
    })
}

/// The additive `JAVA_TOOL_OPTIONS` merge (pure): the bridge option
/// joins the existing value; an existing value that already configures
/// the trustStore (the user's own setting — or a bridge option from an
/// outer dispatch) wins unchanged. `None` = leave the variable alone.
/// `pub` for the CLI's package-run surface.
pub fn java_tool_options_merge(existing: Option<&str>) -> Option<String> {
    match existing {
        // `trustStoreType` contains `trustStore` — the substring check
        // covers both the user's own truststore config and idempotency.
        Some(v) if v.contains("javax.net.ssl.trustStore") => None,
        Some(v) if v.trim().is_empty() => Some(JAVA_WINDOWS_ROOTS.to_string()),
        Some(v) => Some(format!("{} {}", v.trim_end(), JAVA_WINDOWS_ROOTS)),
        None => Some(JAVA_WINDOWS_ROOTS.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mk_ctx(home: &std::path::Path, env: &[(&str, &str)]) -> Ctx {
        Ctx {
            home: home.to_path_buf(),
            cwd: home.to_path_buf(),
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn java_plan(env: Vec<(String, String)>) -> ExecPlan {
        ExecPlan {
            program: PathBuf::from("/cache/java"),
            argv: vec!["java".to_string()],
            env,
            mounts: Vec::new(),
            runtime: RuntimeResolution::Ready(Box::new(tpkg::runtime_store::CachedRuntime {
                engine: "java".to_string(),
                lang_version: "21.0.12".to_string(),
                tebako_version: "2.5.0".to_string(),
                dir: PathBuf::new(),
                exe: PathBuf::from("/cache/java"),
                image: None,
                abi: None,
                implementation: None,
                language_version: None,
            })),
        }
    }

    fn ruby_plan() -> ExecPlan {
        ExecPlan {
            program: PathBuf::from("/cache/ruby"),
            argv: vec!["ruby".to_string()],
            env: Vec::new(),
            mounts: Vec::new(),
            runtime: RuntimeResolution::Ready(Box::new(tpkg::runtime_store::CachedRuntime {
                engine: "ruby".to_string(),
                lang_version: "3.3.12".to_string(),
                tebako_version: "0.16.18".to_string(),
                dir: PathBuf::new(),
                exe: PathBuf::from("/cache/ruby"),
                image: None,
                abi: None,
                implementation: None,
                language_version: None,
            })),
        }
    }

    fn platform_cfg() -> NetworkConfig {
        NetworkConfig {
            tls_roots: TlsRoots::Platform,
            ..Default::default()
        }
    }

    #[test]
    fn the_java_merge_is_additive_and_never_a_stomp() {
        // Unset → the bridge option alone.
        assert_eq!(
            java_tool_options_merge(None).as_deref(),
            Some(JAVA_WINDOWS_ROOTS)
        );
        // Empty/whitespace reads as unset.
        assert_eq!(
            java_tool_options_merge(Some("  ")).as_deref(),
            Some(JAVA_WINDOWS_ROOTS)
        );
        // An unrelated existing value gains the option (additive).
        assert_eq!(
            java_tool_options_merge(Some("-Xmx1g")).as_deref(),
            Some("-Xmx1g -Djavax.net.ssl.trustStoreType=Windows-ROOT")
        );
        // The user's own truststore config wins — never a stomp.
        assert_eq!(
            java_tool_options_merge(Some("-Djavax.net.ssl.trustStore=C:/corp.p12")),
            None
        );
        // Idempotent: an already-bridged value is left alone.
        assert_eq!(
            java_tool_options_merge(Some("-Xmx1g -Djavax.net.ssl.trustStoreType=Windows-ROOT")),
            None
        );
    }

    #[test]
    fn plan_has_java_reads_the_primary_runtime_and_the_lock() {
        // The primary runtime's engine.
        assert!(plan_has_java(&java_plan(Vec::new())));
        assert!(!plan_has_java(&ruby_plan()));
        // A spawned runtime row.
        let plan = {
            let mut p = ruby_plan();
            p.env.push((
                tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
                tpkg::runtime_store::spawn_lock_entry("java", "21.0.12", "2.5.0"),
            ));
            p
        };
        assert!(plan_has_java(&plan));
        // A spawned payload row nests the provider's runtime pair.
        let plan = {
            let mut p = ruby_plan();
            p.env.push((
                tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
                tpkg::runtime_store::spawn_lock_payload_entry(
                    "xml2rfc", "3.34.0", "java", "21.0.12", "2.5.0",
                ),
            ));
            p
        };
        assert!(plan_has_java(&plan));
        // A non-java lock answers false.
        let plan = {
            let mut p = ruby_plan();
            p.env.push((
                tpkg::runtime_store::SPAWN_LOCK_VAR.to_string(),
                tpkg::runtime_store::spawn_lock_entry("python", "3.13.5", "2.1.10"),
            ));
            p
        };
        assert!(!plan_has_java(&plan));
    }

    #[test]
    fn spawned_rows_have_java_reads_both_row_shapes() {
        let pin = |c: char| tpkg::DigestPin::One(c.to_string().repeat(64));
        let artifact = |c: char| tpkg::LockedSpawnedArtifact {
            slot: None,
            sha256: pin(c),
            install_as: None,
        };
        let runtime_row = |engine: &str| tpkg::LockedSpawnedRuntime {
            engine: engine.to_string(),
            implementation: None,
            constraint: tpkg::Constraint::new(">= 21, < 26").unwrap(),
            expose: vec!["java".to_string()],
            version: "21.0.12".to_string(),
            tebako: "2.1.5".to_string(),
            carry: false,
            exe: artifact('a'),
            image: artifact('b'),
            dll: None,
            source: Some(
                "https://github.com/tamatebako/tebako-runtime-openjdk/releases/download"
                    .to_string(),
            ),
        };
        assert!(spawned_rows_have_java(&[tpkg::LockedSpawned::Runtime(
            runtime_row("java")
        )]));
        assert!(!spawned_rows_have_java(&[tpkg::LockedSpawned::Runtime(
            runtime_row("python")
        )]));
        // A payload row's nested runtime pair answers the question too
        // (its expose stays empty — the spawn surface rides the payload
        // row's own expose list, spec 32 §6).
        let mut nested = runtime_row("java");
        nested.expose = Vec::new();
        let payload_row = tpkg::LockedSpawned::Payload(tpkg::LockedSpawnedPayload {
            payload: "xml2rfc".to_string(),
            constraint: tpkg::Constraint::new(">= 3.0").unwrap(),
            expose: vec!["xml2rfc".to_string()],
            version: "3.34.0".to_string(),
            carry: false,
            image: artifact('c'),
            runtime: nested,
            source: Some("https://example.invalid/releases".to_string()),
        });
        assert!(spawned_rows_have_java(&[payload_row]));
        assert!(!spawned_rows_have_java(&[]));
    }

    #[test]
    fn the_conveyance_extends_every_plan() {
        let home =
            std::env::temp_dir().join(format!("tebako-shim-trust-convey-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let ctx = mk_ctx(&home, &[]);
        // The default verdict exports nothing.
        let mut plan = ruby_plan();
        apply_with(&mut plan, &ctx, &NetworkConfig::default()).unwrap();
        assert!(plan.env.is_empty(), "{:?}", plan.env);
        // Platform mode conveys the marker — java or not.
        let mut plan = ruby_plan();
        apply_with(&mut plan, &ctx, &platform_cfg()).unwrap();
        assert_eq!(
            plan.env,
            vec![(tebako_http::PLATFORM_ROOTS_ENV.to_string(), "1".to_string())]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(windows)]
    #[test]
    fn windows_platform_java_appends_the_bridge_option() {
        let home =
            std::env::temp_dir().join(format!("tebako-shim-trust-java-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let ctx = mk_ctx(&home, &[(JAVA_TOOL_OPTIONS, "-Xmx1g")]);
        let mut plan = java_plan(Vec::new());
        apply_with(&mut plan, &ctx, &platform_cfg()).unwrap();
        assert_eq!(
            plan.env
                .iter()
                .find(|(k, _)| k == JAVA_TOOL_OPTIONS)
                .map(|(_, v)| v.as_str()),
            Some("-Xmx1g -Djavax.net.ssl.trustStoreType=Windows-ROOT")
        );
        // The user's own truststore config is never stomped.
        let ctx = ctx(
            &home,
            &[(JAVA_TOOL_OPTIONS, "-Djavax.net.ssl.trustStore=C:/corp.p12")],
        );
        let mut plan = java_plan(Vec::new());
        apply_with(&mut plan, &ctx, &platform_cfg()).unwrap();
        assert!(!plan.env.iter().any(|(k, _)| k == JAVA_TOOL_OPTIONS));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(not(windows))]
    #[test]
    fn posix_platform_java_exports_nothing_for_the_jvm() {
        // The windows-only append is exactly that; POSIX java keeps its
        // own truststore story (the spec names no POSIX java bridge).
        let home =
            std::env::temp_dir().join(format!("tebako-shim-trust-posix-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let ctx = mk_ctx(&home, &[]);
        let mut plan = java_plan(Vec::new());
        apply_with(&mut plan, &ctx, &platform_cfg()).unwrap();
        assert!(!plan.env.iter().any(|(k, _)| k == JAVA_TOOL_OPTIONS));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn extra_ca_with_java_journals_the_gap() {
        let home =
            std::env::temp_dir().join(format!("tebako-shim-trust-gap-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let ctx = mk_ctx(&home, &[]);
        let cfg = NetworkConfig {
            extra_ca: vec![PathBuf::from("/etc/pki/corp.pem")],
            ..Default::default()
        };
        let mut plan = java_plan(Vec::new());
        apply_with(&mut plan, &ctx, &cfg).unwrap();
        // The additive verdict still conveys (the driver merges it for
        // the openssl-fashioned planes)…
        assert!(plan
            .env
            .iter()
            .any(|(k, _)| k == tebako_http::netconfig::EXTRA_CA_ENV));
        // …and the gap is journaled by name.
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        assert!(
            journal.contains("event=trust-bridge-java-gap mode=extra-ca"),
            "{journal}"
        );
        // No java in force → no gap line.
        let home2 =
            std::env::temp_dir().join(format!("tebako-shim-trust-nogap-{}", std::process::id()));
        std::fs::create_dir_all(&home2).unwrap();
        let ctx2 = mk_ctx(&home2, &[]);
        let mut plan = ruby_plan();
        apply_with(&mut plan, &ctx2, &cfg).unwrap();
        assert!(!home2.join("journal.log").exists());
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&home2);
    }
}

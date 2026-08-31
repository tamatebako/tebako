//! tebako-shim — the tebako dispatcher and version manager (spec 07;
//! retires mnenv).
//!
//! ONE static binary, linked per command name under `~/.tebako/shims/`.
//! The link shape is platform's own: symlink on unix; on Windows an
//! NTFS hardlink first (no admin needed), a byte copy as the named
//! fallback — never a `.cmd` wrapper, so argv0 dispatch is
//! byte-identical on every platform (manage.rs `link_one` carries the
//! decision record).
//! Two faces:
//!
//! - **argv0 dispatch** (spec 07 §2): invoked as `~/.tebako/shims/<tool>`,
//!   maps the command name to the installed payload that provides it,
//!   resolves the payload version (env → project pin → user default →
//!   registry default), resolves the runtime (newest COMPATIBLE cached →
//!   download; native entrypoints skip entirely), composes the mount set
//!   and execs via launcher ABI v1 (spec 06).
//! - **management commands**: invoked as `tebako-shim <cmd>` —
//!   `list | enable | disable | which | doctor | install-shell |
//!   uninstall-shell`.
//!
//! Discipline: no async, no clap, no logging framework; hand-rolled argv;
//! named errors and exit codes (spec 06 §4 reused); cache-install mirrors
//! the bootstrap's flock / tmp+rename / trust-marker discipline (spec 05
//! §4) without linking the bootstrap crate (which drags in rnp).
//!
//! The dispatch-time registry-default link resolves EVERY registry form
//! of spec 04 §2 through tebako-resolve (service contents API, pinned
//! release artifact, git blob, `file://`) behind the per-ref dispatch
//! cache in [`regcache`] (24 h TTL, `tebako update-registries`,
//! `TEBAKO_OFFLINE` = cache-or-named-error) — never tebako-cli.
//!
//! The installed payload record (the dispatcher-visible mirror, spec 03
//! §4 tier 3 rationale — resolve without opening every image):
//!
//! ```text
//! ~/.tebako/payloads/<name>/<version>.tfs              # the payload image
//! ~/.tebako/payloads/<name>/<version>.tfs.sha256       # install-time trust anchor
//! ~/.tebako/payloads/<name>/<version>.manifest.yaml    # mirrored manifest fields
//! ```
//!
//! The mirror files are written by the installer (`tebako install`,
//! roadmap 28.1 — from the image's embedded manifest when present, else
//! synthesized from the registry's tier-3 fields); tests and early
//! adopters may also seed them directly.

pub mod config;
pub mod dispatch;
pub mod manage;
pub mod manifest;
pub mod regcache;
pub mod resolve;
pub mod runtime;
pub mod shell;
pub mod shell_windows;
pub mod versions;

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Exit codes: the spec 06 §4 named set, reused by the dispatcher
/// (spec 07 §7 names the error shapes, not new codes).
pub const EX_USAGE: u8 = 64;
pub const EX_TEBAKO_MANIFEST: u8 = 65;
pub const EX_TEBAKO_UNAVAILABLE: u8 = 69;
pub const EX_TEBAKO_SHA: u8 = 70;
pub const EX_TEBAKO_IO: u8 = 74;
/// The runtime release declares a contract this shim does not speak —
/// or none at all (spec 18 C2/S11/S12): a pre-era release manifest, a
/// newer contract era, or a newer contract_version — refused BEFORE any
/// download, both sides named (tebako-resolve::contract owns the
/// semantics).
pub const EX_TEBAKO_CONTRACT: u8 = 75;

/// A named shim error: exit code + full message body (stderr gets
/// "tebako-shim: {message}\n").
#[derive(Debug)]
pub struct ShimError {
    pub code: u8,
    pub message: String,
}

impl ShimError {
    pub fn new(code: u8, message: impl Into<String>) -> ShimError {
        ShimError {
            code,
            message: message.into(),
        }
    }
}

pub fn fail<T>(code: u8, message: impl Into<String>) -> Result<T, ShimError> {
    Err(ShimError::new(code, message))
}

/// Execution context — every outside input the dispatcher reads (home,
/// cwd, environment), injected so library tests never touch the process
/// environment.
pub struct Ctx {
    /// The tebako home (`~/.tebako`, or `$TEBAKO_HOME`).
    pub home: PathBuf,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
}

impl Ctx {
    pub fn from_env() -> Result<Ctx, ShimError> {
        let env: BTreeMap<String, String> = std::env::vars().collect();
        let home = tebako_home(&env)?;
        let cwd = std::env::current_dir()
            .map_err(|e| ShimError::new(EX_TEBAKO_IO, format!("cannot determine cwd: {e}")))?;
        Ok(Ctx { home, cwd, env })
    }

    pub fn env_get(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }
}

/// The tebako home resolution: `$TEBAKO_HOME` > platform default. The
/// grammar's single owner is `tpkg::runtime_store::tebako_home` (spec 00
/// §8/§10 — the store root is the store grammar's concern; the driver
/// resolves through the same function at spawn).
pub fn tebako_home(env: &BTreeMap<String, String>) -> Result<PathBuf, ShimError> {
    tpkg::runtime_store::tebako_home(|k| env.get(k).cloned())
        .map_err(|m| ShimError::new(EX_TEBAKO_IO, m))
}

/// What a run produced: either a hand-off to exec (dispatch) or text to
/// print (management commands; `code` is the process exit code).
#[derive(Debug)]
pub enum Action {
    Exec(Box<dispatch::ExecPlan>),
    Print { text: String, code: u8 },
}

/// The command a shim FILE name refers to: the dispatcher is linked as
/// `<command>` (unix) or `<command>.exe` (Windows — PATHEXT needs the
/// suffix, manage.rs `shim_file_name`), so the suffix is stripped here.
/// Registration and lookup keys are always suffix-free.
pub(crate) fn command_from_shim_name(file_name: &str) -> &str {
    file_name.strip_suffix(".exe").unwrap_or(file_name)
}

/// The binary's two faces (spec 07 §2.0): linked as `<tool>` → dispatch;
/// invoked as `tebako-shim` → management commands.
pub fn run(argv: &[String], ctx: &Ctx) -> Result<Action, ShimError> {
    check_store_layout(ctx)?;
    let argv0 = argv.first().cloned().unwrap_or_default();
    let tool = std::path::Path::new(&argv0)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let tool = command_from_shim_name(&tool).to_string();
    if tool == "tebako-shim" {
        manage::run_command(&argv[1..], ctx)
    } else {
        dispatch::dispatch(&tool, &argv[1..], ctx).map(|p| Action::Exec(Box::new(p)))
    }
}

/// The store layout contract (spec 18 C13/S41/S42), checked once per
/// process before any dispatch or management read: a newer stamp is the
/// upgrade refusal; a pre-versioning store is stamped and the named
/// migration announced (stderr, once — tebako-resolve::store owns the
/// semantics and the message).
fn check_store_layout(ctx: &Ctx) -> Result<(), ShimError> {
    match tebako_resolve::store::check_once(&ctx.home) {
        Ok(tebako_resolve::store::LayoutCheck::Migrated) => {
            eprintln!(
                "tebako-shim: note: {}",
                tebako_resolve::store::migration_message(&ctx.home)
            );
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(e) => fail(EX_TEBAKO_IO, e.to_string()),
    }
}

//! tebako-cli — the packager CLI (item 17's SELF-HOSTING design), a port of
//! the reference gem's lean press (tebako-chainwt lib/tebako):
//!
//!   tebako press -r <root> -e <entry> [-o <output>] [-p <prefix>]
//!                [--cwd <dir>] [-R <ruby>] [-m self-contained|shared-runtime]
//!                [--compose <tebako.yaml>] [--carry ...] [--share ...]
//!                [--image <path>:<mount>]... [--bootstrap <path>]
//!                [--tebako-version <v>] [--emit-recipe [<path>]]
//!                (--emit-recipe: render the feedstock recipe from the
//!                press that just succeeded — the graduation path)
//!   tebako press --suite <suite.yaml>   (one package, N commands —
//!                per-entry imaging + slots + the type-2 package manifest,
//!                spec 03 §6)
//!   tebako cache list
//!   tebako cache prune [--runtimes] [--payloads] [--all] [--older-than Nd]
//!   tebako add-registry <ref> [--name <alias>] [--require-signed] [--default]
//!                [--channel oci] [--expect-fingerprint <hex>]
//!                (the head signing: block's TOFU pin — spec 09 §9.1)
//!   tebako list-registries
//!   tebako update-registries
//!   tebako registry validate <path-or-url> [--json]
//!   tebako registry retire <registry-file> <name>@<version> [--force]
//!   tebako keys import <file> | list | remove <fingerprint>
//!                (the signing-key trust inputs — keyring + pins)
//!   tebako trust add <file.pem> | list | remove <name>
//!                (the operator CA store, spec 22 §4 Rule R4)
//!   tebako install <ref | name[@version]>
//!   tebako uninstall <name>
//!   tebako check <name | image.tfs | package | tebako.yaml>
//!                [--check <c>] [--list] [--record] [--keep-scratch]
//!                [--runtime <exe> --runtime-image <env.tfs>]
//!                (the payload's in-image acceptance contract, spec 26 §2)
//!   tebako blksum <image> [--out <path>] [--json]
//!   tebako solve <request.yaml> --registry <path|ref> [--lock <path>] [--locked]
//!                (the ONE producer of an image's lazy-mount digest
//!                sidecar outside publish — spec 39 §3)
//!
//! Lean press flow (gem's do_press_three_part): resolve the runtime into
//! the shared cache (in-process HTTPS via crates/tebako-http) → seed the
//! packaging environment from its layout → deploy the application under
//! the runtime itself (stub driver) → build the application image
//! in-process (the dwarfs-t Writer, `.tfs`) → stitch onto the bootstrap
//! with a tpkg trailer (runtime_ref + launcher ABI v1).
//!
//! The registry/install surface (spec 04 §2, spec 16 §3.3) resolves and
//! fetches through tebako-resolve, verifies signatures through
//! tebako-signer, and registers shims through tebako-shim — all as
//! libraries, no process spawns (spec 14 §3).
//!
//! Documented deviations from the gem (README carries the full list):
//! - the bootstrap portion comes from the local Rust tebako-bootstrap
//!   binaries first (--bootstrap / TEBAKO_BOOTSTRAP / the sibling of the
//!   tebako binary), else from the spec 19 §4 store flow: the per-triplet
//!   Rust bootstrap published with the CLI's own release, resolved into
//!   ~/.tebako/bootstraps/ (sha256-verified against the release index,
//!   tmp+rename, per-entry lock; TEBAKO_OFFLINE=1 is cache-or-named-error).
//!   The gem's BootstrapManager download of the v1 C++ release stays
//!   RETIRED — its argv0-verbatim handoff is rejected by the image-era
//!   runtime driver, so the fallback produced silently-broken packages;
//! - no mkdwarfs binary anywhere: images are built in-process via the
//!   dwarfs-t Writer and named `.tfs` (dwarfs-t-native FlatBuffers
//!   metadata; `.dwarfs` stays for upstream-compatible images);
//! - the gem/gemspec scenarios are not ported (the RuntimeSdk — native
//!   extension deploy, src/sdk.rs — is: the deploy driver builds gems'
//!   native extensions against the runtime's provisioned header tree);
//! - bundler deploy ops follow the modern rubygems behavior: the gem's
//!   unconditional `bundle install --prefer-local` degrades remote
//!   (re)resolution to dependency-free gems (fontist 3.0.10 → 0.1.0),
//!   so it is opt-in (--prefer-local; a no-op with a complete lockfile),
//!   and the gem's `force_ruby_platform=true` bundle config is not
//!   emitted — precompiled platform gems are the default, with
//!   bundler's own ruby-platform fallback for gems without one (the
//!   ported SDK's source-build path). The deploy's strip re-signs
//!   ad-hoc on macOS so precompiled .so/.bundle stay loadable;
//! - images are stitched densely (tpkg slots carry absolute offsets; the
//!   gem's 8-byte padding is cosmetic);
//! - .tebako.yml is not read.

pub mod audit;
pub mod blksum;
pub mod bundle;
pub mod check;
pub mod closure_tree;
pub mod compose;
pub mod contract;
pub mod deploy;
pub mod detect;
pub mod doctor;
pub mod engine;
pub mod error;
pub mod fetch;
pub mod image;
pub mod image_manifest;
pub mod info;
pub mod inspect;
pub mod install;
pub mod keys;
pub mod node;
pub mod options;
pub mod packager;
pub mod publish;
pub mod recipe;
pub mod registry;
pub mod registry_log;
pub mod resolve;
pub mod run;
pub mod runner;
pub mod runtime_bundle;
pub mod scenario;
pub mod sdk;
pub mod solve;
pub mod spawn;
pub mod strip;
pub mod suite;
pub mod trace;
pub mod trust;

use std::fs;
use std::path::{Path, PathBuf};

use error::{packaging_error, plain_error, TebakoError};
use options::{host_platform, EmbedSet, PressMode, PressOptions};
use resolve::Resolver;
use scenario::{check_ruby_version, ruby_version_with_gemfile, ScenarioManager};

/// Launcher ABI v1 — the bootstrap → runtime handoff contract.
pub const LAUNCHER_ABI: u32 = 1;

/// The tebako version this CLI presses with: the runtime release consumed
/// and the tebako=<...> component of the trailer's runtime_ref. Matches
/// the reference gem's Tebako::VERSION at port time.
/// The tebako release line the CLI presses against — owned by
/// tebako-resolve (the single source every consumer flows from).
pub use tebako_resolve::DEFAULT_TEBAKO_VERSION;

/// The first runtime line whose embedded driver locates the tpkg
/// trailer of a CODESIGNED package (spec 02 §1's logical EOF) — the
/// press-time floor for output the CLI ad-hoc signs. Owned by
/// tebako-resolve; every consumer flows it from there.
pub use tebako_resolve::SIGNED_PKG_RUNTIME_FLOOR;

/// The CLI version banner: the product version IS the crate semver
/// (env!("CARGO_PKG_VERSION") — the single owner; a hand-written copy
/// here froze at 0.15.9 across two releases).
pub const VERSION_BANNER: &str = concat!(
    "Tebako executable packager version ",
    env!("CARGO_PKG_VERSION")
);

pub(crate) const WARN: &str = "
******************************************************************************************************************
*                                                                                                                *
*  WARNING: You are packaging in-place, i.e.: tebako package will be placed inside application root.             *
*  It is not an error but we do not recommend it because it is a way to keep packaging old versions recrsively.  *
*                                                                                                                *
*  For example, ensure that `--root=` differs from `--output=` as described in README.adoc:                      *
*  tebako press --root='~/projects/myproject' --entry=start.rb --output=/temp/myproject.tebako                   *
*                                                                                                                *
******************************************************************************************************************
";

const WARN2: &str = "
******************************************************************************************************************
*                                                                                                                *
*  WARNING: You are creating packaging environment inside application root.                                      *
*  It is not an error but it means that all build-time artifacts will ne included in tebako package.             *
*  You do not need it unless under very special circumstances like tebako packaging tebako itself.               *
*                                                                                                                *
*  Please consider removing your exisitng `--prefix` folder abd use another one that points outside of `--root`  *
*  like tebako press --r ~/projects/myproject -e start.rb -o /temp/myproject.tebako -p ~/.tebako                 *
*                                                                                                                *
******************************************************************************************************************
";

// ---------------------------------------------------------------------
// press
// ---------------------------------------------------------------------

/// The tebako#738 fail-closed gate: a press whose output will be
/// codesigned (every macOS press — `resign_if_needed` ad-hoc signs by
/// default) against a runtime line below [`SIGNED_PKG_RUNTIME_FLOOR`]
/// produces a package that never boots (the pre-floor driver reads the
/// trailer at physical EOF, past the appended superblob) — refuse by
/// name instead of shipping it. The same pin on an UNSIGNED output
/// (every other platform) boots fine; the floor is advisory there.
/// Returns the loud warning to print, or the refusal.
fn signed_pkg_floor_gate(
    tebako_version: &str,
    codesigned: bool,
) -> Result<Option<String>, TebakoError> {
    if !tpkg::versions::below_floor(tebako_version, SIGNED_PKG_RUNTIME_FLOOR) {
        return Ok(None);
    }
    if codesigned {
        return Err(packaging_error(
            130,
            Some(&format!(
                "--tebako-version {tebako_version} is below the signed-package runtime floor {SIGNED_PKG_RUNTIME_FLOOR}: this platform ad-hoc codesigns the package, and a pre-{SIGNED_PKG_RUNTIME_FLOOR} runtime driver locates the trailer at physical EOF — past the appended code-signature superblob — so the package would fail to boot (EINVAL on the payload mount). Press with --tebako-version {SIGNED_PKG_RUNTIME_FLOOR} or newer (the default is {DEFAULT_TEBAKO_VERSION})"
            )),
        ));
    }
    Ok(Some(format!(
        "--tebako-version {tebako_version} is below the signed-package runtime floor {SIGNED_PKG_RUNTIME_FLOOR} — the package boots unsigned, but codesigning it later (a macOS distribution, say) makes it unbootable: pre-{SIGNED_PKG_RUNTIME_FLOOR} drivers read the trailer at physical EOF, past the signature superblob"
    )))
}

/// Refuse an entry that is a native executable image (tebako#925).
///
/// A payload entry is a program the INTERPRETER runs, so a native binary has
/// no interpreter to hand off to: the press resolved a runtime nobody asked
/// for and emitted a package whose first execution died at mount with
/// `EINVAL`. Refusing here, before the runtime is resolved and before any
/// image is written, costs nothing and names both remedies.
fn refuse_native_entry(root: &Path, entrance: &str) -> Result<(), TebakoError> {
    use std::io::Read;
    let path = root.join(entrance);
    let Ok(mut f) = std::fs::File::open(&path) else {
        return Ok(());
    };
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return Ok(());
    }
    let format = match magic {
        [0x7f, b'E', b'L', b'F'] => Some("ELF"),
        [0xcf, 0xfa, 0xed, 0xfe] | [0xce, 0xfa, 0xed, 0xfe] => Some("Mach-O"),
        [0xfe, 0xed, 0xfa, 0xcf] | [0xfe, 0xed, 0xfa, 0xce] => Some("Mach-O"),
        // A FAT/universal Mach-O — every system binary on macOS arm64 is
        // one, so this is the shape a user is likeliest to press. Its magic
        // is a count + fat_arch table, not a thin header (tebako#925).
        [0xca, 0xfe, 0xba, 0xbe] | [0xca, 0xfe, 0xba, 0xbf] => Some("Mach-O universal"),
        [0xbe, 0xba, 0xfe, 0xca] | [0xbf, 0xba, 0xfe, 0xca] => Some("Mach-O universal"),
        [b'M', b'Z', ..] => Some("PE"),
        _ => None,
    };
    match format {
        None => Ok(()),
        Some(format) => Err(packaging_error(
            144,
            Some(&format!(
                "'{}' is a {format} image — say which interpreter runs it (a script entry, or --compose for a payload slice), or wait for the native payload kind: pressing it today would resolve a runtime nobody asked for and emit a package that dies at mount",
                path.display()
            )),
        )),
    }
}

pub fn press(opts: &PressOptions) -> Result<PathBuf, TebakoError> {
    // tebako#460 (spec 19 §6a): the named embed dial is the compose
    // surface's — refuse BEFORE any bootstrap/deploy work when no compose
    // document backs it (never a silent no-op, never a wasted press).
    if let Some(EmbedSet::Names(_)) = &opts.embed {
        if opts.compose.is_none() {
            return Err(plain_error(
                "--embed name,… selects compose slices, but this press carries no compose document — use --embed=all|none or press with a --compose document",
            ));
        }
    }

    // tebako#738: the signed-package floor gates EVERY press shape
    // (suites included) before any heavy work — a refusal must not
    // cost a runtime download.
    if let Some(warning) = signed_pkg_floor_gate(&opts.tebako_version, cfg!(target_os = "macos"))? {
        eprintln!("tebako: WARNING: {warning}");
    }
    // --suite: one package, N entries (spec 03 §6 — src/suite.rs).
    if let Some(suite_path) = &opts.suite {
        // The graduation path renders a SINGLE project's recipe; a suite
        // already carries its own composition document, and a suite-shaped
        // recipe is a different grammar. Refuse by name rather than emit
        // half a document or ignore the flag.
        if opts.emit_recipe.is_some() {
            return Err(plain_error(
                "--emit-recipe renders the recipe for ONE pressed project; a suite press carries its own suite document, so the two do not combine",
            ));
        }
        if opts.compose.is_some() || opts.carry.is_some() || opts.share.is_some() {
            return Err(packaging_error(
                130,
                Some("--compose/--carry/--share do not apply to a suite press (the suite's entries carry their own refs)"),
            ));
        }
        let yaml = fs::read_to_string(suite_path).map_err(|e| {
            plain_error(format!(
                "cannot read the suite file {}: {e}",
                suite_path.display()
            ))
        })?;
        let spec = suite::parse_suite(&yaml, suite_path)?;
        let dir = suite_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        return suite::press_suite(opts, &spec, &dir);
    }
    if opts.mode == PressMode::Runtime {
        return Err(packaging_error(133, None));
    }
    if opts.mode == PressMode::Classic {
        return Err(plain_error(
            "the 'classic' press mode is a later tebako-rs milestone (use --mode=shared-runtime or --mode=self-contained)",
        ));
    }

    // OptionsManager construction order: the --Ruby value is validated
    // before any scenario checks.
    if let Some(requested) = &opts.ruby_requested {
        check_ruby_version(requested)?;
    }

    // spec 08: --jail is validated before any heavy work (a bad spec must
    // not cost a runtime download). The parsed policy becomes the type-2
    // package manifest's `jail:` block at stitch time — the package's
    // host-access REQUEST (the user can tighten it at run time, never
    // loosen it).
    let jail = match &opts.jail {
        Some(spec) => Some(tpkg::HostJail::from_cli_spec(spec).map_err(|e| {
            packaging_error(
                130,
                Some(&format!("invalid --jail specification '{spec}': {e}")),
            )
        })?),
        None => None,
    };

    // spec 23 §3/§13: the composition document parses before any heavy
    // work (a bad document must not cost a runtime download); the
    // closure resolves below, once the ruby version is known. The D5
    // overrides (--carry/--share) refine a composition — they require
    // --compose.
    let compose_doc = match &opts.compose {
        Some(path) => {
            let (doc, warnings) = compose::load(path)?;
            for warning in &warnings {
                eprintln!("tebako: WARNING: {warning}");
            }
            Some(doc)
        }
        None => {
            if opts.carry.is_some() || opts.share.is_some() {
                return Err(packaging_error(
                    130,
                    Some(
                        "--carry/--share refine a composition and require --compose <tebako.yaml>",
                    ),
                ));
            }
            None
        }
    };

    // spec 23 §14: the `quiet_notices` registry setting resolves across
    // the three channels — CLI > env > compose document > default; a
    // malformed env value is a named error, never a silent default.
    let quiet_notices = effective_quiet_notices(opts, compose_doc.as_ref())?;

    // spec 09 §9 / spec 23 §14: the `sign` registry setting resolves
    // through the same channels (CLI > env > compose > default
    // unsigned). A keyid naming no key in $TEBAKO_HOME/keys is a named
    // error raised HERE, before any heavy work (runtime download,
    // imaging) — never a fallback; an opt-out dropping a lower channel's
    // declaration is loud (stderr + the audit journal).
    let sign = effective_sign(opts, compose_doc.as_ref())?;

    // Cli#bootstrap: the cache version guard runs before the press
    // (skipped in devmode, like the gem).
    if !opts.devmode {
        version_cache_check(opts);
    }

    let mut scenario = ScenarioManager::new(&opts.root(), &opts.fs_entrance())?;
    scenario.configure_scenario()?;

    // tebako#925: refuse a native executable entry before any heavy work.
    refuse_native_entry(Path::new(&opts.root()), &opts.fs_entrance())?;

    // tebako#895: a press that names no ruby and no tebako version takes the
    // registries' default pair — the same resolver `tebako install` and the
    // node arm use — rather than the crate constants. Otherwise a machine
    // that just installed a runtime presses on a different one: two
    // runtimes, a second download, and a package running on a Ruby the
    // author never chose.
    let default_pair = if ruby_takes_registry_default(opts, scenario.with_gemfile) {
        Some(resolve_ruby_pair(opts)?)
    } else {
        None
    };

    let ruby_ver = if let Some(pair) = &default_pair {
        pair_versions(&pair.exe, "ruby")?.0
    } else if scenario.with_gemfile {
        ruby_version_with_gemfile(opts.ruby_requested.as_deref(), &scenario.gemfile_path)?
    } else {
        opts.ruby_requested
            .clone()
            .unwrap_or_else(|| scenario::DEFAULT_RUBY_VERSION.to_string())
    };

    check_warnings(opts);
    // The banner names the language version the press will use. Ruby's is
    // known before resolution (the chain picks by requirement); node's IS
    // the resolved pair, so its banner waits for the resolution below.
    if opts.engine == crate::options::PressEngine::Ruby {
        println!("{}", opts.press_announce(&ruby_ver));
    }

    let package = format!("{}{}", opts.package(), scenario.exe_suffix);
    let stem = Path::new(&package)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| package.clone());

    let platform = host_platform()?;
    // The node arm's FIRST gate runs before any runtime is resolved: a
    // project without a lockfile cannot be vendored, and a download that
    // ends in that refusal would be a wasted one.
    if opts.engine == crate::options::PressEngine::Node {
        crate::node::require_lockfile(Path::new(&opts.root()))?;
    }

    // The bootstrap is resolved BEFORE the runtime download: the local
    // sources first, else the spec 19 §4 store flow (the CLI's own
    // release asset, downloaded once into the store). The retired v1 C++
    // download fallback never fires.
    let bootstrap_path = press_bootstrap(opts, &platform)?;

    let runtime_resolver = Resolver::new();
    // The engine axis (arc 02's node backend): ruby resolves through the
    // store's ruby-pair resolver; node resolves through the SAME chain the
    // managed dispatch uses (the factory registries), so a press and a
    // `tebako run node` pick the same pair by construction. An
    // unconstrained ruby press takes that same chain (tebako#895) — the
    // pair it resolved above is already in the store.
    let (resolved, resolved_dll) = if opts.engine == crate::options::PressEngine::Node {
        let pair = resolve_node_pair(opts)?;
        (pair_resolved(&pair), None)
    } else if let Some(pair) = &default_pair {
        (pair_resolved(pair), None)
    } else {
        let resolved =
            runtime_resolver.resolve_runtime(&ruby_ver, &platform, &opts.tebako_version)?;
        // The windows dll-era facet (tebako-runtime-ruby#40), when the
        // resolved release carries one — a carried runtime's third slot.
        let dll = runtime_resolver.resolved_dll(&ruby_ver, &platform, &opts.tebako_version);
        (resolved, dll)
    };
    let runtime_path = resolved.executable.clone();
    // The engine's language version, from the pair that was actually
    // resolved: for ruby the pre-resolution value and the pair agree by
    // construction; for node the pair IS the authority (the press never
    // guesses a node line).
    let (ruby_ver, runtime_tebako_version) = if let Some(pair) = &default_pair {
        pair_versions(&pair.exe, opts.engine.as_str())?
    } else if opts.engine == crate::options::PressEngine::Node {
        let (lv, ver) = pair_versions(&runtime_path, "node")?;
        (lv, ver)
    } else {
        (ruby_ver, opts.tebako_version.clone())
    };
    if opts.engine == crate::options::PressEngine::Node {
        println!("{}", opts.press_announce(&ruby_ver));
    }

    let app_image =
        packager::build_app_image(opts, &mut scenario, &resolved, &ruby_ver, &bootstrap_path)?;

    // The composition (spec 23 §13): the document's preset governs the
    // carry defaults; an explicit --mode OVERRIDES it (the invocation
    // beats authored defaults); --carry/--share rewrite the per-slice
    // verdicts. The FULL closure resolves at press time — the lock bakes
    // exactly what press tested (spec 23 §4).
    let mut compose_slices: Vec<compose::ComposeSlice> = Vec::new();
    let mut runtime_carried = opts.mode == PressMode::Fat;
    // tebako#460 (spec 19 §6a): an explicit --embed wins over the mode
    // preset. `all` carries everything; `none` externalizes every slice
    // (the runtime pair included — resolution-only); the named subset
    // embeds exactly those slices, named-error on a name the resolved
    // closure never produced (never a silent no-op).
    if let Some(embed) = &opts.embed {
        match embed {
            EmbedSet::All => runtime_carried = true,
            EmbedSet::None => runtime_carried = false,
            // The named form passed the top-of-press guard (a compose
            // document exists); the selection against resolve_closure's
            // slice set lands with the closure walk.
            EmbedSet::Names(_) => {}
        }
    }
    // The spawn walk's (preset, home) inputs (spec 23 §13.6): the compose
    // document's resolved pair when one drives the press; else the
    // --mode-derived preset and a lazily resolved store home (a spawn-less
    // plain press never pays for it).
    let mut spawn_preset = match opts.mode {
        PressMode::Fat => tpkg::ComposePreset::SelfContained,
        _ => tpkg::ComposePreset::SharedRuntime,
    };
    let mut spawn_home: Option<PathBuf> = None;
    if let Some(doc) = &compose_doc {
        let mut doc = doc.clone();
        let preset = if opts.mode_explicit {
            match opts.mode {
                PressMode::Fat => tpkg::ComposePreset::SelfContained,
                // Classic/Runtime are gated above; Lean is the shared
                // preset.
                _ => tpkg::ComposePreset::SharedRuntime,
            }
        } else {
            doc.preset
        };
        compose::check_runtime_row(&doc, &ruby_ver)?;
        compose::check_entrypoint(&doc, &stem)?;
        // tebako#460 (spec 19 §6a over spec 23 §13.2): the named embed
        // dial IS carry-by-name on the compose surface — route it into
        // the same D5 override walk (one selection grammar, one set of
        // named errors). An explicit --embed=all|none maps to the same
        // global carry flips the mode preset governs.
        let (embed_carry, embed_share): (Option<String>, Option<String>) = match &opts.embed {
            Some(EmbedSet::All) => (Some("all".to_string()), None),
            Some(EmbedSet::None) => (Some("none".to_string()), None),
            Some(EmbedSet::Names(names)) => (Some(names.join(",")), None),
            None => (None, None),
        };
        let carry = embed_carry.as_deref().or(opts.carry.as_deref());
        let share = embed_share.as_deref().or(opts.share.as_deref());
        compose::apply_overrides(&mut doc, carry, share, &stem)?;
        let home = compose::tebako_home()?;
        let host = install::host_platform()?;
        compose_slices =
            compose::resolve_closure(&home, &tebako_resolve::Fetcher::new(), &doc, preset, host)?;
        runtime_carried = doc
            .runtime
            .carry
            .unwrap_or_else(|| preset.default_carry(true));
        spawn_preset = preset;
        spawn_home = Some(home);
    }

    let mut images: Vec<(PathBuf, String, u32)> = vec![(
        app_image.clone(),
        declared_mount(&scenario.fs_mount_point).to_string(),
        opts.format.tpkg_format_id(),
    )];
    for (path, mount) in opts.images()? {
        images.push((PathBuf::from(path), mount, tpkg::TPKG_FORMAT_DWARFS));
    }
    // Carried compose slices ride as ordinary slots (spec 23 §13.1) with
    // the AUTO format hint — press did not author the bytes, detection
    // stays authoritative (the §4 orthogonality law). The mount is the
    // requiring edge's, else "" (a cache prime).
    let mut carried_slots: Vec<(String, u32)> = Vec::new();
    for s in &compose_slices {
        if s.carry {
            carried_slots.push((s.name.clone(), images.len() as u32));
            images.push((
                s.cache_path.clone(),
                s.mount.clone().unwrap_or_default(),
                tpkg::TPKG_FORMAT_AUTO,
            ));
        }
    }

    // The lock's runtime row (spec 23 §4/§13): EVERY press locks the
    // runtime it resolved. Carried = the two-slot self-contained shape
    // (spec 19 §6.1: exe + env image, + the windows dll facet when the
    // release declares one) with the press-time digest pins; shared =
    // the version alone, resolved at run time through the ordinary
    // spec 05 §5 chain.
    let lock_runtime = if runtime_carried {
        let Some(image_ref) = &resolved.image else {
            return Err(packaging_error(
                130,
                Some("self-contained press requires an image-era runtime (the resolved runtime carries no env image)"),
            ));
        };
        let exe_sha = resolve::sha256_file_hex(&runtime_path)
            .ok_or_else(|| plain_error(format!("cannot hash {}", runtime_path.display())))?;
        let exe_slot = images.len() as u32;
        images.push((runtime_path.clone(), String::new(), tpkg::TPKG_FORMAT_AUTO));
        let image_path = runtime_path
            .parent()
            .map(|dir| dir.join(&image_ref.filename))
            .unwrap_or_else(|| PathBuf::from(&image_ref.filename));
        let image_slot = images.len() as u32;
        images.push((image_path, String::new(), tpkg::TPKG_FORMAT_DWARFS));
        let dll = match &resolved_dll {
            Some(d) => {
                let slot = images.len() as u32;
                images.push((d.path.clone(), String::new(), tpkg::TPKG_FORMAT_AUTO));
                Some(tpkg::LockedArtifact {
                    slot,
                    sha256: tpkg::DigestPin::One(d.sha256.clone()),
                    install_as: Some(d.install_as.clone()),
                })
            }
            None => None,
        };
        tpkg::LockedRuntime {
            version: ruby_ver.clone(),
            carry: true,
            exe: Some(tpkg::LockedArtifact {
                slot: exe_slot,
                sha256: tpkg::DigestPin::One(exe_sha),
                install_as: None,
            }),
            image: Some(tpkg::LockedArtifact {
                slot: image_slot,
                sha256: tpkg::DigestPin::One(image_ref.sha256.clone()),
                install_as: None,
            }),
            dll,
        }
    } else {
        tpkg::LockedRuntime {
            version: ruby_ver.clone(),
            carry: false,
            exe: None,
            image: None,
            dll: None,
        }
    };

    // The lock's slice rows, in mount order: the app payload first
    // (always carried — slot 0), then the resolved composition closure.
    let app_sha = resolve::sha256_file_hex(&app_image)
        .ok_or_else(|| plain_error(format!("cannot hash {}", app_image.display())))?;
    let mut lock_slices = vec![tpkg::LockedSlice {
        name: stem.clone(),
        version: "0.0.0".to_string(),
        carry: true,
        slot: Some(0),
        mount: Some(declared_mount(&scenario.fs_mount_point).to_string()),
        sha256: tpkg::DigestPin::One(app_sha),
        source: None,
    }];
    for s in &compose_slices {
        let slot = carried_slots
            .iter()
            .find(|(name, _)| name == &s.name)
            .map(|(_, slot)| *slot);
        lock_slices.push(tpkg::LockedSlice {
            name: s.name.clone(),
            version: s.version.clone(),
            carry: s.carry,
            slot,
            mount: s.mount.clone(),
            sha256: s.pin.clone(),
            // Carried: provenance; shared: the fetch coordinates the
            // bootstrap resolves from (spec 23 §13.4).
            source: Some(s.source.clone()),
        });
    }
    // The lock's spawned rows (spec 23 §13.6, spec 30 §2, spec 32 §6):
    // press walks the app image's L1 `requires:` — every kind: runtime
    // edge and every expose-carrying kind: executable edge — resolves
    // each exactly as managed dispatch would, and pins the carried bytes
    // (the slots follow the app/slice/carried-runtime slots in walk
    // order). A spawn-less app image composes no rows and never resolves
    // the store home; `tebako-pkg validate` cross-checks the mirror.
    let spawn = spawn::resolve_spawned_edges(
        || match spawn_home {
            Some(home) => Ok(home),
            None => compose::tebako_home(),
        },
        &tebako_resolve::Fetcher::new(),
        &app_image,
        &stem,
        spawn_preset,
        install::host_platform()?,
        images.len() as u32,
    )?;
    images.extend(spawn.images);
    let lock = tpkg::PackageLock {
        runtime: Some(lock_runtime),
        slices: lock_slices,
        spawned: spawn.rows,
    };

    let mut runtime_ref = format!(
        "{}@{ruby_ver};tebako={}",
        opts.engine.as_str(),
        runtime_tebako_version
    );
    if resolved.image.is_some() {
        // item 30b: the runtime is image-era — the bootstrap resolves
        // the .tfs alongside the interpreter at first run.
        runtime_ref.push_str(";image");
    }

    // The L2 package manifest rides EVERY runnable press (spec 03 §6):
    // entries[0] names the in-image dispatcher entry (mount-relative —
    // the driver joins mount+entry, spec 17 §1) and the mounts block
    // declares the app slot's union over the env image at the runtime
    // root (the image-era mount model — the env image owns the root,
    // the app image merges over it). A --jail press composes the policy
    // into the SAME block (spec 08 §4 — one manifest, never two paths).
    // The lock (spec 23 §4) rides the same manifest.
    // The entry the package boots: ruby's dispatcher (which composes the
    // scenario's entry) or, on the node arm, the app's own script — the
    // driver composes `node <entry>` from the runtime's argv contract.
    let entrypoint = if opts.engine == crate::options::PressEngine::Node {
        format!("/local/{}", opts.entrance)
    } else {
        "/local/stub.rb".to_string()
    };
    let interp_env = if opts.engine == crate::options::PressEngine::Node {
        Default::default()
    } else {
        authored_interp_env(&PathBuf::from(opts.root()))
    };
    let package_manifest = press_package_manifest(
        &package,
        &runtime_ref,
        &opts.tebako_version,
        declared_mount(&scenario.fs_mount_point),
        &entrypoint,
        jail,
        Some(lock),
        interp_env,
    );
    stitch(
        &bootstrap_path,
        &images,
        &package,
        &runtime_ref,
        Some(&package_manifest),
        StitchFlags {
            lean: !runtime_carried,
            no_install: opts.no_install,
            quiet_notices,
            sign,
        },
    )?;
    println!("Created tebako package at \"{package}\"");
    // The graduation path: the recipe renders from the press that just
    // succeeded, so it can never describe a press that did not run.
    if let Some(recipe_path) = &opts.emit_recipe {
        let path = if recipe_path.as_os_str().is_empty() {
            recipe::default_path(opts)
        } else {
            recipe_path.clone()
        };
        let slices: Vec<(String, Option<String>)> = compose_slices
            .iter()
            .map(|s| (s.name.clone(), s.mount.clone()))
            .collect();
        let in_image_entry = if opts.engine == crate::options::PressEngine::Node {
            format!("/local/{}", opts.entrance)
        } else {
            scenario.fs_entry_point.clone()
        };
        let lockfile = if opts.engine == crate::options::PressEngine::Node {
            Some("package-lock.json".to_string())
        } else {
            Path::new(&scenario.gemfile_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        };
        recipe::emit(
            &recipe::RecipeInputs {
                opts,
                resolved: &resolved,
                lang_version: &ruby_ver,
                runtime_tebako: &runtime_tebako_version,
                platform: &platform,
                package: Path::new(&package),
                entry_path: &in_image_entry,
                lockfile,
                slices: &slices,
            },
            &path,
        )?;
    }
    ensure_version_file(opts);
    Ok(PathBuf::from(package))
}

/// Resolve the node runtime pair through the managed dispatch chain (the
/// configured factory registries) — the arc's rule: a press and a
/// `tebako run node` must pick the same pair, because they are the same
/// resolver. The requirement (`engines.node`) becomes the constraint when
/// the project declared one; a project that declared none takes the
/// registry's newest line, exactly like a bare `tebako run node`.
fn resolve_node_pair(
    opts: &PressOptions,
) -> Result<tpkg::runtime_store::CachedRuntime, TebakoError> {
    let constraint = match opts.engine_requirement.as_deref() {
        Some(spelled) if !spelled.trim().is_empty() => {
            tpkg::Constraint::new(spelled).map_err(|e| {
                packaging_error(
                    130,
                    Some(&format!(
                        "engines.node \"{spelled}\" is not a constraint this press can honour: {e}"
                    )),
                )
            })?
        }
        _ => any_constraint(),
    };
    // A press bakes the runtime's FILES into the package, so it needs the
    // whole env image on disk: the lazy install (spec 39 §10's default) seeds
    // 4 MiB groups on demand at RUN time and leaves no whole image for the
    // press to read. Force the eager install for the resolution.
    let mut env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    env.insert("TEBAKO_RUNTIME_LAZY".to_string(), "0".to_string());
    let home = tebako_shim::tebako_home(&env)
        .map_err(|e| plain_error(format!("cannot resolve $TEBAKO_HOME: {}", e.message)))?;
    let ctx = tebako_shim::Ctx {
        home,
        cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
        env,
    };
    tebako_shim::runtime::resolve_runtime_edge("node", None, &constraint, None, None, true, &ctx)
        .map_err(|e| {
            plain_error(format!(
                "cannot resolve the node runtime pair: {}",
                e.message
            ))
        })
}

/// A resolved runtime pair, as the press's image slot describes it.
fn pair_resolved(pair: &tpkg::runtime_store::CachedRuntime) -> resolve::Resolved {
    let image = pair.image.as_ref().map(|path| resolve::ImageRef {
        filename: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        sha256: resolve::sha256_file_hex(path).unwrap_or_default(),
        signature: None,
    });
    resolve::Resolved {
        executable: pair.exe.clone(),
        image,
    }
}

/// tebako#895: does this press take the registries' default runtime pair?
/// Only when it named nothing that picks a runtime by itself — no `-R`, no
/// Gemfile `ruby` directive, no `--tebako-version`. Anything the author
/// named wins, as it always has.
fn ruby_takes_registry_default(opts: &PressOptions, with_gemfile: bool) -> bool {
    opts.engine == crate::options::PressEngine::Ruby
        && opts.ruby_requested.is_none()
        && !with_gemfile
        && !opts.tebako_version_explicit
}

/// Resolve the ruby runtime pair through the managed dispatch chain — the
/// same resolver `tebako install` uses, so the pair a press bakes is the
/// pair the machine already has (tebako#895). A project that declared
/// `ruby` in its Gemfile never reaches here: it keeps the store resolver,
/// which maps the requirement onto a line.
fn resolve_ruby_pair(
    opts: &PressOptions,
) -> Result<tpkg::runtime_store::CachedRuntime, TebakoError> {
    let constraint = match opts.engine_requirement.as_deref() {
        Some(spelled) if !spelled.trim().is_empty() => {
            tpkg::Constraint::new(spelled).map_err(|e| {
                packaging_error(
                    130,
                    Some(&format!(
                        "the declared ruby requirement \"{spelled}\" is not a constraint this press can honour: {e}"
                    )),
                )
            })?
        }
        _ => any_constraint(),
    };
    // A press bakes the runtime's FILES into the package, so it needs the
    // whole env image on disk: the lazy install (spec 39 §10's default) seeds
    // 4 MiB groups on demand at RUN time and leaves no whole image for the
    // press to read. Force the eager install for the resolution.
    let mut env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    env.insert("TEBAKO_RUNTIME_LAZY".to_string(), "0".to_string());
    let home = tebako_shim::tebako_home(&env)
        .map_err(|e| plain_error(format!("cannot resolve $TEBAKO_HOME: {}", e.message)))?;
    let ctx = tebako_shim::Ctx {
        home,
        cwd: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
        env,
    };
    tebako_shim::runtime::resolve_runtime_edge("ruby", None, &constraint, None, None, true, &ctx)
        .map_err(|e| {
            plain_error(format!(
                "cannot resolve the ruby runtime pair: {}",
                e.message
            ))
        })
}

/// The "any version" constraint — what an unconstrained press asks the
/// resolver for. There is no `*` spelling in the manifest grammar (it is a
/// named parse error: a clause version must be dot-separated components), so
/// `>= 0` carries the meaning: every published version satisfies it. A wrong
/// spelling here is a PANIC on the ordinary bare-project press rather than an
/// error a user can read, which is why it is pinned by a unit test.
fn any_constraint() -> tpkg::Constraint {
    tpkg::Constraint::new(">= 0").expect(">= 0 parses")
}

/// The language version of an installed runtime pair, read from its store
/// entry name (`<engine>-<lv>-<ver>-<triplet>`): the press names the
/// payload's runtime ref from the pair it vendored with. The expected engine
/// is the caller's; a name that belongs to another engine is a named error
/// rather than a silent misparse.
fn pair_versions(exe: &Path, expected_engine: &str) -> Result<(String, String), TebakoError> {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| {
            plain_error(format!(
                "cannot read the pair's entry name from {}",
                dir.display()
            ))
        })?;
    let platform = host_platform()?;
    let rest = name
        .strip_suffix(&platform)
        .and_then(|s| s.strip_suffix('-'))
        .ok_or_else(|| {
            plain_error(format!(
                "the pair's entry {name} does not end in {platform}"
            ))
        })?;
    let (engine, tail) = rest.split_once('-').ok_or_else(|| {
        plain_error(format!(
            "the pair's entry {name} is not <engine>-<lv>-<ver>"
        ))
    })?;
    let (lv, ver) = tail.rsplit_once('-').ok_or_else(|| {
        plain_error(format!(
            "the pair's entry {name} is not <engine>-<lv>-<ver>"
        ))
    })?;
    if engine != expected_engine {
        return Err(plain_error(format!(
            "the pair's entry {name} names engine {engine}, not {expected_engine}"
        )));
    }
    Ok((lv.to_string(), ver.to_string()))
}

fn check_warnings(opts: &PressOptions) {
    let mut warned = false;
    if opts.package_within_root() {
        println!("{WARN}");
        warned = true;
    }
    if opts.prefix_within_root() {
        println!("{WARN2}");
        warned = true;
    }
    if warned {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

/// Bootstrap lookup: --bootstrap > $TEBAKO_BOOTSTRAP > the Rust
/// tebako-bootstrap next to the tebako binary (the dogfooding /
/// installed-pair layout). `None` means no local source named a binary —
/// [`press_bootstrap`] then resolves the published bootstrap into the
/// store (spec 19 §4). The v1 C++ bootstrap download stays retired (its
/// argv0-verbatim handoff is rejected by the image-era runtime driver, so
/// the fallback produced silently-broken packages).
pub(crate) fn decide_bootstrap(opts: &PressOptions) -> Option<PathBuf> {
    if let Some(path) = &opts.bootstrap {
        return Some(path.clone());
    }
    if let Ok(env_path) = std::env::var("TEBAKO_BOOTSTRAP") {
        if !env_path.is_empty() {
            return Some(PathBuf::from(env_path));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) {
                "tebako-bootstrap.exe"
            } else {
                "tebako-bootstrap"
            };
            let sibling = dir.join(name);
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    None
}

/// The package's bootstrap portion: the local sources first
/// ([`decide_bootstrap`]; a named file that does not exist keeps the
/// gem's parity error 127), else the spec 19 §4 store flow — the Rust
/// tebako-bootstrap published with the CLI's own release, resolved into
/// ~/.tebako/bootstraps/<version>-<triplet>/ (downloaded once,
/// sha256-verified against the release index, tmp+rename installed under
/// the entry lock; named errors 137-142, TEBAKO_OFFLINE=1 is
/// cache-or-named-error 138).
pub(crate) fn press_bootstrap(opts: &PressOptions, platform: &str) -> Result<PathBuf, TebakoError> {
    press_bootstrap_with(opts, platform, &resolve::BootstrapResolver::new())
}

/// [`press_bootstrap`] with the store resolver injected (tests pin the
/// cache root/mirror/version so the unit suite never touches the
/// network or the real store).
pub(crate) fn press_bootstrap_with(
    opts: &PressOptions,
    platform: &str,
    store: &resolve::BootstrapResolver,
) -> Result<PathBuf, TebakoError> {
    if let Some(path) = decide_bootstrap(opts) {
        if !path.is_file() {
            return Err(packaging_error(
                127,
                Some(&format!("runtime not found: {}", path.display())),
            ));
        }
        return Ok(path);
    }
    store.resolve(platform)
}

/// spec 23 §14: the `quiet_notices` registry setting resolved across the
/// three channels — CLI > env > compose document > default (a suite
/// press rides no document and passes `None`). A malformed env value is
/// a named error, never a silent default.
pub(crate) fn effective_quiet_notices(
    opts: &PressOptions,
    compose: Option<&tpkg::ComposeDoc>,
) -> Result<bool, TebakoError> {
    tpkg::settings::resolve_bool(
        &tpkg::settings::QUIET_NOTICES,
        opts.quiet_notices,
        tpkg::settings::QUIET_NOTICES.env_value(),
        compose.and_then(|d| d.quiet_notices),
    )
    .map_err(|e| packaging_error(65, Some(&e.to_string())))
}

/// The signature failure exit (the trust family's 71, shared with the
/// loader and the install/publish surfaces).
const EX_TEBAKO_SIGNATURE: i32 = 71;

/// spec 09 §9 / spec 23 §14: the `sign` registry setting resolved across
/// the three channels — CLI > env > compose document > default unsigned
/// (a suite press rides no document and passes `None`). An opt-out that
/// drops a lower channel's `sign` declaration is LOUD: a stderr warning
/// plus the audit journal's `event=press-sign-opt-out`. A `--sign=<keyid>`
/// naming no key in $TEBAKO_HOME/keys is a NAMED error here — before any
/// heavy work — never a silent fallback to the press-local key.
pub(crate) fn effective_sign(
    opts: &PressOptions,
    compose: Option<&tpkg::ComposeDoc>,
) -> Result<tebako_pkg::SignRequest, TebakoError> {
    let resolved = tpkg::settings::resolve_sign(
        &tpkg::settings::SIGN,
        opts.sign.clone(),
        tpkg::settings::SIGN.env_value(),
        compose.and_then(|d| d.sign),
    )
    .map_err(|e| packaging_error(65, Some(&e.to_string())))?;
    if let Some(opt_out) = &resolved.opt_out {
        eprintln!(
            "tebako: WARNING: the {} channel's sign opt-out overrides the {} channel's sign declaration — the package will be UNSIGNED (journaled: event=press-sign-opt-out)",
            opt_out.by, opt_out.overridden
        );
        // Best-effort, like every journal write: a press that cannot
        // open the journal still presses (unsigned, as declared).
        if let Some(journal) = tfs::journal::open_journal() {
            tfs::journal::journal_press_sign_opt_out(&journal, opt_out.by, opt_out.overridden);
        }
    }
    match resolved.decision {
        tpkg::settings::SignDecision::Unsigned => Ok(tebako_pkg::SignRequest::None),
        tpkg::settings::SignDecision::PressLocal => Ok(tebako_pkg::SignRequest::PressLocal),
        tpkg::settings::SignDecision::Keyid(keyid) => {
            // Fail fast, before the runtime download and the imaging: a
            // keyid naming no key in $TEBAKO_HOME/keys is a named error,
            // never a fallback to the press-local key (spec 09 §9).
            let home = tebako_signer::default_home()
                .map_err(|e| TebakoError::new(e.to_string(), EX_TEBAKO_SIGNATURE))?;
            match tebako_signer::secret_key_by_keyid(&home, &keyid) {
                Ok(Some(_)) => Ok(tebako_pkg::SignRequest::Keyid(keyid)),
                Ok(None) => Err(TebakoError::new(
                    format!(
                        "no secret key with keyid {keyid} under {}/keys — generate one, or use bare --sign for the press-local key",
                        home.display()
                    ),
                    EX_TEBAKO_SIGNATURE,
                )),
                Err(e) => Err(TebakoError::new(e.to_string(), EX_TEBAKO_SIGNATURE)),
            }
        }
    }
}

/// The trailer flag inputs to [`stitch`] (spec 02): `lean` — the runtime
/// resolves at run time (TPKG_FLAG_LEAN; CLEAR only for a self-contained
/// press); `no_install` — publisher-frozen (TPKG_FLAG_NO_INSTALL);
/// `quiet_notices` — the resolved registry setting (TPKG_FLAG_QUIET_NOTICES,
/// spec 23 §14); `sign` — the resolved `sign` registry setting (spec 09
/// §9: `None` is the exact v1-unsigned shape, byte-identical to
/// pre-signing output).
#[derive(Debug, Clone, Default)]
pub(crate) struct StitchFlags {
    pub lean: bool,
    pub no_install: bool,
    pub quiet_notices: bool,
    pub sign: tebako_pkg::SignRequest,
}

/// Stitcher.stitch (lean three-part): validate per the gem's error codes,
/// then assemble with tebako-pkg (dense image layout — tpkg slots carry
/// absolute offsets, so the gem's 8-byte padding is not required), chmod,
/// and re-sign ad-hoc on macOS when the binary was signed.
/// `runtime_ref` is the trailer's 128-byte field as built by the caller
/// (suites: entries[0]'s ref — the type-2 manifest carries the per-entry
/// refs, spec 02 §5b / spec 03 §6). `package_manifest`, when present, is
/// embedded as extension block type 2 (every runnable press writes one —
/// the union mount model, spec 03 §6); `None` keeps the package
/// block-less (the v1 shape — bare payload stitching, tests). `lean` is
/// the TPKG_FLAG_LEAN verdict: CLEAR only for a self-contained press
/// (the runtime rides as the two carried slots, spec 19 §6.1 / spec 23
/// §13.2) — every other package resolves the runtime at run time.
pub(crate) fn stitch(
    bootstrap_path: &Path,
    images: &[(PathBuf, String, u32)],
    package: &str,
    runtime_ref: &str,
    package_manifest: Option<&tpkg::PackageManifest>,
    flags: StitchFlags,
) -> Result<(), TebakoError> {
    if images.is_empty() {
        return Err(packaging_error(126, Some("at least one image is required")));
    }
    if images.len() > tpkg::TPKG_MAX_SLOTS as usize {
        return Err(packaging_error(
            126,
            Some(&format!(
                "{} images given, at most {} are supported",
                images.len(),
                tpkg::TPKG_MAX_SLOTS
            )),
        ));
    }
    if !bootstrap_path.is_file() {
        return Err(packaging_error(
            127,
            Some(&format!("runtime not found: {}", bootstrap_path.display())),
        ));
    }
    for (path, mount, format_id) in images {
        if !path.is_file() {
            return Err(packaging_error(
                127,
                Some(&format!("image not found: {}", path.display())),
            ));
        }
        if *format_id > tpkg::TPKG_FORMAT_LIMNIFS {
            return Err(packaging_error(
                126,
                Some(&format!("invalid format_id {format_id} (0..=5 expected)")),
            ));
        }
        if mount.len() >= tpkg::TPKG_MOUNT_POINT_LEN {
            return Err(packaging_error(
                126,
                Some(&format!(
                    "mount point '{}'... exceeds {} bytes",
                    &mount[..32.min(mount.len())],
                    tpkg::TPKG_MOUNT_POINT_LEN - 1
                )),
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (_, mount, format_id) in images {
        if *format_id == tpkg::TPKG_FORMAT_RUNTIME {
            continue; // payload slots are never mounted
        }
        // Suite members share one mount point by construction (the
        // bootstrap's argv0 selection mounts only the selected entry's
        // slot — spec 03 §6); without a package manifest a duplicate
        // mount stays the error it always was.
        if package_manifest.is_some() {
            continue;
        }
        if !seen.insert(mount) {
            return Err(packaging_error(
                126,
                Some(&format!("duplicate mount point '{mount}'")),
            ));
        }
    }
    if runtime_ref.len() >= tpkg::TPKG_RUNTIME_REF_LEN {
        return Err(packaging_error(
            126,
            Some(&format!(
                "runtime_ref '{runtime_ref}' exceeds {} bytes",
                tpkg::TPKG_RUNTIME_REF_LEN - 1
            )),
        ));
    }

    let output = Path::new(package);
    if let Some(dir) = output.parent() {
        fs::create_dir_all(dir).map_err(|e| plain_error(format!("{e}")))?;
    }
    let pkg_images: Vec<tebako_pkg::PackageImage> = images
        .iter()
        .map(|(path, mount, format_id)| tebako_pkg::PackageImage {
            path: path.clone(),
            mount_point: mount.clone(),
            format_id: *format_id,
        })
        .collect();
    let pkg_options = tebako_pkg::PackageOptions {
        runtime_ref: runtime_ref.to_string(),
        // TPKG_FLAG_LEAN unless the press is self-contained;
        // TPKG_FLAG_NO_INSTALL when the press froze the package
        // (--no-install,); TPKG_FLAG_QUIET_NOTICES when
        // the registry setting resolved true (spec 23 §14).
        package_flags: {
            let mut pf = match (flags.lean, flags.no_install) {
                (true, true) => tpkg::TPKG_FLAG_LEAN | tpkg::TPKG_FLAG_NO_INSTALL,
                (true, false) => tpkg::TPKG_FLAG_LEAN,
                (false, true) => tpkg::TPKG_FLAG_NO_INSTALL,
                (false, false) => 0,
            };
            if flags.quiet_notices {
                pf |= tpkg::TPKG_FLAG_QUIET_NOTICES;
            }
            pf
        },
        launcher_abi: LAUNCHER_ABI,
        // The L2 package manifest rides along when the caller declares
        // one (every runnable press does — the union mount model);
        // block-less stitching stays available for bare payloads. The
        // entry's runtime_ref mirrors the trailer's, so old and new
        // loaders resolve identically (spec 02 §5b).
        package_manifest: package_manifest.cloned(),
        // spec 09 §9: the resolved `sign` setting. `None` never touches
        // key material and produces the exact v1-unsigned shape.
        sign: flags.sign,
    };
    tebako_pkg::bundle_exact(bootstrap_path, &pkg_images, output, &pkg_options)
        .map_err(plain_error)?;
    chmod_755(output);
    resign_if_needed(output);
    Ok(())
}

/// The declared (POSIX) form of a mount point: the uniform VFS
/// namespace name the trailer slots and L2 `mounts:` rows carry on
/// every platform — the physical root minus any windows drive
/// (`A:/t` → `/t`; POSIX roots are unchanged). The driver
/// re-qualifies declared mounts onto the VFS drive at boot (spec 17
/// §1), so the argv grammar never has to carry a drive colon.
pub(crate) fn declared_mount(mount_point: &str) -> &str {
    let b = mount_point.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        &mount_point[2..]
    } else {
        mount_point
    }
}

/// The L2 package manifest every runnable press writes (spec 03 §6 /
/// spec 02 §5b): minimal identity (press has no package-version input —
/// a --package-version flag is a later milestone), one entry naming the
/// package whose entrypoint is the in-image dispatcher (`/local/stub.rb`
/// — mount-relative; the driver joins mount+entry, spec 17 §1),
/// `runtime_ref` mirroring the trailer field, and the `mounts:` row
/// declaring the app slot's union over the env image at the runtime
/// root (`mount_point` — the trailer's own mount point, the root's
/// declared form: `/__tfs__` on POSIX, `/t` on windows — so the two stay
/// consistent by construction). A --jail press composes the policy into
/// the same block — the package's host-access REQUEST the bootstrap
/// tightens at handoff (spec 08 §2). `lock` is the press-time
/// composition lock (spec 23 §4/§13) — baked by every runnable press.
#[allow(clippy::too_many_arguments)]
fn press_package_manifest(
    package: &str,
    runtime_ref: &str,
    tebako_version: &str,
    mount_point: &str,
    entrypoint: &str,
    jail: Option<tpkg::HostJail>,
    lock: Option<tpkg::PackageLock>,
    interp_env: std::collections::BTreeMap<String, String>,
) -> tpkg::PackageManifest {
    let stem = Path::new(package)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| package.to_string());
    tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: stem.clone(),
            version: "0.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: tebako_version.to_string(),
            },
            created: reproducible_stamp(),
        },
        entries: vec![tpkg::PackageEntry {
            name: stem.clone(),
            slot: Some(0),
            entrypoint: entrypoint.to_string(),
            runtime_ref: runtime_ref.to_string(),
            interp_env,
        }],
        jail,
        env: Default::default(),
        lock,
        mounts: vec![tpkg::PackageMount {
            slot: 0,
            point: mount_point.to_string(),
            mode: tpkg::MountMode::Union,
            precedence: Some(tpkg::Precedence::AfterEnv),
        }],
    }
}

/// The press-time read of the app tree's authored payload manifest for
/// the L2 `entries[].interp_env` composition (spec 03 §6): the plain
/// press has no separate packager-refinement surface — the authored L1
/// IS the packager's declaration, mirrored verbatim so the size-gated
/// bootstrap never reads image bytes (spec 17 §2.2). A malformed
/// authored manifest skips the composition LOUDLY — the image already
/// carries it unstamped (image.rs's discipline) and the package's own
/// verify grades it; a silent drop would run the interpreter on its
/// built-in defaults without a word.
fn authored_interp_env(root: &Path) -> std::collections::BTreeMap<String, String> {
    let path = root.join(tpkg::merkle::MANIFEST_DIR).join("manifest.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Default::default();
    };
    match tpkg::PayloadManifest::from_yaml(&text) {
        Ok(m) => m.interp_env,
        Err(e) => {
            eprintln!(
                "tebako: WARNING: cannot compose entries[].interp_env from {}: {e}",
                path.display()
            );
            Default::default()
        }
    }
}

/// RFC 3339 UTC rendering of now (the manifest `created` convention; no
/// chrono in the tree — the civil-from-days algorithm is Howard
/// Hinnant's).
fn rfc3339_now() -> String {
    rfc3339_utc(now_secs())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The press-time stamp every producer-made manifest reads:
/// `SOURCE_DATE_EPOCH` when the operator set it (the reproducible-builds
/// convention — the image writer reads the same variable), the wall clock
/// otherwise.
///
/// With it set, a press of an unchanged tree reproduces byte for byte:
/// the two values that used to read the clock (`identity.version` and
/// `identity.created` in the app manifest, `package.created` in the
/// trailer) all pin to the same instant. Unset, the default stands
/// (tebako#735: every ordinary re-press differs, so install's overwrite
/// rule keeps the newest payload).
pub(crate) fn reproducible_stamp() -> String {
    match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) => stamp_from(Some(&value)),
        Err(_) => stamp_from(None),
    }
}

/// [`reproducible_stamp`]'s body over an explicit value (the testable
/// half): a parseable epoch pins the stamp, anything else falls back to
/// the clock with the operator's error named.
fn stamp_from(source_date_epoch: Option<&str>) -> String {
    let Some(value) = source_date_epoch else {
        return rfc3339_now();
    };
    match value.trim().parse::<u64>() {
        Ok(secs) => rfc3339_utc(secs),
        Err(_) => {
            eprintln!(
                "tebako: WARNING: SOURCE_DATE_EPOCH='{value}' is not an integer number of seconds — using the wall clock"
            );
            rfc3339_now()
        }
    }
}

fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn chmod_755(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = fs::metadata(path) {
            let mut perms = m.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

/// Codesigning: a pressed package must carry a VALID signature to exec
/// on macOS arm64. The press excises the input bootstrap's stale
/// signature (spec 31 §1.2 — tebako-pkg emits unsigned-by-construction
/// Mach-O output, the only shape codesign accepts post-press), so the
/// CLI ad-hoc re-signs the output, best-effort: on failure a warning is
/// printed and the package is kept.
fn resign_if_needed(output: &Path) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let ok = std::process::Command::new("codesign")
        .args(["--sign", "-", "--force"])
        .arg(output)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        // the gem warns on stderr (Kernel#warn)
        eprintln!(
            "Warning: ad-hoc re-sign failed for {}; the package will not execute on macOS arm64 (an unsigned or invalidly-signed Mach-O is killed at exec). Re-sign it with your own identity.",
            output.display()
        );
    }
}

/// CacheManager#ensure_version_file (best effort).
pub(crate) fn ensure_version_file(opts: &PressOptions) {
    let deps = opts.deps();
    let _ = fs::create_dir_all(&deps);
    let version_file = deps.join(".environment.version");
    if let Err(e) = fs::write(&version_file, version_key(opts)) {
        println!(
            "{} .environment.version: {e}",
            error::packaging_message(201).unwrap_or("Warning. Could not create cache version file")
        );
    }
}

/// The version key: "<tebako version> at <CLI source dir>" (the gem uses
/// the gem's source checkout; the CLI uses its crate manifest dir).
fn version_key(opts: &PressOptions) -> String {
    format!("{} at {}", opts.tebako_version, env!("CARGO_MANIFEST_DIR"))
}

/// CacheManager#version_cache_check: a stale or foreign packaging
/// environment is cleaned up before the press. Missing/unparseable file
/// → "not recognized" + clean_cache; version mismatch → "created by a
/// gem version" + clean_cache; source mismatch → "created for a
/// different source directory" + clean_output.
pub(crate) fn version_cache_check(opts: &PressOptions) {
    let version_file = opts.deps().join(".environment.version");
    let parsed = fs::read_to_string(&version_file).ok().and_then(|content| {
        let line = content.lines().next()?.to_string();
        let (version, source) = line.split_once(" at ")?;
        Some((version.to_string(), source.to_string()))
    });
    let Some((version, source)) = parsed else {
        println!("CMake cache version was not recognized, cleaning up");
        clean_cache(opts);
        return;
    };
    if version != opts.tebako_version {
        println!(
            "Tebako cache was created by a gem version {version} and cannot be used for gem version {}",
            opts.tebako_version
        );
        clean_cache(opts);
    } else if source != env!("CARGO_MANIFEST_DIR") {
        println!(
            "CMake cache was created for a different source directory '{source}' and cannot be used for '{}'",
            env!("CARGO_MANIFEST_DIR")
        );
        clean_output(opts);
    }
}

/// clean_cache: rm -rf <prefix>/deps and <prefix>/o.
fn clean_cache(opts: &PressOptions) {
    println!("Cleaning tebako packaging environment");
    let _ = fs::remove_dir_all(opts.deps());
    let _ = fs::remove_dir_all(opts.output_folder());
}

/// clean_output: rm -rf <prefix>/o.
fn clean_output(opts: &PressOptions) {
    println!("Cleaning CMake cache");
    let _ = fs::remove_dir_all(opts.output_folder());
}

// ---------------------------------------------------------------------
// cache subcommands (CacheCli port)
// ---------------------------------------------------------------------

pub fn cache_list() {
    let manager = Resolver::new();
    let entries = manager.entries();
    let payloads = tebako_resolve::PayloadCache::with_root(&manager.cache_root).list();
    if entries.is_empty() {
        println!(
            "Runtime package cache is empty ({})",
            manager.cache_root.join("runtimes").display()
        );
    } else {
        let mut total = 0u64;
        for entry in &entries {
            total += entry.size_bytes;
            // spec 39 §7: a LAZY_SEEDING entry reads distinctly.
            let seeding = lazy_seed_state_of(&entry.path)
                .map(|state| {
                    format!(
                        " (seeding {}%, {}/{} groups)",
                        state.percent(),
                        state.present.len(),
                        state.seed.group_count
                    )
                })
                .unwrap_or_default();
            println!(
                "{:<44} {:>9}  {}{}",
                entry.name,
                human_size(entry.size_bytes),
                human_age(entry.installed_at),
                seeding
            );
        }
        println!(
            "{:<44} {:>9}",
            format!("Total ({} package(s))", entries.len()),
            human_size(total)
        );
    }
    if payloads.is_empty() {
        return;
    }
    // The payload section: the JSON form's fields (name/version/size)
    // plus the age column the runtime lines carry.
    println!(
        "Payload cache ({}):",
        manager.cache_root.join("payloads").display()
    );
    let mut total = 0u64;
    for entry in &payloads {
        let size = fs::metadata(&entry.path).map(|m| m.len()).unwrap_or(0);
        total += size;
        let installed_at = fs::metadata(&entry.path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        println!(
            "{:<44} {:>9}  {}",
            format!("{}@{}", entry.name, entry.version),
            human_size(size),
            human_age(installed_at)
        );
    }
    println!(
        "{:<44} {:>9}",
        format!("Total ({} payload(s))", payloads.len()),
        human_size(total)
    );
}

/// `tebako cache list --json` (spec 15 §4, additive): the cached runtimes
/// AND payloads with their trust anchors, origins and sizes, as one
/// `"info_schema": 1` document. Read-only; the human form is unchanged.
pub fn cache_list_json() {
    use tebako_pkg::{json_to_string, JsonValue as J};

    let s = |v: &str| J::String(v.to_string());
    let n = |v: u64| J::Number(v.to_string());

    let manager = Resolver::new();
    let mut total = 0u64;

    let mut runtimes = Vec::new();
    for entry in manager.entries() {
        total += entry.size_bytes;
        let marker = |name: &str| {
            fs::read_to_string(entry.path.join(name))
                .ok()
                .map(|t| t.trim().to_string())
        };
        // The trust anchor is the digest; tolerate the sha256sum form
        // ("<digest>  <file>") by taking the first token.
        let anchor = marker("sha256").and_then(|t| t.split_whitespace().next().map(str::to_string));
        let mut obj = vec![
            ("name".to_string(), s(&entry.name)),
            ("path".to_string(), s(&entry.path.display().to_string())),
            ("size_bytes".to_string(), n(entry.size_bytes)),
            (
                "installed_at_unix".to_string(),
                n(entry
                    .installed_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()),
            ),
        ];
        if let Some(sha) = anchor {
            obj.push(("trust_anchor_sha256".to_string(), s(&sha)));
        }
        if let Some(origin) = marker("origin") {
            obj.push(("origin".to_string(), s(&origin)));
        }
        // spec 39 §7 (additive): a LAZY_SEEDING entry's seed state.
        if let Some(state) = lazy_seed_state_of(&entry.path) {
            obj.push((
                "lazy_seeding".to_string(),
                J::Object(vec![
                    ("present_groups".to_string(), n(state.present.len() as u64)),
                    ("total_groups".to_string(), n(state.seed.group_count)),
                    ("percent".to_string(), n(state.percent())),
                ]),
            ));
        }
        runtimes.push(J::Object(obj));
    }

    let mut payloads = Vec::new();
    let cache = tebako_resolve::PayloadCache::with_root(&manager.cache_root);
    for entry in cache.list() {
        let size = fs::metadata(&entry.path).map(|m| m.len()).unwrap_or(0);
        total += size;
        let mut obj = vec![
            ("name".to_string(), s(&entry.name)),
            ("version".to_string(), s(&entry.version)),
            ("path".to_string(), s(&entry.path.display().to_string())),
            ("size_bytes".to_string(), n(size)),
            ("trust_anchor_sha256".to_string(), s(&entry.sha256)),
        ];
        if let Some(origin) = &entry.origin {
            obj.push(("origin".to_string(), s(origin)));
        }
        if let Some(registry) = &entry.registry {
            obj.push(("registry".to_string(), s(registry)));
        }
        payloads.push(J::Object(obj));
    }

    let doc = J::Object(vec![
        ("info_schema".to_string(), n(1)),
        (
            "cache_root".to_string(),
            s(&manager.cache_root.display().to_string()),
        ),
        ("runtimes".to_string(), J::Array(runtimes)),
        ("payloads".to_string(), J::Array(payloads)),
        ("total_bytes".to_string(), n(total)),
    ]);
    println!("{}", json_to_string(&doc));
}

// ---------------------------------------------------------------------
// cache seal (spec 39 §5): the toolchain's synchronous seal pass
// ---------------------------------------------------------------------

/// A runtime entry's LAZY_SEEDING state for the listing surfaces (spec
/// 39 §7): the validated seed state when the entry holds a seed
/// descriptor and the image is still absent. A torn descriptor is the
/// doctor's named problem — the listings simply do not mark.
fn lazy_seed_state_of(entry_dir: &Path) -> Option<tpkg::lazy::SeedState> {
    for f in fs::read_dir(entry_dir).ok()?.flatten() {
        let name = f.file_name().to_string_lossy().into_owned();
        let Some(image_base) = name.strip_suffix(".lazy.json") else {
            continue;
        };
        if entry_dir.join(image_base).is_file() {
            continue; // sealed already; the descriptor is stale
        }
        if let Ok(Some(state)) = tpkg::lazy::seed_state(entry_dir, image_base) {
            return Some(state);
        }
    }
    None
}

/// A lazy model failure's toolchain error (spec 39 §6's classes ride
/// the exit code verbatim).
fn seal_lazy_error(e: tpkg::lazy::LazyError) -> TebakoError {
    TebakoError::new(e.to_string(), e.exit_code())
}

/// A byte-source failure's class (spec 39 §6): malformed lazy config 65,
/// fetch/unsupported 69, integrity 70, store IO 74.
fn seal_source_error(image_base: &str, e: tfs::source::SourceError) -> TebakoError {
    let code = match e.kind {
        tfs::source::SourceErrorKind::Invalid => 65,
        tfs::source::SourceErrorKind::Fetch | tfs::source::SourceErrorKind::Unsupported => 69,
        tfs::source::SourceErrorKind::Integrity => 70,
        tfs::source::SourceErrorKind::Io => 74,
    };
    TebakoError::new(format!("the lazy env image {image_base}: {e}"), code)
}

/// The seal verb's sidecar GET: `file://` from disk (the test/airgap
/// spelling), else tebako-http's one agent (https + the loopback
/// carve-out); a non-200 is the named 69, never a guess.
fn get_sidecar(url: &str) -> Result<Vec<u8>, TebakoError> {
    if let Some(path) = url.strip_prefix("file://") {
        // RFC 8089 leaves a windows drive spelling as `/C:/x`; the one
        // inverse owns the peel, and hand-rolling it here is what made the
        // OFF-SPEC `file://C:/x` spelling the only one this verb accepted.
        return fs::read(tebako_http::file_path_from_url(path))
            .map_err(|e| TebakoError::new(format!("{url}: {e}"), 69));
    }
    let response = tebako_http::get_raw(url, Some("application/json"), None)
        .map_err(|e| TebakoError::new(format!("{url}: {e}"), 69))?;
    if response.status != 200 {
        return Err(TebakoError::new(
            format!(
                "{url}: status {} — the blksum sidecar must be a 200",
                response.status
            ),
            69,
        ));
    }
    Ok(response.body)
}

/// `tebako cache seal [<entry>|--all]` (spec 39 §5): the synchronous
/// seal pass from the toolchain — fill each LAZY_SEEDING entry's
/// missing groups over the descriptor's origin (the same per-group
/// verification the driver's background seal rides), then the seal
/// commit flips the entry to SEALED. Unlike the driver's thread the
/// verb is strict: a failure is THE named error of the run (a fetch 69,
/// a digest mismatch 70, store IO 74), never a logged skip. Offline
/// (TEBAKO_OFFLINE=1) the source is cache-only — a complete block map
/// still seals; a miss is the named offline-at-group-granularity error.
pub fn cache_seal(selector: Option<&str>, all: bool) -> Result<(), TebakoError> {
    let manager = Resolver::new();
    let runtimes = tpkg::runtime_store::scan_all_cached(&manager.cache_root);
    let targets: Vec<tpkg::runtime_store::CachedRuntime> = match (selector, all) {
        (None, true) => runtimes
            .into_iter()
            .filter(|rt| rt.lazy.is_some())
            .collect(),
        (Some(name), false) => {
            let Some(rt) = runtimes.into_iter().find(|rt| {
                rt.dir
                    .file_name()
                    .map(|n| n.to_string_lossy() == name)
                    .unwrap_or(false)
            }) else {
                return Err(TebakoError::new(
                    format!(
                        "no cached runtime entry named '{name}' — `tebako cache list` shows the entries"
                    ),
                    65,
                ));
            };
            if rt.lazy.is_none() {
                println!("{name} is sealed already — nothing to do");
                return Ok(());
            }
            vec![rt]
        }
        // main.rs's run_cache validates the grammar; anything else is a
        // programming error, not a user one.
        _ => return Err(plain_error("usage: tebako cache seal [<entry>|--all]")),
    };
    if targets.is_empty() {
        println!("no LAZY_SEEDING runtime entries — nothing to seal");
        return Ok(());
    }
    let offline = tebako_resolve::cache::offline();
    let mut prog = tebako_term::Progress::stderr();
    for rt in &targets {
        seal_one(rt, offline, &mut prog)?;
    }
    Ok(())
}

/// One entry's synchronous seal: open the descriptor's remote source
/// (cache-only offline), fill the missing groups under the byte
/// progress bar, then the seal commit. `pub(crate)` for the engine
/// install's pre-seal (tebako#843 — a LAZY_SEEDING entry has no image
/// file until the seal commits, and the install reads the image).
pub(crate) fn seal_one(
    rt: &tpkg::runtime_store::CachedRuntime,
    offline: bool,
    prog: &mut tebako_term::Progress<std::io::Stderr>,
) -> Result<(), TebakoError> {
    let Some(lazy) = &rt.lazy else {
        return Ok(());
    };
    let seed = &lazy.seed;
    let blocks = tpkg::lazy::blocks_dir(&rt.dir, &lazy.image_base);
    let source = if offline {
        tfs::source_remote::RemoteByteSource::new_cache_only(seed.size_bytes, &blocks, &seed.source)
    } else {
        // The §3 cached-blksum arm, exactly as the driver's mount-open:
        // the install-time prefetch serves first; a miss fetches and
        // writes through (best-effort).
        let blksum = match tpkg::lazy::read_blksum_cache(&rt.dir, &lazy.image_base, seed)
            .map_err(seal_lazy_error)?
        {
            Some(sum) => sum,
            None => {
                let body = get_sidecar(&format!("{}.blksum.json", seed.source))?;
                let sum = tpkg::lazy::verify_blksum(seed, &body).map_err(seal_lazy_error)?;
                let _ = tpkg::lazy::write_blksum_cache(&rt.dir, &lazy.image_base, &body);
                sum
            }
        };
        let url = seed.source.clone();
        // The spec 39 §8 closure shape: the byte source stays
        // transport-free; tebako-http's retry law rides inside
        // get_range (the driver's open_lazy_source is the runtime-side
        // twin of this wiring).
        let fetch = move |offset: u64, len: usize, if_range: Option<&str>| {
            let range = tebako_http::ByteRange {
                offset,
                len: len as u64,
            };
            match tebako_http::get_range(&url, range, if_range, None) {
                Ok(tebako_http::RangeAnswer::Partial(body)) => {
                    Ok(tfs::source_remote::RangeFetchAnswer::Partial {
                        bytes: body.bytes,
                        etag: body.etag,
                    })
                }
                Ok(tebako_http::RangeAnswer::Full(body)) => {
                    Ok(tfs::source_remote::RangeFetchAnswer::Full {
                        bytes: body.bytes,
                        etag: body.etag,
                    })
                }
                Err(e) => Err(e.to_string()),
            }
        };
        tfs::source_remote::RemoteByteSource::new(
            std::sync::Arc::new(fetch),
            blksum,
            &blocks,
            &seed.source,
        )
    }
    .map_err(|e| seal_source_error(&lazy.image_base, e))?;
    let source = std::sync::Arc::new(source);

    prog.download_begin(&format!("sealing {}", lazy.image_base));
    let mut filled = std::cmp::min(lazy.present * tpkg::lazy::LAZY_GROUP_SIZE, seed.size_bytes);
    prog.download_tick(filled, Some(seed.size_bytes));
    loop {
        let missing = source.missing_groups().map_err(|e| {
            prog.download_abort();
            seal_source_error(&lazy.image_base, e)
        })?;
        if missing.is_empty() {
            break;
        }
        for index in missing {
            if let Err(e) = source.seed_group(index) {
                prog.download_abort();
                return Err(seal_source_error(&lazy.image_base, e));
            }
            let (_, len) = seed.group_span(index).expect("a missing group is in range");
            filled = std::cmp::min(filled + len, seed.size_bytes);
            prog.download_tick(filled, Some(seed.size_bytes));
        }
    }
    prog.download_end();
    match tpkg::lazy::seal_entry(&rt.dir, &lazy.image_base) {
        Ok(tpkg::lazy::SealOutcome::Sealed { bytes, groups }) => {
            prog.line(&format!(
                "sealed {} ({}) — {groups} groups; {} is an ordinary cached runtime now",
                lazy.image_base,
                tebako_term::human_bytes(bytes),
                rt.dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        Ok(tpkg::lazy::SealOutcome::AlreadySealed) => {
            prog.line(&format!(
                "{} was already sealed (a concurrent sealer committed first)",
                lazy.image_base
            ));
        }
        Err(e) => return Err(seal_lazy_error(e)),
    }
    Ok(())
}

/// `tebako cache prune [--runtimes] [--payloads] [--all | --older-than Nd]`.
/// Bare (no area flags) prunes runtimes only — today's behavior,
/// unchanged. The payload arm rides tebako-resolve's `PayloadCache::prune`
/// with the protected set ALWAYS applied (no opt-out; `--all` respects it
/// too — prune never strands a pin): config `defaults:` pins, exact
/// `.disabled.yaml` `payload@version` selectors, and the newest installed
/// version of every payload (the roll-forward floor).
pub fn cache_prune(
    runtimes: bool,
    payloads: bool,
    all: bool,
    older_than: Option<&str>,
) -> Result<(), TebakoError> {
    let days = match (all, older_than) {
        (true, _) => Some(None),
        (false, Some(spec)) => parse_days(spec).map(Some),
        (false, None) => None,
    };
    let Some(days) = days else {
        println!("Nothing to do: pass --all or --older-than Nd");
        return Ok(());
    };
    let manager = Resolver::new();
    if project_pins_near_cwd() {
        println!(
            "project pins (.tebako-tools.yaml) are per-directory and not visible to prune — pinned-but-old versions may be removed and will re-fetch on next dispatch (offline: named error)."
        );
    }
    if runtimes || !payloads {
        let removed = manager.prune(all, days)?;
        for name in &removed {
            println!("Removed {name}");
        }
        println!("{} cached runtime package(s) removed", removed.len());
    }
    if payloads {
        let (protected, notes) = protected_payload_versions(&manager.cache_root)?;
        for note in &notes {
            println!("{note}");
        }
        let cache = tebako_resolve::PayloadCache::with_root(&manager.cache_root);
        let removed = cache
            .prune(all, days, &protected)
            .map_err(crate::install::map_resolve)?;
        for (name, version) in &removed {
            println!("Removed {name}@{version}");
        }
        println!("{} cached payload(s) removed", removed.len());
    }
    Ok(())
}

/// The prune protected set (spec 07 §2's pins, the parts prune CAN see):
/// config `defaults:` values through `tpkg::toolpin::ToolPin` (a
/// qualified `payload@version` protects exactly that pair; a bare value
/// protects `(provider, version)` only when the provider scan is
/// unambiguous — otherwise a note and nothing extra), exact
/// `payload@version` selectors from `.disabled.yaml`, and the newest
/// installed version of every payload (the roll-forward floor). The env
/// and per-directory `.tebako-tools.yaml` links are dispatch-time state
/// and stay invisible (the caveat line covers them). Loading the pin
/// state fails CLOSED: pruning without the pins could strand one.
fn protected_payload_versions(
    home: &Path,
) -> Result<(tebako_resolve::ProtectedSet, Vec<String>), TebakoError> {
    let mut protected = tebako_resolve::ProtectedSet::new();
    let mut notes = Vec::new();

    // The roll-forward floor: never prune a name's newest installed
    // version (the dispatch default when nothing is pinned).
    let entries = tebako_resolve::PayloadCache::with_root(home).list();
    let mut by_name: std::collections::BTreeMap<&String, Vec<&String>> =
        std::collections::BTreeMap::new();
    for entry in &entries {
        by_name.entry(&entry.name).or_default().push(&entry.version);
    }
    for (name, versions) in &by_name {
        if let Some(newest) = tpkg::versions::newest(versions.iter().copied()) {
            protected.insert((name.to_string(), newest));
        }
    }

    // Config defaults (the user-pin link, `tebako shim use`).
    let cfg = tebako_shim::config::load_config(home).map_err(|e| {
        plain_error(format!(
            "cannot build the prune protected set: {}",
            e.message
        ))
    })?;
    for (tool, entry) in &cfg.defaults {
        if let Some(value) = entry.version() {
            let pin = tpkg::toolpin::ToolPin::parse(value)
                .map_err(|e| plain_error(format!("config.yaml default `{tool}: {e}")))?;
            match pin.payload {
                Some(payload) => {
                    protected.insert((payload, pin.version));
                }
                None => {
                    match tebako_shim::resolve::provider_for_bare_default(home, tool).map_err(
                        |e| {
                            plain_error(format!(
                                "cannot build the prune protected set: {}",
                                e.message
                            ))
                        },
                    )? {
                        tebako_shim::resolve::BareProvider::One(payload) => {
                            protected.insert((payload, pin.version));
                        }
                        tebako_shim::resolve::BareProvider::Ambiguous(claims) => {
                            notes.push(format!(
                            "default `{tool}: {value}` is bare and more than one payload provides it ({}) — protecting nothing extra; pin `{tool}: <payload>@{version}` to protect exactly",
                            claims.join(", "),
                            version = pin.version,
                        ));
                        }
                        tebako_shim::resolve::BareProvider::None => {}
                    }
                }
            }
        }
        // A pinned slice's bytes are as load-bearing as the base's
        // (spec 07 §4) — prune never strands a pin.
        for s in entry.slices() {
            let sp = tpkg::toolpin::ToolPin::parse(s)
                .map_err(|e| plain_error(format!("config.yaml default `{tool}` slices: {e}")))?;
            if let Some(payload) = sp.payload {
                protected.insert((payload, sp.version));
            }
        }
    }

    // Exact `payload@version` disable selectors pin a version out of
    // dispatch — keep those bytes too (`all` / `payload@all` / bare
    // versions name no exact pair and protect nothing here).
    let disabled = tebako_shim::config::load_disabled(home).map_err(|e| {
        plain_error(format!(
            "cannot build the prune protected set: {}",
            e.message
        ))
    })?;
    for selectors in disabled.values() {
        for selector in selectors {
            if let Ok(tpkg::toolpin::DisableSelector::PayloadVersion(p, v)) =
                tpkg::toolpin::DisableSelector::parse(selector)
            {
                protected.insert((p, v));
            }
        }
    }
    Ok((protected, notes))
}

/// `.tebako-tools.yaml` reachable from the cwd (the shim's walk-up
/// lookup direction): the caveat line prints once per prune.
fn project_pins_near_cwd() -> bool {
    let Ok(mut dir) = std::env::current_dir() else {
        return false;
    };
    loop {
        if dir.join(".tebako-tools.yaml").is_file() {
            return true;
        }
        if !dir.pop() {
            return false;
        }
    }
}

fn parse_days(spec: &str) -> Option<u64> {
    let digits = spec.strip_suffix('d').unwrap_or(spec);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn human_size(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn human_age(installed_at: std::time::SystemTime) -> String {
    let age = std::time::SystemTime::now()
        .duration_since(installed_at)
        .unwrap_or_default()
        .as_secs();
    if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86_400 {
        format!("{}h ago", age / 3600)
    } else {
        format!("{}d ago", age / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconstrained_ruby_press_takes_the_registry_pair() {
        // tebako#895: press and `tebako install` must answer "which
        // runtime?" the same way. The registry default governs only when
        // the press named nothing — anything the author named wins.
        let mut opts = press_opts(None);
        opts.tebako_version_explicit = false;
        assert!(ruby_takes_registry_default(&opts, false));
        // -R names a ruby.
        opts.ruby_requested = Some("3.3.12".to_string());
        assert!(!ruby_takes_registry_default(&opts, false));
        let mut opts = press_opts(None);
        opts.tebako_version_explicit = false;
        // a Gemfile directive names one.
        assert!(!ruby_takes_registry_default(&opts, true));
        // --tebako-version names a runtime line.
        opts.tebako_version_explicit = true;
        assert!(!ruby_takes_registry_default(&opts, false));
        // the node arm has its own resolver and never takes this path.
        let mut opts = press_opts(None);
        opts.tebako_version_explicit = false;
        opts.engine = crate::options::PressEngine::Node;
        assert!(!ruby_takes_registry_default(&opts, false));
    }

    #[test]
    fn the_any_constraint_is_a_parseable_clause() {
        // A bare project's press asks the resolver for "any version". The
        // grammar has no `*`; `>= 0` is the clause that parses and matches
        // everything. Before this, that request panicked.
        let any = tpkg::versions::from_validated(&any_constraint());
        assert!(any.matches("3.3.12"), ">= 0 must match an ordinary version");
        assert!(any.matches("4.0.7"), ">= 0 must match the newest line too");
    }

    #[test]
    fn the_reproducible_stamp_pins_only_a_parseable_epoch() {
        // The pin: one fixed instant for every manifest value a press
        // writes, so an unchanged tree reproduces byte for byte.
        assert_eq!(stamp_from(Some("1700000000")), "2023-11-14T22:13:20Z");
        assert_eq!(stamp_from(Some(" 0 ")), "1970-01-01T00:00:00Z");
        // A malformed value is the operator's error, named — the clock
        // stands in, never a silent empty stamp.
        let clock = stamp_from(Some("not-a-number"));
        assert_ne!(clock, "2023-11-14T22:13:20Z");
        assert!(clock.ends_with('Z'), "{clock}");
        // Unset: the wall clock, the ordinary press.
        assert!(stamp_from(None).ends_with('Z'));
    }

    #[test]
    fn rfc3339_utc_known_dates() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z"); // leap day
        assert_eq!(rfc3339_utc(1_704_067_200), "2024-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1_767_225_600), "2026-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(86_399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn the_default_line_never_sits_below_the_signed_pkg_floor() {
        // tebako#738's invariant: a default press must never compose a
        // package its own runtime line cannot boot once codesigned.
        assert!(!tpkg::versions::below_floor(
            DEFAULT_TEBAKO_VERSION,
            SIGNED_PKG_RUNTIME_FLOOR
        ));
    }

    #[test]
    fn the_signed_pkg_floor_gate_refuses_a_codesigned_press_below_the_floor() {
        // tebako#738: below the floor on a codesigned output is the
        // named refusal, citing the pin, the floor, and the default.
        let err = signed_pkg_floor_gate("0.16.25", true).unwrap_err();
        assert_eq!(err.code, 130, "{err:?}");
        assert!(err.message.contains("0.16.25"), "{err:?}");
        assert!(err.message.contains(SIGNED_PKG_RUNTIME_FLOOR), "{err:?}");
        assert!(err.message.contains(DEFAULT_TEBAKO_VERSION), "{err:?}");
        assert!(err.message.contains("would fail to boot"), "{err:?}");
    }

    #[test]
    fn the_signed_pkg_floor_gate_passes_at_and_above_the_floor() {
        assert!(signed_pkg_floor_gate(SIGNED_PKG_RUNTIME_FLOOR, true)
            .unwrap()
            .is_none());
        assert!(signed_pkg_floor_gate(DEFAULT_TEBAKO_VERSION, true)
            .unwrap()
            .is_none());
    }

    #[test]
    fn the_signed_pkg_floor_gate_warns_and_proceeds_on_unsigned_output() {
        // Off macOS the output is unsigned and boots on any line — the
        // floor is advisory (a later codesign would strand it).
        let warning = signed_pkg_floor_gate("0.16.25", false)
            .unwrap()
            .expect("a loud warning");
        assert!(warning.contains("0.16.25"), "{warning}");
        assert!(warning.contains(SIGNED_PKG_RUNTIME_FLOOR), "{warning}");
    }

    #[test]
    fn press_package_manifest_carries_the_union_model_and_the_jail() {
        let jail = tpkg::HostJail::from_cli_spec("deny:arg").unwrap();
        let m = press_package_manifest(
            "/tmp/out/hello",
            "ruby@3.4.2;tebako=0.15.9;image",
            "0.15.9",
            "/__tfs__",
            "/local/stub.rb",
            Some(jail),
            None,
            Default::default(),
        );
        // Valid per the tpkg discipline (schema version, N>=1 entries, the
        // jail block's own validation, the mounts block's rules).
        m.validate().unwrap();
        assert_eq!(m.package.name, "hello");
        assert_eq!(m.package.producer.tool, "tebako-cli");
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].slot, Some(0));
        // The entry is the in-image dispatcher, mount-relative (the
        // driver joins mount+entry — spec 17 §1).
        assert_eq!(m.entries[0].entrypoint, "/local/stub.rb");
        assert_eq!(m.entries[0].runtime_ref, "ruby@3.4.2;tebako=0.15.9;image");
        // The app slot unions over the env image at the runtime root —
        // the point mirrors the trailer's mount point.
        assert_eq!(m.mounts.len(), 1);
        assert_eq!(m.mounts[0].slot, 0);
        assert_eq!(m.mounts[0].point, "/__tfs__");
        assert_eq!(m.mounts[0].mode, tpkg::MountMode::Union);
        assert_eq!(m.mounts[0].precedence, Some(tpkg::Precedence::AfterEnv));
        let jail = m.jail.as_ref().unwrap();
        assert!(!jail.default_open);
        assert!(jail.argument_files.auto);
        // The YAML form survives a round trip (the block embeds as YAML).
        let back = tpkg::PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn press_package_manifest_without_a_jail_still_declares_the_union() {
        let m = press_package_manifest(
            "/tmp/out/hello",
            "ruby@3.4.2;tebako=0.15.9;image",
            "0.15.9",
            "/__tfs__",
            "/local/stub.rb",
            None,
            None,
            Default::default(),
        );
        m.validate().unwrap();
        assert!(m.jail.is_none());
        assert_eq!(m.mounts.len(), 1);
        assert_eq!(m.mounts[0].mode, tpkg::MountMode::Union);
        let back = tpkg::PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn stitch_bakes_the_no_install_flag_only_when_asked() {
        //: --no-install freezes the package in the trailer
        // (TPKG_FLAG_NO_INSTALL); the default press leaves the bit clear
        // (installable-on-request, pre-era shape).
        let dir = std::env::temp_dir().join(format!("tebako-cli-stitch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let frozen = dir.join("frozen");
        stitch(
            &base,
            &[(img.clone(), "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            frozen.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                no_install: true,
                quiet_notices: false,
                sign: tebako_pkg::SignRequest::None,
            },
        )
        .unwrap();
        let mut f = std::fs::File::open(&frozen).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL != 0);
        assert!(m.package_flags & tpkg::TPKG_FLAG_LEAN != 0);

        let plain = dir.join("plain");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            plain.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                no_install: false,
                quiet_notices: false,
                sign: tebako_pkg::SignRequest::None,
            },
        )
        .unwrap();
        let mut f = std::fs::File::open(&plain).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL, 0);
    }

    #[test]
    fn stitch_bakes_the_quiet_notices_flag_only_when_asked() {
        // tebako#400 / spec 23 §14: the resolved `quiet_notices` setting
        // bakes TPKG_FLAG_QUIET_NOTICES (bit 3); the default press leaves
        // it clear (the legacy warning + progress lines stand).
        let dir = std::env::temp_dir().join(format!("tebako-cli-stitch-q-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let quiet = dir.join("quiet");
        stitch(
            &base,
            &[(img.clone(), "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            quiet.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                no_install: false,
                quiet_notices: true,
                sign: tebako_pkg::SignRequest::None,
            },
        )
        .unwrap();
        let mut f = std::fs::File::open(&quiet).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert!(m.package_flags & tpkg::TPKG_FLAG_QUIET_NOTICES != 0);
        assert!(m.is_quiet_notices());
        // The bit composes with the others, never clobbers them.
        assert!(m.package_flags & tpkg::TPKG_FLAG_LEAN != 0);
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL, 0);

        let loud = dir.join("loud");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            loud.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                no_install: false,
                quiet_notices: false,
                sign: tebako_pkg::SignRequest::None,
            },
        )
        .unwrap();
        let mut f = std::fs::File::open(&loud).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_QUIET_NOTICES, 0);
        assert!(!m.is_quiet_notices());
    }

    // ---- press signing (spec 09 §9, spec 23 §14) ----

    /// Serializes the signing tests: they mutate the process env
    /// (TEBAKO_HOME / TEBAKO_SIGN / TEBAKO_JAIL_JOURNAL /
    /// TEBAKO_REQUIRE_SIGNED), saved and restored by [`SignEnv`].
    static SIGN_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Pins the signing-relevant env to a temp home and restores the
    /// prior values on drop.
    struct SignEnv {
        saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl SignEnv {
        fn set(home: &Path, journal: &Path) -> SignEnv {
            let saved = [
                "TEBAKO_HOME",
                "TEBAKO_SIGN",
                "TEBAKO_JAIL_JOURNAL",
                "TEBAKO_REQUIRE_SIGNED",
            ]
            .into_iter()
            .map(|k| (k, std::env::var_os(k)))
            .collect::<Vec<_>>();
            std::env::set_var("TEBAKO_HOME", home);
            std::env::set_var("TEBAKO_JAIL_JOURNAL", journal);
            std::env::remove_var("TEBAKO_SIGN");
            std::env::remove_var("TEBAKO_REQUIRE_SIGNED");
            SignEnv { saved }
        }
    }

    impl Drop for SignEnv {
        fn drop(&mut self) {
            for (k, v) in self.saved.drain(..) {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    fn sign_scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tebako-cli-sign-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn effective_sign_resolves_channels_and_names_an_unknown_keyid() {
        let _guard = SIGN_ENV_LOCK.lock().unwrap();
        let dir = sign_scratch("resolve");
        let home = dir.join("home");
        let journal = dir.join("journal.log");
        let _env = SignEnv::set(&home, &journal);
        let compose_signed = tpkg::compose::parse_compose(
            "version: 1\nruntime: {ref: \"ruby@~> 3.3\"}\nsign: true\n",
        )
        .unwrap()
        .0;

        // No declaration on any channel: unsigned, no key material.
        let mut opts = press_opts(None);
        assert_eq!(
            effective_sign(&opts, None).unwrap(),
            tebako_pkg::SignRequest::None
        );

        // The compose document declares: the press-local key.
        assert_eq!(
            effective_sign(&opts, Some(&compose_signed)).unwrap(),
            tebako_pkg::SignRequest::PressLocal
        );

        // TEBAKO_SIGN=0 overriding the document is the LOUD opt-out:
        // the decision is unsigned and the journal records the drop.
        std::env::set_var("TEBAKO_SIGN", "0");
        assert_eq!(
            effective_sign(&opts, Some(&compose_signed)).unwrap(),
            tebako_pkg::SignRequest::None
        );
        let j = std::fs::read_to_string(&journal).unwrap();
        assert!(
            j.contains("event=press-sign-opt-out by=env overridden=compose"),
            "{j}"
        );

        // TEBAKO_SIGN=1 is the machine-wide declaration (press-local key).
        std::env::set_var("TEBAKO_SIGN", "1");
        assert_eq!(
            effective_sign(&opts, None).unwrap(),
            tebako_pkg::SignRequest::PressLocal
        );

        // The CLI beats the env: --no-sign over TEBAKO_SIGN=1 is loud…
        opts.sign = Some(tpkg::settings::SignCli::NoSign);
        assert_eq!(
            effective_sign(&opts, Some(&compose_signed)).unwrap(),
            tebako_pkg::SignRequest::None
        );
        let j = std::fs::read_to_string(&journal).unwrap();
        assert!(
            j.contains("event=press-sign-opt-out by=cli overridden=env"),
            "{j}"
        );
        // …and bare --sign over TEBAKO_SIGN=0 signs.
        std::env::set_var("TEBAKO_SIGN", "0");
        opts.sign = Some(tpkg::settings::SignCli::PressLocal);
        assert_eq!(
            effective_sign(&opts, None).unwrap(),
            tebako_pkg::SignRequest::PressLocal
        );
        // An opt-out over silence stays quiet (no new journal lines).
        let before = std::fs::read_to_string(&journal).unwrap();
        opts.sign = Some(tpkg::settings::SignCli::NoSign);
        std::env::remove_var("TEBAKO_SIGN");
        effective_sign(&opts, None).unwrap();
        assert_eq!(std::fs::read_to_string(&journal).unwrap(), before);

        // A malformed env value is a named error, never a silent default.
        std::env::set_var("TEBAKO_SIGN", "maybe");
        opts.sign = None;
        let err = effective_sign(&opts, None).unwrap_err();
        assert_eq!(err.code, 65);
        assert!(err.message.contains("TEBAKO_SIGN"), "{}", err.message);
        std::env::remove_var("TEBAKO_SIGN");

        // --sign=<keyid> naming no key in $TEBAKO_HOME/keys is a NAMED
        // error (71), raised here — before any heavy work — never a
        // fallback to the press-local key.
        opts.sign = Some(tpkg::settings::SignCli::Keyid("0000000000000000".into()));
        let err = effective_sign(&opts, None).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(
            err.message
                .contains("no secret key with keyid 0000000000000000"),
            "{}",
            err.message
        );
        opts.sign = Some(tpkg::settings::SignCli::Keyid("zz".into()));
        let err = effective_sign(&opts, None).unwrap_err();
        assert_eq!(err.code, 71);
        assert!(err.message.contains("invalid keyid"), "{}", err.message);

        // A keyid naming a real key resolves to it.
        let press = tebako_signer::press_local_key(&home).unwrap();
        opts.sign = Some(tpkg::settings::SignCli::Keyid(press.keyid_hex()));
        assert_eq!(
            effective_sign(&opts, None).unwrap(),
            tebako_pkg::SignRequest::Keyid(press.keyid_hex())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stitch_unsigned_is_byte_identical_and_touches_no_key_material() {
        // spec 09 §9: a press with no sign declaration on any channel
        // produces the v1-unsigned trailer — byte-identical to the
        // pre-signing shape — and never touches key material.
        let _guard = SIGN_ENV_LOCK.lock().unwrap();
        let dir = sign_scratch("parity");
        let home = dir.join("home");
        let _env = SignEnv::set(&home, &dir.join("journal.log"));
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let pre_change = dir.join("pre-change");
        stitch(
            &base,
            &[(img.clone(), "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            pre_change.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                ..Default::default()
            },
        )
        .unwrap();
        // The resolved-unsigned request (what press now computes with no
        // channel declaring sign)…
        let resolved = dir.join("resolved");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            resolved.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                sign: tebako_pkg::SignRequest::None,
                ..Default::default()
            },
        )
        .unwrap();
        // …is byte-identical to the pre-change shape…
        assert_eq!(
            std::fs::read(&pre_change).unwrap(),
            std::fs::read(&resolved).unwrap(),
            "resolved-unsigned must equal the pre-signing output"
        );
        // …a plain v1 trailer (no SIGNED_V2, no v2 extension)…
        let m = tpkg::read_from(&mut std::fs::File::open(&resolved).unwrap()).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_SIGNED_V2, 0);
        assert!(m.v2.is_none());
        // …and no key material was generated, loaded, or prompted for.
        assert!(!home.join("keys").exists(), "no keys dir may be created");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stitch_signed_round_trips_through_the_verifier() {
        // spec 09 §9: a signed press carries TPKG_FLAG_SIGNED_V2 + the v2
        // chain-of-trust extension; the bootstrap's verifier path accepts
        // it and the signature cryptographically verifies against the
        // auto-registered trusted keyring.
        use std::io::{Read as _, Seek as _, SeekFrom};

        let _guard = SIGN_ENV_LOCK.lock().unwrap();
        let dir = sign_scratch("roundtrip");
        let home = dir.join("home");
        let _env = SignEnv::set(&home, &dir.join("journal.log"));
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let out = dir.join("signed");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            out.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                sign: tebako_pkg::SignRequest::PressLocal,
                ..Default::default()
            },
        )
        .unwrap();
        // First explicit use generates + caches the press-local key.
        assert!(home.join("keys/press-local.key").exists());

        let m = tpkg::read_from(&mut std::fs::File::open(&out).unwrap()).unwrap();
        assert!(m.package_flags & tpkg::TPKG_FLAG_SIGNED_V2 != 0);
        let v2 =
            m.v2.as_ref()
                .expect("the v2 extension rides a signed press");

        // The bootstrap's chain-of-trust verifier (the run-time path).
        tebako_bootstrap::verify_chain_with_home(&out, &m, &home).expect("chain verifies");

        // The signature itself verifies against the trusted keyring the
        // press auto-registered (the strict, cryptographic check).
        let mut f = std::fs::File::open(&out).unwrap();
        let tlen = tpkg::trailer_len(&m);
        f.seek(SeekFrom::End(-(tlen as i64))).unwrap();
        let mut trailer = vec![0u8; tlen as usize];
        f.read_exact(&mut trailer).unwrap();
        let region = tpkg::v2_signed_region(&trailer).unwrap();
        let keyring = tebako_signer::trusted_keyring_bytes(&home).unwrap();
        let outcome =
            tebako_signer::verify_detached(&keyring, &region, &v2.signature, &v2.signer_keyid)
                .unwrap();
        assert!(
            matches!(outcome, tebako_signer::VerifyOutcome::Trusted(_)),
            "{outcome:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn press_package_manifest_bakes_the_lock() {
        // spec 23 §4/§13: the lock rides the type-2 manifest — the
        // carried runtime pair's slots + pins and the app slice row.
        let hex = |c: char| c.to_string().repeat(64);
        let lock = tpkg::PackageLock {
            runtime: Some(tpkg::LockedRuntime {
                version: "3.3.7".to_string(),
                carry: true,
                exe: Some(tpkg::LockedArtifact {
                    slot: 1,
                    sha256: tpkg::DigestPin::One(hex('a')),
                    install_as: None,
                }),
                image: Some(tpkg::LockedArtifact {
                    slot: 2,
                    sha256: tpkg::DigestPin::One(hex('b')),
                    install_as: None,
                }),
                dll: None,
            }),
            slices: vec![
                tpkg::LockedSlice {
                    name: "hello".to_string(),
                    version: "0.0.0".to_string(),
                    carry: true,
                    slot: Some(0),
                    mount: Some("/__tfs__".to_string()),
                    sha256: tpkg::DigestPin::One(hex('c')),
                    source: None,
                },
                tpkg::LockedSlice {
                    name: "templates".to_string(),
                    version: "3.2".to_string(),
                    carry: false,
                    slot: None,
                    mount: None,
                    sha256: tpkg::DigestPin::One(hex('d')),
                    source: Some("tfs:github:acme/templates:3.2".to_string()),
                },
            ],
            spawned: vec![],
        };
        let m = press_package_manifest(
            "/tmp/out/hello",
            "ruby@3.3.7;tebako=0.15.9;image",
            "0.15.9",
            "/__tfs__",
            "/local/stub.rb",
            None,
            Some(lock.clone()),
            Default::default(),
        );
        m.validate().unwrap();
        assert_eq!(m.lock.as_ref(), Some(&lock));
        // the claimed slots: the carried runtime pair + the app slice
        let mut claimed = lock.claimed_slots();
        claimed.sort_unstable();
        assert_eq!(claimed, vec![0, 1, 2]);
        // the YAML round trip preserves the lock (the bootstrap reads it)
        let back = tpkg::PackageManifest::from_yaml(&m.to_yaml().unwrap()).unwrap();
        assert_eq!(back, m);
        assert_eq!(
            back.lock
                .as_ref()
                .unwrap()
                .runtime
                .as_ref()
                .unwrap()
                .image
                .as_ref()
                .unwrap()
                .slot,
            2
        );
    }

    #[test]
    fn stitch_clears_the_lean_flag_only_when_self_contained() {
        // spec 23 §13.2: a self-contained press resolves nothing at run
        // time — TPKG_FLAG_LEAN stays CLEAR; every other press sets it.
        let dir =
            std::env::temp_dir().join(format!("tebako-cli-stitch-lean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("bootstrap");
        std::fs::write(&base, b"BASE").unwrap();
        let img = dir.join("img.tfs");
        std::fs::write(&img, b"IMG").unwrap();

        let contained = dir.join("contained");
        stitch(
            &base,
            &[(img.clone(), "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            contained.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags::default(),
        )
        .unwrap();
        let mut f = std::fs::File::open(&contained).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_LEAN, 0);
        assert_eq!(m.package_flags & tpkg::TPKG_FLAG_NO_INSTALL, 0);

        let shared = dir.join("shared");
        stitch(
            &base,
            &[(img, "/".to_string(), tpkg::TPKG_FORMAT_DWARFS)],
            shared.to_str().unwrap(),
            "ruby@3.3.7;tebako=9.9.9",
            None,
            StitchFlags {
                lean: true,
                ..Default::default()
            },
        )
        .unwrap();
        let mut f = std::fs::File::open(&shared).unwrap();
        let m = tpkg::read_from(&mut f).unwrap();
        assert!(m.package_flags & tpkg::TPKG_FLAG_LEAN != 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_suite_press_refuses_emit_recipe() {
        // The graduation path renders ONE project's recipe; a suite
        // carries its own document. The refusal lands before the suite
        // file is read, so a stray flag cannot cost a store visit.
        let mut opts = press_opts(None);
        opts.suite = Some(PathBuf::from("/nonexistent/suite.yaml"));
        opts.emit_recipe = Some(PathBuf::new());
        let err = press(&opts).expect_err("suite + emit-recipe refuses");
        let text = err.to_string();
        assert!(text.contains("--emit-recipe"), "{text}");
        assert!(text.contains("suite"), "{text}");
    }

    #[test]
    fn the_named_embed_dial_requires_a_compose_document() {
        // tebako#460 (spec 19 §6a): the named subset selects compose
        // slices; a plain script press (no compose doc) with names is
        // the NAMED error pointing at the valid spellings — never a
        // silent no-op.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rb"), b"puts 'hi'\n").unwrap();
        let mut opts = press_opts(None);
        opts.root_arg = dir.path().to_string_lossy().into_owned();
        opts.entrance = "main.rb".to_string();
        opts.embed = Some(EmbedSet::Names(vec!["openjdk".to_string()]));
        let err = press(&opts).expect_err("named embed without compose refuses");
        let text = err.to_string();
        assert!(text.contains("--embed name,…"), "{text}");
        assert!(text.contains("--embed=all|none"), "{text}");
    }

    fn press_opts(bootstrap: Option<PathBuf>) -> PressOptions {
        PressOptions {
            root_arg: String::new(),
            entrance: String::new(),
            output: None,
            prefix: PathBuf::from("/tmp/prefix"),
            cwd: None,
            ruby_requested: None,
            engine: Default::default(),
            engine_requirement: None,
            mode: PressMode::Lean,
            embed: None,
            log_level: "error".to_string(),
            image_specs: Vec::new(),
            bootstrap,
            tebako_version: DEFAULT_TEBAKO_VERSION.to_string(),
            tebako_version_explicit: true,
            prefer_local: false,
            verbose: false,
            devmode: true,
            fs_current: "/tmp".to_string(),
            suite: None,
            jail: None,
            no_install: false,
            quiet_notices: None,
            sign: None,
            format: options::PressImageFormat::Dwarfs,
            exclude: Vec::new(),
            compose: None,
            carry: None,
            share: None,
            mode_explicit: false,
            emit_recipe: None,
        }
    }

    /// A store resolver that never leaves the fixture directory: offline,
    /// empty cache root, unreachable mirror — the unit suite never touches
    /// the network or the real ~/.tebako.
    fn offline_store(dir: &Path) -> resolve::BootstrapResolver {
        resolve::BootstrapResolver {
            cache_root: dir.to_path_buf(),
            mirror: "file:///nonexistent-tebako-mirror".to_string(),
            version: "0.0.0".to_string(),
            offline: true,
            lock_timeout: std::time::Duration::from_secs(300),
        }
    }

    #[test]
    fn press_bootstrap_prefers_the_option_and_rejects_a_missing_file() {
        let dir = std::env::temp_dir().join(format!("tebako-cli-boot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = offline_store(&dir);
        let boot = dir.join("tebako-bootstrap");
        std::fs::write(&boot, b"BOOT").unwrap();
        // --bootstrap short-circuits every other source.
        let opts = press_opts(Some(boot.clone()));
        assert_eq!(
            press_bootstrap_with(&opts, "macos-arm64", &store).unwrap(),
            boot
        );
        // A named file that does not exist keeps the gem's parity error
        // (127, "runtime not found").
        let missing = dir.join("nope");
        let opts = press_opts(Some(missing.clone()));
        let err = press_bootstrap_with(&opts, "macos-arm64", &store).unwrap_err();
        assert_eq!(err.code, 127);
        assert!(
            err.message
                .contains(&format!("runtime not found: {}", missing.display())),
            "{}",
            err.message
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn press_bootstrap_without_a_local_source_resolves_the_store() {
        // The only test that mutates the process env (TEBAKO_BOOTSTRAP) —
        // kept in one function so parallel tests never observe a
        // half-saved environment; restored on the way out.
        let saved = std::env::var("TEBAKO_BOOTSTRAP").ok();

        // $TEBAKO_BOOTSTRAP beats the sibling probe.
        std::env::set_var("TEBAKO_BOOTSTRAP", "/nonexistent/env-bootstrap");
        let opts = press_opts(None);
        assert_eq!(
            decide_bootstrap(&opts),
            Some(PathBuf::from("/nonexistent/env-bootstrap"))
        );

        // No option, no env: the spec 19 §4 store flow fires — with an
        // offline empty store that is its named error 138 (never the
        // retired v1 C++ download, never a silent fetch). (The unit-test
        // binary runs from target/debug/deps/, which never holds a plain
        // `tebako-bootstrap` sibling — assert that precondition so a
        // change in cargo's layout fails diagnosably.)
        std::env::remove_var("TEBAKO_BOOTSTRAP");
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let sibling = exe_dir.join(if cfg!(windows) {
            "tebako-bootstrap.exe"
        } else {
            "tebako-bootstrap"
        });
        assert!(
            !sibling.is_file(),
            "test precondition: no tebako-bootstrap sibling next to the test binary ({})",
            sibling.display()
        );
        let opts = press_opts(None);
        assert!(decide_bootstrap(&opts).is_none());
        let dir =
            std::env::temp_dir().join(format!("tebako-cli-boot-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let err = press_bootstrap_with(&opts, "macos-arm64", &offline_store(&dir)).unwrap_err();
        assert_eq!(err.code, 138);
        assert!(err.message.contains("not cached"), "{}", err.message);
        let _ = std::fs::remove_dir_all(&dir);

        if let Some(v) = saved {
            std::env::set_var("TEBAKO_BOOTSTRAP", v);
        }
    }
}

#[test]
fn a_native_executable_entry_is_refused_by_name() {
    let dir = std::env::temp_dir().join(format!("tebako-native-entry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(dir.join(name), bytes).unwrap();
    };

    // A script entry — what a payload actually runs — passes.
    write("app.rb", b"#!/usr/bin/env ruby\nputs 1\n");
    assert!(refuse_native_entry(&dir, "app.rb").is_ok());

    // Every native image format the three shipped platforms produce is
    // refused, and the message names the file and the format.
    for (name, magic, format) in [
        ("elf", &[0x7f, b'E', b'L', b'F', 0x02, 0x01][..], "ELF"),
        ("macho64", &[0xcf, 0xfa, 0xed, 0xfe, 0x0c][..], "Mach-O"),
        ("macho32", &[0xce, 0xfa, 0xed, 0xfe, 0x07][..], "Mach-O"),
        ("pe", &[b'M', b'Z', 0x90, 0x00][..], "PE"),
        // The lipo/fat shape: every macOS system binary is one, so it is the
        // likeliest native entry a user presses.
        (
            "universal",
            &[0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x02][..],
            "Mach-O universal",
        ),
        (
            "universal64",
            &[0xca, 0xfe, 0xba, 0xbf, 0x00, 0x00, 0x00, 0x01][..],
            "Mach-O universal",
        ),
    ] {
        write(name, magic);
        let err = refuse_native_entry(&dir, name).unwrap_err();
        assert_eq!(err.code, 144, "{name}");
        assert!(err.message.contains(format), "{}: {}", name, err.message);
        assert!(err.message.contains(name), "{}: {}", name, err.message);
    }

    // A short file and a missing one are somebody else's error, not the
    // native-entry refusal (the press names those itself).
    write("short", b"ab");
    assert!(refuse_native_entry(&dir, "short").is_ok());
    assert!(refuse_native_entry(&dir, "absent.rb").is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

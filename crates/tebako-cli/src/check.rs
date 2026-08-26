//! `tebako check` — the spec 26 §2 check engine (phase T2): run a
//! payload's own in-image acceptance contract (`checks:`, spec 26 §1)
//! against the composition the shim dispatch would mount.
//!
//! Four target forms, classified from the argument (MECE):
//!
//! - `<name>` — an installed payload (managed mode): the dispatch
//!   resolution unchanged (`tebako-shim::resolve::resolve_payload` →
//!   `dispatch::compose_mounts` → per-entrypoint runtime resolution).
//!   The checks come from the EMBEDDED manifest (in-image is
//!   authoritative, spec 26 §0); the store mirror supplies deps/policy,
//!   exactly as dispatch reads it.
//! - `<image.tfs>` — a bare image (the feedstock's press-time gate):
//!   exec checks need `--runtime <exe>`; `--runtime-image <env.tfs>` is
//!   required for non-runtime images and defaults to the checked image
//!   itself for a runtime slice's `entry: self` checks (spec 26 §1.1).
//! - `<package>` — a pressed tpkg: per-slot embedded manifests; an exec
//!   check maps its `entry` (an in-image path) to a `PackageEntry` (the
//!   slot plus the entrypoint NAME, resolved to the path through the
//!   owning slot manifest's dispatchables) and execs the package with
//!   argv0 = the entry name (unix — the bootstrap's argv0 selection,
//!   spec 07 §2.0).
//! - `<doc>.yaml` — a spec 23 D2 composition document: the slices and
//!   runtime it declares, plus its own `checks:` block (spec 26 §2.1).
//!   Slice checks run before composition checks; the report groups by
//!   owner (`slice metanorma: html-xml PASS` / `composition: org-compile
//!   PASS`).
//!
//! Per exec check, in declaration order (the model's checks map sorts;
//! the engine walks the authored YAML mapping for the declared order):
//!
//! 1. `when:` platform filter — a non-matching host SKIPs (loud).
//! 2. `requires: {provides: [...]}` — an unmet capability SKIPs naming it
//!    (`no jvm in the composition`).
//! 3. A fresh host-tmp scratch dir, auto-granted `rw` (an engine grant,
//!    never a declared need); fixtures materialize to the scratch root.
//! 4. The run: the composition's policy ∪ the check's `needs:` (atoms
//!    resolved via tpkg::atoms — the SSOT), `{scratch}` argv substituted,
//!    stdout/stderr captured and teed. `argument_files: auto` resolves
//!    against the process cwd, which is meaningless for a check run —
//!    the engine resolves no argument files (the check's argv is the
//!    author's; the scratch grant covers it).
//! 5. Assertions (exit → files → stdout) and the verdict line:
//!    `check html-xml PASS 41s` / `check pdf SKIP (…)` /
//!    `check html-xml FAIL (expected file missing: test-iso.xml)`.
//!
//! Aggregate exit: 0 when every selected check PASSes or SKIPs;
//! `tpkg::EX_TEBAKO_CHECK` (79 — spec 26 §2/§7 allocated 72, but 72 has
//! been EX_TEBAKO_TRUST since spec 09; see the constant's doc) when any
//! FAIL. Timeouts and engine errors mid-check are FAILs with the reason
//! named; resolution/manifest errors abort with the existing named codes
//! (64 usage, 65 manifest, 69 unavailable, 127 target not found).
//! `--record` runs under the bare `record` policy (spec 23 §8 — it
//! dominates wholesale; grants are inert under allow-all).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tebako_shim::dispatch::MountSpec;
use tebako_shim::manifest::Dispatchable;
use tebako_shim::runtime::{self, RuntimeResolution};
use tebako_shim::{Ctx, ShimError};
use tpkg::{
    Check, CheckEntry, CheckPlatform, Constraint, HostJail, JailAccess, JailMount, PayloadManifest,
    RuntimeRequirement,
};

use crate::error::{packaging_error, plain_error, TebakoError};

/// The check-FAIL aggregate exit (spec 26 §2 — re-allocated from the
/// spec's 72, which collides with EX_TEBAKO_TRUST; tpkg owns the value).
pub const EX_CHECK: i32 = tpkg::EX_TEBAKO_CHECK;

const EX_USAGE: i32 = tebako_shim::EX_USAGE as i32;
const EX_MANIFEST: i32 = tebako_shim::EX_TEBAKO_MANIFEST as i32;
const EX_UNAVAILABLE: i32 = tebako_shim::EX_TEBAKO_UNAVAILABLE as i32;

/// The in-image manifest, backend-relative (tebako-info's constant's
/// spelling; tpkg::PAYLOAD_MANIFEST_PATH is the absolute form).
const MANIFEST_BACKEND_PATH: &str = "__tpkg__/manifest.yaml";
/// tebako-info's cap (1 MiB) — a payload manifest is small by contract.
const MANIFEST_MAX: u64 = 1 << 20;
/// The check-run poll interval.
const POLL: Duration = Duration::from_millis(50);

fn shim_err(e: ShimError) -> TebakoError {
    TebakoError::new(e.message, i32::from(e.code))
}

fn errno_text(e: i32) -> String {
    std::ffi::CStr::from_bytes_until_nul(tfs::errno::strerror(e))
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_else(|_| format!("errno {e}"))
}

// ---------------------------------------------------------------------
// argv
// ---------------------------------------------------------------------

/// The parsed `tebako check` argv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckArgs {
    /// `<name | image.tfs | package | composition.yaml>`.
    pub target: String,
    /// `--check <c>`: run only the check named `<c>`.
    pub check: Option<String>,
    /// `--list`: print the selected checks, run none.
    pub list: bool,
    /// `--record`: run under the record policy (spec 23 §8).
    pub record: bool,
    /// `--keep-scratch`: preserve each check's scratch dir (its path is
    /// printed) for debugging.
    pub keep_scratch: bool,
    /// `--runtime <exe>` (bare-image form).
    pub runtime: Option<PathBuf>,
    /// `--runtime-image <env.tfs>` (bare-image form).
    pub runtime_image: Option<PathBuf>,
}

/// Parse the `check` argv. Usage errors name the offending token.
pub fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut target: Option<String> = None;
    let mut check: Option<String> = None;
    let mut list = false;
    let mut record = false;
    let mut keep_scratch = false;
    let mut runtime: Option<PathBuf> = None;
    let mut runtime_image: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let take_value = |i: &mut usize| -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("option '{flag}' requires a value"))
        };
        match flag {
            "--check" => {
                let v = take_value(&mut i)?;
                if check.replace(v).is_some() {
                    return Err(
                        "option '--check' selects one check (repeat the command to run another)"
                            .to_string(),
                    );
                }
            }
            "--runtime" => runtime = Some(PathBuf::from(take_value(&mut i)?)),
            "--runtime-image" => runtime_image = Some(PathBuf::from(take_value(&mut i)?)),
            "--list" => list = true,
            "--record" => record = true,
            "--keep-scratch" => keep_scratch = true,
            _ if flag.starts_with('-') => {
                return Err(format!("unknown check option '{flag}'"));
            }
            _ => {
                if target.replace(flag.to_string()).is_some() {
                    return Err(format!(
                        "unexpected extra argument '{flag}' (one target per run)"
                    ));
                }
            }
        }
        i += 1;
    }
    if runtime.is_none() && runtime_image.is_some() {
        return Err(
            "--runtime-image needs --runtime (the pair names the gate's runtime)".to_string(),
        );
    }
    let target = target.ok_or_else(|| {
        "usage: tebako check <name | image.tfs | package | composition.yaml> [--check <c>] [--list] [--record] [--keep-scratch] [--runtime <exe> --runtime-image <env.tfs>]".to_string()
    })?;
    Ok(CheckArgs {
        target,
        check,
        list,
        record,
        keep_scratch,
        runtime,
        runtime_image,
    })
}

// ---------------------------------------------------------------------
// Target classification and loading
// ---------------------------------------------------------------------

/// An image a check reads (fixtures, structural assertions, the manifest):
/// a whole bare file, or a trailer-described slot region of a package.
#[derive(Debug, Clone)]
enum ImageRef {
    Whole(PathBuf),
    Region(PathBuf, u64, u64),
}

impl ImageRef {
    fn display(&self) -> String {
        match self {
            ImageRef::Whole(p) => p.display().to_string(),
            ImageRef::Region(p, o, s) => format!("{}[slot @{o}+{s}]", p.display()),
        }
    }

    /// Mount the image read-only (a Mount value, never the global table).
    fn mount(&self) -> Result<tfs::context::Mount, String> {
        let (path, region) = match self {
            ImageRef::Whole(p) => (p.to_string_lossy().into_owned(), None),
            ImageRef::Region(p, o, s) => (p.to_string_lossy().into_owned(), Some((*o, *s))),
        };
        let mounted = match region {
            None => tfs::mount::build_from_file(&path, "/mnt"),
            Some((o, s)) => tfs::mount::build_from_file_at(&path, o, s, "/mnt"),
        };
        mounted.map_err(|e| format!("cannot mount {}: {}", self.display(), errno_text(e)))
    }
}

/// One report owner: a slice (or the single payload of a bare target) or
/// the composition document itself. Checks are in DECLARATION order.
struct OwnerChecks {
    /// The report prefix: `None` renders the bare form (`check <name> …`),
    /// `Some("slice metanorma")` / `Some("composition")` the grouped form
    /// (spec 26 §2.1).
    label: Option<String>,
    checks: Vec<(String, Check)>,
    /// The owner's own image: the fixtures source and the structural
    /// assertion target. `None` for the composition owner (its checks
    /// assert the slice union).
    image: Option<ImageRef>,
    /// Structural assertions read these (one image for a slice owner; the
    /// slice union for the composition owner — spec 26 §2.1).
    structural_images: Vec<ImageRef>,
    /// The owning slot index (the package form's entry mapping).
    slot: Option<u32>,
    /// The owner slice's declared dispatchables (entrypoint name →
    /// in-image path) — the package form's entry table names PROVIDES
    /// entrypoints BY NAME; a check's `entry` is the path, resolved
    /// through this table.
    dispatchables: Vec<Dispatchable>,
    /// The composition document's directory — the base of the
    /// `fixtures_host:` family (spec 26 §2.1). `Some` on the composition
    /// owner only.
    doc_dir: Option<PathBuf>,
}

/// How an exec check's run is planned, per target form.
enum ExecCtx {
    /// Managed/store-backed (the name form): per-check runtime resolution
    /// from the owning dispatchable's `runtime_requirement`; the mounts
    /// are exactly the dispatch composition (composed lazily — a
    /// structural-only run never resolves deps, spec 26 §2). The
    /// resolution is boxed (it dwarfs the other variants).
    Store {
        res: Box<tebako_shim::resolve::Resolution>,
        mounts: Option<Vec<MountSpec>>,
    },
    /// The bare-image gate form: the runtime is GIVEN (`--runtime`); the
    /// env image is `--runtime-image` or, for a runtime slice, the
    /// checked image itself.
    Given {
        exe: Option<PathBuf>,
        env_image: Option<PathBuf>,
        checked: PathBuf,
        kind_is_runtime: bool,
    },
    /// A pressed package: exec the package itself; argv0 selects the
    /// entry (the bootstrap's contract, spec 07 §2.0). The jail the
    /// engine exports rides the USER-tightening channel — the bootstrap
    /// re-intersects it with the pressed request (tpkg::jail::effective),
    /// so under a pressed deny policy the engine's scratch grant is
    /// bounded by the package's own declaration (the intersection
    /// ceiling; `--record` dominates wholesale).
    Package {
        path: PathBuf,
        entries: Vec<tpkg::PackageEntry>,
        has_block: bool,
        base_jail: Option<HostJail>,
    },
    /// A D2 composition document: one runtime for the binding (the doc's
    /// `runtime:` block, else the entrypoint's requirement), every slice
    /// mounted, the doc's policy/mounts/needs the base jail. The
    /// resolution is boxed (it dwarfs the other variants).
    Composition {
        requirement: Option<RuntimeRequirement>,
        runtime: Option<Box<RuntimeResolution>>,
        mounts: Vec<MountSpec>,
        base_jail: Option<HostJail>,
    },
}

/// The loaded target: owners with their checks (declaration order per
/// owner, slice owners before the composition owner), the capability set
/// for `requires:` evaluation, and the exec context.
struct CheckTarget {
    owners: Vec<OwnerChecks>,
    caps: BTreeSet<String>,
    exec: ExecCtx,
}

impl CheckTarget {
    fn total(&self) -> usize {
        self.owners.iter().map(|o| o.checks.len()).sum()
    }

    /// Prepare the exec surface: compose the store mounts (and fold the
    /// dependency images' capabilities in) or resolve the composition's
    /// runtime. Called once, only when a selected check needs it — a
    /// structural-only run never resolves anything (spec 26 §2).
    fn prepare(&mut self, ctx: &Ctx, needs_runtime: bool) -> Result<(), TebakoError> {
        match &mut self.exec {
            ExecCtx::Store { res, mounts } => {
                if mounts.is_none() {
                    let composed =
                        tebako_shim::dispatch::compose_mounts(res, ctx).map_err(shim_err)?;
                    // The composition's capabilities: the payload's own
                    // (already in `caps`) plus every mounted dependency
                    // image's embedded manifest. An unreadable/absent dep
                    // manifest contributes nothing (caps-only read).
                    for m in composed.iter().skip(1) {
                        if let Ok(Some((dep, _))) = read_embedded(&ImageRef::Whole(m.image.clone()))
                        {
                            collect_caps(&dep, &mut self.caps);
                        }
                    }
                    *mounts = Some(composed);
                }
            }
            ExecCtx::Given {
                exe,
                env_image,
                checked,
                kind_is_runtime,
            } => {
                if !needs_runtime {
                    return Ok(());
                }
                if exe.is_none() {
                    return Err(TebakoError::new(
                        format!(
                            "exec checks on {} need --runtime <exe>{}",
                            checked.display(),
                            if *kind_is_runtime {
                                ""
                            } else {
                                " and --runtime-image <env.tfs>"
                            }
                        ),
                        EX_USAGE,
                    ));
                }
                let exe_path = exe.clone().unwrap_or_default();
                if !exe_path.is_file() {
                    return Err(packaging_error(
                        127,
                        Some(&format!("runtime not found: {}", exe_path.display())),
                    ));
                }
                if env_image.is_none() && *kind_is_runtime {
                    // spec 26 §1.1: a runtime slice's checks run against
                    // its own env image by default.
                    *env_image = Some(checked.clone());
                }
                if let Some(image) = env_image {
                    if !image.is_file() {
                        return Err(packaging_error(
                            127,
                            Some(&format!("runtime image not found: {}", image.display())),
                        ));
                    }
                }
            }
            ExecCtx::Package { .. } => {}
            ExecCtx::Composition {
                requirement,
                runtime,
                ..
            } => {
                if needs_runtime && runtime.is_none() {
                    let req = requirement.as_ref().ok_or_else(|| {
                        TebakoError::new(
                            "the composition declares no runtime (add a `runtime:` block) but an exec check needs one",
                            EX_MANIFEST,
                        )
                    })?;
                    let resolved =
                        runtime::resolve_runtime(Some(req), true, ctx).map_err(shim_err)?;
                    if matches!(resolved, RuntimeResolution::Zero) {
                        return Err(TebakoError::new(
                            "the composition's runtime resolved to zero runtimes — an exec check needs one",
                            EX_UNAVAILABLE,
                        ));
                    }
                    *runtime = Some(Box::new(resolved));
                }
            }
        }
        Ok(())
    }
}

enum TargetKind {
    Name(String),
    Image(PathBuf),
    Package(PathBuf),
    Composition(PathBuf),
}

/// Classify the target (MECE): an existing `.yaml`/`.yml` file is a
/// composition document; another existing file with a tpkg trailer is a
/// package; any other existing file is a bare image; anything else is a
/// payload name (managed resolution).
fn classify(target: &str) -> Result<TargetKind, TebakoError> {
    let path = Path::new(target);
    if path.is_file() {
        let lower = target.to_ascii_lowercase();
        if lower.ends_with(".yaml") || lower.ends_with(".yml") {
            return Ok(TargetKind::Composition(path.to_path_buf()));
        }
        if crate::install::is_tpkg_package(path) {
            return Ok(TargetKind::Package(path.to_path_buf()));
        }
        return Ok(TargetKind::Image(path.to_path_buf()));
    }
    if path.exists() {
        return Err(packaging_error(
            127,
            Some(&format!("check target is not a file: {target}")),
        ));
    }
    Ok(TargetKind::Name(target.to_string()))
}

/// The entry point: load the target, select, and run. The return is the
/// process exit code (0, or EX_CHECK when any selected check FAILs);
/// engine-level failures ride the named codes.
pub fn run(parsed: &CheckArgs) -> Result<i32, TebakoError> {
    let ctx = Ctx::from_env().map_err(shim_err)?;
    let mut target = match classify(&parsed.target)? {
        TargetKind::Name(name) => load_name(&name, &ctx)?,
        TargetKind::Image(path) => load_image(&path, parsed)?,
        TargetKind::Package(path) => load_package(&path)?,
        TargetKind::Composition(path) => load_composition(&path, &ctx)?,
    };
    if target.total() == 0 {
        eprintln!("tebako check: {} declares no checks", parsed.target);
        return Ok(0);
    }

    // Selection: every declared check, or the one --check names (matched
    // across owners; declaration order preserved).
    let mut selected: Vec<(usize, usize)> = Vec::new();
    for (oi, owner) in target.owners.iter().enumerate() {
        for (ci, (name, _)) in owner.checks.iter().enumerate() {
            if parsed.check.as_ref().map_or(true, |want| want == name) {
                selected.push((oi, ci));
            }
        }
    }
    if let Some(want) = &parsed.check {
        if selected.is_empty() {
            let mut names = Vec::new();
            for owner in &target.owners {
                for (name, _) in &owner.checks {
                    names.push(match &owner.label {
                        Some(label) => format!("{label}: {name}"),
                        None => name.clone(),
                    });
                }
            }
            return Err(TebakoError::new(
                format!(
                    "no check named {want:?} in {} (declared: {})",
                    parsed.target,
                    names.join(", ")
                ),
                EX_USAGE,
            ));
        }
    }

    if parsed.list {
        for (oi, ci) in &selected {
            let owner = &target.owners[*oi];
            let (name, check) = &owner.checks[*ci];
            let shape = if check.entry.is_some() {
                "exec"
            } else {
                "structural"
            };
            match &owner.label {
                Some(label) => println!("{label}: {name} ({shape})"),
                None => println!("check {name} ({shape})"),
            }
        }
        return Ok(0);
    }

    let needs_runtime = selected
        .iter()
        .any(|(oi, ci)| target.owners[*oi].checks[*ci].1.entry.is_some());
    let needs_caps = selected
        .iter()
        .any(|(oi, ci)| target.owners[*oi].checks[*ci].1.requires.is_some());
    if needs_runtime || needs_caps {
        target.prepare(&ctx, needs_runtime)?;
    }

    let mut any_fail = false;
    for (oi, ci) in selected {
        // `target` is borrowed per check; plan_exec needs no mutation
        // after prepare.
        let owner = &target.owners[oi];
        let (name, check) = &owner.checks[ci];
        let verdict = run_one(&target, owner, name, check, parsed, &ctx)?;
        if matches!(verdict, Verdict::Fail { .. }) {
            any_fail = true;
        }
        println!("{}", verdict_line(owner.label.as_deref(), name, &verdict));
    }
    Ok(if any_fail { EX_CHECK } else { 0 })
}

// ---------------------------------------------------------------------
// Per-form loaders
// ---------------------------------------------------------------------

/// The name form (managed mode): resolve the payload through the dispatch
/// chain, read the checks from the EMBEDDED manifest (in-image is
/// authoritative, spec 26 §0), and keep the resolution for the exec
/// plans. Deps/policy read exactly what dispatch reads (the mirror).
fn load_name(name: &str, ctx: &Ctx) -> Result<CheckTarget, TebakoError> {
    let res = tebako_shim::resolve::resolve_payload(name, ctx).map_err(shim_err)?;
    let image = ImageRef::Whole(res.record.image.clone());
    let embedded = read_embedded(&image)?;
    let mut caps = BTreeSet::new();
    let mut checks = Vec::new();
    let mut dispatchables = res.manifest.dispatchables();
    match &embedded {
        Some((manifest, text)) => {
            collect_caps(manifest, &mut caps);
            checks = declaration_order(text, &manifest.checks);
            dispatchables =
                tebako_shim::manifest::Manifest::from_payload_manifest(manifest.clone())
                    .dispatchables();
        }
        None => {
            // No embedded manifest (a pre-checks image): the mirror still
            // answers the capability question.
            collect_caps(res.manifest.payload_manifest(), &mut caps);
        }
    }
    let owners = if checks.is_empty() {
        Vec::new()
    } else {
        vec![OwnerChecks {
            label: None,
            checks,
            structural_images: vec![image.clone()],
            image: Some(image),
            slot: None,
            dispatchables,
            doc_dir: None,
        }]
    };
    Ok(CheckTarget {
        owners,
        caps,
        exec: ExecCtx::Store {
            res: Box::new(res),
            mounts: None,
        },
    })
}

/// The bare-image form (the press-time gate, spec 26 §3): the image's own
/// embedded manifest; exec checks run against the GIVEN runtime.
fn load_image(path: &Path, parsed: &CheckArgs) -> Result<CheckTarget, TebakoError> {
    let image = ImageRef::Whole(path.to_path_buf());
    let embedded = read_embedded(&image)?;
    let mut caps = BTreeSet::new();
    let mut checks = Vec::new();
    let mut dispatchables = Vec::new();
    let mut kind_is_runtime = false;
    if let Some((manifest, text)) = &embedded {
        kind_is_runtime = manifest.identity.kind == tpkg::PayloadKind::Runtime;
        collect_caps(manifest, &mut caps);
        checks = declaration_order(text, &manifest.checks);
        dispatchables = tebako_shim::manifest::Manifest::from_payload_manifest(manifest.clone())
            .dispatchables();
    }
    let owners = if checks.is_empty() {
        Vec::new()
    } else {
        vec![OwnerChecks {
            label: None,
            checks,
            structural_images: vec![image.clone()],
            image: Some(image),
            slot: None,
            dispatchables,
            doc_dir: None,
        }]
    };
    Ok(CheckTarget {
        owners,
        caps,
        exec: ExecCtx::Given {
            exe: parsed.runtime.clone(),
            env_image: parsed.runtime_image.clone(),
            checked: path.to_path_buf(),
            kind_is_runtime,
        },
    })
}

/// The package form: every non-runtime slot's embedded manifest (the
/// runtime slot is never mounted, spec 17 §1 — its checks belong to the
/// factory/gate, not the package surface). A multi-slice package groups
/// the report by slice; a single-slice package keeps the bare form.
/// The two-slot era (spec 19 §6.1 / spec 23 §13): the lock's runtime
/// artifacts (exe / env image / windows dll) are skipped by slot number
/// — the exe and dll are not images at all, and the env image's checks
/// belong to the factory like the format-4 slot's always did.
fn load_package(path: &Path) -> Result<CheckTarget, TebakoError> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| plain_error(format!("cannot open the package {}: {e}", path.display())))?;
    let m = tpkg::read_from(&mut f).map_err(|e| {
        plain_error(format!(
            "corrupt tebako manifest trailer in {} ({})",
            path.display(),
            tpkg::strerror(e.code())
        ))
    })?;
    let pm = m.package_manifest().map_err(|e| {
        plain_error(format!(
            "invalid package manifest (extension block type 2) in {}: {e}",
            path.display()
        ))
    })?;

    // The lock-claimed runtime artifact slots (the two-slot carried
    // runtime) — never payload surfaces.
    let runtime_slots: BTreeSet<u32> = pm
        .as_ref()
        .and_then(|pm| pm.lock.as_ref())
        .and_then(|lock| lock.runtime.as_ref())
        .map(|runtime| {
            [&runtime.exe, &runtime.image, &runtime.dll]
                .into_iter()
                .flatten()
                .map(|a| a.slot)
                .collect()
        })
        .unwrap_or_default();

    let mut owners = Vec::new();
    let mut caps = BTreeSet::new();
    for (index, slot) in m.slots.iter().enumerate() {
        if slot.format_id == tpkg::TPKG_FORMAT_RUNTIME || runtime_slots.contains(&(index as u32)) {
            continue;
        }
        let image = ImageRef::Region(path.to_path_buf(), slot.offset, slot.size);
        let Some((manifest, text)) = read_embedded(&image)? else {
            continue;
        };
        collect_caps(&manifest, &mut caps);
        let checks = declaration_order(&text, &manifest.checks);
        if checks.is_empty() {
            continue;
        }
        owners.push(OwnerChecks {
            label: Some(format!("slice {}", manifest.identity.name)),
            checks,
            structural_images: vec![image.clone()],
            image: Some(image),
            slot: Some(index as u32),
            dispatchables: tebako_shim::manifest::Manifest::from_payload_manifest(manifest.clone())
                .dispatchables(),
            doc_dir: None,
        });
    }
    // A single-owner package reads like any single-payload target.
    if owners.len() == 1 {
        owners[0].label = None;
    }
    let (entries, has_block, base_jail) = match pm {
        Some(pm) => (pm.entries, true, pm.jail),
        None => (Vec::new(), false, None),
    };
    Ok(CheckTarget {
        owners,
        caps,
        exec: ExecCtx::Package {
            path: path.to_path_buf(),
            entries,
            has_block,
            base_jail,
        },
    })
}

// ---------------------------------------------------------------------
// The composition document (spec 23 D2 + spec 26 §2.1)
// ---------------------------------------------------------------------

/// The D2 document as the check engine reads it. `deny_unknown_fields` is
/// the spec-03 discipline; `checks:` reuses the payload manifest's own
/// duplicate-name refusal and validation (the composition grammar is the
/// slice grammar with the MECE fixture families swapped — tpkg's
/// [`Check::validate_composition`]).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionDoc {
    /// The document version (only 1 exists).
    version: u32,
    #[serde(default)]
    runtime: Option<CompositionRuntime>,
    #[serde(default)]
    slices: Vec<CompositionSlice>,
    #[serde(default)]
    entrypoint: Option<String>,
    #[serde(default)]
    policy: Option<String>,
    #[serde(default)]
    mounts: Vec<CompositionMount>,
    #[serde(default)]
    needs: Option<CompositionNeeds>,
    #[serde(default, deserialize_with = "tpkg::checks_map::deserialize")]
    checks: BTreeMap<String, Check>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionRuntime {
    name: String,
    requirement: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionSlice {
    name: String,
    #[serde(default)]
    requirement: Option<String>,
    /// The mount point (the VFS namespace name). Required for every slice
    /// except the entrypoint provider (which defaults to `/`).
    #[serde(default)]
    mount: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionMount {
    host: String,
    /// Defaults to the host path itself (the identity mount).
    #[serde(default)]
    mount: Option<String>,
    access: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionNeeds {
    #[serde(default)]
    host: Vec<CompositionNeed>,
}

/// A composition-level need: the spec 23 §2 D1 grammar, lowered to a
/// host-mount grant (identity unless `mount` says otherwise).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompositionNeed {
    path: String,
    access: String,
    #[serde(default)]
    mount: Option<String>,
    #[serde(default)]
    when: Vec<CheckPlatform>,
    #[serde(default)]
    #[allow(dead_code)]
    why: Option<String>,
}

fn parse_access_word(word: &str, what: &str) -> Result<JailAccess, TebakoError> {
    match word {
        "ro" => Ok(JailAccess::Ro),
        "rw" => Ok(JailAccess::Rw),
        other => Err(TebakoError::new(
            format!("{what}: unknown access {other:?} (want ro|rw)"),
            EX_MANIFEST,
        )),
    }
}

/// Resolve a slice reference to the newest installed version satisfying
/// its constraint (spec 05's newest-compatible rule, the dispatch
/// dependency resolution's own message shape).
fn resolve_slice(ctx: &Ctx, name: &str, requirement: Option<&str>) -> Result<String, TebakoError> {
    let installed = tebako_shim::resolve::installed_versions(&ctx.home, name).map_err(shim_err)?;
    let version = match requirement {
        Some(req) => {
            let constraint = Constraint::new(req).map_err(|e| {
                TebakoError::new(
                    format!("composition slice {name:?}: invalid requirement {req:?} ({e})"),
                    EX_MANIFEST,
                )
            })?;
            let constraint = tebako_shim::versions::from_validated(&constraint);
            installed
                .iter()
                .filter(|v| constraint.matches(v))
                .max_by(|a, b| tebako_shim::versions::compare(a, b))
                .cloned()
        }
        None => installed
            .iter()
            .max_by(|a, b| tebako_shim::versions::compare(a, b))
            .cloned(),
    };
    version.ok_or_else(|| {
        TebakoError::new(
            format!(
                "composition slice {name:?}: no satisfying version is installed (installed: {})\n  install the slice with `tebako install {name}`",
                if installed.is_empty() {
                    "none".to_string()
                } else {
                    installed.join(", ")
                }
            ),
            EX_MANIFEST,
        )
    })
}

/// The engine's atom binding (spec 23 §2's "resolved at bind, per
/// invocation, per user"): `$HOME`/`%USERPROFILE%` and `$TMPDIR`/`%TEMP%`
/// from the environment, `$CWD` the invoking cwd, `$TEBAKO_HOME` the
/// store. The grammar (and the unknown-atom error) is tpkg's.
fn engine_atom(atom: &str, ctx: &Ctx) -> Option<String> {
    let env = |key: &str| ctx.env.get(key).filter(|v| !v.is_empty()).cloned();
    match atom {
        "HOME" | "USERPROFILE" => env("HOME").or_else(|| env("USERPROFILE")),
        "TMPDIR" | "TEMP" => env("TMPDIR")
            .or_else(|| env("TEMP"))
            .or_else(|| Some(std::env::temp_dir().to_string_lossy().into_owned())),
        "CWD" => Some(ctx.cwd.to_string_lossy().into_owned()),
        "TEBAKO_HOME" => Some(ctx.home.to_string_lossy().into_owned()),
        _ => None,
    }
}

fn expand_atom_path(path: &str, ctx: &Ctx) -> Result<String, TebakoError> {
    tpkg::atoms::expand_symbolic_atoms(path, &|atom| engine_atom(atom, ctx))
        .map_err(|e| TebakoError::new(e, EX_MANIFEST))
}

/// Load a D2 composition document: validate, resolve the slices against
/// the store, read each slice's embedded manifest (checks, capabilities,
/// the entrypoint provider), and compose the base policy from the doc's
/// `policy:`/`mounts:`/`needs:` (spec 23 §5 — declarations turn the jail
/// on, deny by default).
fn load_composition(path: &Path, ctx: &Ctx) -> Result<CheckTarget, TebakoError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        plain_error(format!(
            "cannot read the composition document {}: {e}",
            path.display()
        ))
    })?;
    let doc: CompositionDoc = serde_yml::from_str(&text).map_err(|e| {
        TebakoError::new(
            format!("invalid composition document {}: {e}", path.display()),
            EX_MANIFEST,
        )
    })?;
    let doc_err = |msg: String| TebakoError::new(format!("{}: {msg}", path.display()), EX_MANIFEST);
    if doc.version != 1 {
        return Err(doc_err(format!("version must be 1 (got {})", doc.version)));
    }
    // The checks block: the slice grammar with the composition fixture
    // families (the MECE rule is tpkg's validate_composition).
    for (name, check) in &doc.checks {
        tpkg::check_check_name(name).map_err(|e| doc_err(format!("checks.{name}: {e}")))?;
        check
            .validate_composition()
            .map_err(|e| doc_err(format!("checks.{name}: {e}")))?;
    }
    let mut seen_slices = BTreeSet::new();
    for slice in &doc.slices {
        if !seen_slices.insert(slice.name.clone()) {
            return Err(doc_err(format!(
                "duplicate slice {:?} — an authoring ambiguity, never a silent winner",
                slice.name
            )));
        }
    }
    let policy_open = match doc.policy.as_deref() {
        None => None,
        Some("open") => Some(true),
        Some("deny") => Some(false),
        Some(other) => {
            return Err(doc_err(format!(
                "unknown policy {other:?} (want open|deny — record is the engine's --record, never authored)"
            )));
        }
    };

    // Resolve the slices (newest installed satisfying the requirement)
    // and read their embedded manifests.
    let doc_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    struct ResolvedSlice {
        name: String,
        image: ImageRef,
        manifest: Option<(PayloadManifest, String)>,
        declared_mount: Option<String>,
    }
    let mut slices = Vec::new();
    for slice in &doc.slices {
        let version = resolve_slice(ctx, &slice.name, slice.requirement.as_deref())?;
        let record = tebako_shim::manifest::payload_record(&ctx.home, &slice.name, &version);
        let image = ImageRef::Whole(record.image);
        let manifest = read_embedded(&image)?;
        slices.push(ResolvedSlice {
            name: slice.name.clone(),
            image,
            manifest,
            declared_mount: slice.mount.clone(),
        });
    }

    // The entrypoint provider is the app slice: mounted first, at its
    // declared mount or `/`. Every other slice (and every slice of an
    // entrypoint-less document) needs a declared mount — a composition
    // that would collide at `/` is a named error, never a silent order.
    let provider = match &doc.entrypoint {
        Some(entry) => {
            let providing: Vec<usize> = slices
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.manifest.as_ref().is_some_and(|(m, _)| {
                        tebako_shim::manifest::Manifest::from_payload_manifest(m.clone())
                            .dispatchables()
                            .iter()
                            .any(|d| &d.name == entry)
                    })
                })
                .map(|(i, _)| i)
                .collect();
            match providing.len() {
                1 => Some(providing[0]),
                0 => {
                    return Err(doc_err(format!(
                        "entrypoint {entry:?} is not provided by any slice in the composition"
                    )));
                }
                _ => {
                    return Err(doc_err(format!(
                        "entrypoint {entry:?} is provided by {} slices — an authoring ambiguity",
                        providing.len()
                    )));
                }
            }
        }
        None => None,
    };
    let mut caps = BTreeSet::new();
    let mut mounts = Vec::new();
    let mut seen_mounts = BTreeSet::new();
    let mut owners = Vec::new();
    for (index, slice) in slices.iter().enumerate() {
        let mount = if Some(index) == provider {
            slice
                .declared_mount
                .clone()
                .unwrap_or_else(|| "/".to_string())
        } else {
            match &slice.declared_mount {
                Some(m) => m.clone(),
                None => {
                    return Err(doc_err(format!(
                        "slice {:?} needs a declared mount: (only the entrypoint slice defaults to /)",
                        slice.name
                    )));
                }
            }
        };
        if !seen_mounts.insert(mount.clone()) {
            return Err(doc_err(format!(
                "two slices mount at {mount:?} (EEXIST at boot — spec 17 §1)"
            )));
        }
        if let Some((manifest, text)) = &slice.manifest {
            collect_caps(manifest, &mut caps);
            let checks = declaration_order(text, &manifest.checks);
            if !checks.is_empty() {
                owners.push(OwnerChecks {
                    label: Some(format!("slice {}", slice.name)),
                    checks,
                    structural_images: vec![slice.image.clone()],
                    image: Some(slice.image.clone()),
                    slot: None,
                    dispatchables: tebako_shim::manifest::Manifest::from_payload_manifest(
                        manifest.clone(),
                    )
                    .dispatchables(),
                    doc_dir: None,
                });
            }
        }
        let image_path = match &slice.image {
            ImageRef::Whole(p) => p.clone(),
            ImageRef::Region(p, ..) => p.clone(),
        };
        mounts.push(MountSpec {
            image: image_path,
            slot: 0,
            mount,
        });
    }
    // The entrypoint provider mounts FIRST (the driver resolves the entry
    // against the first image mount, spec 17 §1).
    if let Some(provider) = provider {
        mounts.swap(0, provider);
    }

    // The runtime requirement: the doc's `runtime:` block, else the
    // entrypoint's own requirement (the dispatch rule, spec 23 §6).
    let requirement = match &doc.runtime {
        Some(rt) => Some(RuntimeRequirement {
            engine: rt.name.clone(),
            constraint: Constraint::new(&rt.requirement).map_err(|e| {
                doc_err(format!(
                    "runtime requirement {:?} is invalid ({e})",
                    rt.requirement
                ))
            })?,
            abi: None,
        }),
        None => provider.and_then(|p| {
            slices[p].manifest.as_ref().and_then(|(m, _)| {
                tebako_shim::manifest::Manifest::from_payload_manifest(m.clone())
                    .dispatchables()
                    .into_iter()
                    .find(|d| Some(&d.name) == doc.entrypoint.as_ref())
                    .and_then(|d| d.runtime_requirement)
            })
        }),
    };
    if let Some(req) = &requirement {
        caps.insert(req.engine.clone());
    }

    // The base policy: the doc's mounts + needs as grants; deny by
    // default once anything is declared (spec 23 §5).
    let mut grants = Vec::new();
    for m in &doc.mounts {
        let host = expand_atom_path(&m.host, ctx)?;
        let host = if Path::new(&host).is_absolute() {
            host
        } else {
            doc_dir.join(host).to_string_lossy().into_owned()
        };
        grants.push(JailMount {
            mount: m.mount.clone().unwrap_or_else(|| host.clone()),
            host,
            access: parse_access_word(&m.access, "composition mounts[]")?,
        });
    }
    for n in doc.needs.map(|n| n.host).unwrap_or_default() {
        if !in_force_on(&n.when, host_platform()) {
            continue; // a platform-filtered need is inert (spec 23 §2)
        }
        let host = expand_atom_path(&n.path, ctx)?;
        grants.push(JailMount {
            mount: n.mount.clone().unwrap_or_else(|| host.clone()),
            host,
            access: parse_access_word(&n.access, "composition needs[]")?,
        });
    }
    let base_jail = if policy_open.is_some() || !grants.is_empty() {
        Some(HostJail {
            default_open: policy_open.unwrap_or(false),
            record: false,
            mounts: grants,
            argument_files: tpkg::ArgumentFiles::default(),
        })
    } else {
        None
    };

    // The composition owner runs last (slice checks diagnose a broken
    // slice before the binding that depends on it, spec 26 §2.1).
    let doc_checks = declaration_order(&text, &doc.checks);
    if !doc_checks.is_empty() {
        owners.push(OwnerChecks {
            label: Some("composition".to_string()),
            checks: doc_checks,
            structural_images: slices.iter().map(|s| s.image.clone()).collect(),
            image: None,
            slot: None,
            dispatchables: Vec::new(),
            doc_dir: Some(doc_dir.clone()),
        });
    }

    Ok(CheckTarget {
        owners,
        caps,
        exec: ExecCtx::Composition {
            requirement,
            runtime: None,
            mounts,
            base_jail,
        },
    })
}

// ---------------------------------------------------------------------
// Manifest reading + declaration order + capabilities
// ---------------------------------------------------------------------

/// Read the embedded payload manifest from an image (a Mount value, never
/// the global table — tebako-info's pattern). `None` = the image carries
/// no manifest (a legal, if boring, image); a corrupt one is a named 65.
fn read_embedded(image: &ImageRef) -> Result<Option<(PayloadManifest, String)>, TebakoError> {
    let mount = image
        .mount()
        .map_err(|e| TebakoError::new(e, EX_MANIFEST))?;
    let backend: &dyn tfs::Backend = &*mount.backend;
    let st = match backend.stat(MANIFEST_BACKEND_PATH) {
        Ok(st) => st,
        Err(_) => return Ok(None), // ENOENT and friends: absent
    };
    if st.entry_type != tfs::EntryType::File {
        return Err(TebakoError::new(
            format!(
                "payload manifest {MANIFEST_BACKEND_PATH} in {} is not a regular file",
                image.display()
            ),
            EX_MANIFEST,
        ));
    }
    let size = u64::try_from(st.size)
        .map_err(|_| TebakoError::new("payload manifest has a negative size", EX_MANIFEST))?;
    if size > MANIFEST_MAX {
        return Err(TebakoError::new(
            format!("payload manifest exceeds {MANIFEST_MAX} bytes"),
            EX_MANIFEST,
        ));
    }
    let mut buf = vec![0u8; size as usize];
    let mut off = 0u64;
    while off < size {
        let n = backend
            .pread(MANIFEST_BACKEND_PATH, &mut buf[off as usize..], off)
            .map_err(|e| {
                TebakoError::new(
                    format!("cannot read the payload manifest (errno {e})"),
                    EX_MANIFEST,
                )
            })?;
        if n == 0 {
            return Err(TebakoError::new(
                "short read on the payload manifest",
                EX_MANIFEST,
            ));
        }
        off += n as u64;
    }
    let text = String::from_utf8(buf)
        .map_err(|_| TebakoError::new("payload manifest is not valid UTF-8", EX_MANIFEST))?;
    let manifest = PayloadManifest::from_yaml(&text).map_err(|e| {
        TebakoError::new(
            format!("corrupt payload manifest in {}: {e}", image.display()),
            EX_MANIFEST,
        )
    })?;
    Ok(Some((manifest, text)))
}

/// The checks of one manifest in DECLARATION order (spec 26 §2): the
/// validated model's map sorts by name, so the engine walks the authored
/// YAML mapping (serde_yml preserves document order) and looks each key
/// up in the model. Model entries the mapping did not yield (never
/// happens — both read the same document) append in map order.
fn declaration_order(text: &str, checks: &BTreeMap<String, Check>) -> Vec<(String, Check)> {
    let mut ordered: Vec<(String, Check)> = Vec::new();
    if let Ok(serde_yml::Value::Mapping(root)) = serde_yml::from_str::<serde_yml::Value>(text) {
        for (key, value) in &root {
            let is_checks = matches!(key, serde_yml::Value::String(s) if s == "checks");
            if !is_checks {
                continue;
            }
            if let serde_yml::Value::Mapping(map) = value {
                for (name, _) in map {
                    if let serde_yml::Value::String(name) = name {
                        if let Some(check) = checks.get(name) {
                            ordered.push((name.clone(), check.clone()));
                        }
                    }
                }
            }
        }
    }
    for (name, check) in checks {
        if !ordered.iter().any(|(seen, _)| seen == name) {
            ordered.push((name.clone(), check.clone()));
        }
    }
    ordered
}

/// The capability set a `requires:` evaluates against: over the
/// composition's slices, the union of the slice name, the app
/// entrypoints, the toolkit executables/libraries, and the runtime
/// engines (spec 26 §2 step 2 — the spec's `jvm` is the illustrative
/// case; an openjdk slice provides the `java` executable).
fn collect_caps(manifest: &PayloadManifest, caps: &mut BTreeSet<String>) {
    caps.insert(manifest.identity.name.clone());
    match &manifest.provides {
        tpkg::Provides::App(app) => {
            for e in &app.entrypoints {
                caps.insert(e.name.clone());
            }
        }
        tpkg::Provides::Toolkit(tk) => {
            for e in &tk.executables {
                caps.insert(e.name.clone());
            }
            for l in &tk.libraries {
                caps.insert(l.name.clone());
            }
        }
        tpkg::Provides::Runtime(rt) => {
            for e in &rt.provides {
                caps.insert(e.engine.clone());
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------
// The per-check pipeline
// ---------------------------------------------------------------------

/// A check's verdict. FAIL carries the first failing assertion's reason
/// (or the engine failure's name); SKIP carries the unmet prerequisite.
enum Verdict {
    Pass { secs: u64 },
    Skip { reason: String },
    Fail { reason: String },
}

fn verdict_line(label: Option<&str>, name: &str, verdict: &Verdict) -> String {
    let head = match label {
        Some(label) => format!("{label}: {name}"),
        None => format!("check {name}"),
    };
    match verdict {
        Verdict::Pass { secs } => format!("{head} PASS {secs}s"),
        Verdict::Skip { reason } => format!("{head} SKIP ({reason})"),
        Verdict::Fail { reason } => format!("{head} FAIL ({reason})"),
    }
}

/// The host's OS family (the `when:` axis — spec 26 §1's three names,
/// matching std::env::consts::OS's spellings on the supported hosts).
fn host_platform() -> Option<CheckPlatform> {
    match std::env::consts::OS {
        "windows" => Some(CheckPlatform::Windows),
        "macos" => Some(CheckPlatform::Macos),
        "linux" => Some(CheckPlatform::Linux),
        _ => None,
    }
}

/// A `when:` filter holds: empty = every platform; else the host family
/// must be listed (an unrecognized host family matches nothing).
fn in_force_on(when: &[CheckPlatform], host: Option<CheckPlatform>) -> bool {
    when.is_empty() || host.is_some_and(|h| when.contains(&h))
}

fn platform_name(p: CheckPlatform) -> &'static str {
    match p {
        CheckPlatform::Windows => "windows",
        CheckPlatform::Macos => "macos",
        CheckPlatform::Linux => "linux",
    }
}

/// The mid-check failure channel: `Fail` is this check's verdict (spec 26
/// §2 — timeouts and engine errors mid-check are FAILs with the reason
/// named); `Abort` is a named engine error that ends the run (resolution,
/// manifest corruption — the existing named codes).
enum CheckStep {
    Fail(String),
    Abort(TebakoError),
}

impl From<TebakoError> for CheckStep {
    fn from(e: TebakoError) -> CheckStep {
        CheckStep::Abort(e)
    }
}

fn run_one(
    target: &CheckTarget,
    owner: &OwnerChecks,
    name: &str,
    check: &Check,
    parsed: &CheckArgs,
    ctx: &Ctx,
) -> Result<Verdict, TebakoError> {
    let started = Instant::now();
    // 1. The platform filter.
    if !in_force_on(&check.when, host_platform()) {
        return Ok(Verdict::Skip {
            reason: format!(
                "not for {} (when: {})",
                std::env::consts::OS,
                check
                    .when
                    .iter()
                    .map(|p| platform_name(*p))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    // 2. The composition prerequisites — unmet is a SKIP, never a FAIL.
    if let Some(requires) = &check.requires {
        for cap in &requires.provides {
            if !target.caps.contains(cap) {
                return Ok(Verdict::Skip {
                    reason: format!("no {cap} in the composition"),
                });
            }
        }
    }
    // The structural shape (no entry): mount the image(s) and assert —
    // no runtime, no scratch, no jail (spec 26 §1.1).
    if check.entry.is_none() {
        return Ok(
            match assert_image_files(&owner.structural_images, &check.expect.image_files) {
                None => Verdict::Pass {
                    secs: started.elapsed().as_secs(),
                },
                Some(reason) => Verdict::Fail { reason },
            },
        );
    }

    match run_exec(target, owner, name, check, parsed, ctx, started) {
        Ok(verdict) => Ok(verdict),
        Err(CheckStep::Fail(reason)) => Ok(Verdict::Fail { reason }),
        Err(CheckStep::Abort(e)) => Err(e),
    }
}

/// The exec path: scratch → fixtures → plan → jail → run → assertions.
/// Everything that is THIS check's failure returns Fail; only engine-level
/// named errors abort.
fn run_exec(
    target: &CheckTarget,
    owner: &OwnerChecks,
    name: &str,
    check: &Check,
    parsed: &CheckArgs,
    ctx: &Ctx,
    started: Instant,
) -> Result<Verdict, CheckStep> {
    // 3. A fresh scratch dir, auto-granted rw for the check's duration.
    let scratch = Scratch::new(name, parsed.keep_scratch).map_err(CheckStep::Fail)?;
    // Fixtures materialize into the scratch root (host-spelled, never
    // VFS — spec 26 §1).
    materialize_fixtures(owner, check, scratch.path()).map_err(CheckStep::Fail)?;
    // 4. The run.
    let mut plan = plan_exec(target, owner, name, check, scratch.path(), ctx)?;
    let jail_env = check_jail_env(target, check, scratch.path(), parsed.record, ctx)
        .map_err(CheckStep::Fail)?;
    plan.env.extend(jail_env);
    let outcome = run_child(&plan, scratch.path(), check.timeout).map_err(CheckStep::Fail)?;
    if outcome.timed_out {
        return Ok(Verdict::Fail {
            reason: format!("timeout after {}s", check.timeout.unwrap_or(0)),
        });
    }
    // 5. The assertions.
    Ok(match assert_run(check, &outcome, scratch.path()) {
        None => Verdict::Pass {
            secs: started.elapsed().as_secs(),
        },
        Some(reason) => Verdict::Fail { reason },
    })
}

/// The composed run: program, argv (WITHOUT the program), env on top of
/// the inherited environment, and the zero-runtime preload scrub.
struct RunPlan {
    program: PathBuf,
    /// unix argv[0] override (the package form's entry selector, spec 07
    /// §2.0); `None` = the program path itself.
    argv0: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    scrub_preload: bool,
}

/// Plan one exec check's run per target form. Resolution failures abort
/// (the engine's named codes); per-check mismatches FAIL with the reason
/// named.
fn plan_exec(
    target: &CheckTarget,
    owner: &OwnerChecks,
    name: &str,
    check: &Check,
    scratch: &Path,
    ctx: &Ctx,
) -> Result<RunPlan, CheckStep> {
    let Some(entry) = &check.entry else {
        return Err(CheckStep::Fail(
            "a structural check (no entry) never reaches the exec path".to_string(),
        ));
    };
    let user_args = substitute_argv(&check.argv, scratch);
    match &target.exec {
        ExecCtx::Store { res, mounts } => {
            let entry_path = match entry {
                CheckEntry::SelfExe => {
                    return Err(CheckStep::Fail(
                        "entry: self checks belong to runtime slices — check the bare-image form with --runtime"
                            .to_string(),
                    ));
                }
                CheckEntry::Path(p) => p,
            };
            let dispatchables = res.manifest.dispatchables();
            let d = match dispatchables.iter().find(|d| &d.path == entry_path) {
                Some(d) => d,
                None if dispatchables.len() == 1 => &dispatchables[0],
                None => {
                    return Err(CheckStep::Abort(TebakoError::new(
                        format!(
                            "check {name:?} entry {entry_path} matches no declared entrypoint of \"{}\" (declared: {})",
                            res.payload_name,
                            dispatchables
                                .iter()
                                .map(|d| format!("{}={}", d.name, d.path))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        EX_MANIFEST,
                    )));
                }
            };
            plan_store(
                res,
                mounts.as_deref().unwrap_or(&[]),
                d,
                entry_path,
                user_args,
                ctx,
            )
        }
        ExecCtx::Given {
            exe,
            env_image,
            checked,
            kind_is_runtime,
        } => {
            // prepare() guaranteed the pair for exec checks.
            let exe = exe.clone().unwrap_or_default();
            let mut args = Vec::new();
            let mut env = Vec::new();
            if let Some(image) = env_image {
                env.push((
                    "TEBAKO_RUNTIME_IMAGE".to_string(),
                    image.to_string_lossy().into_owned(),
                ));
            }
            match entry {
                // spec 17 §1: a bare NAME is the interpreter keyword —
                // the boot starts the interpreter with the user's args.
                CheckEntry::SelfExe => {
                    args.push("--tebako-entry".to_string());
                    args.push("self".to_string());
                }
                CheckEntry::Path(p) => {
                    if !kind_is_runtime {
                        args.push("--tebako-image".to_string());
                        args.push(format!("{}:0:/", checked.display()));
                    }
                    // A runtime slice's path entry resolves against the
                    // runtime root (no image specs, spec 17 §1).
                    args.push("--tebako-entry".to_string());
                    args.push(p.clone());
                }
            }
            args.extend(user_args);
            Ok(RunPlan {
                program: exe,
                argv0: None,
                args,
                env,
                scrub_preload: false,
            })
        }
        ExecCtx::Package {
            path,
            entries,
            has_block,
            ..
        } => {
            if !has_block {
                return Err(CheckStep::Fail(
                    "the package carries no type-2 manifest — exec checks need its entry table"
                        .to_string(),
                ));
            }
            let entry_path = match entry {
                CheckEntry::SelfExe => {
                    return Err(CheckStep::Fail(
                        "entry: self belongs to runtime slices — a package's runtime slot is never checked"
                            .to_string(),
                    ));
                }
                CheckEntry::Path(p) => p,
            };
            // The entry table names PROVIDES entrypoints BY NAME; the
            // check's entry is the in-image path — resolve the name
            // through the owning slice's own manifest (the name's one
            // authority), then compare paths.
            let package_entry = entries.iter().find(|e| {
                e.slot == owner.slot
                    && owner
                        .dispatchables
                        .iter()
                        .any(|d| d.name == e.entrypoint && &d.path == entry_path)
            });
            let Some(package_entry) = package_entry else {
                return Err(CheckStep::Fail(format!(
                    "check entry {entry_path} matches no declared package entry for slot {} (declared: {})",
                    owner.slot.unwrap_or(0),
                    entries
                        .iter()
                        .map(|e| {
                            let slot = match e.slot {
                                Some(s) => s.to_string(),
                                None => "shared".to_string(),
                            };
                            format!("{}→slot {}:{}", e.name, slot, e.entrypoint)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            };
            // argv0 selects the entry (the bootstrap's contract). unix
            // only — Command has no cross-platform argv0; elsewhere only
            // the primary entry runs (the basename fallback selects it).
            #[cfg(unix)]
            let argv0 = Some(package_entry.name.clone());
            #[cfg(not(unix))]
            let argv0 = {
                if package_entry.name != entries[0].name {
                    return Err(CheckStep::Fail(format!(
                        "entry {:?} needs argv0 selection (a unix exec surface); only the primary entry {:?} runs on this platform",
                        package_entry.name, entries[0].name
                    )));
                }
                None
            };
            Ok(RunPlan {
                program: path.clone(),
                argv0,
                args: user_args,
                env: Vec::new(),
                scrub_preload: false,
            })
        }
        ExecCtx::Composition {
            runtime, mounts, ..
        } => {
            let entry_path = match entry {
                CheckEntry::SelfExe => {
                    return Err(CheckStep::Fail(
                        "entry: self is reserved for runtime slices — a composition check names a mounted slice's executable"
                            .to_string(),
                    ));
                }
                CheckEntry::Path(p) => p,
            };
            let rt = match runtime.as_deref() {
                Some(RuntimeResolution::Ready(rt)) => rt,
                _ => {
                    return Err(CheckStep::Abort(TebakoError::new(
                        "the composition's runtime is unresolved (prepare did not run)",
                        EX_UNAVAILABLE,
                    )));
                }
            };
            let mut args = Vec::new();
            let mut env = Vec::new();
            for m in mounts {
                args.push("--tebako-image".to_string());
                args.push(m.triple());
            }
            args.push("--tebako-entry".to_string());
            args.push(entry_path.clone());
            if let Some(image) = &rt.image {
                env.push((
                    "TEBAKO_RUNTIME_IMAGE".to_string(),
                    image.to_string_lossy().into_owned(),
                ));
            }
            args.extend(user_args);
            Ok(RunPlan {
                program: rt.exe.clone(),
                argv0: None,
                args,
                env,
                scrub_preload: false,
            })
        }
    }
}

/// The store form's plan: resolve the runtime from the owning
/// dispatchable's requirement (the dispatch rule), then either the
/// driver-contract handoff or the zero-runtime materialized exec.
fn plan_store(
    res: &tebako_shim::resolve::Resolution,
    mounts: &[MountSpec],
    d: &Dispatchable,
    entry_path: &str,
    user_args: Vec<String>,
    ctx: &Ctx,
) -> Result<RunPlan, CheckStep> {
    match runtime::resolve_runtime(d.runtime_requirement.as_ref(), true, ctx)
        .map_err(shim_err)
        .map_err(CheckStep::Abort)?
    {
        RuntimeResolution::Zero => {
            // Zero-runtime: the install-time materialization is the
            // program (the dispatch rule — a run never materializes; the
            // preload shim's env is scrubbed so it cannot intercept the
            // child's own IO).
            let host = res.record.tree.join(entry_path.trim_start_matches('/'));
            if !host.is_file() {
                return Err(CheckStep::Abort(TebakoError::new(
                    format!(
                        "zero-runtime entrypoint {} of \"{}\" {} is not materialized at {}\n  materialize it with `tebako install {}`",
                        entry_path,
                        res.payload_name,
                        res.version,
                        host.display(),
                        res.payload_name,
                    ),
                    EX_UNAVAILABLE,
                )));
            }
            Ok(RunPlan {
                program: host,
                argv0: None,
                args: user_args,
                env: Vec::new(),
                scrub_preload: true,
            })
        }
        RuntimeResolution::Ready(rt) => {
            let mut args = Vec::new();
            let mut env = Vec::new();
            for m in mounts {
                args.push("--tebako-image".to_string());
                args.push(m.triple());
            }
            args.push("--tebako-entry".to_string());
            args.push(entry_path.to_string());
            if let Some(image) = &rt.image {
                env.push((
                    "TEBAKO_RUNTIME_IMAGE".to_string(),
                    image.to_string_lossy().into_owned(),
                ));
            }
            args.extend(user_args);
            Ok(RunPlan {
                program: rt.exe,
                argv0: None,
                args,
                env,
                scrub_preload: false,
            })
        }
    }
}

// ---------------------------------------------------------------------
// Jail composition (spec 26 §2 steps 3-4)
// ---------------------------------------------------------------------

/// The check run's jail env: the composition's effective policy ∪ the
/// engine's scratch grant ∪ the check's in-force `needs:` (identity
/// mounts, atoms resolved via tpkg::atoms). With NOTHING declared and no
/// needs, nothing is exported (spec 23 §5's open-with-no-policy rule —
/// the scratch grant is inert under an open world). Under `--record` the
/// bare `record` token dominates wholesale (grants are inert under
/// allow-all; the journal captures the run).
///
/// Errors are this check's FAIL reasons, never run-aborting: a need that
/// does not resolve or bind is the check's own declaration failing, and
/// sibling checks still run.
fn check_jail_env(
    target: &CheckTarget,
    check: &Check,
    scratch: &Path,
    record: bool,
    ctx: &Ctx,
) -> Result<Vec<(String, String)>, String> {
    let journal = ctx.home.join("journal.log").to_string_lossy().into_owned();
    if record {
        return Ok(vec![
            ("TEBAKO_JAIL".to_string(), "record".to_string()),
            ("TEBAKO_JAIL_SOURCE".to_string(), "user".to_string()),
            ("TEBAKO_JAIL_JOURNAL".to_string(), journal),
        ]);
    }
    let base = match &target.exec {
        ExecCtx::Store { res, .. } => res.manifest.host_jail().cloned(),
        ExecCtx::Given { .. } => None,
        ExecCtx::Package { base_jail, .. } => base_jail.clone(),
        ExecCtx::Composition { base_jail, .. } => base_jail.clone(),
    };
    let scratch = scratch.to_string_lossy().into_owned();
    let mut grants = vec![JailMount {
        host: scratch.clone(),
        mount: scratch,
        access: JailAccess::Rw,
    }];
    for need in &check.needs {
        if !in_force_on(&need.when, host_platform()) {
            continue;
        }
        let host = tpkg::atoms::expand_symbolic_atoms(&need.path, &|atom| engine_atom(atom, ctx))
            .map_err(|e| format!("cannot resolve need {:?}: {e}", need.path))?;
        grants.push(JailMount {
            mount: host.clone(),
            host,
            access: need.access,
        });
    }
    let needs_in_force = check
        .needs
        .iter()
        .any(|n| in_force_on(&n.when, host_platform()));
    let jail = match base {
        // Nothing declared, no needs: the open world needs no export
        // (the scratch grant exists to OPEN a deny world; under open it
        // is inert).
        None if !needs_in_force => return Ok(Vec::new()),
        None => HostJail {
            default_open: true,
            record: false,
            mounts: grants,
            argument_files: tpkg::ArgumentFiles::default(),
        },
        Some(mut base) => {
            base.mounts.extend(grants);
            base
        }
    };
    if jail.is_trivially_open() {
        return Ok(Vec::new());
    }
    // `argument_files` resolves against the process cwd, which is not the
    // check's scratch — the engine resolves no argument files (the
    // check's argv is the author's; the scratch grant covers it).
    let spec = jail.to_env_spec(&[]);
    bind_check(&spec)?;
    // The package form's export rides the bootstrap's user-tightening
    // channel (it re-intersects with the pressed request); every other
    // form's export IS the driver's policy, labeled by its surface.
    let source = if matches!(target.exec, ExecCtx::Package { .. }) {
        "user"
    } else {
        "check"
    };
    Ok(vec![
        ("TEBAKO_JAIL".to_string(), spec),
        ("TEBAKO_JAIL_SOURCE".to_string(), source.to_string()),
        ("TEBAKO_JAIL_JOURNAL".to_string(), journal),
    ])
}

/// Bind-check the composed env spec (grant paths must exist at check
/// time — run.rs's fail-early contract, degraded to the check's FAIL
/// reason instead of a dispatch abort).
#[cfg(unix)]
fn bind_check(spec: &str) -> Result<(), String> {
    let parsed =
        tfs::policy::JailSpec::parse(spec).map_err(|e| format!("invalid jail spec: {e}"))?;
    tfs::policy::HostPolicy::bind(parsed.default, parsed.mounts, parsed.arg_files)
        .map_err(|e| format!("cannot bind the check policy: {}", errno_text(e)))?;
    Ok(())
}

/// Non-unix platforms skip the eager bind (the driver validates at
/// install; the preload/exec path is unix-first, spec 07 §8).
#[cfg(not(unix))]
fn bind_check(_spec: &str) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------
// Scratch + fixtures
// ---------------------------------------------------------------------

/// The per-check scratch directory (spec 26 §2 step 3): fresh under the
/// host tmp, removed at the check's end unless --keep-scratch (then its
/// path is printed).
struct Scratch {
    dir: PathBuf,
    keep: bool,
}

impl Scratch {
    fn new(check_name: &str, keep: bool) -> Result<Scratch, String> {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tebako-check-{check_name}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create the scratch dir {}: {e}", dir.display()))?;
        Ok(Scratch { dir, keep })
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.keep {
            println!("scratch kept: {}", self.dir.display());
        } else {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// The `{scratch}` argv substitution (the ONE token, spec 26 §1 — the
/// model validated at most one occurrence per entry).
fn substitute_argv(argv: &[String], scratch: &Path) -> Vec<String> {
    let scratch = scratch.to_string_lossy();
    argv.iter()
        .map(|a| a.replace("{scratch}", scratch.as_ref()))
        .collect()
}

/// Materialize a check's fixtures into the scratch root: the slice's
/// in-image `fixtures:` dir CONTENTS (host-spelled, never VFS — the
/// consumer may be a raw-surface component, spec 26 §1), the composition
/// family's `fixtures_inline:` files, or the `fixtures_host:` tree
/// relative to the composition file.
fn materialize_fixtures(owner: &OwnerChecks, check: &Check, scratch: &Path) -> Result<(), String> {
    if let Some(fixtures) = &check.fixtures {
        let image = owner.image.as_ref().ok_or_else(|| {
            format!("fixtures {fixtures} need the owning slice's image (a composition check speaks fixtures_inline/fixtures_host)")
        })?;
        let mount = image.mount()?;
        let rel = fixtures.trim_start_matches('/');
        match mount.backend.stat(rel) {
            Ok(st) if st.entry_type == tfs::EntryType::Directory => {}
            Ok(_) => {
                return Err(format!(
                    "fixtures path is not a directory in the image: {fixtures}"
                ));
            }
            Err(_) => {
                return Err(format!("fixtures dir missing in the image: {fixtures}"));
            }
        }
        copy_backend_dir(&*mount.backend, rel, scratch, 0)?;
    }
    for (name, content) in &check.fixtures_inline {
        let dest = scratch.join(name);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create fixture dir for {name}: {e}"))?;
        }
        std::fs::write(&dest, content)
            .map_err(|e| format!("cannot write inline fixture {name}: {e}"))?;
    }
    if let Some(host) = &check.fixtures_host {
        let dir = owner
            .doc_dir
            .as_ref()
            .ok_or_else(|| "internal: fixtures_host outside a composition document".to_string())?;
        let src = dir.join(host);
        let meta = std::fs::metadata(&src)
            .map_err(|_| format!("fixtures host path missing: {}", src.display()))?;
        if meta.is_dir() {
            // The CONTENTS land at the scratch root (the in-image
            // family's semantics, spec 26 §1).
            copy_host_dir(&src, scratch, 0)?;
        } else {
            let name = src
                .file_name()
                .ok_or_else(|| format!("fixtures host path has no file name: {}", src.display()))?;
            std::fs::copy(&src, scratch.join(name))
                .map_err(|e| format!("cannot copy fixture {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// Copy a host fixtures directory's CONTENTS into `dst` (recursively).
/// The composition repo's fixtures are trusted checked-in content —
/// symlinks followed by the metadata walk.
fn copy_host_dir(src: &Path, dst: &Path, depth: u32) -> Result<(), String> {
    if depth > 32 {
        return Err(format!("fixtures nest too deep at {}", src.display()));
    }
    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("cannot list fixtures dir {}: {e}", src.display()))?
    {
        let entry = entry.map_err(|e| format!("cannot read a fixture entry: {e}"))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let meta = std::fs::metadata(&from)
            .map_err(|e| format!("cannot stat fixture {}: {e}", from.display()))?;
        if meta.is_dir() {
            std::fs::create_dir_all(&to)
                .map_err(|e| format!("cannot create {}: {e}", to.display()))?;
            copy_host_dir(&from, &to, depth + 1)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("cannot copy fixture {}: {e}", from.display()))?;
        }
    }
    Ok(())
}

/// Copy one backend directory's CONTENTS into `out_dir` (recursively,
/// preserving the relative structure). Entry names are names, never
/// paths — a `/` or `..` in one is a corrupt image, named.
fn copy_backend_dir(
    backend: &dyn tfs::Backend,
    in_dir: &str,
    out_dir: &Path,
    depth: u32,
) -> Result<(), String> {
    if depth > 32 {
        return Err(format!("fixtures nest too deep at {in_dir:?}"));
    }
    for entry in backend
        .read_dir(in_dir)
        .map_err(|e| format!("cannot list fixtures dir {in_dir:?}: {}", errno_text(e)))?
    {
        if entry.name.contains('/') || entry.name == ".." || entry.name == "." {
            return Err(format!(
                "corrupt fixtures dir entry {:?} in {in_dir:?}",
                entry.name
            ));
        }
        let in_path = if in_dir.is_empty() {
            entry.name.clone()
        } else {
            format!("{in_dir}/{}", entry.name)
        };
        let out = out_dir.join(&entry.name);
        if entry.is_dir {
            std::fs::create_dir_all(&out)
                .map_err(|e| format!("cannot create {}: {e}", out.display()))?;
            copy_backend_dir(backend, &in_path, &out, depth + 1)?;
        } else {
            let st = backend
                .stat(&in_path)
                .map_err(|e| format!("cannot stat fixture {in_path:?}: {}", errno_text(e)))?;
            let size = u64::try_from(st.size)
                .map_err(|_| format!("fixture {in_path:?} has a negative size"))?;
            let mut file = std::fs::File::create(&out)
                .map_err(|e| format!("cannot create {}: {e}", out.display()))?;
            let mut off = 0u64;
            let mut chunk = vec![0u8; 1 << 20];
            while off < size {
                let want = chunk.len().min((size - off) as usize);
                let n = backend
                    .pread(&in_path, &mut chunk[..want], off)
                    .map_err(|e| format!("cannot read fixture {in_path:?}: {}", errno_text(e)))?;
                if n == 0 {
                    return Err(format!("short read on fixture {in_path:?}"));
                }
                file.write_all(&chunk[..n])
                    .map_err(|e| format!("cannot write {}: {e}", out.display()))?;
                off += n as u64;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// The child run + the assertions
// ---------------------------------------------------------------------

struct RunOutcome {
    /// The child's exit code; `None` = terminated by signal (unix).
    exit: Option<i32>,
    /// The captured stdout (the stdout-regex assertion's input).
    stdout: String,
    timed_out: bool,
}

/// Spawn the plan with cwd = the scratch dir (expect.files are
/// scratch-relative), stdout/stderr captured and teed live (they are the
/// payload's, spec 26 §2 step 4), timeout enforced by poll-and-kill.
fn run_child(plan: &RunPlan, scratch: &Path, timeout: Option<u64>) -> Result<RunOutcome, String> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args)
        .current_dir(scratch)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    if plan.scrub_preload {
        // The zero-runtime rule (dispatch.rs): the child runs host paths,
        // and the inherited preload shim would intercept its IO.
        cmd.env_remove("LD_PRELOAD");
        cmd.env_remove("DYLD_INSERT_LIBRARIES");
        cmd.env_remove("DYLD_PRINT_LIBRARIES");
        cmd.env_remove("TEBAKO_TFS_MOUNTS");
    }
    #[cfg(unix)]
    if let Some(argv0) = &plan.argv0 {
        use std::os::unix::process::CommandExt as _;
        cmd.arg0(argv0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", plan.program.display()))?;

    let out_pipe = child.stdout.take().ok_or("stdout was not piped")?;
    let err_pipe = child.stderr.take().ok_or("stderr was not piped")?;
    let out_thread = std::thread::spawn(move || pump_stream(out_pipe, true));
    let err_thread = std::thread::spawn(move || pump_stream(err_pipe, false));

    let started = Instant::now();
    let deadline = timeout.map(|t| started + Duration::from_secs(t));
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    timed_out = true;
                    let _ = child.kill();
                    match child.wait() {
                        Ok(status) => break status,
                        Err(e) => {
                            return Err(format!(
                                "cannot reap the timed-out {}: {e}",
                                plan.program.display()
                            ));
                        }
                    }
                }
                std::thread::sleep(POLL);
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("cannot wait on {}: {e}", plan.program.display()));
            }
        }
    };
    let stdout = out_thread
        .join()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let _ = err_thread.join();
    Ok(RunOutcome {
        exit: status.code(),
        stdout,
        timed_out,
    })
}

/// Read a child stream to EOF, teeing each chunk to the matching report
/// stream (stdout/stderr are the payload's, spec 26 §2 step 4) while
/// capturing for the assertions.
fn pump_stream(mut r: impl Read, tee_out: bool) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match r.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                let chunk = &chunk[..n];
                if tee_out {
                    let _ = std::io::stdout().lock().write_all(chunk);
                } else {
                    let _ = std::io::stderr().lock().write_all(chunk);
                }
            }
        }
    }
    buf
}

/// The exec assertions, in order (the first failure names the verdict):
/// exit code, then `expect.files` (exist + non-empty, scratch-relative),
/// then the one stdout regex.
fn assert_run(check: &Check, outcome: &RunOutcome, scratch: &Path) -> Option<String> {
    let expected = i32::try_from(check.expect.exit).unwrap_or(i32::MAX);
    match outcome.exit {
        Some(code) if code == expected => {}
        Some(code) => return Some(format!("exit code {code} (expected {expected})")),
        None => return Some(format!("terminated by signal (expected exit {expected})")),
    }
    for f in &check.expect.files {
        let path = scratch.join(f);
        match std::fs::metadata(&path) {
            Ok(m) if m.is_file() && m.len() > 0 => {}
            Ok(m) if m.is_file() => return Some(format!("expected file empty: {f}")),
            _ => return Some(format!("expected file missing: {f}")),
        }
    }
    if let Some(pattern) = &check.expect.stdout {
        let re = match regex::Regex::new(pattern) {
            Ok(re) => re,
            Err(e) => return Some(format!("invalid stdout pattern: {e}")),
        };
        if !re.is_match(&outcome.stdout) {
            return Some(format!("stdout pattern not matched: {pattern}"));
        }
    }
    None
}

/// The structural assertion (spec 26 §1.1): each `expect.image_files`
/// entry must exist as a non-empty regular file in the owner's image —
/// or, for a composition-level structural check, in ANY of the mounted
/// slice images (the union: a file empty in one slice fails `empty` only
/// when no slice carries it non-empty).
fn assert_image_files(images: &[ImageRef], files: &[String]) -> Option<String> {
    let mut mounts = Vec::new();
    for image in images {
        match image.mount() {
            Ok(m) => mounts.push(m),
            Err(e) => return Some(e),
        }
    }
    for f in files {
        let rel = f.trim_start_matches('/');
        let mut found = false;
        let mut max_size = 0i64;
        for mount in &mounts {
            if let Ok(st) = mount.backend.stat(rel) {
                if st.entry_type == tfs::EntryType::File {
                    found = true;
                    max_size = max_size.max(st.size);
                }
            }
        }
        if !found {
            return Some(format!("expected image file missing: {f}"));
        }
        if max_size == 0 {
            return Some(format!("expected image file empty: {f}"));
        }
    }
    None
}

// ---------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn args(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    /// A check with every field defaulted (the model's fields all carry
    /// serde defaults).
    fn bare_check() -> Check {
        serde_yml::from_str::<Check>("{}").expect("the all-default check parses")
    }

    fn test_ctx(home: &Path) -> Ctx {
        let mut env = BTreeMap::new();
        env.insert("HOME".to_string(), "/home/test".to_string());
        Ctx {
            home: home.to_path_buf(),
            cwd: home.to_path_buf(),
            env,
        }
    }

    fn given_target() -> CheckTarget {
        CheckTarget {
            owners: Vec::new(),
            caps: BTreeSet::new(),
            exec: ExecCtx::Given {
                exe: None,
                env_image: None,
                checked: PathBuf::from("/x.tfs"),
                kind_is_runtime: false,
            },
        }
    }

    // -- argv ---------------------------------------------------------

    #[test]
    fn parse_plain_target() {
        let p = parse_check_args(&args(&["metanorma"])).unwrap();
        assert_eq!(
            p,
            CheckArgs {
                target: "metanorma".to_string(),
                check: None,
                list: false,
                record: false,
                keep_scratch: false,
                runtime: None,
                runtime_image: None,
            }
        );
    }

    #[test]
    fn parse_flags_separate_and_inline() {
        let p = parse_check_args(&args(&[
            "img.tfs",
            "--check",
            "boot",
            "--list",
            "--record",
            "--keep-scratch",
            "--runtime=/rt/ruby",
            "--runtime-image",
            "/rt/env.tfs",
        ]))
        .unwrap();
        assert_eq!(p.check.as_deref(), Some("boot"));
        assert!(p.list && p.record && p.keep_scratch);
        assert_eq!(p.runtime, Some(PathBuf::from("/rt/ruby")));
        assert_eq!(p.runtime_image, Some(PathBuf::from("/rt/env.tfs")));
    }

    #[test]
    fn parse_errors_name_the_token() {
        let e = parse_check_args(&args(&["t", "--check"])).unwrap_err();
        assert!(e.contains("option '--check' requires a value"), "{e}");
        let e = parse_check_args(&args(&["t", "--check", "a", "--check", "b"])).unwrap_err();
        assert!(e.contains("selects one check"), "{e}");
        let e = parse_check_args(&args(&["t", "--runtime-image", "x"])).unwrap_err();
        assert!(e.contains("--runtime-image needs --runtime"), "{e}");
        let e = parse_check_args(&args(&["a", "b"])).unwrap_err();
        assert!(e.contains("unexpected extra argument 'b'"), "{e}");
        let e = parse_check_args(&args(&["t", "--frobnicate"])).unwrap_err();
        assert!(e.contains("unknown check option '--frobnicate'"), "{e}");
        let e = parse_check_args(&args(&["--list"])).unwrap_err();
        assert!(e.contains("usage: tebako check"), "{e}");
    }

    // -- declaration order + capabilities -----------------------------

    #[test]
    fn declaration_order_follows_the_text() {
        let text = "checks:\n  zebra: {}\n  alpha: {}\n";
        let mut checks = BTreeMap::new();
        checks.insert("alpha".to_string(), bare_check());
        checks.insert("extra".to_string(), bare_check());
        checks.insert("zebra".to_string(), bare_check());
        let ordered = declaration_order(text, &checks);
        let names: Vec<&str> = ordered.iter().map(|(n, _)| n.as_str()).collect();
        // The document's order, not the BTreeMap's; a check the raw scan
        // missed is appended defensively.
        assert_eq!(names, vec!["zebra", "alpha", "extra"]);
    }

    #[test]
    fn collect_caps_per_kind() {
        let app = PayloadManifest::from_yaml(
            "identity:\n  schema_version: 1\n  kind: app\n  name: acme-app\n  version: \"1\"\n\
            \x20 producer: {tool: t, tool_version: \"1\"}\n  created: \"2026-08-19T00:00:00Z\"\n\
            \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
            \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
            \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
            provides:\n  entrypoints: [{name: acme, path: /bin/acme}, {name: acme-two, path: /bin/acme-two}]\n\
            \x20 platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}\n",
        )
        .unwrap();
        let mut caps = BTreeSet::new();
        collect_caps(&app, &mut caps);
        assert_eq!(
            caps.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["acme", "acme-app", "acme-two"]
        );

        let runtime = PayloadManifest::from_yaml(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: 4.0.6\n\
            \x20 producer: {tool: t, tool_version: \"1\"}\n  created: \"2026-08-19T00:00:00Z\"\n\
            \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
            \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
            \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
            provides:\n  provides: {engine: ruby, version: 4.0.6, abi_line: \"4.0\", platform: aarch64-macos}\n\
            \x20 built_from: {src_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1, patch_set: v0.2.8}\n\
            \x20 capabilities: {exec: true, read: true, runtime: true}\n",
        )
        .unwrap();
        let mut caps = BTreeSet::new();
        collect_caps(&runtime, &mut caps);
        assert_eq!(
            caps.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["ruby", "tebako-runtime-ruby"]
        );

        let data = PayloadManifest::from_yaml(
            "identity:\n  schema_version: 1\n  kind: data\n  name: acme-templates\n  version: \"3\"\n\
            \x20 producer: {tool: t, tool_version: \"1\"}\n  created: \"2026-08-19T00:00:00Z\"\n\
            \x20 digest:\n    tree_hash: \"sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3\"\n\
            \x20   blob_sha256: 7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1\n\
            \x20 signing: {state: unsigned}\n  encryption: {state: none}\n\
            provides:\n  mount_semantics: {suggested: /templates/acme}\n  capabilities: {exec: false, read: true}\n",
        )
        .unwrap();
        let mut caps = BTreeSet::new();
        collect_caps(&data, &mut caps);
        assert_eq!(
            caps.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["acme-templates"]
        );
    }

    // -- verdicts + the when filter ------------------------------------

    #[test]
    fn verdict_line_shapes() {
        assert_eq!(
            verdict_line(None, "boot", &Verdict::Pass { secs: 41 }),
            "check boot PASS 41s"
        );
        assert_eq!(
            verdict_line(
                None,
                "pdf",
                &Verdict::Skip {
                    reason: "no jvm in the composition".to_string()
                }
            ),
            "check pdf SKIP (no jvm in the composition)"
        );
        assert_eq!(
            verdict_line(
                Some("slice metanorma"),
                "layout",
                &Verdict::Fail {
                    reason: "expected image file missing: /x".to_string()
                }
            ),
            "slice metanorma: layout FAIL (expected image file missing: /x)"
        );
    }

    #[test]
    fn when_filter_semantics() {
        assert!(in_force_on(&[], None));
        assert!(in_force_on(
            &[CheckPlatform::Linux],
            Some(CheckPlatform::Linux)
        ));
        assert!(!in_force_on(
            &[CheckPlatform::Macos],
            Some(CheckPlatform::Linux)
        ));
        // An unrecognized host family matches nothing.
        assert!(!in_force_on(&[CheckPlatform::Macos], None));
        let expected = match std::env::consts::OS {
            "windows" => Some(CheckPlatform::Windows),
            "macos" => Some(CheckPlatform::Macos),
            "linux" => Some(CheckPlatform::Linux),
            _ => None,
        };
        assert_eq!(host_platform(), expected);
        assert_eq!(platform_name(CheckPlatform::Windows), "windows");
        assert_eq!(platform_name(CheckPlatform::Macos), "macos");
        assert_eq!(platform_name(CheckPlatform::Linux), "linux");
    }

    // -- the jail env composition --------------------------------------

    #[test]
    fn jail_env_record_is_the_bare_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        let env = check_jail_env(&given_target(), &bare_check(), tmp.path(), true, &ctx).unwrap();
        let journal = tmp
            .path()
            .join("journal.log")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            env,
            vec![
                ("TEBAKO_JAIL".to_string(), "record".to_string()),
                ("TEBAKO_JAIL_SOURCE".to_string(), "user".to_string()),
                ("TEBAKO_JAIL_JOURNAL".to_string(), journal),
            ]
        );
    }

    #[test]
    fn jail_env_open_world_exports_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        // The Given form has no declared policy and this check declares
        // no needs: the open world needs no export (the scratch grant
        // exists to OPEN a deny world; under open it is inert).
        let env = check_jail_env(&given_target(), &bare_check(), tmp.path(), false, &ctx).unwrap();
        assert!(env.is_empty(), "{env:?}");
    }

    #[test]
    fn jail_env_needs_open_the_world_with_grants() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        let scratch = tmp.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        let mut check = bare_check();
        check.needs.push(tpkg::CheckNeed {
            path: tmp.path().to_string_lossy().into_owned(),
            access: JailAccess::Ro,
            when: vec![],
            why: None,
        });
        let env = check_jail_env(&given_target(), &check, &scratch, false, &ctx).unwrap();
        let get = |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("TEBAKO_JAIL_SOURCE"), Some("check"));
        let spec = get("TEBAKO_JAIL").expect("the jail spec is exported");
        // The env spec renders the grants as authored (canonicalization
        // is the driver's bind-time job, not the wire format's).
        let need = tmp.path().to_string_lossy().into_owned();
        assert!(spec.contains(&need), "{spec}");
        assert!(spec.contains("open"), "{spec}");
    }

    #[test]
    fn jail_env_when_filtered_needs_are_inert() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        let not_host = match host_platform() {
            Some(CheckPlatform::Windows) => CheckPlatform::Linux,
            _ => CheckPlatform::Windows,
        };
        let mut check = bare_check();
        check.needs.push(tpkg::CheckNeed {
            path: tmp.path().to_string_lossy().into_owned(),
            access: JailAccess::Ro,
            when: vec![not_host],
            why: None,
        });
        // The one need is out of force on this host and no base policy is
        // declared: nothing is exported.
        let env = check_jail_env(&given_target(), &check, tmp.path(), false, &ctx).unwrap();
        assert!(env.is_empty(), "{env:?}");
    }

    #[test]
    fn jail_env_package_deny_base_is_user_labeled() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        let scratch = tmp.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        let target = CheckTarget {
            owners: Vec::new(),
            caps: BTreeSet::new(),
            exec: ExecCtx::Package {
                path: PathBuf::from("/x.tpkg"),
                entries: Vec::new(),
                has_block: true,
                base_jail: Some(HostJail {
                    default_open: false,
                    record: false,
                    mounts: Vec::new(),
                    argument_files: tpkg::ArgumentFiles::default(),
                }),
            },
        };
        let env = check_jail_env(&target, &bare_check(), &scratch, false, &ctx).unwrap();
        let get = |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        // The package form's export rides the bootstrap's user-tightening
        // channel; the deny base gains the scratch grant.
        assert_eq!(get("TEBAKO_JAIL_SOURCE"), Some("user"));
        let spec = get("TEBAKO_JAIL").expect("the jail spec is exported");
        let raw = scratch.to_string_lossy().into_owned();
        assert!(spec.contains(&raw), "{spec}");
        assert!(spec.contains("deny"), "{spec}");
    }

    // -- atoms ----------------------------------------------------------

    #[test]
    fn engine_atoms_bind_from_the_ctx() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        assert_eq!(engine_atom("HOME", &ctx).as_deref(), Some("/home/test"));
        assert_eq!(
            engine_atom("CWD", &ctx).as_deref(),
            Some(tmp.path().to_string_lossy().as_ref())
        );
        assert_eq!(
            engine_atom("TEBAKO_HOME", &ctx).as_deref(),
            Some(tmp.path().to_string_lossy().as_ref())
        );
        // TMPDIR/TEMP unset in the fixture env: the host temp dir is the
        // fallback (spec 23 §2's per-invocation binding).
        assert!(engine_atom("TMPDIR", &ctx).is_some());
        assert_eq!(engine_atom("QUX", &ctx), None);

        // HOME falls back to USERPROFILE.
        let mut env = BTreeMap::new();
        env.insert("USERPROFILE".to_string(), "C:/Users/test".to_string());
        let ctx = Ctx {
            home: tmp.path().to_path_buf(),
            cwd: tmp.path().to_path_buf(),
            env,
        };
        assert_eq!(engine_atom("HOME", &ctx).as_deref(), Some("C:/Users/test"));
    }

    #[test]
    fn expand_atom_path_names_the_unknown_atom() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(tmp.path());
        let expanded = expand_atom_path("$HOME/docs", &ctx).unwrap();
        assert_eq!(expanded, "/home/test/docs");
        let err = expand_atom_path("$QUX/x", &ctx).unwrap_err();
        assert!(err.message.contains("QUX"), "{err:?}");
    }

    // -- scratch + argv substitution ------------------------------------

    #[test]
    fn scratch_lifecycle() {
        let path;
        {
            let scratch = Scratch::new("unit", false).unwrap();
            path = scratch.path().to_path_buf();
            assert!(path.is_dir());
        }
        assert!(!path.exists(), "dropped scratch is removed: {path:?}");

        let kept = Scratch::new("unit", true).unwrap();
        let kept_path = kept.path().to_path_buf();
        drop(kept);
        assert!(kept_path.is_dir(), "--keep-scratch preserves the dir");
        std::fs::remove_dir_all(&kept_path).unwrap();
    }

    #[test]
    fn substitute_argv_replaces_the_one_token() {
        let out = substitute_argv(
            &[
                "--out".to_string(),
                "{scratch}/out.txt".to_string(),
                "literal".to_string(),
            ],
            Path::new("/tmp/scr"),
        );
        assert_eq!(
            out,
            vec![
                "--out".to_string(),
                "/tmp/scr/out.txt".to_string(),
                "literal".to_string()
            ]
        );
    }

    // -- the exec assertions ---------------------------------------------

    fn outcome(exit: Option<i32>, stdout: &str) -> RunOutcome {
        RunOutcome {
            exit,
            stdout: stdout.to_string(),
            timed_out: false,
        }
    }

    #[test]
    fn assert_run_exit_codes() {
        let tmp = tempfile::tempdir().unwrap();
        let check = bare_check();
        assert_eq!(assert_run(&check, &outcome(Some(0), ""), tmp.path()), None);
        assert_eq!(
            assert_run(&check, &outcome(Some(1), ""), tmp.path()),
            Some("exit code 1 (expected 0)".to_string())
        );
        assert_eq!(
            assert_run(&check, &outcome(None, ""), tmp.path()),
            Some("terminated by signal (expected exit 0)".to_string())
        );
        let mut check = bare_check();
        check.expect.exit = 3;
        assert_eq!(assert_run(&check, &outcome(Some(3), ""), tmp.path()), None);
    }

    #[test]
    fn assert_run_files() {
        let tmp = tempfile::tempdir().unwrap();
        let mut check = bare_check();
        check.expect.files = vec!["out.txt".to_string()];
        assert_eq!(
            assert_run(&check, &outcome(Some(0), ""), tmp.path()),
            Some("expected file missing: out.txt".to_string())
        );
        std::fs::write(tmp.path().join("out.txt"), b"").unwrap();
        assert_eq!(
            assert_run(&check, &outcome(Some(0), ""), tmp.path()),
            Some("expected file empty: out.txt".to_string())
        );
        std::fs::write(tmp.path().join("out.txt"), b"x").unwrap();
        assert_eq!(assert_run(&check, &outcome(Some(0), ""), tmp.path()), None);
    }

    #[test]
    fn assert_run_stdout_regex() {
        let tmp = tempfile::tempdir().unwrap();
        let mut check = bare_check();
        check.expect.stdout = Some("\"ok\":1".to_string());
        assert_eq!(
            assert_run(&check, &outcome(Some(0), "{\"ok\":1}\n"), tmp.path()),
            None
        );
        assert_eq!(
            assert_run(&check, &outcome(Some(0), "nope\n"), tmp.path()),
            Some("stdout pattern not matched: \"ok\":1".to_string())
        );
        check.expect.stdout = Some("[".to_string());
        let reason = assert_run(&check, &outcome(Some(0), "x"), tmp.path()).unwrap();
        assert!(reason.starts_with("invalid stdout pattern: "), "{reason}");
    }

    // -- the composition document's serde discipline ----------------------

    #[test]
    fn composition_doc_minimal_parses() {
        let doc = serde_yml::from_str::<CompositionDoc>("version: 1\n").unwrap();
        assert_eq!(doc.version, 1);
        assert!(doc.checks.is_empty());
        assert!(doc.slices.is_empty());
    }

    #[test]
    fn composition_doc_refuses_unknown_fields() {
        let err = serde_yml::from_str::<CompositionDoc>("version: 1\nbogus: 1\n").unwrap_err();
        assert!(err.to_string().contains("unknown field"), "{err}");
    }

    #[test]
    fn composition_doc_refuses_duplicate_check_names() {
        let err = serde_yml::from_str::<CompositionDoc>("version: 1\nchecks:\n  a: {}\n  a: {}\n")
            .unwrap_err();
        assert!(err.to_string().contains("duplicate check name"), "{err}");
    }

    #[test]
    fn access_words() {
        assert_eq!(parse_access_word("ro", "t").unwrap(), JailAccess::Ro);
        assert_eq!(parse_access_word("rw", "t").unwrap(), JailAccess::Rw);
        let err = parse_access_word("xx", "mounts[0]").unwrap_err();
        assert!(
            err.message
                .contains("mounts[0]: unknown access \"xx\" (want ro|rw)"),
            "{err:?}"
        );
    }
}

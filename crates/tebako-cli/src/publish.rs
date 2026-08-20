//! `tebako publish` (spec 16 §5, roadmap 41): persona C's release flow in
//! one command, from a feedstock checkout:
//!
//!   collect the per-triplet payloads the CI matrix legs pressed
//!   (prebuilt `.tfs` per triplet — one mechanical `tebako press` per
//!   leg, spec 16 §5; publish is the single release-tail step)
//!   → optional OpenPGP sign (detached `.asc` per artifact + signed
//!     SHA256SUMS; press-local key by default, spec 09 §3)
//!   → upload every asset to the GitHub release (in-process HTTP via
//!     tebako-resolve's publish client — delete-then-upload replace
//!     semantics, so re-publishing a version is idempotent)
//!   → generate/update `tpkg-registry.yaml` (the lossless model upsert)
//!   → optionally write/bump the Homebrew tap formula (the
//!     tamatebako/homebrew-tap template, committed through the contents
//!     API and/or written to a local checkout)
//!   → verify the loop: resolve the registry from a CLEAN temp cache and
//!     `tebako install` the just-published payload.
//!
//! No shell-outs anywhere (spec 14 §3): no `gh`, no `git`, no `gpg` —
//! the release API and the contents API are plain in-process HTTPS.
//!
//! Known model limit: the registry carries ONE signature pin per version
//! (spec 04 §2's `signature: {keyid, asc}`); a signed per-triplet publish
//! uploads every per-triplet `.asc` but the version-level block names the
//! alphabetically-first triplet's asc. Per-platform signature pins are a
//! spec 04 model extension, not roadmap 41.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tebako_resolve::publish::EntrySpec;
use tebako_resolve::registry::{
    PlatformArtifact, RegistryRuntimeRequirement, SignaturePin,
};
use tebako_resolve::{
    Fetcher, PublishTransport, Registry, Transport,
};
use tpkg::{PayloadKind, Platform};

use crate::error::TebakoError;
use crate::install;

// Exit codes: the spec 06 §4 named set (as install.rs).
const EX_USAGE: i32 = 64;
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_UNAVAILABLE: i32 = 69;
const EX_TEBAKO_SIGNATURE: i32 = 71;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, code)
}

/// The recipe file publish detects in a feedstock checkout.
pub const RECIPE_FILE: &str = "recipe.yml";
/// The registry file publish generates (default-branch root, spec 04 §2).
pub const REGISTRY_FILE: &str = "tpkg-registry.yaml";
/// The tamatebako/homebrew-tap app-formula template, vendored.
const FORMULA_TEMPLATE: &str = include_str!("../templates/app-formula.rb.template");
/// The four triplets the brew formula template covers.
const BREW_TRIPLETS: [(&str, Platform); 4] = [
    ("@@SHA256_MACOS_ARM64@@", Platform::Aarch64Macos),
    ("@@SHA256_MACOS_X86_64@@", Platform::X86_64Macos),
    ("@@SHA256_LINUX_GNU_ARM64@@", Platform::Aarch64LinuxGnu),
    ("@@SHA256_LINUX_GNU_X86_64@@", Platform::X86_64LinuxGnu),
];

// ---------------------------------------------------------------------
// The recipe (feedstock checkout's recipe.yml)
// ---------------------------------------------------------------------

/// The publish recipe: what the feedstock checkout declares about the
/// payload being shipped. Unknown keys are tolerated (the tpkg manifest
/// discipline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default = "default_kind")]
    pub kind: PayloadKind,
    /// `owner/repo` — the GitHub repository whose releases host the
    /// payloads (spec 13 §9: the feedstock's own repo).
    pub repo: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub runtime_requirement: Option<RecipeRuntimeRequirement>,
    /// `triplet|universal → path` of the prebuilt `.tfs` payloads.
    #[serde(default)]
    pub payloads: BTreeMap<String, String>,
    /// `triplet → path` of the standalone binaries (the tap formula's
    /// downloads; optional — only needed for `--tap`).
    #[serde(default)]
    pub binaries: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeRuntimeRequirement {
    pub engine: String,
    pub constraint: String,
}

fn default_kind() -> PayloadKind {
    PayloadKind::App
}

impl Recipe {
    pub fn from_yaml(text: &str) -> Result<Recipe, TebakoError> {
        let recipe: Recipe = serde_yml::from_str(text)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, format!("cannot parse the recipe yaml: {e}")))?;
        if recipe.schema_version != 1 {
            return Err(err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "unsupported recipe schema_version {} (this build reads 1)",
                    recipe.schema_version
                ),
            ));
        }
        check_path_safe("payload name", &recipe.name)?;
        check_repo(&recipe.repo)?;
        Ok(recipe)
    }
}

/// Names/versions become cache path components and asset names.
fn check_path_safe(what: &str, value: &str) -> Result<(), TebakoError> {
    let bad = value.is_empty()
        || value == "."
        || value == ".."
        || value
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_control() || c.is_whitespace());
    if bad {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("{what} '{value}' must be a single non-empty path component"),
        ));
    }
    Ok(())
}

/// `owner/repo` — exactly two path-safe components.
fn check_repo(repo: &str) -> Result<(), TebakoError> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|p| p.is_empty()) {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("repo '{repo}' must be owner/repo"),
        ));
    }
    check_path_safe("repo owner", parts[0])?;
    check_path_safe("repo name", parts[1])?;
    Ok(())
}

// ---------------------------------------------------------------------
// Options + outcome
// ---------------------------------------------------------------------

/// What one `tebako publish` run works with. CLI flags override the
/// recipe; a missing recipe is fine when `--name`/`--repo`/`--payload`
/// carry the same facts.
#[derive(Debug, Default)]
pub struct PublishOptions {
    /// The feedstock checkout (relative payload paths resolve here).
    pub dir: PathBuf,
    /// --recipe (default `<dir>/recipe.yml` when it exists).
    pub recipe: Option<PathBuf>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub repo: Option<String>,
    /// --registry (default `<dir>/tpkg-registry.yaml`).
    pub registry: Option<PathBuf>,
    /// --payload `triplet|universal=path` (repeatable).
    pub payloads: Vec<(String, PathBuf)>,
    /// --binary `triplet=path` (repeatable; the tap formula's downloads).
    pub binaries: Vec<(String, PathBuf)>,
    /// --sign[=<keyid>] (None: unsigned; Some(None): press-local key).
    pub sign: Option<Option<String>>,
    /// --tap `org/homebrew-tap` (commit Formula/<name>.rb via the API).
    pub tap: Option<String>,
    /// --tap-output <dir> (write Formula/<name>.rb locally).
    pub tap_output: Option<PathBuf>,
    pub no_verify: bool,
    /// The publisher's $TEBAKO_HOME (keys, trusted keyring).
    pub home: PathBuf,
    /// The GitHub token (Bearer value); required for upload/tap.
    pub token: Option<String>,
    /// The dispatcher binary the verify step's shims link to (tests).
    pub shim_binary: Option<PathBuf>,
}

/// What a publish produced (the CLI's summary lines).
#[derive(Debug)]
pub struct PublishOutcome {
    pub name: String,
    pub version: String,
    pub tag: String,
    pub uploaded: Vec<String>,
    pub replaced: Vec<String>,
    /// The signer keyid (16 hex) when the release was signed.
    pub signer: Option<String>,
    pub registry_path: PathBuf,
    /// The tap formula outcomes (`file <path>` / `org/repo:Formula/<name>.rb`).
    pub tap: Vec<String>,
    /// The verify step's one-line proof (None when skipped).
    pub verified: Option<String>,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------
// The flow
// ---------------------------------------------------------------------

/// The production entry: tebako-http transport, the real fetcher for the
/// verify step.
pub fn publish(opts: &PublishOptions) -> Result<PublishOutcome, TebakoError> {
    let api = tebako_resolve::HttpPublishTransport;
    publish_with(opts, &api, &Fetcher::new())
}

/// The transport-injected half (tests): `api` answers the release/
/// contents APIs, `fetcher` resolves the verify step's downloads.
pub fn publish_with<T: PublishTransport, U: Transport>(
    opts: &PublishOptions,
    api: &T,
    fetcher: &Fetcher<U>,
) -> Result<PublishOutcome, TebakoError> {
    let mut notes = Vec::new();
    let plan = load_plan(opts, &mut notes)?;

    // 1. collect the payloads (+ binaries): read, hash, name the assets
    let mut artifacts: Vec<(String, Vec<u8>)> = Vec::new();
    let mut platforms: BTreeMap<Platform, PlatformArtifact> = BTreeMap::new();
    let mut universal: Option<(String, String)> = None;
    match &plan.payloads {
        PayloadSet::Universal(path) => {
            let (bytes, sha) = read_artifact(&plan.dir, path)?;
            let name = tebako_resolve::artifact_name(&plan.name, &plan.version, None);
            universal = Some((name.clone(), sha));
            artifacts.push((name, bytes));
        }
        PayloadSet::PerTriplet(map) => {
            for (platform, path) in map {
                let (bytes, sha) = read_artifact(&plan.dir, path)?;
                let name =
                    tebako_resolve::artifact_name(&plan.name, &plan.version, Some(*platform));
                platforms.insert(
                    *platform,
                    PlatformArtifact {
                        artifact: name.clone(),
                        sha256: sha,
                    },
                );
                artifacts.push((name, bytes));
            }
        }
    }
    let mut binary_shas: BTreeMap<Platform, String> = BTreeMap::new();
    for (platform, path) in &plan.binaries {
        let (bytes, sha) = read_artifact(&plan.dir, path)?;
        let name = tebako_resolve::binary_asset_name(&plan.name, &plan.version, *platform);
        binary_shas.insert(*platform, sha);
        artifacts.push((name, bytes));
    }

    // 2. optional sign: a detached armored .asc per artifact + the
    //    SHA256SUMS sidecar pair (tebako-pkg sign's release convention)
    let mut sums = String::new();
    for (name, bytes) in &artifacts {
        sums.push_str(&format!("{}  {name}\n", tebako_resolve::sha256_hex(bytes)));
    }
    let signing = resolve_signing_key(opts)?;
    if let Some(key) = &signing {
        tebako_signer::register_trusted(&opts.home, &key.public_key)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?;
        let mut signed = Vec::new();
        for (name, bytes) in artifacts {
            let asc = sign_artifact(key, &name, &bytes)?;
            signed.push((name, bytes));
            signed.push(asc);
        }
        artifacts = signed;
        artifacts.push(("SHA256SUMS".to_string(), sums.clone().into_bytes()));
        artifacts.push(sign_artifact(key, "SHA256SUMS", sums.as_bytes())?);
    } else {
        artifacts.push(("SHA256SUMS".to_string(), sums.into_bytes()));
    }

    // 3. the registry entries (the lossless model upsert) — generated
    //    BEFORE the upload so the registry file rides the release itself
    //    (spec 13 §9: releases carry their tpkg-registry.yaml)
    let registry_path = opts
        .registry
        .clone()
        .unwrap_or_else(|| plan.dir.join(REGISTRY_FILE));
    let mut registry = match fs::read_to_string(&registry_path) {
        Ok(text) => Registry::from_yaml(&text).map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!("{} does not parse: {e}", registry_path.display()),
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Registry {
            schema_version: tebako_resolve::registry::REGISTRY_SCHEMA_VERSION,
            payloads: Vec::new(),
        },
        Err(e) => {
            return Err(err(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", registry_path.display()),
            ))
        }
    };
    let tag = tebako_resolve::release_tag(&plan.version);
    let signature = signing.as_ref().map(|key| SignaturePin {
        keyid: key.keyid_hex(),
        asc: format!(
            "{}.asc",
            match &universal {
                Some((name, _)) => name.clone(),
                None => platforms
                    .values()
                    .next()
                    .expect("per-triplet is non-empty")
                    .artifact
                    .clone(),
            }
        ),
    });
    let entry = EntrySpec {
        name: plan.name.clone(),
        kind: plan.kind,
        version: plan.version.clone(),
        universal,
        per_triplet: platforms.clone(),
        release_ref: format!("tfs:github:{}:{tag}", plan.repo),
        signature,
        runtime_requirement: plan.runtime_requirement.clone(),
        entrypoints: plan.entrypoints.clone(),
        set_default: true,
    };
    tebako_resolve::upsert_entry(&mut registry, &entry)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
    let registry_yaml = registry
        .to_yaml()
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
    if let Some(parent) = registry_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| err(EX_TEBAKO_IO, format!("cannot create {}: {e}", parent.display())))?;
    }
    fs::write(&registry_path, &registry_yaml).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot write {}: {e}", registry_path.display()),
        )
    })?;
    artifacts.push((REGISTRY_FILE.to_string(), registry_yaml.into_bytes()));

    // 4. upload to the GitHub release (create-or-reuse, replace per asset)
    let token = opts.token.clone().ok_or_else(|| {
        err(
            EX_TEBAKO_UNAVAILABLE,
            "publishing needs a GitHub token — set GITHUB_TOKEN (contents+release write scope)",
        )
    })?;
    let (owner, repo) = plan.repo.split_once('/').expect("check_repo ran");
    let client = tebako_resolve::GithubReleaseClient {
        transport: api,
        owner,
        repo,
        token: &token,
    };
    let released = client
        .publish(&tag, &artifacts)
        .map_err(|e| err(EX_TEBAKO_UNAVAILABLE, e.to_string()))?;

    // 5. the tap formula (render from the vendored template; commit via
    //    the contents API and/or write to a local checkout)
    let mut tap = Vec::new();
    if opts.tap.is_some() || opts.tap_output.is_some() {
        let formula = render_formula(&plan, &binary_shas)?;
        if let Some(dir) = &opts.tap_output {
            let path = dir.join("Formula").join(format!("{}.rb", plan.name));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    err(EX_TEBAKO_IO, format!("cannot create {}: {e}", parent.display()))
                })?;
            }
            fs::write(&path, &formula)
                .map_err(|e| err(EX_TEBAKO_IO, format!("cannot write {}: {e}", path.display())))?;
            tap.push(format!("file {}", path.display()));
        }
        if let Some(tap_repo) = &opts.tap {
            check_repo(tap_repo)?;
            let (tap_owner, tap_name) = tap_repo.split_once('/').expect("check_repo ran");
            let path = format!("Formula/{}.rb", plan.name);
            let outcome = tebako_resolve::commit_file(
                api,
                tap_owner,
                tap_name,
                &path,
                formula.as_bytes(),
                &format!("{} {}", plan.name, plan.version),
                &token,
            )
            .map_err(|e| err(EX_TEBAKO_UNAVAILABLE, e.to_string()))?;
            tap.push(format!("{tap_repo}:{path} ({outcome:?})"));
        }
    }

    // 6. verify the loop: a CLEAN temp cache installs the just-published
    //    payload through the freshly written registry
    let verified = if opts.no_verify {
        notes.push("verify step skipped (--no-verify)".to_string());
        None
    } else {
        verify_install(
            opts,
            &plan,
            &registry_path,
            &platforms,
            signing.as_ref(),
            fetcher,
            &mut notes,
        )?
    };

    Ok(PublishOutcome {
        name: plan.name,
        version: plan.version,
        tag,
        uploaded: released.uploaded,
        replaced: released.replaced,
        signer: signing.as_ref().map(|k| k.keyid_hex()),
        registry_path,
        tap,
        verified,
        notes,
    })
}

// ---------------------------------------------------------------------
// The plan (recipe + CLI overrides)
// ---------------------------------------------------------------------

enum PayloadSet {
    Universal(PathBuf),
    PerTriplet(BTreeMap<Platform, PathBuf>),
}

struct Plan {
    dir: PathBuf,
    name: String,
    kind: PayloadKind,
    version: String,
    repo: String,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    entrypoints: Vec<String>,
    runtime_requirement: Option<RegistryRuntimeRequirement>,
    payloads: PayloadSet,
    binaries: BTreeMap<Platform, PathBuf>,
}

fn load_plan(opts: &PublishOptions, notes: &mut Vec<String>) -> Result<Plan, TebakoError> {
    let dir = &opts.dir;
    let recipe_path = opts.recipe.clone().unwrap_or_else(|| dir.join(RECIPE_FILE));
    let recipe = match fs::read_to_string(&recipe_path) {
        Ok(text) => Some(Recipe::from_yaml(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && opts.recipe.is_none() => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(err(
                EX_USAGE,
                format!("recipe not found: {}", recipe_path.display()),
            ))
        }
        Err(e) => {
            return Err(err(
                EX_TEBAKO_IO,
                format!("cannot read {}: {e}", recipe_path.display()),
            ))
        }
    };

    let name = opts
        .name
        .clone()
        .or_else(|| recipe.as_ref().map(|r| r.name.clone()))
        .ok_or_else(|| {
            err(
                EX_USAGE,
                "no payload name — recipe.yml declares it, or pass --name",
            )
        })?;
    check_path_safe("payload name", &name)?;
    let version = opts
        .version
        .clone()
        .or_else(|| recipe.as_ref().and_then(|r| r.version.clone()))
        .ok_or_else(|| {
            err(
                EX_USAGE,
                "no version — recipe.yml declares it, or pass --version",
            )
        })?;
    check_path_safe("version", &version)?;
    let repo = opts
        .repo
        .clone()
        .or_else(|| recipe.as_ref().map(|r| r.repo.clone()))
        .ok_or_else(|| {
            err(
                EX_USAGE,
                "no release repo — recipe.yml declares it, or pass --repo owner/repo",
            )
        })?;
    check_repo(&repo)?;

    let kind = recipe.as_ref().map(|r| r.kind).unwrap_or(PayloadKind::App);
    let mut entrypoints = recipe
        .as_ref()
        .map(|r| r.entrypoints.clone())
        .unwrap_or_default();
    if entrypoints.is_empty() && kind == PayloadKind::App {
        entrypoints = vec![name.clone()];
        notes.push(format!("no entrypoints declared — defaulting to [{name}]"));
    }
    let runtime_requirement = recipe
        .as_ref()
        .and_then(|r| r.runtime_requirement.clone())
        .map(|r| RegistryRuntimeRequirement {
            engine: r.engine,
            constraint: r.constraint,
        });

    // CLI --payload entries override the recipe's per key.
    let mut payload_map: BTreeMap<String, PathBuf> = recipe
        .as_ref()
        .map(|r| {
            r.payloads
                .iter()
                .map(|(k, v)| (k.clone(), PathBuf::from(v)))
                .collect()
        })
        .unwrap_or_default();
    for (key, path) in &opts.payloads {
        payload_map.insert(key.clone(), path.clone());
    }
    let payloads = parse_payload_set(&payload_map)?;

    let mut binaries: BTreeMap<Platform, PathBuf> = BTreeMap::new();
    if let Some(recipe) = &recipe {
        for (key, path) in &recipe.binaries {
            binaries.insert(parse_triplet(key)?, PathBuf::from(path));
        }
    }
    for (key, path) in &opts.binaries {
        binaries.insert(parse_triplet(key)?, path.clone());
    }

    Ok(Plan {
        dir: dir.clone(),
        name,
        kind,
        version,
        repo,
        description: recipe.as_ref().and_then(|r| r.description.clone()),
        homepage: recipe.as_ref().and_then(|r| r.homepage.clone()),
        license: recipe.as_ref().and_then(|r| r.license.clone()),
        entrypoints,
        runtime_requirement,
        payloads,
        binaries,
    })
}

fn parse_triplet(key: &str) -> Result<Platform, TebakoError> {
    let platform = Platform::from_triplet(key).ok_or_else(|| {
        err(
            EX_USAGE,
            format!("'{key}' is not a spec 03 §3 triplet"),
        )
    })?;
    if platform.is_reserved() {
        return Err(err(
            EX_USAGE,
            format!("'{key}' names the reserved triplet (spec 03 §3)"),
        ));
    }
    Ok(platform)
}

fn parse_payload_set(map: &BTreeMap<String, PathBuf>) -> Result<PayloadSet, TebakoError> {
    if map.is_empty() {
        return Err(err(
            EX_USAGE,
            "nothing to publish — declare payloads: in recipe.yml or pass --payload <triplet|universal>=<path>",
        ));
    }
    let universal = map.get("universal");
    let mut per_triplet: BTreeMap<Platform, PathBuf> = BTreeMap::new();
    for (key, path) in map {
        if key == "universal" {
            continue;
        }
        per_triplet.insert(parse_triplet(key)?, path.clone());
    }
    match (universal, per_triplet.is_empty()) {
        (Some(path), true) => Ok(PayloadSet::Universal(path.clone())),
        (None, false) => Ok(PayloadSet::PerTriplet(per_triplet)),
        (Some(_), false) => Err(err(
            EX_USAGE,
            "payloads mix 'universal' with per-triplet entries — the registry's platforms axis is one or the other (spec 04 §2)",
        )),
        (None, true) => unreachable!("map is non-empty and holds only unknown keys"),
    }
}

fn read_artifact(dir: &Path, path: &Path) -> Result<(Vec<u8>, String), TebakoError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        dir.join(path)
    };
    let bytes = fs::read(&path).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot read payload {}: {e}", path.display()),
        )
    })?;
    let sha = tebako_resolve::sha256_hex(&bytes);
    Ok((bytes, sha))
}

// ---------------------------------------------------------------------
// signing
// ---------------------------------------------------------------------

fn resolve_signing_key(opts: &PublishOptions) -> Result<Option<tebako_signer::PressKey>, TebakoError> {
    let Some(request) = &opts.sign else {
        return Ok(None);
    };
    let key = match request {
        Some(keyid) => tebako_signer::secret_key_by_keyid(&opts.home, keyid)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?
            .ok_or_else(|| {
                err(
                    EX_TEBAKO_SIGNATURE,
                    format!(
                        "no secret key with keyid {keyid} under {}",
                        opts.home.join("keys").display()
                    ),
                )
            })?,
        None => tebako_signer::press_local_key(&opts.home)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?,
    };
    Ok(Some(key))
}

/// One artifact's armored detached `.asc` (`<name>.asc`).
fn sign_artifact(
    key: &tebako_signer::PressKey,
    name: &str,
    bytes: &[u8],
) -> Result<(String, Vec<u8>), TebakoError> {
    let signature = tebako_signer::sign_detached(bytes, &key.secret_key, &key.fingerprint)
        .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    let armored =
        tebako_signer::armor_signature(&signature).map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    Ok((format!("{name}.asc"), armored))
}

// ---------------------------------------------------------------------
// the tap formula
// ---------------------------------------------------------------------

/// CamelCase of the formula class name (the template's rule:
/// metanorma.rb → class Metanorma; hyphen/underscore segments capitalize).
pub fn camel_case(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Render the vendored app-formula template. Fail-closed: every
/// placeholder must be filled (a binary missing for one of the four brew
/// triplets is a named error listing them — never a formula carrying an
/// `@@…@@` slot).
fn render_formula(
    plan: &Plan,
    binary_shas: &BTreeMap<Platform, String>,
) -> Result<String, TebakoError> {
    let missing: Vec<&str> = BREW_TRIPLETS
        .iter()
        .filter(|(_, p)| !binary_shas.contains_key(p))
        .map(|(_, p)| p.as_triplet())
        .collect();
    if !missing.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "the tap formula needs standalone binaries for {}; provide them via recipe binaries: or --binary <triplet>=<path>",
                missing.join(", ")
            ),
        ));
    }
    let mut out = FORMULA_TEMPLATE.to_string();
    let fields = [
        ("@@CAMELAPP@@", camel_case(&plan.name)),
        (
            "@@APP_DESC@@",
            plan.description.clone().ok_or_else(|| {
                err(
                    EX_TEBAKO_MANIFEST,
                    "the tap formula needs the recipe's description:",
                )
            })?,
        ),
        (
            "@@APP_HOMEPAGE@@",
            plan.homepage.clone().ok_or_else(|| {
                err(EX_TEBAKO_MANIFEST, "the tap formula needs the recipe's homepage:")
            })?,
        ),
        ("@@APP_VERSION@@", plan.version.clone()),
        (
            "@@APP_LICENSE_SPDX@@",
            plan.license.clone().ok_or_else(|| {
                err(EX_TEBAKO_MANIFEST, "the tap formula needs the recipe's license:")
            })?,
        ),
        (
            "@@RELEASE_BASE_URL@@",
            format!("https://github.com/{}/releases/download", plan.repo),
        ),
        ("@@APP@@", plan.name.clone()),
    ];
    for (slot, value) in fields {
        out = out.replace(slot, &value);
    }
    for (slot, platform) in BREW_TRIPLETS {
        out = out.replace(
            slot,
            binary_shas.get(&platform).expect("missing checked above"),
        );
    }
    if out.contains("@@") {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            "the formula template carries an unfilled @@PLACEHOLDER@@ (template drift?)",
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// the verify step (spec 16 §5's loop proof)
// ---------------------------------------------------------------------

/// Resolve the freshly written registry from a clean temp cache and
/// install the just-published payload through it. Returns the one-line
/// proof, `Ok(None)` (with a note) when the host triplet is not among the
/// published ones (a release job publishing every triplet from one leg).
fn verify_install<U: Transport>(
    opts: &PublishOptions,
    plan: &Plan,
    registry_path: &Path,
    platforms: &BTreeMap<Platform, PlatformArtifact>,
    signing: Option<&tebako_signer::PressKey>,
    fetcher: &Fetcher<U>,
    notes: &mut Vec<String>,
) -> Result<Option<String>, TebakoError> {
    if !matches!(plan.payloads, PayloadSet::Universal(_)) {
        let host = install_host_platform()?;
        if !platforms.contains_key(&host) {
            // Cross-publish: the host has no leg in this payload set.
            notes.push(format!(
                "verify step skipped: {host} is not among the published triplets"
            ));
            return Ok(None);
        }
    }
    let verify_home = std::env::temp_dir().join(format!(
        "tebako-publish-verify-{}-{}",
        std::process::id(),
        plan.name
    ));
    let _ = fs::remove_dir_all(&verify_home);
    fs::create_dir_all(&verify_home).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", verify_home.display()),
        )
    })?;
    let result = verify_install_at(
        &verify_home,
        opts,
        plan,
        registry_path,
        signing,
        fetcher,
    );
    let _ = fs::remove_dir_all(&verify_home);
    result.map(Some)
}

fn verify_install_at<U: Transport>(
    verify_home: &Path,
    opts: &PublishOptions,
    plan: &Plan,
    registry_path: &Path,
    signing: Option<&tebako_signer::PressKey>,
    fetcher: &Fetcher<U>,
) -> Result<String, TebakoError> {
    // Our own key signs what we just published: the clean cache trusts it
    // (the publisher's machine is its own first consumer).
    if let Some(key) = signing {
        tebako_signer::register_trusted(verify_home, &key.public_key)
            .map_err(|e| err(EX_TEBAKO_SIGNATURE, e.to_string()))?;
    }
    let registry_abs = registry_path
        .canonicalize()
        .unwrap_or_else(|_| registry_path.to_path_buf());
    install::add_registry_with(verify_home, &format!("file://{}", registry_abs.display()), fetcher)?;
    let outcome = install::install_with(
        verify_home,
        &format!("{}@{}", plan.name, plan.version),
        None,
        opts.shim_binary.as_deref(),
        fetcher,
    )?;
    let signer = outcome
        .signer
        .as_ref()
        .map(|s| format!("; signature trusted (signer {s})"))
        .unwrap_or_default();
    Ok(format!(
        "{} {} installed from a clean cache ({}; sha256 {}{signer})",
        outcome.name,
        outcome.version,
        match outcome.status {
            tebako_resolve::InstallStatus::Hit => "cache hit",
            tebako_resolve::InstallStatus::Installed => "downloaded",
        },
        outcome.sha256
    ))
}

fn install_host_platform() -> Result<Platform, TebakoError> {
    let host = crate::options::host_platform()?;
    Platform::from_release_asset_name(&host).ok_or_else(|| {
        err(
            EX_TEBAKO_UNAVAILABLE,
            format!("the host platform '{host}' is not on the spec 03 §3 triplet axis"),
        )
    })
}

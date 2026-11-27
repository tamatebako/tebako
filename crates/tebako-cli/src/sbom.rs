//! `tebako sbom` — the evidence surface (spec 15 §8 + §8a): emit a
//! CycloneDX 1.6 SBOM for one payload image (or an installed payload
//! from the store), for one runtime env image (`--runtime`), or for one
//! runtime interpreter exe (`--runtime-exe --crates`).
//!
//! The payload lane's inputs: the image's own L1 manifest
//! (`/__tpkg__/manifest.yaml`, read through the tfs mount — nothing is
//! extracted) carries the payload's identity/kind; the feedstock
//! closure file, when passed, carries the complete per-artifact
//! inventory (`<artifact> <sha256>`, or the sha256sum-shaped
//! `<sha256> <artifact>` spelling — both parse through one grammar).
//! Without a closure the inventory degrades to the image's own gem
//! specifications (`specifications/*.gemspec` names + versions +
//! licenses — no hashes, completeness not claimed) and the document
//! says so (`tebako:closure:complete=false` plus an annotation).
//!
//! The runtime lanes (spec 15 §8a): the env image document carries the
//! manifest-declared interpreter build (engine, version, abi line,
//! platform, source pin, patch set) plus the image's FULL gem inventory
//! (the bundled `specifications/` class and the default-gem class —
//! never "complete" in the closure sense; the annotation names what
//! the inventory honestly is). The exe document re-subjects the
//! build-time crate capture (`crate-inventory.json`, tebako-sbom's
//! capture grammar) to the exe's bytes — the vendored crates are what
//! the build resolved, never a post-hoc guess at stripped bytes.
//!
//! The emission rides the workspace's one CycloneDX owner (the
//! tebako-sbom crate): components sort by purl, the serial derives from
//! the document's identity content, and the timestamp reads
//! `SOURCE_DATE_EPOCH` when the operator pins it (the press's
//! reproducibility convention) — two runs on unchanged inputs are
//! byte-identical. `--random-serial` opts into a per-run v4 serial.

use std::path::{Path, PathBuf};

use tebako_sbom::{
    Annotation, Component, ComponentType, Dependency, DependencyGraph, Document, SbomError,
    SerialMode,
};

use crate::error::TebakoError;

const EX_USAGE: i32 = 64;
const EX_TEBAKO_MANIFEST: i32 = 65;
const EX_TEBAKO_IO: i32 = 74;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError::new(message.into(), code)
}

fn sbom_err(e: SbomError) -> TebakoError {
    err(e.exit_code(), e.to_string())
}

const USAGE: &str = "usage: tebako sbom <image.tfs | name[@version]> [--closure <path>] [--ecosystem gem|pypi|npm|cargo|generic] [--triplet <triplet>] [--out <path>] [--random-serial]\n       tebako sbom --runtime <env-image.tfs> [--triplet <triplet>] [--out <path>] [--random-serial]\n       tebako sbom --runtime-exe <exe> --crates <crate-inventory.json> --version <ver> [--name <name>] [--engine <engine> --engine-version <ver>] [--triplet <triplet>] [--out <path>] [--random-serial]";

/// The ecosystem a closure's rows (or the degraded inventory) map to
/// purls under — spec 15 §8's mapping table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ecosystem {
    Gem,
    Pypi,
    Npm,
    Cargo,
    Generic,
}

impl Ecosystem {
    fn parse(value: &str) -> Result<Self, TebakoError> {
        match value {
            "gem" => Ok(Self::Gem),
            "pypi" => Ok(Self::Pypi),
            "npm" => Ok(Self::Npm),
            "cargo" => Ok(Self::Cargo),
            "generic" => Ok(Self::Generic),
            other => Err(err(
                EX_USAGE,
                format!("unknown ecosystem '{other}' (gem|pypi|npm|cargo|generic)\n{USAGE}"),
            )),
        }
    }

    /// The purl type token (registered purl types only).
    fn purl_type(self) -> &'static str {
        match self {
            Self::Gem => "gem",
            Self::Pypi => "pypi",
            Self::Npm => "npm",
            Self::Cargo => "cargo",
            Self::Generic => "generic",
        }
    }

    /// The CycloneDX component `type` for a dependency of this
    /// ecosystem: language packages are libraries; a generic-mapped row
    /// is a native tool or binary, which CycloneDX classifies as an
    /// application.
    fn component_type(self) -> ComponentType {
        match self {
            Self::Generic => ComponentType::Application,
            _ => ComponentType::Library,
        }
    }
}

/// One inventory row's identity after ecosystem parsing. `qualifier`
/// carries the artifact's ecosystem-specific suffix (the gem platform,
/// the wheel tag triplet) — evidence, never part of the purl.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactId {
    name: String,
    version: String,
    qualifier: Option<String>,
}

/// Split `<name>-<version>[<-suffix…>]` at the FIRST dash-segment that
/// starts with an ASCII digit (the ecosystem rule: versions begin with a
/// digit, name segments never do). Everything after the version segment
/// is the platform/tag suffix. Gem versions never contain a dash, so the
/// version is exactly one segment.
fn split_versioned(full: &str) -> Result<(String, String, Option<String>), TebakoError> {
    let segments: Vec<&str> = full.split('-').collect();
    let Some(index) = segments
        .iter()
        .position(|s| s.bytes().next().is_some_and(|b| b.is_ascii_digit()))
    else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("'{full}' carries no version segment (a dash-segment starting with a digit)"),
        ));
    };
    if index == 0 {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("'{full}' has an empty name (the version starts at the first segment)"),
        ));
    }
    let name = segments[..index].join("-");
    let version = segments[index].to_string();
    let qualifier = if index + 1 < segments.len() {
        Some(segments[index + 1..].join("-"))
    } else {
        None
    };
    Ok((name, version, qualifier))
}

/// PEP 503's normalized project name (lowercase; any run of `-_.`
/// collapses to `-`) — the purl name rule for pypi.
fn pypi_normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut was_sep = false;
    for c in name.chars() {
        if matches!(c, '-' | '_' | '.') {
            if !was_sep {
                out.push('-');
            }
            was_sep = true;
        } else {
            out.push(c.to_ascii_lowercase());
            was_sep = false;
        }
    }
    out
}

/// Parse one closure row's artifact token under `ecosystem`.
fn parse_artifact(full: &str, ecosystem: Ecosystem) -> Result<ArtifactId, TebakoError> {
    match ecosystem {
        Ecosystem::Gem | Ecosystem::Cargo | Ecosystem::Generic | Ecosystem::Npm => {
            // npm names may be scoped (@scope/name); the version split is
            // the same digit rule applied after any scope prefix.
            let (name, version, qualifier) = split_versioned(full)?;
            Ok(ArtifactId {
                name,
                version,
                qualifier,
            })
        }
        Ecosystem::Pypi => {
            // PEP 427's wheel filename: {distribution}-{version}(-{build})?
            // -{python tag}-{abi tag}-{platform tag}.whl. Distribution and
            // version never contain a dash; the tag triplet (when present)
            // is the qualifier. The bare `<name>-<version>` form parses too.
            let stem = full.strip_suffix(".whl").unwrap_or(full);
            let parts: Vec<&str> = stem.split('-').collect();
            if parts.len() < 2 {
                return Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!("'{full}' is not a wheel name (<name>-<version>[-<tags>])"),
                ));
            }
            let version = parts[1];
            if !version.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
                return Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "'{full}' is not a wheel name (the version segment '{version}' starts with no digit)"
                    ),
                ));
            }
            let qualifier = if parts.len() > 2 {
                Some(parts[2..].join("-"))
            } else {
                None
            };
            Ok(ArtifactId {
                name: pypi_normalize(parts[0]),
                version: version.to_string(),
                qualifier,
            })
        }
    }
}

/// The purl for one artifact (spec 15 §8's mapping table): canonical
/// `pkg:<type>/<name>@<version>` — qualifiers stay OUT (vulnerability
/// consumers match on name+version; the gem platform / wheel tags ride
/// as component properties instead).
fn purl_of(ecosystem: Ecosystem, artifact: &ArtifactId) -> String {
    format!(
        "pkg:{}/{}@{}",
        ecosystem.purl_type(),
        purl_encode(&artifact.name),
        purl_encode(&artifact.version)
    )
}

/// purl's percent-encoding: the unreserved set plus the path-safe
/// punctuation the ecosystems use (`/` stays literal — the npm scope
/// separator). Everything else is UTF-8 percent-encoded uppercase-hex
/// (the npm scope's leading `@` encodes to `%40`).
fn purl_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// One parsed closure row.
#[derive(Debug, Clone)]
struct ClosureRow {
    artifact: String,
    sha256: String,
}

fn is_sha256_hex(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse a closure file: `<artifact> <sha256>` (the gem-shape rows of
/// the ruby feedstocks) OR `<sha256> <artifact>` (the sha256sum-shaped
/// rows of the wheel feedstocks), `#` comments and blank lines skipped.
/// The hash column is recognized BY SHAPE (64 hex) — never by position,
/// so both spellings parse through one grammar.
fn parse_closure(text: &str) -> Result<Vec<ClosureRow>, TebakoError> {
    let mut rows = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let row = match tokens.as_slice() {
            [a, b] if is_sha256_hex(a) && !is_sha256_hex(b) => ClosureRow {
                artifact: b.to_string(),
                sha256: a.to_ascii_lowercase(),
            },
            [a, b] if is_sha256_hex(b) => ClosureRow {
                artifact: a.to_string(),
                sha256: b.to_ascii_lowercase(),
            },
            _ => {
                return Err(err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "closure line {}: expected '<artifact> <sha256>' or '<sha256> <artifact>', got '{line}'",
                        number + 1
                    ),
                ))
            }
        };
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            "the closure carries no inventory rows",
        ));
    }
    Ok(rows)
}

/// The payload's own identity for `metadata.component`, resolved from
/// the L1 manifest when the image carries one (the authoritative read),
/// else from the artifact's filename with the loud note the caller
/// journals.
#[derive(Clone)]
struct PayloadId {
    name: String,
    version: String,
    kind: Option<tpkg::PayloadKind>,
    /// The manifest's language engine, when declared (`requires:` edge)
    /// — the closure ecosystem's default mapping.
    language_engine: Option<String>,
    /// The manifest's single declared platform, when exactly one.
    triplet: Option<String>,
    /// The manifest's signing declaration (spec 09 — DESCRIBED, never
    /// verified here: the SBOM is evidence, not a trust anchor).
    signing: Option<tpkg::SigningState>,
    /// The store markers, when the target resolved through the cache:
    /// the `.origin` source URL and the `.registry` canonical
    /// reference (spec 37 §7's origin binding).
    origin: Option<String>,
    registry: Option<String>,
    /// The payload image's own sha256.
    image_sha256: String,
}

/// The CycloneDX component `type` for a payload kind (spec 15 §8's kind
/// table): app/toolkit are things you run, runtime/language the platform
/// they run on, data is data.
fn payload_component_type(kind: Option<tpkg::PayloadKind>) -> ComponentType {
    match kind {
        Some(tpkg::PayloadKind::App) | Some(tpkg::PayloadKind::Toolkit) | None => {
            ComponentType::Application
        }
        Some(tpkg::PayloadKind::Runtime) | Some(tpkg::PayloadKind::Language) => {
            ComponentType::Platform
        }
        Some(tpkg::PayloadKind::Data) => ComponentType::Data,
    }
}

fn payload_kind_label(kind: Option<tpkg::PayloadKind>) -> &'static str {
    match kind {
        Some(tpkg::PayloadKind::App) => "app",
        Some(tpkg::PayloadKind::Data) => "data",
        Some(tpkg::PayloadKind::Toolkit) => "toolkit",
        Some(tpkg::PayloadKind::Runtime) => "runtime",
        Some(tpkg::PayloadKind::Language) => "language",
        None => "unknown",
    }
}

fn signing_label(state: tpkg::SigningState) -> &'static str {
    match state {
        tpkg::SigningState::Unsigned => "unsigned",
        tpkg::SigningState::Signed => "signed",
    }
}

// ---------------------------------------------------------------------
// In-image reads (the tfs mount — never an extraction)
// ---------------------------------------------------------------------

/// Read one regular file out of the mounted backend (bounded), UTF-8.
fn read_backend_file(
    backend: &dyn tfs::Backend,
    path: &str,
    max: u64,
) -> Result<Option<String>, TebakoError> {
    let st = match backend.stat(path) {
        Ok(st) => st,
        Err(_) => return Ok(None),
    };
    if st.entry_type != tfs::EntryType::File {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("{path} is not a regular file inside the image"),
        ));
    }
    let size = u64::try_from(st.size)
        .map_err(|_| err(EX_TEBAKO_MANIFEST, format!("{path} has a negative size")))?;
    if size > max {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("{path} exceeds {max} bytes inside the image"),
        ));
    }
    let mut buf = vec![0u8; size as usize];
    let mut off = 0u64;
    while off < size {
        let n = backend
            .pread(path, &mut buf[off as usize..], off)
            .map_err(|e| err(EX_TEBAKO_IO, format!("cannot read {path} (errno {e})")))?;
        if n == 0 {
            return Err(err(
                EX_TEBAKO_IO,
                format!("short read on {path} inside the image"),
            ));
        }
        off += n as u64;
    }
    String::from_utf8(buf)
        .map(Some)
        .map_err(|_| err(EX_TEBAKO_MANIFEST, format!("{path} is not valid UTF-8")))
}

/// The mounted image view the verb consumes.
struct ImageView {
    manifest: Option<tpkg::PayloadManifest>,
    /// Why there is no manifest (reported, never silent).
    manifest_note: Option<String>,
    backend: Box<dyn tfs::Backend>,
}

fn mount_image(path: &Path) -> Result<ImageView, TebakoError> {
    let mount = tfs::mount::build_from_file(&path.to_string_lossy(), "/mnt").map_err(|errno| {
        let message = std::ffi::CStr::from_bytes_until_nul(tfs::errno::strerror(errno))
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown error".to_string());
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "{}: cannot mount the image (errno {errno}): {message}",
                path.display()
            ),
        )
    })?;
    let backend = mount.backend;
    let (manifest, manifest_note) =
        match read_backend_file(&*backend, "__tpkg__/manifest.yaml", 1 << 20)? {
            Some(text) => match tpkg::PayloadManifest::from_yaml(&text) {
                Ok(m) => (Some(m), None),
                Err(e) => (
                    None,
                    Some(format!("the in-image manifest does not parse: {e}")),
                ),
            },
            None => (
                None,
                Some("no /__tpkg__/manifest.yaml — a plain image".to_string()),
            ),
        };
    Ok(ImageView {
        manifest,
        manifest_note,
        backend,
    })
}

/// The platform coverage of a manifest, whichever kind-specialized
/// provides shape carries it (app/toolkit; runtime/language carry their
/// own grammar, data is universal by construction — never guessed here).
fn manifest_platforms(manifest: &tpkg::PayloadManifest) -> Option<String> {
    let platforms = match &manifest.provides {
        tpkg::Provides::App(p) => Some(&p.platforms),
        tpkg::Provides::Toolkit(p) => Some(&p.platforms),
        _ => None,
    }?;
    match platforms {
        tpkg::Platforms::Triplets(ts) if ts.len() == 1 => Some(ts[0].as_triplet().to_string()),
        _ => None,
    }
}

/// The manifest's language engine: the FIRST `kind: language` requires
/// edge (the payload's own interpreter — a spawned `kind: runtime` edge
/// is another payload's, never this payload's ecosystem).
fn manifest_language_engine(manifest: &tpkg::PayloadManifest) -> Option<String> {
    manifest.requires.iter().find_map(|r| match r {
        tpkg::Requirement::Language { engine, .. } => Some(engine.clone()),
        _ => None,
    })
}

/// Derive the payload identity. The manifest wins when present; the
/// filename fallback names the payload from `<name>-<version>-<platform>
/// .tfs` stems (kind unknown, ecosystem by flag-or-generic), loudly.
fn payload_identity(
    path: &Path,
    view: &ImageView,
    image_sha256: String,
    origin: Option<String>,
    registry: Option<String>,
) -> Result<PayloadId, TebakoError> {
    if let Some(m) = &view.manifest {
        return Ok(PayloadId {
            name: m.identity.name.clone(),
            version: m.identity.version.clone(),
            kind: Some(m.identity.kind),
            language_engine: manifest_language_engine(m),
            triplet: manifest_platforms(m),
            signing: Some(m.identity.signing.state),
            origin,
            registry,
            image_sha256,
        });
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| {
            err(
                EX_USAGE,
                format!("cannot derive a name from {}", path.display()),
            )
        })?;
    let (name, version, _) = split_versioned(&stem).map_err(|_| {
        err(
            EX_TEBAKO_MANIFEST,
            format!(
                "{} carries no readable manifest and its name is not '<name>-<version>…' — no identity to describe",
                path.display()
            ),
        )
    })?;
    Ok(PayloadId {
        name,
        version,
        kind: None,
        language_engine: None,
        triplet: None,
        signing: None,
        origin,
        registry,
        image_sha256,
    })
}

// ---------------------------------------------------------------------
// The degraded inventory: specifications/*.gemspec from the image itself
// ---------------------------------------------------------------------

/// The gemspec directory candidates, in precedence order: a bare
/// gem-home layout first, then the staged runtime layout's one-version
/// deep dir.
fn find_gemspec_dir(backend: &dyn tfs::Backend) -> Result<Option<String>, TebakoError> {
    if let Ok(st) = backend.stat("specifications") {
        if st.entry_type == tfs::EntryType::Directory {
            return Ok(Some("specifications".to_string()));
        }
    }
    if let Ok(st) = backend.stat("lib/ruby/gems") {
        if st.entry_type == tfs::EntryType::Directory {
            let entries = backend.read_dir("lib/ruby/gems").map_err(|e| {
                err(
                    EX_TEBAKO_IO,
                    format!("cannot list lib/ruby/gems inside the image (errno {e})"),
                )
            })?;
            for entry in entries {
                if !entry.is_dir {
                    continue;
                }
                let candidate = format!("lib/ruby/gems/{}/specifications", entry.name);
                if let Ok(st) = backend.stat(&candidate) {
                    if st.entry_type == tfs::EntryType::Directory {
                        return Ok(Some(candidate));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// One installed gemspec's inventory row: name, version, and the
/// declared licenses (rubygems renders `s.licenses = ["MIT", …]` — the
/// singular `s.license = "MIT"` form parses too). Name and version must
/// both be present; a spec missing either is no inventory row.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GemspecRow {
    name: String,
    version: String,
    licenses: Vec<String>,
}

fn parse_gemspec(text: &str) -> Option<GemspecRow> {
    let mut name = None;
    let mut version = None;
    let mut licenses = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if name.is_none() {
            if let Some(value) = strip_assignment(line, "name") {
                name = Some(value.to_string());
            }
        }
        if version.is_none() {
            if let Some(value) = strip_assignment(line, "version") {
                version = Some(value.to_string());
            }
        }
        if licenses.is_empty() {
            if let Some(value) = strip_array_assignment(line, "licenses") {
                licenses = parse_string_array(value);
            } else if let Some(value) = strip_assignment(line, "license") {
                licenses = vec![value.to_string()];
            }
        }
    }
    Some(GemspecRow {
        name: name?,
        version: version?,
        licenses,
    })
}

/// `<anything>.name = "value"` (optionally `.freeze`-suffixed) →
/// `value`.
fn strip_assignment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!(".{key}");
    let at = line.find(&marker)?;
    let rest = line[at + marker.len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

/// `<anything>.licenses = […]` → the bracketed array literal through its
/// closing bracket (a trailing `.freeze` is rubygems' own spelling and
/// stays outside). No nesting exists inside a string array, so the first
/// `]` closes it.
fn strip_array_assignment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!(".{key}");
    let at = line.find(&marker)?;
    let rest = line[at + marker.len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    if !rest.starts_with('[') {
        return None;
    }
    let end = rest.find(']')?;
    Some(&rest[..=end])
}

/// `["MIT", "Apache-2.0"]` → the quoted strings. Rubygems renders each
/// element individually frozen (`["BSD-2-Clause".freeze]` — the real
/// installed-specification spelling), so a trailing `.freeze` drops off
/// each token first; the hand-written `["MIT", …].freeze` form (freeze
/// on the array, outside the bracket the caller captured) works too. A
/// token that is not a non-empty quoted string skips.
fn parse_string_array(value: &str) -> Vec<String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    inner
        .split(',')
        .filter_map(|token| {
            let token = token.trim().trim_end_matches(".freeze").trim_end();
            let quote = token.chars().next()?;
            if quote != '"' && quote != '\'' {
                return None;
            }
            let body = token[1..].strip_suffix(quote)?;
            (!body.is_empty()).then(|| body.to_string())
        })
        .collect()
}

/// The degraded inventory: the image's own gem specifications, name +
/// version + licenses only (no hashes — the gemspec carries none).
/// Returns the rows and the names of any unparseable specs (reported in
/// the note).
struct GemspecInventory {
    rows: Vec<(ArtifactId, Vec<String>)>,
    skipped: Vec<String>,
}

/// Read every parseable `*.gemspec` directly inside `dir` (never
/// recursive — the default class lives in the `default/` child and gets
/// its own pass). Rows carry the spec's declared name/version (the
/// authoritative identity, never the filename's).
fn gemspec_dir_inventory(
    backend: &dyn tfs::Backend,
    dir: &str,
) -> Result<GemspecInventory, TebakoError> {
    let entries = backend.read_dir(dir).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot list {dir} inside the image (errno {e})"),
        )
    })?;
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for entry in entries {
        if entry.is_dir || !entry.name.ends_with(".gemspec") {
            continue;
        }
        let path = format!("{dir}/{}", entry.name);
        let Some(text) = read_backend_file(backend, &path, 1 << 20)? else {
            continue;
        };
        match parse_gemspec(&text) {
            Some(row) => rows.push((
                ArtifactId {
                    name: row.name,
                    version: row.version,
                    qualifier: None,
                },
                row.licenses,
            )),
            None => skipped.push(entry.name),
        }
    }
    Ok(GemspecInventory { rows, skipped })
}

fn gemspec_inventory(backend: &dyn tfs::Backend) -> Result<GemspecInventory, TebakoError> {
    let Some(dir) = find_gemspec_dir(backend)? else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            "no closure given and the image carries no gem specifications (no specifications/ directory) — pass --closure <feedstock closure> for the complete, hashed inventory",
        ));
    };
    let mut inventory = gemspec_dir_inventory(backend, &dir)?;
    if inventory.rows.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!("{dir} inside the image carries no parseable gemspecs"),
        ));
    }
    inventory
        .rows
        .sort_by(|a, b| a.0.name.cmp(&b.0.name).then(a.0.version.cmp(&b.0.version)));
    inventory
        .rows
        .dedup_by(|a, b| a.0.name == b.0.name && a.0.version == b.0.version);
    Ok(inventory)
}

// ---------------------------------------------------------------------
// The runtime env image inventory (spec 15 §8a): the declared
// interpreter build + the GEM_HOME's two specification classes
// ---------------------------------------------------------------------

/// The runtime facts the in-image manifest declares (kind: runtime —
/// the authoritative read): the provided engine entries plus the
/// build's source pin and patch set.
#[derive(Debug, Clone)]
struct RuntimeFacts {
    /// One per DISTINCT provided engine entry (a multi-platform image
    /// lists one entry per platform — each is a build facet of the same
    /// interpreter, evidence in its own right).
    engines: Vec<EngineFacts>,
    src_sha256: Option<String>,
    patch_set: Option<String>,
}

#[derive(Debug, Clone)]
struct EngineFacts {
    engine: String,
    version: String,
    abi_line: String,
    platform: String,
}

fn runtime_facts(manifest: &tpkg::PayloadManifest) -> Result<RuntimeFacts, TebakoError> {
    if manifest.identity.kind != tpkg::PayloadKind::Runtime {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "--runtime describes the runtime env image (kind: runtime); '{}' is kind: {} — the payload lane (`tebako sbom <image>`) covers it",
                manifest.identity.name,
                payload_kind_label(Some(manifest.identity.kind))
            ),
        ));
    }
    let tpkg::Provides::Runtime(provides) = &manifest.provides else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "'{}' declares kind: runtime but carries no runtime provides block — the manifest is malformed",
                manifest.identity.name
            ),
        ));
    };
    if provides.provides.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "'{}' declares no provided engine — a runtime image without an interpreter is no runtime",
                manifest.identity.name
            ),
        ));
    }
    let engines = provides
        .provides
        .iter()
        .map(|e| EngineFacts {
            engine: e.engine.clone(),
            version: e.version.clone(),
            abi_line: e.abi_line.clone(),
            platform: e.platform.as_triplet().to_string(),
        })
        .collect();
    Ok(RuntimeFacts {
        engines,
        src_sha256: Some(provides.built_from.src_sha256.clone()),
        patch_set: Some(provides.built_from.patch_set.clone()),
    })
}

/// The runtime image's gem inventory: `<gemhome>/specifications/` (the
/// bundled class) plus its `default/` child (the interpreter's default
/// gems) — BOTH classes are the shipped inventory, and the class is
/// evidence (`tebako:gem:default`), never a purl facet. Rows:
/// (artifact, licenses, is_default).
struct RuntimeGems {
    rows: Vec<(ArtifactId, Vec<String>, bool)>,
    skipped: Vec<String>,
    bundled: usize,
    default: usize,
    /// The specifications dir the classes were read from (the note's
    /// evidence), when a gem layout was found at all.
    dir: Option<String>,
}

fn runtime_gem_inventory(backend: &dyn tfs::Backend) -> Result<RuntimeGems, TebakoError> {
    let Some(dir) = find_gemspec_dir(backend)? else {
        return Ok(RuntimeGems {
            rows: Vec::new(),
            skipped: Vec::new(),
            bundled: 0,
            default: 0,
            dir: None,
        });
    };
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    let bundled_inv = gemspec_dir_inventory(backend, &dir)?;
    let bundled = bundled_inv.rows.len();
    rows.extend(
        bundled_inv
            .rows
            .into_iter()
            .map(|(id, licenses)| (id, licenses, false)),
    );
    skipped.extend(bundled_inv.skipped);
    let mut default = 0;
    let default_dir = format!("{dir}/default");
    if let Ok(st) = backend.stat(&default_dir) {
        if st.entry_type == tfs::EntryType::Directory {
            let default_inv = gemspec_dir_inventory(backend, &default_dir)?;
            default = default_inv.rows.len();
            rows.extend(
                default_inv
                    .rows
                    .into_iter()
                    .map(|(id, licenses)| (id, licenses, true)),
            );
            skipped.extend(default_inv.skipped);
        }
    }
    rows.sort_by(|a, b| a.0.name.cmp(&b.0.name).then(a.0.version.cmp(&b.0.version)));
    rows.dedup_by(|a, b| a.0.name == b.0.name && a.0.version == b.0.version && a.2 == b.2);
    Ok(RuntimeGems {
        rows,
        skipped,
        bundled,
        default,
        dir: Some(dir),
    })
}

// ---------------------------------------------------------------------
// The store form: name[@version] resolves through the payload cache
// ---------------------------------------------------------------------

/// Resolve `name[@version]` against the machine store (read-only): the
/// exact version, else the name's newest installed. A miss is the named
/// error, never a guess.
struct StoreResolution {
    path: PathBuf,
    name: String,
    version: String,
    origin: Option<String>,
    registry: Option<String>,
}

fn resolve_store(home: &Path, target: &str) -> Result<StoreResolution, TebakoError> {
    let (name, version) = match target.split_once('@') {
        Some((n, v)) => (n.to_string(), Some(v.to_string())),
        None => (target.to_string(), None),
    };
    if name.is_empty() || version.as_deref() == Some("") {
        return Err(err(EX_USAGE, format!("invalid target '{target}'\n{USAGE}")));
    }
    let cache = tebako_resolve::PayloadCache::with_root(home);
    let version = match version {
        Some(version) => version,
        None => {
            let versions: Vec<String> = cache
                .list()
                .into_iter()
                .filter(|e| e.name == name)
                .map(|e| e.version)
                .collect();
            tpkg::versions::newest(versions.iter()).ok_or_else(|| {
                err(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "no installed payload named '{name}' under {} — `tebako info payloads` lists the cache",
                        home.display()
                    ),
                )
            })?
        }
    };
    let entry = cache
        .get(&name, &version)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot read the payload store: {e}")))?
        .ok_or_else(|| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "payload {name}@{version} is not installed under {} — `tebako info payloads` lists the cache",
                    home.display()
                ),
            )
        })?;
    Ok(StoreResolution {
        path: entry.path,
        name,
        version,
        origin: entry.origin,
        registry: entry.registry,
    })
}

// ---------------------------------------------------------------------
// The verb
// ---------------------------------------------------------------------

/// The verb's parsed options.
#[derive(Debug)]
pub struct SbomOptions {
    /// The payload image path (or store `name[@version]`), the runtime
    /// env image path (`--runtime`), or the runtime exe path
    /// (`--runtime-exe`).
    pub target: String,
    pub closure: Option<PathBuf>,
    pub ecosystem: Option<String>,
    pub triplet: Option<String>,
    pub out: Option<PathBuf>,
    pub random_serial: bool,
    /// The spec 15 §8a runtime env image mode: the target is the image,
    /// the inventory is the declared interpreter build + the image's own
    /// gem specifications (bundled + default).
    pub runtime: bool,
    /// The spec 15 §8a runtime exe mode: the target is the interpreter
    /// exe, the inventory is the build-time crate capture.
    pub runtime_exe: bool,
    /// `--crates <crate-inventory.json>` — the build-time capture the
    /// exe mode re-subjects (required there, forbidden elsewhere).
    pub crates: Option<PathBuf>,
    /// The exe subject's name (default: the exe's file stem).
    pub name: Option<String>,
    /// The exe subject's version — required in the exe mode (the bytes
    /// carry no recoverable version).
    pub version: Option<String>,
    /// The runtime's engine (evidence property in the exe mode).
    pub engine: Option<String>,
    /// The engine's version (evidence property in the exe mode).
    pub engine_version: Option<String>,
}

/// The emission's outcome (counts for the progress line).
#[derive(Debug)]
pub struct SbomOutcome {
    pub components: usize,
    pub complete: bool,
    pub output: Option<PathBuf>,
}

/// The ecosystem the inventory maps under: the explicit flag first,
/// then the manifest's language engine, then generic (documented, never
/// a guess).
fn decide_ecosystem(flag: Option<&str>, payload: &PayloadId) -> Result<Ecosystem, TebakoError> {
    if let Some(value) = flag {
        return Ecosystem::parse(value);
    }
    match payload.language_engine.as_deref() {
        Some("ruby") => Ok(Ecosystem::Gem),
        Some("python") => Ok(Ecosystem::Pypi),
        Some("node") => Ok(Ecosystem::Npm),
        Some(other) => {
            eprintln!(
                "sbom: note: the payload's language engine '{other}' has no purl mapping — inventory rows map pkg:generic (override with --ecosystem)"
            );
            Ok(Ecosystem::Generic)
        }
        None => {
            eprintln!(
                "sbom: note: no --ecosystem and no language engine in the manifest — inventory rows map pkg:generic (gems? pass --ecosystem gem)"
            );
            Ok(Ecosystem::Generic)
        }
    }
}

/// The emission's shared tail: emit the document through the one
/// CycloneDX owner, write/print it, and report the counts.
fn finish(
    document: Document,
    out: &Option<PathBuf>,
    complete: bool,
    source: &str,
) -> Result<SbomOutcome, TebakoError> {
    let json = document.emit().map_err(sbom_err)?;
    let output = match out {
        Some(path) => {
            std::fs::write(path, &json).map_err(|e| {
                err(
                    EX_TEBAKO_IO,
                    format!("cannot write {}: {e}", path.display()),
                )
            })?;
            Some(path.clone())
        }
        None => {
            print!("{json}");
            None
        }
    };
    let count = document.components.len();
    eprintln!(
        "sbom: {} component(s), inventory {} ({}){}",
        count,
        if complete { "complete" } else { "degraded" },
        source,
        output
            .as_ref()
            .map(|p| format!(" -> {}", p.display()))
            .unwrap_or_default()
    );
    Ok(SbomOutcome {
        components: count,
        complete,
        output,
    })
}

/// Build and emit the document. `tebako_home` is the store root the
/// `name[@version]` form resolves against (the file form never touches
/// the store — and the runtime modes take paths only).
pub fn sbom(opts: &SbomOptions, tebako_home: &Path) -> Result<SbomOutcome, TebakoError> {
    if opts.runtime_exe {
        return exe_document(opts);
    }
    if opts.runtime {
        return runtime_document(opts);
    }
    payload_document(opts, tebako_home)
}

/// The payload lane (spec 15 §8): one payload image — complete mode from
/// the feedstock closure, degraded mode from the image's own gemspecs.
fn payload_document(opts: &SbomOptions, tebako_home: &Path) -> Result<SbomOutcome, TebakoError> {
    // -- the payload image: a file path, or the store's name[@version]
    let target_path = PathBuf::from(&opts.target);
    let (image_path, store_label, origin, registry) = if target_path.is_file() {
        (target_path, None, None, None)
    } else {
        let resolved = resolve_store(tebako_home, &opts.target)?;
        (
            resolved.path,
            Some(format!("{}@{}", resolved.name, resolved.version)),
            resolved.origin,
            resolved.registry,
        )
    };
    if let Some(label) = &store_label {
        eprintln!("sbom: resolved {label} -> {}", image_path.display());
    }
    let image_sha256 = crate::resolve::sha256_file_hex(&image_path).ok_or_else(|| {
        err(
            EX_TEBAKO_IO,
            format!("cannot hash {}", image_path.display()),
        )
    })?;

    // -- the in-image reads: the manifest (authoritative identity)
    let view = mount_image(&image_path)?;
    if let Some(note) = &view.manifest_note {
        eprintln!(
            "sbom: note: {} — the payload identity falls back to the filename",
            note
        );
    }
    let payload = payload_identity(&image_path, &view, image_sha256, origin, registry)?;
    if payload.kind == Some(tpkg::PayloadKind::Runtime) {
        eprintln!(
            "sbom: note: the image is a runtime env image (kind: runtime) — `tebako sbom --runtime <image>` emits the runtime-artifact document (the declared interpreter build + the full gem inventory including the default-gem class); the payload lane stays on its degraded inventory below"
        );
    }

    // -- the ecosystem decision ONCE (closure mode: flag → manifest
    //    engine → generic-with-note; degraded mode is gem by
    //    construction — specifications/ is a rubygems layout)
    let ecosystem = if opts.closure.is_some() {
        decide_ecosystem(opts.ecosystem.as_deref(), &payload)?
    } else {
        Ecosystem::Gem
    };

    // -- the inventory
    let (artifacts, complete, source, skipped) = match &opts.closure {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| {
                err(
                    EX_TEBAKO_IO,
                    format!("cannot read the closure {}: {e}", path.display()),
                )
            })?;
            let rows = parse_closure(&text)?;
            let mut out = Vec::with_capacity(rows.len());
            for row in &rows {
                let id = parse_artifact(&row.artifact, ecosystem)?;
                out.push((id, Some(row.sha256.clone()), Vec::new()));
            }
            (out, true, "closure", Vec::new())
        }
        None => {
            let GemspecInventory { rows, skipped } = gemspec_inventory(&*view.backend)?;
            let out = rows
                .into_iter()
                .map(|(id, licenses)| (id, None, licenses))
                .collect();
            (out, false, "image-gemspecs", skipped)
        }
    };

    // -- the components (dedup exact rows; purl collisions get distinct
    //    bom-refs downstream — the schema's uniqueness law)
    let mut components: Vec<Component> = artifacts
        .into_iter()
        .map(|(id, sha256, licenses)| {
            let mut properties = Vec::new();
            if let Some(q) = &id.qualifier {
                let key = match ecosystem {
                    Ecosystem::Gem => "tebako:gem:platform",
                    Ecosystem::Pypi => "tebako:pypi:tags",
                    _ => "tebako:artifact:suffix",
                };
                properties.push((key.to_string(), q.clone()));
            }
            let mut c = Component::new(
                ecosystem.component_type(),
                &id.name,
                &id.version,
                &purl_of(ecosystem, &id),
            );
            c.sha256 = sha256;
            c.license_expression = (!licenses.is_empty()).then(|| licenses.join(" AND "));
            c.properties = properties;
            c
        })
        .collect();
    components.sort_by(|a, b| a.purl.cmp(&b.purl).then(a.sha256.cmp(&b.sha256)));
    components.dedup_by(|a, b| {
        a.purl == b.purl && a.sha256 == b.sha256 && a.license_expression == b.license_expression
    });
    if components.is_empty() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            "the inventory is empty — nothing to describe",
        ));
    }

    let triplet = opts
        .triplet
        .clone()
        .or_else(|| payload.triplet.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let payload_purl = format!(
        "pkg:generic/{}@{}",
        purl_encode(&payload.name),
        purl_encode(&payload.version)
    );

    // -- the document
    let mut subject = Component::new(
        payload_component_type(payload.kind),
        &payload.name,
        &payload.version,
        &payload_purl,
    );
    subject.sha256 = Some(payload.image_sha256.clone());
    subject.properties = {
        let mut p = vec![
            (
                "tebako:payload:kind".to_string(),
                payload_kind_label(payload.kind).to_string(),
            ),
            ("tebako:triplet".to_string(), triplet.clone()),
        ];
        if let Some(engine) = &payload.language_engine {
            p.push(("tebako:payload:engine".to_string(), engine.clone()));
        }
        if let Some(signing) = payload.signing {
            p.push((
                "tebako:payload:signing".to_string(),
                signing_label(signing).to_string(),
            ));
        }
        if let Some(registry) = &payload.registry {
            p.push(("tebako:payload:registry".to_string(), registry.clone()));
        }
        p
    };
    if let Some(origin) = &payload.origin {
        subject
            .external_references
            .push(("distribution".to_string(), origin.clone()));
    }

    let annotations = if complete {
        Vec::new()
    } else {
        let mut text = "Inventory derived from the image's own gem specifications (specifications/*.gemspec): \
             name+version+licenses only — no per-artifact hashes, completeness not claimed (tebako:closure:complete=false). \
             Pass --closure <feedstock closure> for the complete, hashed inventory.".to_string();
        if !skipped.is_empty() {
            text.push_str(&format!(
                " Unparseable specs skipped: {}.",
                skipped.join(", ")
            ));
        }
        vec![Annotation {
            subjects: vec![payload_purl.clone()],
            text,
        }]
    };

    let document = Document {
        metadata_component: subject,
        metadata_properties: vec![
            (
                "tebako:closure:complete".to_string(),
                if complete { "true" } else { "false" }.to_string(),
            ),
            (
                "tebako:sbom:inventory-source".to_string(),
                source.to_string(),
            ),
        ],
        tool: ("tebako".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        components,
        graph: DependencyGraph::Flat,
        annotations,
        serial_mode: if opts.random_serial {
            SerialMode::Random
        } else {
            SerialMode::ContentDerived
        },
        epoch_override: None,
    };
    finish(document, &opts.out, complete, source)
}

// ---------------------------------------------------------------------
// The runtime env image document (spec 15 §8a, --runtime)
// ---------------------------------------------------------------------

/// The runtime env image document: the declared interpreter build plus
/// the image's own gem specifications (bundled + default). Never
/// "complete" in the closure sense — no hashed closure exists for a
/// runtime image — so the annotation names what the inventory honestly
/// is, and `tebako:closure:complete` stays false by construction.
fn runtime_document(opts: &SbomOptions) -> Result<SbomOutcome, TebakoError> {
    let image_path = PathBuf::from(&opts.target);
    if !image_path.is_file() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "--runtime names the runtime env image (a .tfs file); '{}' is not a file — the runtime modes take paths, never store references",
                opts.target
            ),
        ));
    }
    let image_sha256 = crate::resolve::sha256_file_hex(&image_path).ok_or_else(|| {
        err(
            EX_TEBAKO_IO,
            format!("cannot hash {}", image_path.display()),
        )
    })?;
    let view = mount_image(&image_path)?;
    let Some(manifest) = &view.manifest else {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "{} carries no readable /__tpkg__/manifest.yaml — a runtime env image declares its interpreter in the manifest; the payload lane covers plain images",
                image_path.display()
            ),
        ));
    };
    let facts = runtime_facts(manifest)?;
    let gems = runtime_gem_inventory(&*view.backend)?;

    // -- the components: one interpreter build per declared engine
    //    entry, then the gem rows with their class evidence
    let mut components: Vec<Component> = Vec::new();
    for engine in &facts.engines {
        let mut c = Component::new(
            ComponentType::Platform,
            &engine.engine,
            &engine.version,
            &format!(
                "pkg:generic/{}@{}",
                purl_encode(&engine.engine),
                purl_encode(&engine.version)
            ),
        );
        c.properties = vec![
            (
                "tebako:runtime:abi_line".to_string(),
                engine.abi_line.clone(),
            ),
            (
                "tebako:runtime:platform".to_string(),
                engine.platform.clone(),
            ),
        ];
        if let Some(src) = &facts.src_sha256 {
            c.properties
                .push(("tebako:runtime:src_sha256".to_string(), src.clone()));
        }
        if let Some(patch) = &facts.patch_set {
            c.properties
                .push(("tebako:runtime:patch_set".to_string(), patch.clone()));
        }
        components.push(c);
    }
    for (id, licenses, is_default) in gems.rows {
        let mut c = Component::new(
            ComponentType::Library,
            &id.name,
            &id.version,
            &purl_of(Ecosystem::Gem, &id),
        );
        c.license_expression = (!licenses.is_empty()).then(|| licenses.join(" AND "));
        c.properties = vec![(
            "tebako:gem:default".to_string(),
            if is_default { "true" } else { "false" }.to_string(),
        )];
        components.push(c);
    }
    components.sort_by(|a, b| a.purl.cmp(&b.purl).then(a.sha256.cmp(&b.sha256)));
    // Two engine entries that differ only by platform are one
    // interpreter — dedup the identical rows; distinct platforms keep
    // their own entries (the property order is deterministic by
    // construction).
    components.dedup_by(|a, b| a.purl == b.purl && a.properties == b.properties);

    // -- the subject: the env image itself
    let platforms: Vec<&str> = facts
        .engines
        .iter()
        .map(|e| e.platform.as_str())
        .collect::<Vec<_>>();
    let mut platforms_dedup = platforms.clone();
    platforms_dedup.dedup();
    let triplet = opts
        .triplet
        .clone()
        .unwrap_or_else(|| platforms_dedup.join(","));
    let payload_purl = format!(
        "pkg:generic/{}@{}",
        purl_encode(&manifest.identity.name),
        purl_encode(&manifest.identity.version)
    );
    let mut subject = Component::new(
        ComponentType::Platform,
        &manifest.identity.name,
        &manifest.identity.version,
        &payload_purl,
    );
    subject.sha256 = Some(image_sha256);
    subject.properties = vec![
        ("tebako:payload:kind".to_string(), "runtime".to_string()),
        ("tebako:triplet".to_string(), triplet),
        (
            "tebako:runtime:engine".to_string(),
            facts.engines[0].engine.clone(),
        ),
        (
            "tebako:runtime:engine_version".to_string(),
            facts.engines[0].version.clone(),
        ),
    ];

    // -- the annotation: what the inventory honestly IS
    let engine_desc = facts
        .engines
        .iter()
        .map(|e| format!("{} {}", e.engine, e.version))
        .collect::<Vec<_>>()
        .join(" + ");
    let build_facts = [
        facts
            .src_sha256
            .as_ref()
            .map(|s| format!("source sha256 {s}")),
        facts.patch_set.as_ref().map(|p| format!("patch set {p}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let build_desc = if build_facts.is_empty() {
        "no source pin declared".to_string()
    } else {
        build_facts
    };
    let gem_clause = match &gems.dir {
        Some(dir) => format!(
            "the image's own gem specifications under {dir} ({} bundled + {} default) — name+version+licenses only, no per-artifact hashes",
            gems.bundled, gems.default
        ),
        None => "no gem specifications (the image carries no gem layout)".to_string(),
    };
    let mut text = format!(
        "Runtime env image inventory: the manifest-declared interpreter build ({engine_desc}; {build_desc}) plus {gem_clause}. \
         No hashed closure exists for a runtime image, so completeness is not claimed in the closure sense (tebako:closure:complete=false); \
         the stdlib tree is the interpreter's own versioned content, represented by the interpreter component. \
         The interpreter's native objects and the runtime exe's vendored Rust crates are the sibling exe SBOM's inventory \
         (`tebako sbom --runtime-exe`, captured at build time)."
    );
    if !gems.skipped.is_empty() {
        text.push_str(&format!(
            " Unparseable specs skipped: {}.",
            gems.skipped.join(", ")
        ));
    }

    let document = Document {
        metadata_component: subject,
        metadata_properties: vec![
            (
                "tebako:sbom:inventory-source".to_string(),
                "runtime-image".to_string(),
            ),
            ("tebako:closure:complete".to_string(), "false".to_string()),
        ],
        tool: ("tebako".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        components,
        graph: DependencyGraph::Flat,
        annotations: vec![Annotation {
            subjects: vec![payload_purl],
            text,
        }],
        serial_mode: if opts.random_serial {
            SerialMode::Random
        } else {
            SerialMode::ContentDerived
        },
        epoch_override: None,
    };
    let count = document.components.len();
    let outcome = finish(document, &opts.out, false, "runtime-image")?;
    debug_assert_eq!(count, outcome.components);
    Ok(outcome)
}

// ---------------------------------------------------------------------
// The runtime exe document (spec 15 §8a, --runtime-exe --crates)
// ---------------------------------------------------------------------

/// The runtime exe document: the interpreter exe as the subject, the
/// build-time crate capture as the inventory. The crate set is complete
/// by construction — the capture IS the build's full normal+build
/// resolve closure — so this document carries
/// `tebako:closure:complete=true` with no annotation.
fn exe_document(opts: &SbomOptions) -> Result<SbomOutcome, TebakoError> {
    let exe_path = PathBuf::from(&opts.target);
    if !exe_path.is_file() {
        return Err(err(
            EX_TEBAKO_MANIFEST,
            format!(
                "--runtime-exe names the runtime interpreter exe; '{}' is not a file",
                opts.target
            ),
        ));
    }
    let crates = opts
        .crates
        .as_ref()
        .expect("run() validates --crates in the exe mode");
    let capture_text = std::fs::read_to_string(crates).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot read the crate capture {}: {e}", crates.display()),
        )
    })?;
    let capture = tebako_sbom::capture::CrateCapture::parse(&capture_text)
        .map_err(|e| err(e.exit_code(), e.to_string()))?;
    let exe_sha256 = crate::resolve::sha256_file_hex(&exe_path)
        .ok_or_else(|| err(EX_TEBAKO_IO, format!("cannot hash {}", exe_path.display())))?;

    // -- the subject identity: the bytes carry no recoverable version,
    //    so --version is required; the name defaults to the exe's file
    //    name minus a windows .exe suffix (a suffix strip, never the
    //    asset-name grammar — `file_stem` would eat the dotted version
    //    inside `tebako-runtime-<ver>-<lang>-<lv>-<triplet>`).
    let file_name = exe_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| {
            err(
                EX_USAGE,
                format!("cannot derive a name from {}", exe_path.display()),
            )
        })?;
    let stem = file_name
        .strip_suffix(".exe")
        .map(str::to_string)
        .unwrap_or(file_name);
    let name = opts.name.clone().unwrap_or(stem);
    let version = opts
        .version
        .clone()
        .expect("run() validates --version in the exe mode");
    let subject_purl = format!(
        "pkg:generic/{}@{}",
        purl_encode(&name),
        purl_encode(&version)
    );
    let mut subject = Component::new(ComponentType::Platform, &name, &version, &subject_purl);
    subject.sha256 = Some(exe_sha256);
    subject.properties = {
        let mut p = vec![
            ("tebako:payload:kind".to_string(), "runtime".to_string()),
            (
                "tebako:capture:package".to_string(),
                capture.package.clone(),
            ),
            ("tebako:target".to_string(), capture.target.clone()),
        ];
        if let Some(triplet) = &opts.triplet {
            p.push(("tebako:platform".to_string(), triplet.clone()));
        }
        if let Some(engine) = &opts.engine {
            p.push(("tebako:runtime:engine".to_string(), engine.clone()));
        }
        if let Some(engine_version) = &opts.engine_version {
            p.push((
                "tebako:runtime:engine_version".to_string(),
                engine_version.clone(),
            ));
        }
        p
    };

    // -- the components + graph from the capture
    let mut components: Vec<Component> = Vec::with_capacity(capture.components.len());
    for c in &capture.components {
        let mut component = Component::new(ComponentType::Library, &c.name, &c.version, &c.purl);
        component.sha256 = c.sha256.clone();
        component.license_expression = c.license.clone();
        component.properties = tebako_sbom::capture::source_properties(&c.source)
            .map_err(|e| err(e.exit_code(), e.to_string()))?;
        components.push(component);
    }
    let mut edges: Vec<Dependency> = capture
        .edges
        .iter()
        .map(|e| Dependency {
            refr: e.refr.clone(),
            depends_on: e.depends_on.clone(),
        })
        .collect();
    edges.push(Dependency {
        refr: subject_purl.clone(),
        depends_on: vec![capture.root.clone()],
    });

    let document = Document {
        metadata_component: subject,
        metadata_properties: vec![
            (
                "tebako:sbom:inventory-source".to_string(),
                "cargo-resolution-capture".to_string(),
            ),
            ("tebako:closure:complete".to_string(), "true".to_string()),
            ("tebako:capture:tool".to_string(), capture.tool.0.clone()),
            (
                "tebako:capture:tool_version".to_string(),
                capture.tool.1.clone(),
            ),
        ],
        tool: ("tebako".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        components,
        graph: DependencyGraph::Explicit(edges),
        annotations: Vec::new(),
        serial_mode: if opts.random_serial {
            SerialMode::Random
        } else {
            SerialMode::ContentDerived
        },
        epoch_override: None,
    };
    finish(document, &opts.out, true, "cargo-capture")
}

/// `tebako sbom <target> [flags]` — the argv surface.
pub fn run(args: &[String]) -> Result<(), TebakoError> {
    let mut target: Option<String> = None;
    let mut closure: Option<PathBuf> = None;
    let mut ecosystem: Option<String> = None;
    let mut triplet: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut random_serial = false;
    let mut runtime = false;
    let mut runtime_exe = false;
    let mut crates: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut engine: Option<String> = None;
    let mut engine_version: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--closure" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--closure needs a path\n{USAGE}")))?;
                closure = Some(PathBuf::from(value));
                i += 2;
            }
            "--ecosystem" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--ecosystem needs a value\n{USAGE}")))?;
                ecosystem = Some(value.clone());
                i += 2;
            }
            "--triplet" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--triplet needs a value\n{USAGE}")))?;
                triplet = Some(value.clone());
                i += 2;
            }
            "--out" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--out needs a path\n{USAGE}")))?;
                out = Some(PathBuf::from(value));
                i += 2;
            }
            "--random-serial" => {
                random_serial = true;
                i += 1;
            }
            "--runtime" => {
                runtime = true;
                i += 1;
            }
            "--runtime-exe" => {
                runtime_exe = true;
                i += 1;
            }
            "--crates" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--crates needs a path\n{USAGE}")))?;
                crates = Some(PathBuf::from(value));
                i += 2;
            }
            "--name" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--name needs a value\n{USAGE}")))?;
                name = Some(value.clone());
                i += 2;
            }
            "--version" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--version needs a value\n{USAGE}")))?;
                version = Some(value.clone());
                i += 2;
            }
            "--engine" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| err(EX_USAGE, format!("--engine needs a value\n{USAGE}")))?;
                engine = Some(value.clone());
                i += 2;
            }
            "--engine-version" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    err(EX_USAGE, format!("--engine-version needs a value\n{USAGE}"))
                })?;
                engine_version = Some(value.clone());
                i += 2;
            }
            flag if flag.starts_with('-') => {
                return Err(err(EX_USAGE, format!("unknown option '{flag}'\n{USAGE}")));
            }
            value => {
                if target.is_some() {
                    return Err(err(
                        EX_USAGE,
                        format!("tebako sbom takes exactly one target\n{USAGE}"),
                    ));
                }
                target = Some(value.to_string());
                i += 1;
            }
        }
    }
    let Some(target) = target else {
        return Err(err(EX_USAGE, USAGE));
    };
    // -- the mode grammar (spec 15 §8a): the modes are MECE, their
    //    flags never mix, and the exe mode's required identity is
    //    explicit (the exe's bytes carry no recoverable version).
    if runtime && runtime_exe {
        return Err(err(
            EX_USAGE,
            format!("--runtime and --runtime-exe are mutually exclusive\n{USAGE}"),
        ));
    }
    if runtime_exe {
        let mut missing = Vec::new();
        if crates.is_none() {
            missing.push("--crates <crate-inventory.json>");
        }
        if version.is_none() {
            missing.push("--version <ver>");
        }
        if !missing.is_empty() {
            return Err(err(
                EX_USAGE,
                format!(
                    "--runtime-exe requires {} (the exe's bytes carry no recoverable version; the crate set is the build-time capture)\n{USAGE}",
                    missing.join(" and ")
                ),
            ));
        }
    }
    if crates.is_some() && !runtime_exe {
        return Err(err(
            EX_USAGE,
            format!("--crates belongs to --runtime-exe\n{USAGE}"),
        ));
    }
    if (name.is_some() || engine.is_some() || engine_version.is_some()) && !runtime_exe {
        return Err(err(
            EX_USAGE,
            format!("--name/--engine/--engine-version belong to --runtime-exe\n{USAGE}"),
        ));
    }
    if closure.is_some() && (runtime || runtime_exe) {
        return Err(err(
            EX_USAGE,
            format!("--closure belongs to the payload lane\n{USAGE}"),
        ));
    }
    if ecosystem.is_some() && (runtime || runtime_exe) {
        return Err(err(
            EX_USAGE,
            format!("--ecosystem belongs to the payload lane\n{USAGE}"),
        ));
    }
    let env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    let home = tebako_shim::tebako_home(&env).map_err(|e| {
        err(
            EX_TEBAKO_IO,
            format!("cannot resolve $TEBAKO_HOME: {}", e.message),
        )
    })?;
    sbom(
        &SbomOptions {
            target,
            closure,
            ecosystem,
            triplet,
            out,
            random_serial,
            runtime,
            runtime_exe,
            crates,
            name,
            version,
            engine,
            engine_version,
        },
        &home,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_versioned_opens_the_version_at_the_first_digit_segment() {
        let (name, version, qualifier) = split_versioned("activesupport-8.1.4.1").unwrap();
        assert_eq!(
            (name.as_str(), version.as_str(), qualifier),
            ("activesupport", "8.1.4.1", None)
        );
        let (name, version, qualifier) = split_versioned("nokogiri-1.18.9-arm64-darwin").unwrap();
        assert_eq!(
            (name.as_str(), version.as_str(), qualifier.as_deref()),
            ("nokogiri", "1.18.9", Some("arm64-darwin"))
        );
        // A name with dashes and no version segment is the named error.
        assert!(split_versioned("no-version-here").is_err());
        // A leading version segment means an empty name.
        assert!(split_versioned("1.2.3").is_err());
    }

    #[test]
    fn closure_rows_parse_both_spellings_by_hash_shape() {
        let gem_shape =
            "rack-3.1.0 1111111111111111111111111111111111111111111111111111111111111111";
        let sum_shape =
            "2222222222222222222222222222222222222222222222222222222222222222  rake-13.2.1";
        let rows = parse_closure(&format!("# comment\n\n{gem_shape}\n{sum_shape}\n")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].artifact, "rack-3.1.0");
        assert_eq!(
            rows[0].sha256,
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(rows[1].artifact, "rake-13.2.1");
        // Uppercase hex normalizes to lowercase.
        let rows = parse_closure(
            "AA11111111111111111111111111111111111111111111111111111111111111  rack-3.1.0",
        )
        .unwrap();
        assert_eq!(
            rows[0].sha256,
            "aa11111111111111111111111111111111111111111111111111111111111111"
        );
        // A row with no hash-shaped token refuses named.
        assert!(parse_closure("rack-3.1.0 rack-3.1.0").is_err());
        // An empty closure refuses named.
        assert!(parse_closure("# nothing\n").is_err());
    }

    #[test]
    fn pypi_wheel_names_normalize_and_carry_tags_as_qualifiers() {
        let id = parse_artifact(
            "charset_normalizer-3.4.0-cp310-cp310-macosx_11_0_arm64.whl",
            Ecosystem::Pypi,
        )
        .unwrap();
        assert_eq!(id.name, "charset-normalizer");
        assert_eq!(id.version, "3.4.0");
        assert_eq!(
            id.qualifier.as_deref(),
            Some("cp310-cp310-macosx_11_0_arm64")
        );
        assert_eq!(
            purl_of(Ecosystem::Pypi, &id),
            "pkg:pypi/charset-normalizer@3.4.0"
        );
        // The bare form parses too.
        let id = parse_artifact("idna-3.10", Ecosystem::Pypi).unwrap();
        assert_eq!((id.name.as_str(), id.version.as_str()), ("idna", "3.10"));
        // A non-version second segment refuses named.
        assert!(parse_artifact("idna-latest", Ecosystem::Pypi).is_err());
    }

    #[test]
    fn npm_scoped_names_encode_the_scope_marker() {
        let id = parse_artifact("@scope/pkg-1.0.0", Ecosystem::Npm).unwrap();
        assert_eq!(id.name, "@scope/pkg");
        assert_eq!(purl_of(Ecosystem::Npm, &id), "pkg:npm/%40scope/pkg@1.0.0");
    }

    #[test]
    fn gemspecs_parse_name_version_and_licenses() {
        let spec = r#"Gem::Specification.new do |s|
  s.name = "rack"
  s.version = "3.1.0"
  s.licenses = ["MIT", "Apache-2.0"].freeze
end
"#;
        let row = parse_gemspec(spec).unwrap();
        assert_eq!(row.name, "rack");
        assert_eq!(row.version, "3.1.0");
        assert_eq!(
            row.licenses,
            vec!["MIT".to_string(), "Apache-2.0".to_string()]
        );
        // The installed-specification rendering: rubygems freezes each
        // scalar and each array ELEMENT individually.
        let installed = r#"Gem::Specification.new do |s|
  s.name = "rubyzip".freeze
  s.version = "3.4.1".freeze
  s.licenses = ["BSD-2-Clause".freeze]
end
"#;
        let row = parse_gemspec(installed).unwrap();
        assert_eq!(row.name, "rubyzip");
        assert_eq!(row.version, "3.4.1");
        assert_eq!(row.licenses, vec!["BSD-2-Clause".to_string()]);
        // The singular form parses too.
        let row = parse_gemspec("s.name = 'x'\ns.version = '1'\ns.license = 'MIT'\n").unwrap();
        assert_eq!(row.licenses, vec!["MIT".to_string()]);
        // A spec missing the version is no inventory row.
        assert!(parse_gemspec("s.name = 'x'\n").is_none());
    }

    #[test]
    fn string_array_parsing_skips_non_quoted_tokens() {
        assert_eq!(
            parse_string_array("[\"MIT\", 'Apache-2.0']"),
            vec!["MIT".to_string(), "Apache-2.0".to_string()]
        );
        assert!(parse_string_array("[MIT]").is_empty());
    }

    #[test]
    fn ecosystem_decision_prefers_flag_then_engine_then_generic() {
        let payload = PayloadId {
            name: "p".to_string(),
            version: "1".to_string(),
            kind: None,
            language_engine: Some("ruby".to_string()),
            triplet: None,
            signing: None,
            origin: None,
            registry: None,
            image_sha256: "00".repeat(32),
        };
        assert!(matches!(
            decide_ecosystem(Some("cargo"), &payload),
            Ok(Ecosystem::Cargo)
        ));
        assert!(matches!(
            decide_ecosystem(None, &payload),
            Ok(Ecosystem::Gem)
        ));
        let mut python = payload.clone();
        python.language_engine = Some("python".to_string());
        assert!(matches!(
            decide_ecosystem(None, &python),
            Ok(Ecosystem::Pypi)
        ));
        let mut unknown = payload;
        unknown.language_engine = None;
        assert!(matches!(
            decide_ecosystem(None, &unknown),
            Ok(Ecosystem::Generic)
        ));
    }

    fn runtime_manifest() -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: runtime\n  name: tebako-runtime-ruby\n  version: \"0.0.7\"\n  producer: {{tool: tebako-runtime-ruby, tool_version: \"0.0.7\"}}\n  created: \"2026-11-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  provides:\n    engine: ruby\n    version: \"4.0.9\"\n    abi_line: \"4.0\"\n    platform: aarch64-macos\n  built_from:\n    src_sha256: \"{}\"\n    patch_set: test-chain\n  capabilities: {{exec: true, read: true, runtime: true}}\n",
            "0".repeat(64),
            "0".repeat(64),
            "c".repeat(64)
        )
    }

    #[test]
    fn runtime_facts_read_the_declared_interpreter_build() {
        let manifest = tpkg::PayloadManifest::from_yaml(&runtime_manifest()).unwrap();
        let facts = runtime_facts(&manifest).unwrap();
        assert_eq!(facts.engines.len(), 1);
        let engine = &facts.engines[0];
        assert_eq!(
            (
                engine.engine.as_str(),
                engine.version.as_str(),
                engine.abi_line.as_str(),
                engine.platform.as_str()
            ),
            ("ruby", "4.0.9", "4.0", "aarch64-macos")
        );
        assert_eq!(facts.src_sha256.as_deref(), Some("c".repeat(64).as_str()));
        assert_eq!(facts.patch_set.as_deref(), Some("test-chain"));
    }

    #[test]
    fn runtime_facts_refuse_a_non_runtime_kind_named() {
        let app = format!(
            "identity:\n  schema_version: 1\n  kind: app\n  name: probe\n  version: \"1.0.0\"\n  producer: {{tool: probe, tool_version: \"1\"}}\n  created: \"2026-11-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{}\"\n    blob_sha256: \"{}\"\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: probe\n      path: /bin/probe\n  platforms: universal\n  capabilities: {{exec: true, read: true}}\n",
            "0".repeat(64),
            "0".repeat(64)
        );
        let manifest = tpkg::PayloadManifest::from_yaml(&app).unwrap();
        let error = runtime_facts(&manifest).unwrap_err();
        assert_eq!(error.code, EX_TEBAKO_MANIFEST);
        assert!(error.message.contains("kind: app"));
        assert!(error.message.contains("payload lane"));
    }

    #[test]
    fn mode_grammar_validates_before_any_io() {
        // The modes are mutually exclusive.
        let error = run(&[
            "--runtime".to_string(),
            "--runtime-exe".to_string(),
            "x".to_string(),
        ])
        .unwrap_err();
        assert_eq!(error.code, EX_USAGE);
        assert!(error.message.contains("mutually exclusive"));
        // The exe mode requires the capture AND the version — both
        // named in one refusal (the exe's bytes carry no recoverable
        // version).
        let error = run(&["--runtime-exe".to_string(), "x".to_string()]).unwrap_err();
        assert_eq!(error.code, EX_USAGE);
        assert!(error.message.contains("--crates"));
        assert!(error.message.contains("--version"));
        // --crates without the exe mode.
        let error = run(&[
            "--crates".to_string(),
            "c.json".to_string(),
            "x".to_string(),
        ])
        .unwrap_err();
        assert_eq!(error.code, EX_USAGE);
        assert!(error.message.contains("--runtime-exe"));
        // The payload-lane flags never mix into the runtime modes.
        let error = run(&[
            "--runtime".to_string(),
            "--closure".to_string(),
            "c.txt".to_string(),
            "x".to_string(),
        ])
        .unwrap_err();
        assert_eq!(error.code, EX_USAGE);
        assert!(error.message.contains("payload lane"));
        let error = run(&[
            "--runtime-exe".to_string(),
            "--ecosystem".to_string(),
            "gem".to_string(),
            "--crates".to_string(),
            "c.json".to_string(),
            "--version".to_string(),
            "1".to_string(),
            "x".to_string(),
        ])
        .unwrap_err();
        assert_eq!(error.code, EX_USAGE);
        assert!(error.message.contains("payload lane"));
        // The exe identity flags belong to the exe mode only.
        let error = run(&["--name".to_string(), "n".to_string(), "x".to_string()]).unwrap_err();
        assert_eq!(error.code, EX_USAGE);
    }
}

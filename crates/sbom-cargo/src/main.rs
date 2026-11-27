//! `sbom-cargo` — the spec 13 §5a release-side SBOM generator: emit one
//! released part's CycloneDX 1.6 document from the build's REAL Cargo
//! resolution, captured at build time:
//!
//! - `cargo metadata --format-version 1 --filter-platform <target>` —
//!   the target-filtered resolve set (windows-only crates never appear
//!   in a linux SBOM) plus each crate's declared license;
//! - the build's own `Cargo.lock` — the per-crate registry checksums
//!   (the workspace lockfile floats by design, so the checksum evidence
//!   is captured in CI at build time, never checked in).
//!
//! `metadata.component` is the asset itself with the staged bytes'
//! sha256 in `hashes` — the pipeline generates at the same post-signing
//! point as the sha256 fragments, so sign-then-hash keeps the SBOM's
//! digest on the shipped bytes. The document emits through the
//! tebako-sbom crate (one CycloneDX owner): sorted, content-serialled,
//! `SOURCE_DATE_EPOCH`-pinned — a pipeline re-run on unchanged inputs
//! reproduces byte-identical SBOMs.
//!
//! `--inventory-only` (spec 15 §8a, tebako#1121) is the capture mode:
//! no document, no asset — just the crate-inventory capture
//! (`crate-inventory.json`, the tebako-sbom capture grammar) from the
//! same resolution walk. The link-unit legs run it BEFORE the tar so
//! the capture rides inside the link-unit tarball; the runtime
//! factories re-emit it as the exe's CycloneDX document (`tebako sbom
//! --runtime-exe`) — the exe's vendored crates are exactly what the
//! build resolved, never a post-hoc guess at stripped bytes.
//!
//! Exit codes (the spec 15 grammar): 0 emitted; 64 usage; 65 the
//! metadata/lock inputs are malformed or the named package is not a
//! workspace member; 74 I/O.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;

use tebako_json::Value as Json;
use tebako_sbom::capture::{self, CaptureEdge, CapturedCrate, CrateCapture};
use tebako_sbom::{Component, ComponentType, Dependency, DependencyGraph, Document, SerialMode};

const EX_USAGE: u8 = 64;
const EX_MALFORMED: u8 = 65;
const EX_IO: u8 = 74;

/// The named-error surface: a message plus its exit class. No silent
/// fallbacks (spec 00 invariant 9).
#[derive(Debug)]
struct Fatal {
    code: u8,
    message: String,
}

impl Fatal {
    fn usage(message: impl Into<String>) -> Fatal {
        Fatal {
            code: EX_USAGE,
            message: message.into(),
        }
    }
    fn malformed(message: impl Into<String>) -> Fatal {
        Fatal {
            code: EX_MALFORMED,
            message: message.into(),
        }
    }
    fn io(message: impl Into<String>) -> Fatal {
        Fatal {
            code: EX_IO,
            message: message.into(),
        }
    }
}

const USAGE: &str = "usage: sbom-cargo --package <cargo-pkg> --tool <tool-id> --asset <file> --name <asset-name> --version <ver> --platform <tebako-platform> --target <rust-triple> --metadata <cargo-metadata.json> --lock <Cargo.lock> --out <file> [--release-url <base>]\n       sbom-cargo --inventory-only --package <cargo-pkg> --target <rust-triple> --metadata <cargo-metadata.json> --lock <Cargo.lock> --out <crate-inventory.json>";

/// The verb's parsed options. The document mode's fields are options so
/// the capture mode can forbid them by name (a capture run naming an
/// asset is a usage error, not a silently ignored flag).
#[derive(Debug)]
struct Options {
    package: String,
    target: String,
    metadata: PathBuf,
    lock: PathBuf,
    out: PathBuf,
    inventory_only: bool,
    tool: Option<String>,
    asset: Option<PathBuf>,
    name: Option<String>,
    version: Option<String>,
    platform: Option<String>,
    release_url: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Options, Fatal> {
    let mut package = None;
    let mut tool = None;
    let mut asset = None;
    let mut name = None;
    let mut version = None;
    let mut platform = None;
    let mut target = None;
    let mut metadata = None;
    let mut lock = None;
    let mut out = None;
    let mut release_url = None;
    let mut inventory_only = false;
    let mut i = 0;
    let value_of = |flag: &str, i: &mut usize| -> Result<String, Fatal> {
        let value = args
            .get(*i + 1)
            .ok_or_else(|| Fatal::usage(format!("{flag} needs a value\n{USAGE}")))?;
        *i += 2;
        Ok(value.clone())
    };
    while i < args.len() {
        let flag = args[i].as_str();
        match flag {
            "--inventory-only" => {
                inventory_only = true;
                i += 1;
            }
            "--package" => package = Some(value_of(flag, &mut i)?),
            "--tool" => tool = Some(value_of(flag, &mut i)?),
            "--asset" => asset = Some(PathBuf::from(value_of(flag, &mut i)?)),
            "--name" => name = Some(value_of(flag, &mut i)?),
            "--version" => version = Some(value_of(flag, &mut i)?),
            "--platform" => platform = Some(value_of(flag, &mut i)?),
            "--target" => target = Some(value_of(flag, &mut i)?),
            "--metadata" => metadata = Some(PathBuf::from(value_of(flag, &mut i)?)),
            "--lock" => lock = Some(PathBuf::from(value_of(flag, &mut i)?)),
            "--out" => out = Some(PathBuf::from(value_of(flag, &mut i)?)),
            "--release-url" => release_url = Some(value_of(flag, &mut i)?),
            other => {
                return Err(Fatal::usage(format!("unknown option '{other}'\n{USAGE}")));
            }
        }
    }
    let missing_common = [
        ("--package", package.is_none()),
        ("--target", target.is_none()),
        ("--metadata", metadata.is_none()),
        ("--lock", lock.is_none()),
        ("--out", out.is_none()),
    ]
    .into_iter()
    .filter(|(_, absent)| *absent)
    .map(|(f, _)| f)
    .collect::<Vec<_>>();
    if !missing_common.is_empty() {
        return Err(Fatal::usage(format!(
            "missing required option(s): {}\n{USAGE}",
            missing_common.join(", ")
        )));
    }
    let document_flags = [
        ("--tool", tool.is_some()),
        ("--asset", asset.is_some()),
        ("--name", name.is_some()),
        ("--version", version.is_some()),
        ("--platform", platform.is_some()),
        ("--release-url", release_url.is_some()),
    ];
    if inventory_only {
        let forbidden = document_flags
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(f, _)| f)
            .collect::<Vec<_>>();
        if !forbidden.is_empty() {
            return Err(Fatal::usage(format!(
                "--inventory-only takes no document flag(s): {}\n{USAGE}",
                forbidden.join(", ")
            )));
        }
    } else {
        let missing = document_flags
            .into_iter()
            .filter(|(f, present)| !present && *f != "--release-url")
            .map(|(f, _)| f)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(Fatal::usage(format!(
                "missing required option(s): {}\n{USAGE}",
                missing.join(", ")
            )));
        }
    }
    Ok(Options {
        package: package.unwrap(),
        target: target.unwrap(),
        metadata: metadata.unwrap(),
        lock: lock.unwrap(),
        out: out.unwrap(),
        inventory_only,
        tool,
        asset,
        name,
        version,
        platform,
        release_url,
    })
}

// ---------------------------------------------------------------------
// The cargo resolution (metadata JSON + the build's Cargo.lock)
// ---------------------------------------------------------------------

/// One crate in the target-filtered resolve set.
#[derive(Debug, Clone)]
struct CrateRow {
    /// The cargo metadata package id (opaque; the graph's node key).
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    /// `registry+…` / `git+…` / `path+…`-flavored source, or null
    /// (workspace member / path dep).
    source: Option<String>,
    /// The Cargo.lock checksum (registry crates only).
    checksum: Option<String>,
    /// The dependency edges cargo resolved for this target, excluding
    /// dev-deps (a shipped binary links normal+build inputs only).
    edges: Vec<String>,
}

fn sha256_file(path: &std::path::Path) -> Result<String, Fatal> {
    use sha2::Digest as _;
    let bytes = std::fs::read(path)
        .map_err(|e| Fatal::io(format!("cannot read the asset {}: {e}", path.display())))?;
    Ok(format!("{:x}", sha2::Sha256::digest(&bytes)))
}

fn json_string(v: &Json) -> Option<String> {
    v.as_string()
}

/// A member lookup yielding an array (objects only).
fn json_array<'a>(v: &'a Json, key: &str) -> Option<&'a Vec<Json>> {
    match v.find(key) {
        Some(Json::Array(items)) => Some(items),
        _ => None,
    }
}

/// The value itself as an array (no member lookup).
fn as_array(v: &Json) -> Option<&Vec<Json>> {
    match v {
        Json::Array(items) => Some(items),
        _ => None,
    }
}

/// A member lookup yielding a string.
fn json_string_of(v: &Json, key: &str) -> Option<String> {
    v.find(key).and_then(json_string)
}

/// Parse the lock's `[[package]]` rows into the (name, version) →
/// checksum join table. Registry-sourced rows carry `checksum`; the
/// workspace's own rows and git/path rows do not — absence is a fact,
/// never an error.
fn parse_lock(text: &str) -> Result<BTreeMap<(String, String), String>, Fatal> {
    let doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| Fatal::malformed(format!("Cargo.lock does not parse as TOML: {e}")))?;
    let packages = doc
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| Fatal::malformed("Cargo.lock carries no [[package]] rows"))?;
    let mut map = BTreeMap::new();
    for table in packages {
        let get = |key: &str| {
            table
                .get(key)
                .and_then(toml_edit::Item::as_value)
                .and_then(toml_edit::Value::as_str)
                .map(str::to_string)
        };
        let (Some(name), Some(version)) = (get("name"), get("version")) else {
            return Err(Fatal::malformed(
                "Cargo.lock [[package]] row without name/version",
            ));
        };
        if let Some(checksum) = get("checksum") {
            map.insert((name, version), checksum);
        }
    }
    Ok(map)
}

/// Parse the cargo metadata document and walk the resolve graph from
/// the named workspace member, collecting the target-filtered crate
/// set. `--filter-platform` already pruned the edges at capture time;
/// dev-dep edges are excluded here (a shipped binary links normal+build
/// inputs only).
fn resolve_crates(
    metadata_text: &str,
    package: &str,
    lock: &BTreeMap<(String, String), String>,
) -> Result<Vec<CrateRow>, Fatal> {
    let doc = tebako_json::parse(metadata_text)
        .map_err(|e| Fatal::malformed(format!("cargo metadata JSON does not parse: {e}")))?;
    let packages = json_array(&doc, "packages")
        .ok_or_else(|| Fatal::malformed("cargo metadata carries no packages array"))?;
    let workspace_members = json_array(&doc, "workspace_members")
        .ok_or_else(|| Fatal::malformed("cargo metadata carries no workspace_members"))?;
    let resolve = doc
        .find("resolve")
        .ok_or_else(|| Fatal::malformed("cargo metadata carries no resolve object"))?;
    let nodes = json_array(resolve, "nodes")
        .ok_or_else(|| Fatal::malformed("cargo metadata resolve carries no nodes"))?;

    // Package id → (name, version, license, source).
    struct PkgInfo {
        name: String,
        version: String,
        license: Option<String>,
        source: Option<String>,
    }
    let mut info: BTreeMap<String, PkgInfo> = BTreeMap::new();
    for pkg in packages {
        let id = json_string_of(pkg, "id")
            .ok_or_else(|| Fatal::malformed("cargo metadata package without id"))?;
        let name = json_string_of(pkg, "name")
            .ok_or_else(|| Fatal::malformed("cargo metadata package without name"))?;
        let version = json_string_of(pkg, "version")
            .ok_or_else(|| Fatal::malformed("cargo metadata package without version"))?;
        info.insert(
            id,
            PkgInfo {
                name,
                version,
                license: pkg.find("license").and_then(json_string),
                source: pkg.find("source").and_then(json_string),
            },
        );
    }

    // The root: the workspace member carrying the requested name.
    let root_id = workspace_members
        .iter()
        .filter_map(json_string)
        .find(|id| info.get(id).is_some_and(|p| p.name == package))
        .ok_or_else(|| {
            Fatal::malformed(format!(
                "'{package}' is not a workspace member in this cargo metadata"
            ))
        })?;

    // Node id → resolved edges (dev excluded).
    let mut node_edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in nodes {
        let id = json_string_of(node, "id")
            .ok_or_else(|| Fatal::malformed("cargo metadata node without id"))?;
        let mut edges = Vec::new();
        if let Some(deps) = node.find("deps").and_then(as_array) {
            for dep in deps {
                let kinds = dep.find("dep_kinds").and_then(as_array);
                let dev_only = match kinds {
                    Some(kinds) if !kinds.is_empty() => kinds
                        .iter()
                        .all(|k| json_string_of(k, "kind").as_deref() == Some("dev")),
                    _ => false,
                };
                if dev_only {
                    continue;
                }
                if let Some(child) = json_string_of(dep, "pkg") {
                    edges.push(child);
                }
            }
        }
        node_edges.insert(id, edges);
    }

    // The walk: transitive closure over the target-filtered edges.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut stack = vec![root_id.clone()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(edges) = node_edges.get(&id) {
            stack.extend(edges.iter().cloned());
        }
    }

    let mut rows = Vec::with_capacity(seen.len());
    for id in &seen {
        let pkg = info.get(id).ok_or_else(|| {
            Fatal::malformed(format!("cargo metadata node '{id}' has no package row"))
        })?;
        let checksum = lock.get(&(pkg.name.clone(), pkg.version.clone())).cloned();
        rows.push(CrateRow {
            id: id.clone(),
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            license: pkg.license.clone(),
            source: pkg.source.clone(),
            checksum,
            edges: node_edges.get(id).cloned().unwrap_or_default(),
        });
    }
    Ok(rows)
}

// ---------------------------------------------------------------------
// The document
// ---------------------------------------------------------------------

/// The CycloneDX component type for the asset's own shape: exes are
/// applications, the preload shims libraries, the link-unit tarballs
/// plain file containers.
fn asset_type(name: &str) -> ComponentType {
    if name.ends_with(".tar.gz") {
        ComponentType::File
    } else if name.ends_with(".dylib") || name.ends_with(".so") {
        ComponentType::Library
    } else {
        ComponentType::Application
    }
}

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

fn crate_purl(name: &str, version: &str) -> String {
    format!("pkg:cargo/{}@{}", purl_encode(name), purl_encode(version))
}

fn build_document(
    opts: &Options,
    metadata_text: &str,
    lock_text: &str,
) -> Result<(Document, usize), Fatal> {
    let (tool, asset, name, version, platform) = (
        opts.tool.as_deref().unwrap(),
        opts.asset.as_ref().unwrap(),
        opts.name.as_deref().unwrap(),
        opts.version.as_deref().unwrap(),
        opts.platform.as_deref().unwrap(),
    );
    let lock = parse_lock(lock_text)?;
    let rows = resolve_crates(metadata_text, &opts.package, &lock)?;
    let asset_sha256 = sha256_file(asset)?;
    let count = rows.len();

    // -- the crate components + edges. purl per crate; the resolve id
    //    itself never leaks into the document (it is an opaque, unstable
    //    token — the purl is the identity).
    let mut components = Vec::with_capacity(rows.len());
    let mut purl_of_id: BTreeMap<String, String> = BTreeMap::new();
    for row in &rows {
        purl_of_id.insert(row.id.clone(), crate_purl(&row.name, &row.version));
    }
    let mut edges: Vec<Dependency> = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut c = Component::new(
            ComponentType::Library,
            &row.name,
            &row.version,
            &crate_purl(&row.name, &row.version),
        );
        c.sha256 = row.checksum.clone();
        c.license_expression = row.license.clone();
        // The source property mapping is the capture grammar's own
        // (tebako-sbom owns it): non-registry sources name themselves —
        // a hash can never be claimed for them, and a silent absence
        // would read as a gap.
        let source = capture::classify_cargo_source(row.source.as_deref())
            .map_err(|e| Fatal::malformed(e.to_string()))?;
        c.properties =
            capture::source_properties(&source).map_err(|e| Fatal::malformed(e.to_string()))?;
        components.push(c);
        edges.push(Dependency {
            refr: crate_purl(&row.name, &row.version),
            depends_on: row
                .edges
                .iter()
                .filter_map(|id| purl_of_id.get(id).cloned())
                .collect(),
        });
    }

    // -- the subject: the asset itself, with its staged bytes' digest
    let subject_purl = format!("pkg:generic/{}@{}", purl_encode(tool), purl_encode(version));
    let mut subject = Component::new(asset_type(name), name, version, &subject_purl);
    subject.sha256 = Some(asset_sha256);
    subject.properties = vec![
        ("tebako:tool".to_string(), tool.to_string()),
        ("tebako:platform".to_string(), platform.to_string()),
        ("tebako:target".to_string(), opts.target.clone()),
        ("tebako:cargo:package".to_string(), opts.package.clone()),
    ];
    if let Some(base) = &opts.release_url {
        let base = base.trim_end_matches('/');
        subject.external_references = vec![
            ("attestation".to_string(), format!("{base}/{name}.asc")),
            ("distribution".to_string(), format!("{base}/SHA256SUMS")),
        ];
    }

    // -- the root edge: the asset depends on its own crate (the
    //    workspace member whose closure this document is)
    let root_row = rows
        .iter()
        .find(|r| r.name == opts.package)
        .ok_or_else(|| {
            Fatal::malformed(format!(
                "the resolve set carries no '{}''s own crate row",
                opts.package
            ))
        })?;
    edges.push(Dependency {
        refr: subject_purl.clone(),
        depends_on: vec![crate_purl(&root_row.name, &root_row.version)],
    });

    let document = Document {
        metadata_component: subject,
        metadata_properties: vec![
            (
                "tebako:sbom:inventory-source".to_string(),
                "cargo-resolution".to_string(),
            ),
            ("tebako:closure:complete".to_string(), "true".to_string()),
        ],
        tool: ("tebako".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        components,
        graph: DependencyGraph::Explicit(edges),
        annotations: Vec::new(),
        serial_mode: SerialMode::ContentDerived,
        epoch_override: None,
    };
    Ok((document, count))
}

/// The capture mode (spec 15 §8a): the same resolution walk, emitted as
/// the crate-inventory capture the link-unit tarball carries and
/// `tebako sbom --runtime-exe` re-subjects. No asset, no document —
/// the exe's bytes do not exist in this pipeline; what exists is the
/// resolution, and that is exactly what the capture claims.
fn build_capture(
    opts: &Options,
    metadata_text: &str,
    lock_text: &str,
) -> Result<(CrateCapture, usize), Fatal> {
    let lock = parse_lock(lock_text)?;
    let rows = resolve_crates(metadata_text, &opts.package, &lock)?;

    let mut purl_of_id: BTreeMap<String, String> = BTreeMap::new();
    for row in &rows {
        purl_of_id.insert(row.id.clone(), crate_purl(&row.name, &row.version));
    }
    let root = rows
        .iter()
        .find(|r| r.name == opts.package)
        .ok_or_else(|| {
            Fatal::malformed(format!(
                "the resolve set carries no '{}''s own crate row",
                opts.package
            ))
        })?;
    let count = rows.len();
    let mut components = Vec::with_capacity(count);
    let mut edges = Vec::with_capacity(count);
    for row in &rows {
        components.push(CapturedCrate {
            name: row.name.clone(),
            version: row.version.clone(),
            purl: crate_purl(&row.name, &row.version),
            sha256: row.checksum.clone(),
            license: row.license.clone(),
            source: capture::classify_cargo_source(row.source.as_deref())
                .map_err(|e| Fatal::malformed(e.to_string()))?,
        });
        edges.push(CaptureEdge {
            refr: crate_purl(&row.name, &row.version),
            depends_on: row
                .edges
                .iter()
                .filter_map(|id| purl_of_id.get(id).cloned())
                .collect(),
        });
    }
    Ok((
        CrateCapture {
            tool: (
                "sbom-cargo".to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
            ),
            package: opts.package.clone(),
            target: opts.target.clone(),
            root: crate_purl(&root.name, &root.version),
            components,
            edges,
        },
        count,
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(fatal) => {
            eprintln!("sbom-cargo: {}", fatal.message);
            ExitCode::from(fatal.code)
        }
    }
}

fn run(args: &[String]) -> Result<(), Fatal> {
    let opts = parse_args(args)?;
    let metadata_text = std::fs::read_to_string(&opts.metadata).map_err(|e| {
        Fatal::io(format!(
            "cannot read the cargo metadata {}: {e}",
            opts.metadata.display()
        ))
    })?;
    let lock_text = std::fs::read_to_string(&opts.lock).map_err(|e| {
        Fatal::io(format!(
            "cannot read Cargo.lock {}: {e}",
            opts.lock.display()
        ))
    })?;
    if opts.inventory_only {
        let (capture, count) = build_capture(&opts, &metadata_text, &lock_text)?;
        std::fs::write(&opts.out, capture.emit())
            .map_err(|e| Fatal::io(format!("cannot write {}: {e}", opts.out.display())))?;
        eprintln!(
            "sbom-cargo: {} crate component(s) captured -> {}",
            count,
            opts.out.display()
        );
        return Ok(());
    }
    let (document, count) = build_document(&opts, &metadata_text, &lock_text)?;
    let json = document
        .emit()
        .map_err(|e| Fatal::malformed(e.to_string()))?;
    std::fs::write(&opts.out, &json)
        .map_err(|e| Fatal::io(format!("cannot write {}: {e}", opts.out.display())))?;
    eprintln!(
        "sbom-cargo: {} — {} crate component(s) -> {}",
        opts.name.as_deref().unwrap_or("<capture>"),
        count,
        opts.out.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic workspace: root crate `probe` depending on `dep`
    /// (registry, checksummed, licensed), `dep` on `dev-tool` (dev edge —
    /// excluded) and `buildme` (build edge — kept), plus a git dep.
    fn metadata_fixture() -> String {
        r#"{
  "packages": [
    {"id": "path+file:///w#probe@2.0.0", "name": "probe", "version": "2.0.0", "license": "BSD-2-Clause", "source": null},
    {"id": "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0", "name": "dep", "version": "1.0.0", "license": "MIT OR Apache-2.0", "source": "registry+https://github.com/rust-lang/crates.io-index"},
    {"id": "registry+https://github.com/rust-lang/crates.io-index#dev-tool@3.0.0", "name": "dev-tool", "version": "3.0.0", "license": null, "source": "registry+https://github.com/rust-lang/crates.io-index"},
    {"id": "registry+https://github.com/rust-lang/crates.io-index#buildme@0.1.0", "name": "buildme", "version": "0.1.0", "license": "MIT", "source": "registry+https://github.com/rust-lang/crates.io-index"},
    {"id": "git+https://example.com/thing#thing@0.4.0", "name": "thing", "version": "0.4.0", "license": null, "source": "git+https://example.com/thing?rev=abc123"}
  ],
  "workspace_members": ["path+file:///w#probe@2.0.0"],
  "resolve": {
    "nodes": [
      {"id": "path+file:///w#probe@2.0.0", "deps": [
        {"pkg": "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0", "dep_kinds": [{"kind": "normal", "target": null}]},
        {"pkg": "git+https://example.com/thing#thing@0.4.0", "dep_kinds": [{"kind": "normal", "target": null}]}
      ], "features": []},
      {"id": "registry+https://github.com/rust-lang/crates.io-index#dep@1.0.0", "deps": [
        {"pkg": "registry+https://github.com/rust-lang/crates.io-index#dev-tool@3.0.0", "dep_kinds": [{"kind": "dev", "target": null}]},
        {"pkg": "registry+https://github.com/rust-lang/crates.io-index#buildme@0.1.0", "dep_kinds": [{"kind": "build", "target": null}]}
      ], "features": []},
      {"id": "registry+https://github.com/rust-lang/crates.io-index#dev-tool@3.0.0", "deps": [], "features": []},
      {"id": "registry+https://github.com/rust-lang/crates.io-index#buildme@0.1.0", "deps": [], "features": []},
      {"id": "git+https://example.com/thing#thing@0.4.0", "deps": [], "features": []}
    ],
    "root": null
  }
}"#
        .to_string()
    }

    fn lock_fixture() -> String {
        r#"version = 4

[[package]]
name = "probe"
version = "2.0.0"

[[package]]
name = "dep"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aaaa111111111111111111111111111111111111111111111111111111111111"

[[package]]
name = "buildme"
version = "0.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbbb222222222222222222222222222222222222222222222222222222222222"

[[package]]
name = "thing"
version = "0.4.0"
source = "git+https://example.com/thing?rev=abc123"
"#
        .to_string()
    }

    fn options(dir: &std::path::Path) -> Options {
        let asset = dir.join("probe-2.0.0-testos-arm64");
        std::fs::write(&asset, b"staged bytes").unwrap();
        Options {
            package: "probe".to_string(),
            target: "aarch64-testos".to_string(),
            metadata: dir.join("metadata.json"),
            lock: dir.join("Cargo.lock"),
            out: dir.join("out.json"),
            inventory_only: false,
            tool: Some("probe".to_string()),
            asset: Some(asset),
            name: Some("probe-2.0.0-testos-arm64".to_string()),
            version: Some("2.0.0".to_string()),
            platform: Some("testos-arm64".to_string()),
            release_url: Some("https://example.com/releases/download/v2.0.0".to_string()),
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sbom-cargo-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_walks_normal_and_build_edges_only() {
        let lock = parse_lock(&lock_fixture()).unwrap();
        let rows = resolve_crates(&metadata_fixture(), "probe", &lock).unwrap();
        let names: BTreeSet<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["probe", "dep", "thing", "buildme"].into_iter().collect()
        );
        // dev-tool sits behind a dev edge — never in a shipped SBOM.
        assert!(!names.contains("dev-tool"));
        // Registry checksums join by (name, version); the git row names
        // its source instead.
        let dep = rows.iter().find(|r| r.name == "dep").unwrap();
        assert_eq!(
            dep.checksum.as_deref(),
            Some("aaaa111111111111111111111111111111111111111111111111111111111111")
        );
        let thing = rows.iter().find(|r| r.name == "thing").unwrap();
        assert_eq!(thing.checksum, None);
        assert_eq!(
            thing.source.as_deref(),
            Some("git+https://example.com/thing?rev=abc123")
        );
        // The workspace member's source is null → workspace property.
        let probe = rows.iter().find(|r| r.name == "probe").unwrap();
        assert_eq!(probe.source, None);
        // A non-member package name refuses named.
        assert!(resolve_crates(&metadata_fixture(), "absent", &lock).is_err());
    }

    #[test]
    fn document_content_model_and_determinism() {
        let dir = scratch("document");
        let opts = options(&dir);
        let (doc, count) = build_document(&opts, &metadata_fixture(), &lock_fixture()).unwrap();
        assert_eq!(count, 4);
        let first = doc.emit().unwrap();
        let second = doc.emit().unwrap();
        assert_eq!(first, second, "two emits are byte-identical");
        // The subject carries the staged bytes' digest and the
        // provenance links.
        assert!(first.contains("\"name\": \"probe-2.0.0-testos-arm64\""));
        assert!(first.contains("\"tebako:platform\""));
        assert!(first.contains("\"type\": \"attestation\""));
        assert!(first.contains("/probe-2.0.0-testos-arm64.asc"));
        assert!(first.contains("/SHA256SUMS"));
        // The crate set: purls, checksum hashes, license expressions,
        // the git source property.
        assert!(first.contains("pkg:cargo/dep@1.0.0"));
        assert!(first.contains("pkg:cargo/buildme@0.1.0"));
        assert!(first.contains("pkg:cargo/thing@0.4.0"));
        assert!(first.contains("MIT OR Apache-2.0"));
        assert!(first.contains("git+https://example.com/thing?rev=abc123"));
        // The root edge points at the tool's own crate.
        let deps = first.split("\"dependencies\"").nth(1).unwrap();
        let root = deps
            .split("\"ref\": \"pkg:generic/probe@2.0.0\"")
            .nth(1)
            .unwrap();
        assert!(root.contains("pkg:cargo/probe@2.0.0"));
        // Real edges: probe depends on dep + thing; dep depends on
        // buildme (build edge kept), never dev-tool.
        let probe_edge = deps
            .split("\"ref\": \"pkg:cargo/probe@2.0.0\"")
            .nth(1)
            .unwrap();
        assert!(probe_edge.contains("pkg:cargo/dep@1.0.0"));
        assert!(probe_edge.contains("pkg:cargo/thing@0.4.0"));
        let dep_edge = deps
            .split("\"ref\": \"pkg:cargo/dep@1.0.0\"")
            .nth(1)
            .unwrap();
        assert!(dep_edge.contains("pkg:cargo/buildme@0.1.0"));
        assert!(!dep_edge.contains("dev-tool"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn asset_type_maps_from_the_name() {
        assert_eq!(
            asset_type("tebako-2.0.0-macos-arm64"),
            ComponentType::Application
        );
        assert_eq!(
            asset_type("libtfs_preload-2.0.0-macos-arm64.dylib"),
            ComponentType::Library
        );
        assert_eq!(
            asset_type("link-unit-2.0.0-macos-arm64.tar.gz"),
            ComponentType::File
        );
    }

    #[test]
    fn lock_parse_requires_package_rows() {
        assert!(parse_lock("version = 4\n").is_err());
    }

    #[test]
    fn capture_mode_carries_the_same_resolution() {
        let dir = scratch("capture");
        let mut opts = options(&dir);
        opts.inventory_only = true;
        let (capture, count) = build_capture(&opts, &metadata_fixture(), &lock_fixture()).unwrap();
        assert_eq!(count, 4);
        assert_eq!(capture.root, "pkg:cargo/probe@2.0.0");
        assert_eq!(capture.package, "probe");
        assert_eq!(capture.target, "aarch64-testos");
        // The capture round-trips through its own grammar (the reader's
        // gate: schema, required members, the root among the crates).
        let text = capture.emit();
        let parsed = CrateCapture::parse(&text).unwrap();
        assert_eq!(parsed.components.len(), 4);
        let dep = parsed.components.iter().find(|c| c.name == "dep").unwrap();
        assert_eq!(
            dep.sha256.as_deref(),
            Some("aaaa111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(dep.source, "registry");
        let thing = parsed
            .components
            .iter()
            .find(|c| c.name == "thing")
            .unwrap();
        assert_eq!(thing.source, "git+https://example.com/thing?rev=abc123");
        // dev-tool never enters the capture (dev edges excluded).
        assert!(!parsed.components.iter().any(|c| c.name == "dev-tool"));
        // The edges survive: probe → dep + thing, dep → buildme only.
        let probe_edge = parsed
            .edges
            .iter()
            .find(|e| e.refr == "pkg:cargo/probe@2.0.0")
            .unwrap();
        assert!(probe_edge
            .depends_on
            .contains(&"pkg:cargo/dep@1.0.0".to_string()));
        assert!(probe_edge
            .depends_on
            .contains(&"pkg:cargo/thing@0.4.0".to_string()));
        let dep_edge = parsed
            .edges
            .iter()
            .find(|e| e.refr == "pkg:cargo/dep@1.0.0")
            .unwrap();
        assert_eq!(dep_edge.depends_on, vec!["pkg:cargo/buildme@0.1.0"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn arg_modes_validate_their_flag_sets() {
        let base = [
            "--package",
            "probe",
            "--target",
            "aarch64-testos",
            "--metadata",
            "m.json",
            "--lock",
            "Cargo.lock",
            "--out",
            "out.json",
        ]
        .map(String::from);
        // The document mode needs its flags.
        assert!(parse_args(&base).is_err());
        let document = [
            &base[..],
            &[
                "--tool",
                "probe",
                "--asset",
                "a",
                "--name",
                "n",
                "--version",
                "1",
                "--platform",
                "p",
            ]
            .map(String::from),
        ]
        .concat();
        assert!(parse_args(&document).is_ok());
        // The capture mode stands on the common flags alone …
        let capture = [&base[..], &["--inventory-only".to_string()]].concat();
        assert!(parse_args(&capture).unwrap().inventory_only);
        // … and refuses document flags by name.
        let mixed = [&capture[..], &["--asset".to_string(), "a".to_string()]].concat();
        let err = parse_args(&mixed).unwrap_err();
        assert!(err
            .message
            .contains("--inventory-only takes no document flag"));
    }
}

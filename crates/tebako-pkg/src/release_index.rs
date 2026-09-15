//! roadmap 85 (spec 13 §2a): `tebako-pkg release-index` — derive the
//! legacy release monoliths (`manifest.json` + `SHA256SUMS.txt`) from a
//! release's per-package shards, READ-ONLY, for humans and audit. The
//! shards and the per-asset `.sha256` sidecars are the sidecar-era
//! authority; the derived documents exist so the pre-85 readers keep
//! working on old releases (spec 00 invariant 7) and `--check` can prove
//! a release's own published monoliths agree with its shards — or name
//! the drift (a stale monolith names the first missing/extra entry, then
//! the first differing line).
//!
//! The render is BYTE-EXACT against the factory's own derived monoliths
//! (pinned by the golden run against tebako-runtime-ruby v0.16.22):
//!
//! ```text
//! manifest.json  = "[\n" + shard bodies (sorted by the entry's
//!                  `filename`, rstripped of trailing newlines, every
//!                  line re-indented by 2 spaces) joined by ",\n" + "\n]\n"
//! SHA256SUMS.txt = per entry (filename order) the lines [exe, image,
//!                  dll?] as "<sha>  <name>\n" (exactly two spaces) — the
//!                  sha from the asset's `<asset>.sha256` sidecar's first
//!                  token WHEN LISTED, else the shard's own sha field.
//! ```

use std::path::PathBuf;

use tebako_json::{parse as json_parse, Value as JsonValue};
use tebako_resolve::{HttpTransport, Reference, Transport};

/// The release's asset namespace: a listing of names plus by-name reads.
/// `read` answers `Ok(None)` for an unlisted name WITHOUT touching the
/// network — the listing is authoritative (a service release's 404 probe
/// is exactly what the shard era abolished).
pub trait AssetSpace {
    /// Every asset name in the release (order irrelevant).
    fn names(&self) -> Vec<String>;
    /// The asset's UTF-8 text; `Ok(None)` when the name is not listed.
    fn read(&self, name: &str) -> Result<Option<String>, String>;
}

/// A local release directory (the `file://` form — fixtures, mirrors).
struct DirSpace {
    dir: PathBuf,
}

impl AssetSpace for DirSpace {
    fn names(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            for entry in rd.flatten() {
                let ok = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
                if ok {
                    out.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        out
    }

    fn read(&self, name: &str) -> Result<Option<String>, String> {
        match std::fs::read_to_string(self.dir.join(name)) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!(
                "cannot read {} in {}: {e}",
                name,
                self.dir.display()
            )),
        }
    }
}

/// A service release (`tfs:github:`/gitlab/bb): the adapter's asset
/// listing, reads through the spec-04 transport.
struct ServiceSpace {
    assets: Vec<tebako_resolve::adapters::Asset>,
    transport: HttpTransport,
}

impl AssetSpace for ServiceSpace {
    fn names(&self) -> Vec<String> {
        self.assets.iter().map(|a| a.name.clone()).collect()
    }

    fn read(&self, name: &str) -> Result<Option<String>, String> {
        let Some(asset) = self.assets.iter().find(|a| a.name == name) else {
            return Ok(None);
        };
        // the asset descriptor's declared requirements ride the fetch
        // (spec 04 §3 — an API asset URL answers JSON without them)
        let bytes = self
            .transport
            .get_asset(&asset.url, asset.accept.as_deref(), asset.authenticate)
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|e| format!("invalid UTF-8 in {name}: {e}"))
    }
}

/// Open a release reference (`tfs:github:owner/repo:tag` … or
/// `file:///dir`) as its asset space. A `#artifact` selector is a named
/// error: release-index reads a WHOLE release.
pub fn open_release(reference: &str) -> Result<Box<dyn AssetSpace>, String> {
    match Reference::parse(reference).map_err(|e| e.to_string())? {
        Reference::Service {
            service,
            owner,
            repo,
            version,
            artifact,
            ..
        } => {
            if let Some(a) = artifact {
                return Err(format!(
                    "release-index reads a whole release — drop the #{a} artifact selector from {reference}"
                ));
            }
            let adapter = tebako_resolve::adapters::adapter_for(service);
            let assets = adapter
                .assets(&HttpTransport, &owner, &repo, &version)
                .map_err(|e| e.to_string())?;
            Ok(Box::new(ServiceSpace {
                assets,
                transport: HttpTransport,
            }))
        }
        Reference::File { path, .. } => Ok(Box::new(DirSpace {
            dir: PathBuf::from(path),
        })),
        other => Err(format!(
            "release-index needs a release reference (tfs:github:/tfs:gitlab:/tfs:bb: or file://<dir>), not {other}"
        )),
    }
}

/// One shard's contribution to the derived documents: the identity entry
/// plus the raw body (the manifest render re-indents the ORIGINAL text —
/// the shards are the authority byte-for-byte, never reserialized).
struct ShardEntry {
    /// The exe asset name (the entry's `filename` — also the sort key).
    filename: String,
    /// The raw shard text, rstripped of trailing newlines.
    body: String,
    exe_sha: String,
    /// `(filename, sha256)` of the additive facets, when declared.
    image: Option<(String, String)>,
    dll: Option<(String, String)>,
}

/// The derived monoliths.
pub struct DerivedIndex {
    pub manifest_json: String,
    pub sha256sums_txt: String,
    /// The shard count (the derived manifest's entry count).
    pub entries: usize,
}

fn facet(doc: &JsonValue, key: &str) -> Option<(String, String)> {
    let obj = doc.find(key)?;
    let filename = obj.find("filename")?.as_string()?;
    let sha256 = obj.find("sha256")?.as_string()?;
    Some((filename, sha256))
}

fn parse_shard(name: &str, text: &str) -> Result<ShardEntry, String> {
    let malformed = |why: String| format!("malformed shard {name}: {why}");
    let doc = json_parse(text).map_err(|e| malformed(e.to_string()))?;
    let field = |key: &str| doc.find(key).and_then(|v| v.as_string());
    let filename = field("filename").ok_or_else(|| malformed("no \"filename\"".to_string()))?;
    let exe_sha = field("sha256").ok_or_else(|| malformed("no \"sha256\"".to_string()))?;
    Ok(ShardEntry {
        filename,
        body: text.trim_end_matches(['\r', '\n']).to_string(),
        exe_sha,
        image: facet(&doc, "image"),
        dll: facet(&doc, "dll"),
    })
}

/// Every line of the (newline-free) body re-indented by 2 spaces.
fn indent2(body: &str) -> String {
    body.lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The asset's declared sha256: its `<asset>.sha256` sidecar's first
/// token when the release lists one, else the shard's own field.
fn declared_sha(space: &dyn AssetSpace, asset: &str, field_sha: &str) -> Result<String, String> {
    let sidecar = format!("{asset}.sha256");
    match space.read(&sidecar)? {
        Some(text) => text
            .split_whitespace()
            .next()
            .filter(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(str::to_string)
            .ok_or_else(|| format!("malformed sidecar {sidecar}: no leading sha256 token")),
        None => Ok(field_sha.to_string()),
    }
}

/// Derive the monoliths from the release's shards (spec 13 §2a). A
/// release carrying no shards is a named error — its own manifest.json
/// (when it has one) is already the index; there is nothing to derive.
pub fn derive(space: &dyn AssetSpace) -> Result<DerivedIndex, String> {
    let mut shards: Vec<ShardEntry> = Vec::new();
    for name in space
        .names()
        .iter()
        .filter(|n| n.ends_with(".manifest.json"))
    {
        let text = space
            .read(name)?
            .ok_or_else(|| format!("asset {name} vanished mid-listing"))?;
        shards.push(parse_shard(name, &text)?);
    }
    if shards.is_empty() {
        return Err(
            "the release carries no <stem>.manifest.json shards — nothing to derive (a pre-sidecar-era release's own manifest.json is already the index)"
                .to_string(),
        );
    }
    shards.sort_by(|a, b| a.filename.cmp(&b.filename));

    let manifest_json = format!(
        "[\n{}\n]\n",
        shards
            .iter()
            .map(|s| indent2(&s.body))
            .collect::<Vec<_>>()
            .join(",\n")
    );

    let mut sha256sums_txt = String::new();
    for shard in &shards {
        let mut assets = vec![(&shard.filename, &shard.exe_sha)];
        if let Some((f, s)) = &shard.image {
            assets.push((f, s));
        }
        if let Some((f, s)) = &shard.dll {
            assets.push((f, s));
        }
        for (asset, field_sha) in assets {
            let sha = declared_sha(space, asset, field_sha)?;
            sha256sums_txt.push_str(&format!("{sha}  {asset}\n"));
        }
    }

    Ok(DerivedIndex {
        manifest_json,
        sha256sums_txt,
        entries: shards.len(),
    })
}

/// The entry names of a published monolith (its entries' `filename`
/// fields). A monolith that does not parse as a JSON array is a named
/// error — drift detection never guesses.
fn monolith_entries(text: &str) -> Result<Vec<String>, String> {
    match json_parse(text).map_err(|e| format!("the published manifest.json is not JSON: {e}"))? {
        JsonValue::Array(entries) => Ok(entries
            .iter()
            .filter_map(|e| e.find("filename").and_then(|f| f.as_string()))
            .collect()),
        _ => Err("the published manifest.json is not a JSON array".to_string()),
    }
}

/// The asset names of a published SHA256SUMS.txt (each line's second
/// whitespace token).
fn sums_entries(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

/// The first line index (1-based) where the two texts differ, with both
/// lines. `None` when one is a line-prefix of the other (the entry
/// set-diff covers that case) or they are equal.
fn first_differing_line<'a>(
    derived: &'a str,
    published: &'a str,
) -> Option<(usize, &'a str, &'a str)> {
    let mut d = derived.lines();
    let mut p = published.lines();
    let mut n = 0usize;
    loop {
        n += 1;
        match (d.next(), p.next()) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => return Some((n, a, b)),
            _ => return None,
        }
    }
}

/// Byte-compare one derived document against its published form; drift
/// is a named error — the entry set-diff first (the first missing/extra
/// entry, sorted), then the first differing line.
fn compare_document(
    name: &str,
    derived: &str,
    published: &str,
    published_entries: Vec<String>,
    derived_entries: &[String],
) -> Result<(), String> {
    if derived == published {
        return Ok(());
    }
    let mut missing: Vec<&String> = derived_entries
        .iter()
        .filter(|e| !published_entries.contains(*e))
        .collect();
    let mut extra: Vec<&String> = published_entries
        .iter()
        .filter(|e| !derived_entries.contains(*e))
        .collect();
    missing.sort();
    extra.sort();
    if let Some(first) = missing.first() {
        return Err(format!(
            "drift: {name} is stale — entry \"{first}\" is in the shards but not in the published {name}"
        ));
    }
    if let Some(first) = extra.first() {
        return Err(format!(
            "drift: {name} is stale — entry \"{first}\" is in the published {name} but not in the shards"
        ));
    }
    // Same entry sets, different bytes: the first differing line, or a
    // bare length drift when one text is a line-prefix of the other
    // (a trailing-newline-only difference — never a panic).
    match first_differing_line(derived, published) {
        Some((n, d, p)) => Err(format!(
            "drift: {name} first differs from the derived index at line {n}:\n  published: {p}\n  derived:   {d}"
        )),
        None => Err(format!(
            "drift: {name} differs from the derived index (derived {} bytes, published {} bytes)",
            derived.len(),
            published.len()
        )),
    }
}

/// `--check` (the audit mode, read-only): byte-compare the derived
/// documents against the release's own published monoliths WHEN LISTED.
/// `Ok(None)` = the release ships no monoliths at all (the post-85 line)
/// — nothing to check, success. `Ok(Some(report))` = byte-identical.
/// `Err` = drift, named.
pub fn check(space: &dyn AssetSpace, derived: &DerivedIndex) -> Result<Option<String>, String> {
    let manifest = space.read("manifest.json")?;
    let sums = space.read("SHA256SUMS.txt")?;
    if manifest.is_none() && sums.is_none() {
        return Ok(None);
    }
    let derived_entries: Vec<String> = {
        // the entry order is the render's sort key — re-derive it from
        // the derived document itself (single pass, no second listing)
        monolith_entries(&derived.manifest_json)?
    };
    let mut report = String::new();
    if let Some(published) = manifest {
        compare_document(
            "manifest.json",
            &derived.manifest_json,
            &published,
            monolith_entries(&published)?,
            &derived_entries,
        )?;
        report.push_str(&format!(
            "manifest.json: byte-identical with the derived index ({} bytes)\n",
            published.len()
        ));
    }
    if let Some(published) = sums {
        compare_document(
            "SHA256SUMS.txt",
            &derived.sha256sums_txt,
            &published,
            sums_entries(&published),
            &sums_entries(&derived.sha256sums_txt),
        )?;
        report.push_str(&format!(
            "SHA256SUMS.txt: byte-identical with the derived index ({} bytes)\n",
            published.len()
        ));
    }
    Ok(Some(report))
}

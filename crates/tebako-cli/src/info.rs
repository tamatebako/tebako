//! `tebako info [what]` — the store/system introspection surface
//! (spec 15 §4's machine view): what the machine HAS (cached runtimes,
//! payloads, shims, registries) and, with `--remote`, what the world
//! OFFERS (the runtime release index, the registered registries'
//! catalogs). Read-only always; `--json` is the machine form of every
//! view (`"info_schema": 1`).
//!
//! Topics:
//! - (bare) / `system` — tebako version, platform, home, store counts.
//! - `runtimes` — cached runtimes (engine, version, tebako, abi, image);
//!   `--remote` adds the release index (what a download would offer).
//! - `payloads` — cached payloads (name, version, kind, size, origin);
//!   `--remote` adds each registered registry's catalog.
//! - `shims` — the registered shims and what a dispatch WOULD run today
//!   (the version chain, resolved read-only).
//! - `registries` — registered registries and their dispatch-cache
//!   freshness.
//! - `store` — disk usage breakdown of the home.

use std::path::{Path, PathBuf};

use tebako_shim::{config, regcache, runtime, Ctx};

use crate::error::TebakoError;

const EX_USAGE: i32 = 64;
const EX_TEBAKO_IO: i32 = 74;
const EX_TEBAKO_MANIFEST: i32 = 65;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError {
        code,
        message: message.into(),
    }
}

/// The topics `tebako info <what>` accepts.
const TOPICS: [&str; 6] = [
    "system",
    "runtimes",
    "payloads",
    "shims",
    "registries",
    "store",
];

pub fn run(
    home: &Path,
    topic: Option<&str>,
    remote: bool,
    json: bool,
) -> Result<(String, i32), TebakoError> {
    let topic = topic.unwrap_or("system");
    if !TOPICS.contains(&topic) {
        return Err(err(
            EX_USAGE,
            format!(
                "unknown info topic '{topic}' (topics: {})",
                TOPICS.join(", ")
            ),
        ));
    }
    let out = match topic {
        "runtimes" => runtimes(home, remote, json)?,
        "payloads" => payloads(home, remote, json)?,
        "shims" => shims(home, json)?,
        "registries" => registries(home, json)?,
        "store" => store(home, json)?,
        _ => system(home, json)?,
    };
    Ok((out, 0))
}

// ---------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(m) = p.metadata() {
                    total += m.len();
                }
            }
        }
    }
    total
}

fn human(n: u64) -> String {
    tebako_info::format_size(n)
}

fn json_str(v: &tebako_json::Value) -> String {
    format!("{}\n", tebako_json::to_string(v))
}

fn s(text: &str) -> tebako_json::Value {
    tebako_json::Value::String(text.to_string())
}

fn obj(members: Vec<(&str, tebako_json::Value)>) -> tebako_json::Value {
    tebako_json::Value::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn arr(items: Vec<tebako_json::Value>) -> tebako_json::Value {
    tebako_json::Value::Array(items)
}

// ---------------------------------------------------------------------
// system
// ---------------------------------------------------------------------

/// The product version the banner carries (the crate semver — the
/// banner's single source is CARGO_PKG_VERSION).
fn product_version() -> &'static str {
    crate::VERSION_BANNER
        .strip_prefix("Tebako executable packager version ")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn system(home: &Path, json: bool) -> Result<String, TebakoError> {
    let runtimes = runtime::scan_all_cached(home);
    let payload_count = payload_dirs(home).len();
    let shim_count = std::fs::read_dir(home.join("shims"))
        .map(|rd| rd.flatten().count())
        .unwrap_or(0);
    let registry_count = config::load_config(home)
        .map(|c| c.registries.len())
        .unwrap_or(0);
    let home_size = dir_size(home);
    if json {
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            ("tebako_version", s(product_version())),
            ("platform", s(&crate::options::host_platform()?)),
            ("home", s(&home.display().to_string())),
            (
                "runtimes",
                tebako_json::Value::Number(runtimes.len().to_string()),
            ),
            (
                "payloads",
                tebako_json::Value::Number(payload_count.to_string()),
            ),
            ("shims", tebako_json::Value::Number(shim_count.to_string())),
            (
                "registries",
                tebako_json::Value::Number(registry_count.to_string()),
            ),
            (
                "store_bytes",
                tebako_json::Value::Number(home_size.to_string()),
            ),
        ])));
    }
    Ok(format!(
        "tebako {}\n  platform: {}\n  home: {} ({})\n  runtimes: {} cached\n  payloads: {} cached\n  shims: {} registered\n  registries: {} registered\n",
        product_version(),
        crate::options::host_platform()?,
        home.display(),
        human(home_size),
        runtimes.len(),
        payload_count,
        shim_count,
        registry_count
    ))
}

// ---------------------------------------------------------------------
// runtimes
// ---------------------------------------------------------------------

fn runtime_json(rt: &runtime::CachedRuntime) -> tebako_json::Value {
    let mut members = vec![
        ("engine", s(&rt.engine)),
        ("version", s(&rt.lang_version)),
        ("tebako", s(&rt.tebako_version)),
        (
            "abi",
            rt.abi.as_deref().map(s).unwrap_or(tebako_json::Value::Null),
        ),
        ("image", tebako_json::Value::Bool(rt.image.is_some())),
        ("exe", s(&rt.exe.display().to_string())),
    ];
    if let Ok(m) = rt.exe.metadata() {
        members.push(("exe_bytes", tebako_json::Value::Number(m.len().to_string())));
    }
    obj(members)
}

fn runtimes(home: &Path, remote: bool, json: bool) -> Result<String, TebakoError> {
    let mut cached = runtime::scan_all_cached(home);
    cached.sort_by(|a, b| {
        (&a.engine, &a.lang_version, &a.tebako_version).cmp(&(
            &b.engine,
            &b.lang_version,
            &b.tebako_version,
        ))
    });
    let mut out = String::new();
    if !json {
        if cached.is_empty() {
            out.push_str("no cached runtimes\n");
        }
        for rt in &cached {
            out.push_str(&format!(
                "{} {} (tebako {}){}{}  {}\n",
                rt.engine,
                rt.lang_version,
                rt.tebako_version,
                rt.abi
                    .as_deref()
                    .map(|a| format!(", abi {a}"))
                    .unwrap_or_default(),
                if rt.image.is_some() { "" } else { ", no image" },
                human(rt.exe.metadata().map(|m| m.len()).unwrap_or(0)),
            ));
        }
    }
    let mut remote_doc = Vec::new();
    if remote {
        let offers = remote_runtimes()?;
        if !json {
            out.push_str("\nremote (tamatebako/tebako-runtime-ruby):\n");
            for o in &offers {
                out.push_str(&format!("  {o}\n"));
            }
        } else {
            remote_doc = offers
                .iter()
                .map(|line| s(line))
                .collect::<Vec<tebako_json::Value>>();
        }
    }
    if json {
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            ("cached", arr(cached.iter().map(runtime_json).collect())),
            ("remote", arr(remote_doc)),
        ])));
    }
    Ok(out)
}

/// The release index of the runtime factory, flattened to lines
/// (`<tag>: ruby <ver> <platform> [abi …] [contract N]`). Bounded to the
/// newest few releases — the index is informational, not a resolution.
fn remote_runtimes() -> Result<Vec<String>, TebakoError> {
    let api = "https://api.github.com/repos/tamatebako/tebako-runtime-ruby/releases?per_page=5";
    let body = tebako_http::get_text(api)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot list runtime releases: {e}")))?;
    let parsed = tebako_json::parse(&body)
        .map_err(|e| err(EX_TEBAKO_IO, format!("cannot parse the releases list: {e}")))?;
    let tebako_json::Value::Array(releases) = &parsed else {
        return Err(err(EX_TEBAKO_IO, "the releases list is not an array"));
    };
    let mut lines = Vec::new();
    for release in releases {
        let Some(tag) = release.find("tag_name").and_then(|t| t.as_string()) else {
            continue;
        };
        let manifest_url = format!(
            "https://github.com/tamatebako/tebako-runtime-ruby/releases/download/{tag}/manifest.json"
        );
        let Ok(manifest) = tebako_http::get_text(&manifest_url) else {
            lines.push(format!("{tag}: (no manifest.json)"));
            continue;
        };
        let Ok(parsed) = tebako_json::parse(&manifest) else {
            lines.push(format!("{tag}: (unparseable manifest.json)"));
            continue;
        };
        let tebako_json::Value::Array(entries) = &parsed else {
            continue;
        };
        for entry in entries {
            let lang = entry
                .find("ruby_version")
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "?".into());
            let platform = entry
                .find("platform")
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "?".into());
            let abi = entry
                .find("abi")
                .and_then(|v| v.as_string())
                .map(|a| format!(" abi {a}"))
                .unwrap_or_default();
            let contract = entry
                .find("contract_version")
                .and_then(|v| v.as_u64())
                .map(|c| format!(" contract {c}"))
                .unwrap_or_default();
            lines.push(format!("{tag}: ruby {lang} {platform}{abi}{contract}"));
        }
    }
    Ok(lines)
}

// ---------------------------------------------------------------------
// payloads
// ---------------------------------------------------------------------

fn payload_dirs(home: &Path) -> Vec<PathBuf> {
    let dir = home.join("payloads");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    out
}

fn payloads(home: &Path, remote: bool, json: bool) -> Result<String, TebakoError> {
    let mut entries: Vec<(String, String, String, u64, String)> = Vec::new(); // (name, version, kind, bytes, origin)
    for dir in payload_dirs(home) {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let versions = tebako_shim::resolve::installed_versions(home, &name)
            .map_err(|e| err(EX_TEBAKO_IO, e.message))?;
        for version in versions {
            let record = tebako_shim::manifest::payload_record(home, &name, &version);
            let kind = tebako_shim::manifest::Manifest::load(&record.manifest_mirror)
                .map(|m| format!("{:?}", m.kind()))
                .unwrap_or_else(|_| "?".into());
            let bytes = record.image.metadata().map(|m| m.len()).unwrap_or(0);
            let origin = std::fs::read_to_string(record.image.with_extension("tfs.origin"))
                .map(|o| o.trim().to_string())
                .unwrap_or_default();
            entries.push((name.clone(), version, kind, bytes, origin));
        }
    }
    if json {
        let mut remote_doc = Vec::new();
        if remote {
            for (reg_ref, line) in remote_payloads(home)? {
                remote_doc.push(obj(vec![("registry", s(&reg_ref)), ("payloads", s(&line))]));
            }
        }
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            (
                "cached",
                arr(entries
                    .iter()
                    .map(|(name, version, kind, bytes, origin)| {
                        obj(vec![
                            ("name", s(name)),
                            ("version", s(version)),
                            ("kind", s(kind)),
                            ("bytes", tebako_json::Value::Number(bytes.to_string())),
                            ("origin", s(origin)),
                        ])
                    })
                    .collect()),
            ),
            ("remote", arr(remote_doc)),
        ])));
    }
    let mut out = String::new();
    if entries.is_empty() {
        out.push_str("no cached payloads\n");
    }
    for (name, version, kind, bytes, origin) in &entries {
        out.push_str(&format!(
            "{name} {version} ({kind})  {}{}\n",
            human(*bytes),
            if origin.is_empty() {
                String::new()
            } else {
                format!("  ← {origin}")
            }
        ));
    }
    if remote {
        out.push_str("\nremote (registered registries):\n");
        for (reg_ref, line) in remote_payloads(home)? {
            out.push_str(&format!("  {reg_ref}\n"));
            for l in line.lines() {
                out.push_str(&format!("    {l}\n"));
            }
        }
    }
    Ok(out)
}

/// Each registered registry's catalog, one line per payload
/// (`<name> (<kind>): versions <list> [default <v>]`).
fn remote_payloads(home: &Path) -> Result<Vec<(String, String)>, TebakoError> {
    let cfg = config::load_config(home).map_err(|e| err(EX_TEBAKO_IO, e.message))?;
    let fetcher = tebako_resolve::Fetcher::new();
    let mut out = Vec::new();
    for reg_ref in &cfg.registries {
        let r = tebako_resolve::registry::RegistryRef::parse(reg_ref)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
        let registry = fetcher
            .resolve_registry(&r)
            .map_err(|e| err(EX_TEBAKO_IO, e.to_string()))?;
        let mut lines = String::new();
        for p in &registry.payloads {
            let versions: Vec<&str> = p.versions.iter().map(|v| v.version.as_str()).collect();
            let default = p.default.as_deref().unwrap_or("-");
            lines.push_str(&format!(
                "{} ({:?}): versions {}{}\n",
                p.name,
                p.kind,
                versions.join(", "),
                if p.default.is_some() {
                    format!(" (default {default})")
                } else {
                    String::new()
                }
            ));
        }
        out.push((reg_ref.clone(), lines));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// shims
// ---------------------------------------------------------------------

fn shims(home: &Path, json: bool) -> Result<String, TebakoError> {
    let dir = home.join("shims");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok(if json {
            json_str(&obj(vec![
                ("info_schema", tebako_json::Value::Number("1".into())),
                ("shims", arr(Vec::new())),
            ]))
        } else {
            "no registered shims\n".to_string()
        });
    };
    let ctx = Ctx {
        home: home.to_path_buf(),
        cwd: std::env::current_dir().map_err(|e| err(EX_TEBAKO_IO, e.to_string()))?,
        env: std::env::vars().collect(),
    };
    let mut rows: Vec<(String, String)> = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let preview = match tebako_shim::resolve::resolve(&name, &ctx) {
            Ok(res) => format!(
                "{} {}{}",
                res.payload_name,
                res.version,
                res.manifest
                    .entrypoint(&name)
                    .and_then(|e| e.runtime_requirement)
                    .map(|r| format!(" (runtime: {} {})", r.engine, r.constraint))
                    .unwrap_or_default()
            ),
            Err(e) => format!(
                "(unresolvable: {})",
                e.message.lines().next().unwrap_or("?")
            ),
        };
        rows.push((name, preview));
    }
    rows.sort();
    if json {
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            (
                "shims",
                arr(rows
                    .iter()
                    .map(|(name, preview)| {
                        obj(vec![("command", s(name)), ("dispatches_to", s(preview))])
                    })
                    .collect()),
            ),
        ])));
    }
    let mut out = String::new();
    for (name, preview) in &rows {
        out.push_str(&format!("{name} → {preview}\n"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// registries
// ---------------------------------------------------------------------

fn registries(home: &Path, json: bool) -> Result<String, TebakoError> {
    let cfg = config::load_config(home).map_err(|e| err(EX_TEBAKO_IO, e.message))?;
    let mut rows: Vec<(String, String)> = Vec::new();
    for reg_ref in &cfg.registries {
        let state = match regcache::freshness(home, reg_ref) {
            regcache::RegistryFreshness::Local => "local".to_string(),
            regcache::RegistryFreshness::Fresh(age) => format!("fresh ({}s old)", age),
            regcache::RegistryFreshness::Stale(age) => {
                format!("stale ({}s old — next dispatch refreshes)", age)
            }
            regcache::RegistryFreshness::Missing => "not cached (fetched on demand)".to_string(),
            regcache::RegistryFreshness::BadRef(e) => format!("invalid ref: {e}"),
        };
        rows.push((reg_ref.clone(), state));
    }
    if json {
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            (
                "registries",
                arr(rows
                    .iter()
                    .map(|(r, state)| obj(vec![("ref", s(r)), ("cache", s(state))]))
                    .collect()),
            ),
        ])));
    }
    if rows.is_empty() {
        return Ok("no registered registries — tebako add-registry <ref>\n".to_string());
    }
    let mut out = String::new();
    for (r, state) in &rows {
        out.push_str(&format!("{r}  ({state})\n"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// store
// ---------------------------------------------------------------------

fn store(home: &Path, json: bool) -> Result<String, TebakoError> {
    let sections = [
        "runtimes",
        "payloads",
        "shims",
        "registries",
        "tmp",
        "locks",
        "keys",
        "trust",
    ];
    let mut sizes: Vec<(String, u64)> = sections
        .iter()
        .map(|s| (s.to_string(), dir_size(&home.join(s))))
        .collect();
    let total: u64 = sizes.iter().map(|(_, n)| *n).sum();
    sizes.sort_by(|a, b| b.1.cmp(&a.1));
    if json {
        return Ok(json_str(&obj(vec![
            ("info_schema", tebako_json::Value::Number("1".into())),
            (
                "sections",
                arr(sizes
                    .iter()
                    .map(|(name, bytes)| {
                        obj(vec![
                            ("name", s(name)),
                            ("bytes", tebako_json::Value::Number(bytes.to_string())),
                        ])
                    })
                    .collect()),
            ),
            ("total_bytes", tebako_json::Value::Number(total.to_string())),
        ])));
    }
    let mut out = format!("store {} ({} total):\n", home.display(), human(total));
    for (name, bytes) in &sizes {
        out.push_str(&format!("  {name:<12} {}\n", human(*bytes)));
    }
    Ok(out)
}

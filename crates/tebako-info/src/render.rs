//! Human rendering of the payload manifest view (spec 15 §2): sectioned,
//! aligned output; computed facts labeled `derived:`. The default
//! (flag-less) outputs of both CLIs do NOT pass through here — parity is
//! their business; this is the additive surface.

use tpkg::{PayloadManifest, Platforms, Provides, Requirement};

use crate::derived::{Derived, RuntimeCompat};
use crate::format_size;
use crate::payload::PayloadInspection;

/// Which optional sections the caller asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sections {
    /// `--manifest`: append the full parsed model re-serialized as YAML.
    pub manifest: bool,
    /// `--provides`: the kind-specialized PROVIDES section.
    pub provides: bool,
    /// `--requires`: the DEPENDS edges.
    pub requires: bool,
    /// `--platforms`: expand the platform axis (triplet ↔ asset name).
    pub platforms: bool,
}

impl Sections {
    /// True when any section flag is set.
    pub fn any(self) -> bool {
        self.manifest || self.provides || self.requires || self.platforms
    }
}

fn digest_short(text: &str) -> String {
    // `9c37…` in the spec example: the first hex octet, ellipsized. A
    // tree_hash carries its algorithm prefix (`sha256:<hex>`) — abbreviate
    // the hash, keep the algorithm.
    let (prefix, hash) = text.split_once(':').unwrap_or(("", text));
    let short = if hash.len() > 8 {
        format!("{}…", &hash[..8])
    } else {
        hash.to_string()
    };
    if prefix.is_empty() {
        short
    } else {
        format!("{prefix}:{short}")
    }
}

fn platforms_line(m: &PayloadManifest, out: &mut String) {
    if let Provides::App(app) = &m.provides {
        let text = match &app.platforms {
            Platforms::Universal => "universal".to_string(),
            Platforms::Triplets(ts) => ts
                .iter()
                .map(|t| format!("{} ({})", t.as_triplet(), t.release_asset_name()))
                .collect::<Vec<_>>()
                .join(", "),
        };
        out.push_str(&format!("  platforms: {text}\n"));
    } else if let Provides::Runtime(rt) = &m.provides {
        let text = rt
            .provides
            .iter()
            .map(|e| {
                format!(
                    "{} ({})",
                    e.platform.as_triplet(),
                    e.platform.release_asset_name()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  platforms: {text}\n"));
    }
}

/// The spec-03 kind name (`app`, `data`, `toolkit`, `runtime`, `language`).
pub fn kind_name(kind: tpkg::PayloadKind) -> &'static str {
    match kind {
        tpkg::PayloadKind::App => "app",
        tpkg::PayloadKind::Data => "data",
        tpkg::PayloadKind::Toolkit => "toolkit",
        tpkg::PayloadKind::Runtime => "runtime",
        tpkg::PayloadKind::Language => "language",
    }
}

fn header(m: &PayloadManifest, sections: Sections, out: &mut String) {
    let id = &m.identity;
    out.push_str(&format!(
        "  kind: {}  name: {}  version: {}\n",
        kind_name(id.kind),
        id.name,
        id.version
    ));
    if sections.platforms {
        platforms_line(m, out);
    }
    out.push_str(&format!(
        "  digests: blob_sha256 {}  tree_hash {}\n",
        digest_short(&id.digest.blob_sha256),
        digest_short(&id.digest.tree_hash)
    ));
    let signing = match id.signing.state {
        tpkg::SigningState::Unsigned => "unsigned".to_string(),
        tpkg::SigningState::Signed => {
            let keyid = id.signing.keyid.as_deref().unwrap_or("(no keyid)");
            format!("signed (keyid {keyid})")
        }
    };
    out.push_str(&format!("  signing: {signing}\n"));
    let encryption = match id.encryption.state {
        tpkg::EncryptionState::None => "none".to_string(),
        tpkg::EncryptionState::Encrypted => {
            format!("encrypted ({} part(s))", id.encryption.parts.len())
        }
    };
    out.push_str(&format!("  encryption: {encryption}\n"));
}

fn provides_section(m: &PayloadManifest, out: &mut String) {
    out.push_str("  provides:\n");
    match &m.provides {
        Provides::App(app) => {
            for ep in &app.entrypoints {
                let mut line = format!("    entrypoint {} → {}", ep.name, ep.path);
                if !ep.args_default.is_empty() {
                    line.push_str(&format!("  args: {}", ep.args_default.join(" ")));
                }
                line.push_str(&format!(
                    "  runtime: {} {}",
                    ep.runtime_requirement.engine, ep.runtime_requirement.constraint
                ));
                out.push_str(&line);
                out.push('\n');
            }
        }
        Provides::Runtime(rt) => {
            for e in &rt.provides {
                out.push_str(&format!(
                    "    provides {} {} (abi {}) {} ({})\n",
                    e.engine,
                    e.version,
                    e.abi_line,
                    e.platform.as_triplet(),
                    e.platform.release_asset_name()
                ));
            }
            out.push_str(&format!(
                "    built_from: src_sha256 {}  patch_set {}\n",
                digest_short(&rt.built_from.src_sha256),
                rt.built_from.patch_set
            ));
            for (k, v) in &rt.env {
                out.push_str(&format!("    env {k}={v}\n"));
            }
        }
        Provides::Data(data) => {
            out.push_str(&format!(
                "    mount_semantics: suggested {}\n",
                data.mount_semantics.suggested
            ));
            if !data.consumers.is_empty() {
                out.push_str(&format!("    consumers: {}\n", data.consumers.join(", ")));
            }
        }
        Provides::Other(map) => {
            for (k, v) in map {
                let rendered = serde_yml::to_string(v)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "(unrenderable)".to_string());
                out.push_str(&format!("    {k}: {rendered}\n"));
            }
        }
    }
}

fn requires_section(m: &PayloadManifest, out: &mut String) {
    out.push_str("  requires:\n");
    for req in &m.requires {
        match req {
            Requirement::Language { engine, constraint } => {
                out.push_str(&format!("    language:{engine}:{constraint}\n"));
            }
            Requirement::Toolkit {
                name,
                constraint,
                triplets,
                mount,
            } => {
                let mut line = format!("    toolkit:{name}:{constraint}");
                if let Some(m) = mount {
                    line.push_str(&format!(" → {m}"));
                }
                if let Some(ts) = triplets {
                    line.push_str(&format!(
                        " (triplets: {})",
                        ts.iter()
                            .map(|t| t.as_triplet())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                out.push_str(&line);
                out.push('\n');
            }
            Requirement::Data {
                name,
                constraint,
                mount,
            } => {
                let mut line = format!("    data:{name}:{constraint}");
                if let Some(m) = mount {
                    line.push_str(&format!(" → {m}"));
                }
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
}

fn derived_section(d: &Derived, out: &mut String) {
    out.push_str("  derived:\n");
    if !d.shims.is_empty() {
        out.push_str(&format!("    shims: {}\n", d.shims.join(", ")));
    }
    for compat in &d.runtime_compat {
        match compat {
            RuntimeCompat::SatisfiedBy { entry } => {
                out.push_str(&format!("    runtime: satisfied-by {entry} (cached)\n"));
            }
            RuntimeCompat::RequiresDownload { requirement } => {
                out.push_str(&format!(
                    "    runtime: requires-download: {requirement} (no compatible runtime cached)\n"
                ));
            }
            RuntimeCompat::Incompatible { reason } => {
                out.push_str(&format!("    runtime: incompatible: {reason}\n"));
            }
        }
    }
    if !d.dependency_names.is_empty() {
        out.push_str(&format!(
            "    dependencies: {}\n",
            d.dependency_names.join(", ")
        ));
    }
}

/// The full manifest view for one inspected payload (the `tfs info`
/// manifest sections; reused for `tebako-pkg info --slot`).
pub fn manifest_view(p: &PayloadInspection, sections: Sections) -> String {
    let mut out = String::new();
    out.push_str(&format!("image: {}\n", p.path_display));
    if let Some(format) = &p.format {
        out.push_str(&format!(
            "  format: {}  ro  {}\n",
            format.label,
            format_size(p.size_bytes)
        ));
    }
    if let Some(note) = &p.mount_error {
        out.push_str(&format!("  format: unreadable ({note})\n"));
    }
    match (&p.manifest, &p.derived) {
        (Some(m), _) => {
            header(m, sections, &mut out);
            if sections.provides {
                provides_section(m, &mut out);
            }
            if sections.requires && !m.requires.is_empty() {
                requires_section(m, &mut out);
            }
            if let Some(note) = &p.manifest_validation {
                out.push_str(&format!("  validation: FAILED ({note})\n"));
            }
            if let Some(d) = &p.derived {
                derived_section(d, &mut out);
            }
            if sections.manifest {
                out.push_str("  manifest:\n");
                let yaml = m
                    .to_yaml()
                    .unwrap_or_else(|_| p.manifest_text.clone().unwrap_or_default());
                for line in yaml.trim_end().lines() {
                    out.push_str(&format!("    {line}\n"));
                }
            }
        }
        (None, _) => {
            if let Some(note) = &p.manifest_note {
                out.push_str(&format!("  manifest: {note}\n"));
            } else if let Some(err) = &p.manifest_validation {
                out.push_str(&format!("  manifest: invalid ({err})\n"));
            }
        }
    }
    out
}

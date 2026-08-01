//! `tebako inspect <artifact>` — the artifact introspection surface
//! (spec 15): a payload image (`.tfs`) or a packed tpkg binary,
//! auto-detected. The two MECE halves live in tebako-info
//! ([`tebako_info::payload`] and [`tebako_info::package`]); this is the
//! umbrella verb on the product CLI. Read-only always; `--verify` is the
//! named, strict mode with the spec-15 §5 exit codes.

use std::path::Path;

use crate::error::TebakoError;

const EX_TEBAKO_MANIFEST: i32 = 65;

fn err(code: i32, message: impl Into<String>) -> TebakoError {
    TebakoError {
        code,
        message: message.into(),
    }
}

/// What the user asked to see.
#[derive(Default)]
pub struct InspectOptions {
    pub manifest: bool,
    pub provides: bool,
    pub requires: bool,
    pub platforms: bool,
    pub json: bool,
    pub verify: bool,
    pub require_signed: bool,
    pub backend_json: bool,
    /// Package slots only: `--slot N`.
    pub slot: Option<usize>,
    /// The spec-18 §6 contract card: era, contract versions, mount_root,
    /// abi, trust + the verdict against this tebako (crates/tebako-cli's
    /// contract.rs — packages, payload images, runtime directories,
    /// tebako-bootstrap binaries).
    pub contract: bool,
}

/// `tebako inspect <path>`: detect the artifact kind and inspect it.
/// Returns the output; the process exit code is TebakoError::code on
/// error (0 normally; the spec-15 §5 codes under `--verify`).
pub fn inspect(path: &Path, opts: &InspectOptions) -> Result<(String, i32), TebakoError> {
    if opts.contract {
        let card = crate::contract::inspect(path)?;
        return Ok((
            if opts.json {
                crate::contract::render_json(&card, path)
            } else {
                crate::contract::render(&card, path)
            },
            0,
        ));
    }
    if crate::install::is_tpkg_package(path) {
        inspect_package(path, opts)
    } else {
        inspect_image(path, opts)
    }
}

fn sections(opts: &InspectOptions) -> tebako_info::render::Sections {
    tebako_info::render::Sections {
        manifest: opts.manifest,
        provides: opts.provides,
        requires: opts.requires,
        platforms: opts.platforms,
    }
}

fn inspect_image(path: &Path, opts: &InspectOptions) -> Result<(String, i32), TebakoError> {
    if opts.verify {
        let checks = tebako_info::verify::verify_image(path, opts.require_signed)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
        let code = tebako_info::verify::exit_code_of(&checks);
        if opts.json {
            let p = tebako_info::payload::inspect_image(path)
                .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
            let mut doc = tebako_info::payload::payload_json(&p, opts.backend_json);
            if let tebako_json::Value::Object(members) = &mut doc {
                members.push((
                    "checks".to_string(),
                    tebako_info::verify::checks_json(&checks),
                ));
            }
            return Ok((format!("{}\n", tebako_json::to_string(&doc)), code));
        }
        let mut out = String::new();
        let p = tebako_info::payload::inspect_image(path)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
        out.push_str(&tebako_info::render::manifest_view(&p, sections(opts)));
        out.push_str(&tebako_info::verify::render_report(
            &path.display().to_string(),
            &checks,
        ));
        return Ok((out, code));
    }

    let p = tebako_info::payload::inspect_image(path)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
    if let Some(e) = &p.mount_error {
        return Err(err(EX_TEBAKO_MANIFEST, format!("{}: {e}", path.display())));
    }
    if opts.json {
        let doc = tebako_info::payload::payload_json(&p, opts.backend_json);
        return Ok((format!("{}\n", tebako_json::to_string(&doc)), 0));
    }
    let mut out = tebako_info::render::manifest_view(&p, sections(opts));
    if opts.backend_json {
        if let Some(f) = &p.format {
            if let Some(json) = &f.backend_json {
                out.push_str(&format!("  backend: {json}\n"));
            }
        }
    }
    Ok((out, 0))
}

fn inspect_package(path: &Path, opts: &InspectOptions) -> Result<(String, i32), TebakoError> {
    let depth = if opts.slot.is_some()
        || opts.manifest
        || opts.provides
        || opts.requires
        || opts.backend_json
    {
        tebako_info::package::Depth::Manifests
    } else {
        tebako_info::package::Depth::Trailer
    };
    if opts.verify {
        let (checks, inspection) = tebako_info::verify::verify_package(path, opts.require_signed)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
        let code = tebako_info::verify::exit_code_of(&checks);
        if opts.json {
            let mut doc = match &inspection {
                Some(p) => tebako_info::package::package_json(p, depth, None),
                None => tebako_json::Value::Object(Vec::new()),
            };
            if let tebako_json::Value::Object(members) = &mut doc {
                members.push((
                    "checks".to_string(),
                    tebako_info::verify::checks_json(&checks),
                ));
            }
            return Ok((format!("{}\n", tebako_json::to_string(&doc)), code));
        }
        let mut out = match &inspection {
            Some(p) => tebako_info::package::render_full(p, depth, None),
            None => String::new(),
        };
        out.push_str(&tebako_info::verify::render_report(
            &path.display().to_string(),
            &checks,
        ));
        return Ok((out, code));
    }
    let p = tebako_info::package::inspect_package(path, depth, opts.slot)
        .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?;
    if opts.json {
        let doc = tebako_info::package::package_json(&p, depth, None);
        return Ok((format!("{}\n", tebako_json::to_string(&doc)), 0));
    }
    let out = match opts.slot {
        Some(n) => tebako_info::package::render_slot(&p, n)
            .map_err(|e| err(EX_TEBAKO_MANIFEST, e.to_string()))?,
        None => tebako_info::package::render_full(&p, depth, None),
    };
    Ok((out, 0))
}

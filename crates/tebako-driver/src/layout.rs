//! The env-image layout declaration (spec 18 C3):
//! `/lib/tebako/layout.yaml` inside the mounted env image, verified
//! post-mount, BEFORE any interpreter handoff — a mismatched exe↔image
//! pair is exit 78, never a ruby LoadError (S17–S19).
//!
//! The grammar's single owner is `docs/spec/schemas/layout.yaml`; this
//! module is its only reader in the product. Fields: `schema_version`,
//! `era`, `image_layout`, `mount_root`, `interpreter_api_version` — the
//! era-2 driver parses the last but does not gate on it — and the
//! additive `mount_root_override` (schema_minor 1): the image's grant
//! that its rbconfig follows `TEBAKO_MOUNT_ROOT`, gating the driver's
//! run-time root override (spec 17 §1); `preload_shim` (schema_minor 2)
//! and `runtime_dll` (schema_minor 3) — flowed into the handoff env by
//! the boot (see `crate::injection`); `interpreter` and `visibility`
//! (schema_minor 4, spec 29 §2/§3) — the wrapper-exe pattern's
//! declaration: parsed and flowed verbatim here, consumed and
//! grammar-checked by `crate::wrapper` (the keys are wrapper-only — the
//! linked driver never gates on them, and spec 29 §4 assigns the
//! wrapper's refusals exit 65, not this module's 78s).

use serde::Deserialize;

use crate::driver::DriverError;
use crate::EX_TEBAKO_LAYOUT;

/// The in-image path of the declaration, relative to the runtime root.
pub const LAYOUT_IMAGE_PATH: &str = "lib/tebako/layout.yaml";
/// Sanity bound on the declaration's size (a named error, never an
/// unbounded read).
pub(crate) const LAYOUT_MAX_BYTES: usize = 65536;
/// The layout.yaml schema MAJOR this driver speaks.
pub const LAYOUT_SCHEMA_VERSION: u32 = 1;
/// The image layout generation this driver speaks.
pub const IMAGE_LAYOUT_VERSION: u32 = 1;
/// The contract era this driver speaks (era 1 = the pre-era, refused).
pub const DRIVER_ERA: u32 = 2;

fn layout(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_LAYOUT, message.into())
}

/// The parsed declaration — every field required by the grammar
/// (`docs/spec/schemas/layout.yaml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageLayout {
    /// The layout.yaml schema MAJOR.
    pub schema_version: u32,
    /// The image's contract era (>= 2 once declared).
    pub era: u32,
    /// The image's internal layout generation.
    pub image_layout: u32,
    /// The mount root the image was built for — must equal the exe's
    /// compiled-in root (the value `tebako_main` forwards).
    pub mount_root: String,
    /// The interpreter API version line the image carries (parsed, not
    /// gated on in era 2).
    pub interpreter_api_version: String,
    /// The image's grant that its rbconfig follows `TEBAKO_MOUNT_ROOT`
    /// (additive, schema_minor 1): absent ⇒ `false` ⇒ the driver refuses
    /// a run-time root override by name (exit 78) rather than booting an
    /// interpreter whose load paths point at an unmounted root.
    pub mount_root_override: bool,
    /// The preload shim's in-image path, relative to the mount root
    /// (additive, schema_minor 2): the image declares, the driver flows
    /// it — materialized to the exec cache for the child-injection env
    /// and exported as `TEBAKO_PRELOAD_SHIM` for the interpreter's spawn
    /// hook (spec 22 §3; no second hand-written copy of the path).
    /// Absent ⇒ no preload env (an older image — children get no VFS).
    pub preload_shim: Option<String>,
    /// The runtime's own PE module basename, e.g. `x64-ucrt-ruby340.dll`
    /// (additive, schema_minor 3; MSYS images only): the image declares,
    /// the driver flows it — exported verbatim as `TEBAKO_RUNTIME_DLL`
    /// into the handoff env (spec 17 §2), where the tfs PE closure walk
    /// reads it to exclude the runtime's own DLL from payload
    /// materialization (spec 22 §2.1). Bare-name grammar enforced here
    /// (no separator, no drive qualifier — a violation is a named 78).
    /// Absent ⇒ no export (an older image — the exclusion is simply
    /// off).
    pub runtime_dll: Option<String>,
    /// The interpreter's in-image path (additive, schema_minor 4 —
    /// spec 29 §2): POSIX-absolute, resolved against the env image's
    /// mount by the wrapper exe. Absent ⇒ the linked pattern (this
    /// driver compiled into the interpreter). Flowed verbatim — the
    /// wrapper ([`crate::wrapper`]) validates form and resolution with
    /// the spec 29 §4 exit-65 refusals.
    pub interpreter: Option<String>,
    /// The wrapper's kernel-visibility mechanism (additive,
    /// schema_minor 4 — spec 29 §3): `preload` / `seccomp-notify` /
    /// `exec-cache`. Absent ⇒ the spec's locked default order. Flowed
    /// verbatim; the wrapper maps and honors it (or fails closed).
    pub visibility: Option<String>,
}

/// The tolerant serde view: every field optional so the checks below can
/// name each missing/older/newer case precisely; unknown keys tolerated
/// within the MAJOR (spec 18 §3.2).
#[derive(Deserialize)]
struct LayoutView {
    schema_version: Option<u32>,
    era: Option<u32>,
    image_layout: Option<u32>,
    mount_root: Option<String>,
    interpreter_api_version: Option<String>,
    mount_root_override: Option<bool>,
    preload_shim: Option<String>,
    runtime_dll: Option<String>,
    interpreter: Option<String>,
    visibility: Option<String>,
}

impl ImageLayout {
    /// Parse and verify the declaration against this exe's expectations
    /// (`runtime_root` is the exe's compiled-in root; `image` names the
    /// env image in messages). The refusal order follows spec 18:
    /// malformed → missing `schema_version` (era 1, §3.4) → newer MAJOR
    /// (upgrade) → era (pre-era / newer) → incomplete → `image_layout`
    /// newer → `mount_root` mismatch (S19). Every refusal is exit 78
    /// ([`EX_TEBAKO_LAYOUT`]); on success the parsed declaration is
    /// returned.
    pub fn check(text: &str, runtime_root: &str, image: &str) -> Result<ImageLayout, DriverError> {
        let view: LayoutView = serde_yml::from_str(text).map_err(|e| {
            layout(format!(
                "env image '{image}' carries a malformed /lib/tebako/layout.yaml ({e})"
            ))
        })?;
        let Some(schema_version) = view.schema_version else {
            return Err(layout(format!(
                "env image '{image}' layout.yaml declares no schema_version — pre-era document (era 1): regenerate the image with the current factory (spec 18 §3.4)"
            )));
        };
        if schema_version > LAYOUT_SCHEMA_VERSION {
            return Err(layout(format!(
                "env image '{image}' layout schema {schema_version} is newer than this driver speaks ({LAYOUT_SCHEMA_VERSION}) — upgrade your tebako"
            )));
        }
        let era = view.era.unwrap_or(1);
        if era < 2 {
            return Err(layout(format!(
                "env image '{image}' layout.yaml declares era {era} — pre-era images are refused by name: rebuild the runtime with the current factory (spec 18 C3)"
            )));
        }
        if era > DRIVER_ERA {
            return Err(layout(format!(
                "env image '{image}' is from a newer tebako (era {era}) — upgrade your tebako (speaks era {DRIVER_ERA})"
            )));
        }
        let (Some(image_layout), Some(mount_root), Some(api_version)) = (
            view.image_layout,
            view.mount_root,
            view.interpreter_api_version,
        ) else {
            return Err(layout(format!(
                "env image '{image}' layout.yaml is incomplete (image_layout, mount_root and interpreter_api_version are required) — rebuild the runtime with the current factory"
            )));
        };
        if image_layout > IMAGE_LAYOUT_VERSION {
            return Err(layout(format!(
                "env image '{image}' layout generation {image_layout} is newer than this runtime speaks ({IMAGE_LAYOUT_VERSION}) — upgrade your tebako"
            )));
        }
        if mount_root != runtime_root {
            return Err(layout(format!(
                "env image '{image}' was built for mount root '{mount_root}' but this runtime's root is '{runtime_root}' — a mismatched exe↔image pair (spec 18 C3)"
            )));
        }
        if api_version.is_empty() {
            return Err(layout(format!(
                "env image '{image}' layout.yaml carries an empty interpreter_api_version"
            )));
        }
        // schema_minor 3: the runtime DLL is a bare PE module basename —
        // a separator or drive qualifier is the declaration lying.
        let runtime_dll = view.runtime_dll.filter(|s| !s.is_empty());
        if let Some(dll) = &runtime_dll {
            if dll.bytes().any(|b| b == b'/' || b == b'\\' || b == b':') {
                return Err(layout(format!(
                    "env image '{image}' layout.yaml declares runtime_dll '{dll}' carrying a path separator or drive qualifier — a basename owns no path: rebuild the runtime with the current factory"
                )));
            }
        }
        Ok(ImageLayout {
            schema_version,
            era,
            image_layout,
            mount_root,
            interpreter_api_version: api_version,
            mount_root_override: view.mount_root_override.unwrap_or(false),
            preload_shim: view.preload_shim.filter(|s| !s.is_empty()),
            runtime_dll,
            interpreter: view.interpreter.filter(|s| !s.is_empty()),
            visibility: view.visibility.filter(|s| !s.is_empty()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "schema_version: 1\nera: 2\nimage_layout: 1\nmount_root: /__tfs__\ninterpreter_api_version: \"3.4\"\n";

    #[test]
    fn the_happy_path_parses_and_verifies() {
        let l = ImageLayout::check(GOOD, "/__tfs__", "/rt/ruby.tfs").unwrap();
        assert_eq!(
            l,
            ImageLayout {
                schema_version: 1,
                era: 2,
                image_layout: 1,
                mount_root: "/__tfs__".to_string(),
                interpreter_api_version: "3.4".to_string(),
                mount_root_override: false,
                preload_shim: None,
                runtime_dll: None,
                interpreter: None,
                visibility: None,
            }
        );
        // unknown keys within the MAJOR are tolerated (spec 18 §3.2 / S57)
        let with_future = format!("{GOOD}future_field: {{anything: goes}}\n");
        ImageLayout::check(&with_future, "/__tfs__", "/rt/ruby.tfs").unwrap();
        // the windows root spelling pairs the same way
        let win = GOOD.replace("/__tfs__", "A:/t");
        ImageLayout::check(&win, "A:/t", "C:/rt/ruby.tfs").unwrap();
    }

    #[test]
    fn the_override_grant_is_additive_and_defaults_closed() {
        // absent (the pre-override era's images) ⇒ closed
        assert!(
            !ImageLayout::check(GOOD, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .mount_root_override
        );
        // declared ⇒ the image's rbconfig follows TEBAKO_MOUNT_ROOT
        let granted = format!("{GOOD}mount_root_override: true\n");
        assert!(
            ImageLayout::check(&granted, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .mount_root_override
        );
    }

    #[test]
    fn the_preload_shim_declaration_is_additive() {
        // absent (older images) ⇒ no preload env downstream
        assert_eq!(
            ImageLayout::check(GOOD, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .preload_shim,
            None
        );
        // declared (schema_minor 2) ⇒ the in-image path flows
        let declared = format!("{GOOD}preload_shim: lib/tebako/libtfs_preload.so\n");
        assert_eq!(
            ImageLayout::check(&declared, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .preload_shim
                .as_deref(),
            Some("lib/tebako/libtfs_preload.so")
        );
        // an empty declaration is no declaration
        let empty = format!("{GOOD}preload_shim: \"\"\n");
        assert_eq!(
            ImageLayout::check(&empty, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .preload_shim,
            None
        );
    }

    #[test]
    fn the_runtime_dll_declaration_is_additive_and_grammar_checked() {
        // absent (older images) ⇒ no TEBAKO_RUNTIME_DLL export downstream
        assert_eq!(
            ImageLayout::check(GOOD, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .runtime_dll,
            None
        );
        // declared (schema_minor 3) ⇒ the PE module basename flows
        // verbatim (the reader — the tfs PE closure walk — lowercases
        // for the windows loader's comparison)
        let declared = format!("{GOOD}runtime_dll: x64-ucrt-ruby340.dll\n");
        assert_eq!(
            ImageLayout::check(&declared, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .runtime_dll
                .as_deref(),
            Some("x64-ucrt-ruby340.dll")
        );
        // an empty declaration is no declaration
        let empty = format!("{GOOD}runtime_dll: \"\"\n");
        assert_eq!(
            ImageLayout::check(&empty, "/__tfs__", "/rt/ruby.tfs")
                .unwrap()
                .runtime_dll,
            None
        );
        // a separator or drive qualifier is the declaration lying — a
        // named 78, never a flowed path (the spec 03 §2.5 byte rule)
        for bad in ["lib/x64-ucrt-ruby340.dll", "lib\\x.dll", "C:x.dll"] {
            let text = format!("{GOOD}runtime_dll: {bad}\n");
            let err = ImageLayout::check(&text, "/__tfs__", "/rt/ruby.tfs").unwrap_err();
            assert_eq!(err.code, 78, "{bad}: {}", err.message);
            assert!(err.message.contains(bad), "{err}");
            assert!(err.message.contains("/rt/ruby.tfs"), "{err}");
        }
    }

    #[test]
    fn the_wrapper_keys_are_additive_and_flow_verbatim() {
        // absent (linked-pattern images) ⇒ None on both — the wrapper
        // refuses by name downstream (spec 29 §2), the linked driver
        // never reads them
        let plain = ImageLayout::check(GOOD, "/__tfs__", "/rt/ruby.tfs").unwrap();
        assert_eq!(plain.interpreter, None);
        assert_eq!(plain.visibility, None);
        // declared ⇒ flowed verbatim (the wrapper grammar-checks — spec
        // 29 §4's refusals are exit 65, not this module's 78)
        let declared = format!("{GOOD}interpreter: /bin/java\nvisibility: preload\n");
        let l = ImageLayout::check(&declared, "/__tfs__", "/rt/java.tfs").unwrap();
        assert_eq!(l.interpreter.as_deref(), Some("/bin/java"));
        assert_eq!(l.visibility.as_deref(), Some("preload"));
        // an empty declaration is no declaration
        let empty = format!("{GOOD}interpreter: \"\"\nvisibility: \"\"\n");
        let l = ImageLayout::check(&empty, "/__tfs__", "/rt/java.tfs").unwrap();
        assert_eq!(l.interpreter, None);
        assert_eq!(l.visibility, None);
    }

    #[test]
    fn every_refusal_is_exit_78() {
        for (name, text) in [
            ("malformed", "schema_version: [1\n"),
            ("no schema_version", "era: 2\n"),
            (
                "newer MAJOR",
                &GOOD.replace("schema_version: 1", "schema_version: 2"),
            ),
            ("era 1", &GOOD.replace("era: 2", "era: 1")),
            ("era newer", &GOOD.replace("era: 2", "era: 3")),
            ("incomplete", "schema_version: 1\nera: 2\n"),
            (
                "layout newer",
                &GOOD.replace("image_layout: 1", "image_layout: 2"),
            ),
            ("root mismatch", &GOOD.replace("/__tfs__", "/__other__")),
            ("empty api", &GOOD.replace("\"3.4\"", "\"\"")),
        ] {
            let err = ImageLayout::check(text, "/__tfs__", "/rt/ruby.tfs").unwrap_err();
            assert_eq!(err.code, 78, "{name}: {}", err.message);
        }
    }

    #[test]
    fn the_named_messages_carry_both_sides() {
        // S18: the upgrade refusal names both versions
        let err = ImageLayout::check(
            &GOOD.replace("schema_version: 1", "schema_version: 2"),
            "/__tfs__",
            "/rt/ruby.tfs",
        )
        .unwrap_err();
        assert!(err.message.contains("schema 2"), "{err}");
        assert!(err.message.contains("speaks (1)"), "{err}");
        assert!(err.message.contains("upgrade your tebako"), "{err}");

        // S19: the mount_root mismatch prints both values
        let err = ImageLayout::check(
            &GOOD.replace("/__tfs__", "/__other__"),
            "/__tfs__",
            "/rt/ruby.tfs",
        )
        .unwrap_err();
        assert!(err.message.contains("'/__other__'"), "{err}");
        assert!(err.message.contains("'/__tfs__'"), "{err}");
        assert!(err.message.contains("/rt/ruby.tfs"), "{err}");

        // the pre-era refusal names the image and the factory remedy
        let err = ImageLayout::check("era: 2\n", "/__tfs__", "/rt/ruby.tfs").unwrap_err();
        assert!(err.message.contains("pre-era"), "{err}");
        assert!(err.message.contains("/rt/ruby.tfs"), "{err}");
    }
}

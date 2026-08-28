//! The `TEBAKO_TFS_MOUNTS` mount-spec grammar (spec 07 §8 tier 1, spec 17
//! §2.1) — the env form that seeds a process's mount table, shared by the
//! preload shim (which parses the env var at init) and `tfs exec` (which
//! validates and re-serializes it for the exec'd child). One grammar, one
//! parser, one serializer.
//!
//! A REPEATED mount point declares union members in shadow order — the
//! serialization of a spec 17 §1 union mount is the incumbent's
//! declaration followed by each member's at the same point. Consumers
//! that can union (the preload shim) layer each later declaration over
//! the earlier; consumers that cannot fail closed.
//!
//! Pure safe Rust; named errors on malformed input (spec 14 §3).

use std::fmt;
use std::path::Path;

/// One `image[:slot]:mount` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountDecl {
    /// Absolute path of the image file on the host.
    pub image: String,
    /// The package slot to mount when the image is a stitched tpkg
    /// package; `None` mounts the whole file (a bare image). Parsed from
    /// an all-digits field; `-` on the wire means the same as absent.
    pub slot: Option<u32>,
    /// Absolute virtual mount point (never `/` — see [`parse_mount_entry`]).
    pub mount: String,
}

/// A named, human-readable mount-spec parse error (the offending entry is
/// always quoted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpecError(pub String);

impl fmt::Display for MountSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid mount spec: {}", self.0)
    }
}

impl std::error::Error for MountSpecError {}

/// Split one entry from the RIGHT (spec 17 §2.1): the mount is the field
/// after the last ':'; a slot is recognized only when the remainder's
/// rightmost field is all digits or exactly '-' — anything else stays
/// part of the image path, which may itself contain colons (and, on
/// windows, a drive colon). `-` parses to `None` (whole file).
fn split_entry(entry: &str) -> Result<(&str, Option<u32>, &str), MountSpecError> {
    let Some((rest, mount)) = entry.rsplit_once(':') else {
        return Err(MountSpecError(format!(
            "entry {entry:?} needs the image[:slot]:mount shape"
        )));
    };
    if let Some((image, field)) = rest.rsplit_once(':') {
        if field == "-" {
            return Ok((image, None, mount));
        }
        if !field.is_empty() && field.bytes().all(|b| b.is_ascii_digit()) {
            let slot = field.parse::<u32>().map_err(|_| {
                MountSpecError(format!("slot {field:?} out of range in entry {entry:?}"))
            })?;
            return Ok((image, Some(slot), mount));
        }
    }
    Ok((rest, None, mount))
}

/// Validate one `image[:slot]:mount` triple (shared tail of the env and
/// CLI forms).
fn validate(
    image: &str,
    slot: Option<u32>,
    mount: &str,
    context: &str,
) -> Result<MountDecl, MountSpecError> {
    let err = |msg: &str| MountSpecError(format!("{msg} in {context:?}"));
    if image.is_empty() {
        return Err(err("empty image path"));
    }
    if !Path::new(image).is_absolute() {
        return Err(err("image path is not absolute"));
    }
    if !mount.starts_with('/') {
        return Err(err("mount point is not absolute"));
    }
    // A mount at "/" is legitimate (the app payload mounts there, spec
    // 17): covered-but-not-held paths fall through to the host WITH the
    // policy gate consulted (spec 08), so the jail is engaged exactly as
    // for any other mount. (An earlier revision rejected "/" outright on
    // the grounds that longest-prefix dispatch would swallow the host;
    // the passthrough decision moots that.)
    Ok(MountDecl {
        image: image.to_string(),
        slot,
        mount: mount.to_string(),
    })
}

/// Parse one `image[:slot]:mount` entry. Split at the LAST ':' so image
/// paths containing ':' survive; the slot field is recognized only on an
/// exact all-digits (or `-`) rightmost remainder (spec 17 §2.1).
pub fn parse_mount_entry(entry: &str) -> Result<MountDecl, MountSpecError> {
    if entry.is_empty() {
        return Err(MountSpecError("empty entry".to_string()));
    }
    let (image, slot, mount) = split_entry(entry)?;
    validate(image, slot, mount, entry)
}

/// Parse the `TEBAKO_TFS_MOUNTS` env form:
/// `image[:slot]:mount,image[:slot]:mount,…`.
pub fn parse_mounts(spec: &str) -> Result<Vec<MountDecl>, MountSpecError> {
    if spec.trim().is_empty() {
        return Err(MountSpecError("empty spec".to_string()));
    }
    let mut out = Vec::new();
    for entry in spec.split(',') {
        if entry.is_empty() {
            return Err(MountSpecError(format!(
                "empty entry (stray ',') in {spec:?}"
            )));
        }
        out.push(parse_mount_entry(entry)?);
    }
    Ok(out)
}

/// Serialize mount declarations to the `TEBAKO_TFS_MOUNTS` env form (the
/// inverse of [`parse_mounts`]). The slot field is emitted iff the mount
/// was established from a package slot; `-` is never spelled (spec 17
/// §2.1's emit rule).
pub fn to_env_spec(decls: &[MountDecl]) -> String {
    decls
        .iter()
        .map(|d| match d.slot {
            Some(slot) => format!("{}:{}:{}", d.image, slot, d.mount),
            None => format!("{}:{}", d.image, d.mount),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Parse the `tfs exec` CLI form of one image argument: `image[:mount]`,
/// default mount `/mnt` (the tfs-cli convention). The ':' is a delimiter
/// only when what follows it looks like a mount point (starts with '/'),
/// so a bare image path containing ':' is still accepted. The CLI form
/// has no slot field (`slot` is always `None`).
pub fn parse_cli_image_mount(token: &str) -> Result<MountDecl, MountSpecError> {
    match token.rsplit_once(':') {
        Some((image, mount)) if mount.starts_with('/') => validate(image, None, mount, token),
        _ => {
            if token.is_empty() {
                return Err(MountSpecError("empty image argument".to_string()));
            }
            validate(token, None, "/mnt", token)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(image: &str, slot: Option<u32>, mount: &str) -> MountDecl {
        MountDecl {
            image: image.to_string(),
            slot,
            mount: mount.to_string(),
        }
    }

    #[test]
    fn parses_env_form() {
        let decls = parse_mounts("/a/img.zip:/tfs,/b/other.zip:/data").unwrap();
        assert_eq!(
            decls,
            vec![
                decl("/a/img.zip", None, "/tfs"),
                decl("/b/other.zip", None, "/data"),
            ]
        );
    }

    #[test]
    fn env_round_trip_is_identity() {
        let spec = "/a/img.zip:/tfs,/b/other.zip:/data";
        assert_eq!(to_env_spec(&parse_mounts(spec).unwrap()), spec);
    }

    #[test]
    fn image_path_may_contain_colons() {
        let d = parse_mount_entry("/Volumes/a:b/img.zip:/tfs").unwrap();
        assert_eq!(d.image, "/Volumes/a:b/img.zip");
        assert_eq!(d.slot, None);
        assert_eq!(d.mount, "/tfs");
    }

    #[test]
    fn parses_slot_form() {
        let d = parse_mount_entry("/a/pkg.tebako:0:/tfs").unwrap();
        assert_eq!(d, decl("/a/pkg.tebako", Some(0), "/tfs"));
        let d = parse_mount_entry("/b/pkg.tebako:12:/data").unwrap();
        assert_eq!(d, decl("/b/pkg.tebako", Some(12), "/data"));
    }

    #[test]
    fn slot_round_trip_is_identity() {
        let spec = "/a/pkg.tebako:0:/tfs,/b/img.zip:/data";
        assert_eq!(to_env_spec(&parse_mounts(spec).unwrap()), spec);
    }

    #[test]
    fn dash_slot_means_whole_file() {
        // '-' on the wire is the same as absent, and is never re-spelled.
        let d = parse_mount_entry("/a/img.zip:-:/tfs").unwrap();
        assert_eq!(d, decl("/a/img.zip", None, "/tfs"));
        assert_eq!(to_env_spec(&[d]), "/a/img.zip:/tfs");
    }

    #[test]
    fn slot_field_recognized_only_on_exact_match() {
        // A non-digit, non-'-' field stays part of the image path (spec
        // 17 §2.1: paths may contain colons; the grammar never rejects
        // them).
        let d = parse_mount_entry("/a.tfs:q:/mnt").unwrap();
        assert_eq!(d, decl("/a.tfs:q", None, "/mnt"));
    }

    #[test]
    fn slot_overflow_is_a_named_error() {
        let e = parse_mount_entry("/a.zip:99999999999999999999:/mnt").unwrap_err();
        assert!(
            e.0.contains("slot") && e.0.contains("out of range"),
            "unexpected error: {e:?}"
        );
    }

    #[test]
    fn windows_drive_shapes_split_from_the_right() {
        // split_entry is validation-free, so the windows shapes parse
        // identically on every host (spec 17 §2.1's examples).
        let (image, slot, mount) = split_entry("C:\\pkg.tebako:0:/__tfs__").unwrap();
        assert_eq!(
            (image, slot, mount),
            ("C:\\pkg.tebako", Some(0), "/__tfs__")
        );
        let (image, slot, mount) = split_entry("C:\\image.tfs:/data").unwrap();
        assert_eq!((image, slot, mount), ("C:\\image.tfs", None, "/data"));
    }

    #[test]
    fn rejects_malformed_specs_with_named_errors() {
        for (spec, frag) in [
            ("", "empty spec"),
            ("  ", "empty spec"),
            ("/a.zip", "image[:slot]:mount"),
            ("relative.zip:/tfs", "not absolute"),
            ("/a.zip:tfs", "not absolute"),
            ("/a.zip:/tfs,", "empty entry"),
            (",/a.zip:/tfs", "empty entry"),
            (":/tfs", "empty image"),
        ] {
            let e = parse_mounts(spec).unwrap_err();
            assert!(
                e.0.contains(frag),
                "spec {spec:?}: error {e:?} should mention {frag:?}"
            );
        }
    }

    #[test]
    fn root_mount_is_legitimate() {
        // the app payload mounts at "/" (spec 17); covered-but-not-held
        // paths fall through to the host with the policy gate consulted
        let d = parse_mounts("/a.zip:/").unwrap();
        assert_eq!(d[0].mount, "/");
    }

    #[test]
    fn cli_form_defaults_mount_to_mnt() {
        let d = parse_cli_image_mount("/a/img.zip").unwrap();
        assert_eq!(d.mount, "/mnt");
        assert_eq!(d.slot, None);
        let d = parse_cli_image_mount("/a/img.zip:/tfs").unwrap();
        assert_eq!(d.image, "/a/img.zip");
        assert_eq!(d.mount, "/tfs");
        // A bare image path containing ':' stays one image.
        let d = parse_cli_image_mount("/Volumes/a:b/img.zip").unwrap();
        assert_eq!(d.image, "/Volumes/a:b/img.zip");
        assert_eq!(d.mount, "/mnt");
        assert!(parse_cli_image_mount("").is_err());
        assert!(parse_cli_image_mount("rel.zip").is_err());
        assert_eq!(parse_cli_image_mount("/a.zip:/").unwrap().mount, "/");
    }
}

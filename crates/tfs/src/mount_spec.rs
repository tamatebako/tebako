//! The `TEBAKO_TFS_MOUNTS` mount-spec grammar (spec 07 §8 tier 1) — the
//! env form that seeds a process's mount table, shared by the preload
//! shim (which parses the env var at init) and `tfs exec` (which
//! validates and re-serializes it for the exec'd child). One grammar,
//! one parser, one serializer.
//!
//! Pure safe Rust; named errors on malformed input (spec 14 §3).

use std::fmt;
use std::path::Path;

/// One `image:mount` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountDecl {
    /// Absolute path of the image file on the host.
    pub image: String,
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

/// Validate one `image:mount` pair (shared tail of the env and CLI forms).
fn validate(image: &str, mount: &str, context: &str) -> Result<MountDecl, MountSpecError> {
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
        mount: mount.to_string(),
    })
}

/// Parse one `image:mount` entry. Split at the LAST ':' so image paths
/// containing ':' survive.
pub fn parse_mount_entry(entry: &str) -> Result<MountDecl, MountSpecError> {
    if entry.is_empty() {
        return Err(MountSpecError("empty entry".to_string()));
    }
    let Some((image, mount)) = entry.rsplit_once(':') else {
        return Err(MountSpecError(format!(
            "entry {entry:?} needs the image:mount shape"
        )));
    };
    validate(image, mount, entry)
}

/// Parse the `TEBAKO_TFS_MOUNTS` env form: `image:mount,image:mount,…`.
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
/// inverse of [`parse_mounts`]).
pub fn to_env_spec(decls: &[MountDecl]) -> String {
    decls
        .iter()
        .map(|d| format!("{}:{}", d.image, d.mount))
        .collect::<Vec<_>>()
        .join(",")
}

/// Parse the `tfs exec` CLI form of one image argument: `image[:mount]`,
/// default mount `/mnt` (the tfs-cli convention). The ':' is a delimiter
/// only when what follows it looks like a mount point (starts with '/'),
/// so a bare image path containing ':' is still accepted.
pub fn parse_cli_image_mount(token: &str) -> Result<MountDecl, MountSpecError> {
    match token.rsplit_once(':') {
        Some((image, mount)) if mount.starts_with('/') => validate(image, mount, token),
        _ => {
            if token.is_empty() {
                return Err(MountSpecError("empty image argument".to_string()));
            }
            validate(token, "/mnt", token)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_form() {
        let decls = parse_mounts("/a/img.zip:/tfs,/b/other.zip:/data").unwrap();
        assert_eq!(
            decls,
            vec![
                MountDecl {
                    image: "/a/img.zip".to_string(),
                    mount: "/tfs".to_string(),
                },
                MountDecl {
                    image: "/b/other.zip".to_string(),
                    mount: "/data".to_string(),
                },
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
        assert_eq!(d.mount, "/tfs");
    }

    #[test]
    fn rejects_malformed_specs_with_named_errors() {
        for (spec, frag) in [
            ("", "empty spec"),
            ("  ", "empty spec"),
            ("/a.zip", "image:mount"),
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

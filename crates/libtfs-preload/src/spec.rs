//! The shim's configuration surface: the `TEBAKO_TFS_MOUNTS` grammar is
//! [`tfs::mount_spec`] (one grammar, one parser — shared with `tfs exec`,
//! re-exported here for convenience); this module adds the shim's own
//! named exit code.
//!
//! Pure safe Rust.

#![forbid(unsafe_code)]

pub use tfs::mount_spec::{
    parse_cli_image_mount, parse_mount_entry, parse_mounts, to_env_spec, MountDecl, MountSpecError,
};

/// Exit code for shim-init configuration failures (misformatted
/// `TEBAKO_TFS_MOUNTS` / `TEBAKO_JAIL`, unmountable image): sysexits
/// `EX_CONFIG`. The shim writes a clear stderr message naming the env var
/// and the offending token, then exits with this code.
pub const EX_CONFIG: i32 = 78;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_resolve() {
        let decls = parse_mounts("/a/img.zip:/tfs").unwrap();
        assert_eq!(to_env_spec(&decls), "/a/img.zip:/tfs");
        // a `/` mount is legitimate (the app payload mounts there; the
        // covered-but-not-held passthrough keeps the jail engaged)
        assert_eq!(parse_mounts("/a.zip:/").unwrap()[0].mount, "/");
        assert_eq!(parse_cli_image_mount("/a.zip").unwrap().mount, "/mnt");
        let _ = MountSpecError("x".to_string());
        let _ = MountDecl {
            image: "/a".to_string(),
            slot: None,
            mount: "/t".to_string(),
        };
    }

    #[test]
    fn ex_config_is_sysexits_78() {
        assert_eq!(EX_CONFIG, 78);
    }
}

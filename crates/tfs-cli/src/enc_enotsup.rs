//! The windows surface of the ENC verbs (TODO.v2-1/02): the ENC
//! transform ships only in the POSIX tfs build (rnp's mingw build is
//! unproven — TODO.v2-1/08), so on windows every encryption verb is the
//! named ENOTSUP error — never a compile error, never a silent skip.
//!
//! Same module API as enc.rs (the POSIX implementation) — keep the two
//! in lockstep (the `#[path]` split lives in lib.rs).

use std::path::{Path, PathBuf};

/// The named ENOTSUP error with the crate's uniform error exit (1) —
/// mirroring the exec verb's not-unix stub.
fn enotsup<T>() -> Result<T, (String, i32)> {
    Err((
        "ENOTSUP: the encryption verbs are not available in this build \
         (the ENC transform ships in the POSIX tfs build only — TODO.v2-1/08)"
            .to_string(),
        1,
    ))
}

/// One `--subtree <path>=<pubkey-file>` grant of `tfs encrypt`.
#[derive(Debug, Clone)]
pub struct SubtreeGrant {
    /// The subtree root (absolute, e.g. "/a/b").
    pub path: String,
    /// The recipient public key file.
    pub public_key: PathBuf,
}

/// Options of `tfs encrypt`.
#[derive(Debug, Clone, Default)]
pub struct EncryptOptions {
    /// Root-grant recipient public key files (≥ 1 required).
    pub recipients: Vec<PathBuf>,
    /// Subtree grants (selective disclosure).
    pub subtrees: Vec<SubtreeGrant>,
}

/// `tfs encrypt` — the named ENOTSUP error (see the module docs).
pub fn cmd_encrypt(_src: &Path, _out: &Path, _opts: &EncryptOptions) -> Result<(), (String, i32)> {
    enotsup()
}

/// `tfs encrypt --rewrap` — the named ENOTSUP error (see the module docs).
pub fn cmd_rewrap(
    _src: &Path,
    _out: &Path,
    _key_file: &Path,
    _recipients: &[PathBuf],
) -> Result<(), (String, i32)> {
    enotsup()
}

/// `tfs decrypt` — the named ENOTSUP error (see the module docs).
pub fn cmd_decrypt(_src: &Path, _out: &Path, _key_file: &Path) -> Result<(), (String, i32)> {
    enotsup()
}

/// `tfs mount --key` — the named ENOTSUP error (see the module docs).
pub fn cmd_mount_enc(_image: &Path, _key_file: &Path) -> Result<String, (String, i32)> {
    enotsup()
}

//! The encryption verbs (spec 10 §7): `tfs encrypt` / `tfs decrypt` /
//! `tfs mount --key`, plus envelope re-wrap rotation.
//!
//! - **encrypt** mounts a plaintext image, derives the key schedule
//!   (root DEK + HKDF subtree keys), wraps the grants to the recipients
//!   (rnp PKESK — tebako-signer), stages the CIPHERTEXT tree (plaintext
//!   never touches disk — the staging area holds ciphertext, the
//!   plaintext `/__tpkg__/` metadata, and nothing else), and writes the
//!   encrypted image through the same dwarfs-t Writer path as mkimage.
//! - **decrypt** unlocks the image with the recipient key and streams
//!   the plaintext tree into a tar file — plaintext flows mount → tar
//!   stream, never a staging tree (the explicit output is the only
//!   plaintext on disk).
//! - **mount** unlocks the image with the recipient key and reports the
//!   opened grant (the unlock/grant surface; FUSE/serve mounts are
//!   spec-11 §6 PLANNED). Wrong key → the named EKEY error.
//! - **rewrap** rotates grants to a new recipient set: unwrap each
//!   grant the presented key opens, re-wrap to the new recipients, and
//!   copy the bulk ciphertext BYTE-IDENTICAL (never re-encrypted).
//!
//! Encryption is opt-in, never default (spec 10): nothing here runs
//! unless one of these verbs is invoked explicitly.

use std::io::Read;
use std::path::{Path, PathBuf};

use tfs::backends_enc::{self, EncBackend, KeySource, BLOCK_SIZE, ENOKEY};
use tfs::{Backend, EntryType, RawStat};

/// The `algorithm` id recorded in the payload manifest's
/// `encryption.parts` (the block cipher; the suite registry id is in
/// the envelope manifest — spec 10 §5).
const ALGORITHM_ID: &str = "aes-256-gcm";

const MANIFEST_BACKEND_PATH: &str = "__tpkg__/manifest.yaml";
const ENVELOPES_BACKEND_PATH: &str = "__tpkg__/envelopes.yaml";

/// The (message, exit-code) pair every CLI engine error is built from.
fn et(msg: impl Into<String>) -> (String, i32) {
    (msg.into(), 1)
}

fn err<T>(msg: impl Into<String>) -> Result<T, (String, i32)> {
    Err(et(msg))
}

fn io_et(what: impl Into<String>, e: impl std::fmt::Display) -> (String, i32) {
    (format!("{}: {e}", what.into()), 1)
}

fn errno_err<T>(what: impl Into<String>, e: i32) -> Result<T, (String, i32)> {
    err(format!("{} (errno {e})", what.into()))
}

/// The named EKEY-class CLI error (spec 10 §7 — never garbage).
fn ekey_err<T>(detail: impl Into<String>) -> Result<T, (String, i32)> {
    err(format!("EKEY: {}", detail.into()))
}

// ---------------------------------------------------------------------
// Mount + small backend helpers
// ---------------------------------------------------------------------

/// Mount an image read-only via the tfs detection chain.
fn mount_image(image: &Path) -> Result<tfs::context::Mount, (String, i32)> {
    tfs::mount::build_from_file(&image.to_string_lossy(), "/mnt")
        .map_err(|e| (format!("cannot mount {} (errno {e})", image.display()), 1))
}

/// Read a whole backend file (bounded).
fn read_backend_file(
    backend: &dyn Backend,
    path: &str,
    max: i64,
) -> Result<Vec<u8>, (String, i32)> {
    let st = backend
        .stat(path)
        .map_err(|e| (format!("cannot stat {path} (errno {e})"), 1))?;
    if st.entry_type != EntryType::File || st.size < 0 || st.size > max {
        return err(format!(
            "cannot read {path}: not a regular file or too large"
        ));
    }
    let mut buf = vec![0u8; st.size as usize];
    let mut off = 0u64;
    while off < st.size as u64 {
        let n = backend
            .pread(path, &mut buf[off as usize..], off)
            .map_err(|e| (format!("cannot read {path} (errno {e})"), 1))?;
        if n == 0 {
            return err(format!("short read on {path}"));
        }
        off += n as u64;
    }
    Ok(buf)
}

/// Every entry of the tree, directories before their contents
/// (pre-order), stable within each directory.
fn walk(backend: &dyn Backend, dir: &str, out: &mut Vec<(String, RawStat)>) -> Result<(), i32> {
    for e in backend.read_dir(dir)? {
        let path = if dir.is_empty() {
            e.name.clone()
        } else {
            format!("{dir}/{}", e.name)
        };
        let st = backend.stat(&path)?;
        out.push((path.clone(), st));
        if st.entry_type == EntryType::Directory {
            walk(backend, &path, out)?;
        }
    }
    Ok(())
}

/// Build the image from a staged tree (the mkimage writer path; the
/// staged manifest is already final — no stamping here).
fn write_image(staging: &Path, out: &Path) -> Result<(), (String, i32)> {
    if out.exists() {
        std::fs::remove_file(out)
            .map_err(|e| io_et(format!("cannot replace {}", out.display()), e))?;
    }
    let mut writer = dwarfs_t::Writer::new(dwarfs_t::WriterOptions::default())
        .map_err(|e| et(format!("dwarfs writer: {e}")))?;
    writer.add_tree(staging, "/").map_err(|e| {
        et(format!(
            "dwarfs writer: scanning {}: {e}",
            staging.display()
        ))
    })?;
    writer
        .write(out)
        .map_err(|e| et(format!("dwarfs writer: {}: {e}", out.display())))
}

/// Read a public/secret key file.
fn read_key_file(path: &Path) -> Result<Vec<u8>, (String, i32)> {
    std::fs::read(path)
        .map_err(|e| io_et(format!("cannot read the key file {}", path.display()), e))
}

/// Loaded recipient public keys with their keyids.
type RecipientKeys = (Vec<Vec<u8>>, Vec<String>);

/// Load recipient public keys (files) with their keyids.
fn load_recipients(files: &[PathBuf]) -> Result<RecipientKeys, (String, i32)> {
    let mut pubs = Vec::with_capacity(files.len());
    let mut keyids = Vec::with_capacity(files.len());
    for f in files {
        let public = read_key_file(f)?;
        let keyid = tebako_signer::public_key_keyid(&public)
            .map_err(|e| et(format!("{}: {e}", f.display())))?;
        pubs.push(public);
        keyids.push(keyid);
    }
    Ok((pubs, keyids))
}

/// Stage one symlink; ENOTSUP from the backend is the named capability
/// error (the dwarfs-t C ABI readlink binding is pending).
fn stage_symlink(backend: &dyn Backend, path: &str, dest: &Path) -> Result<(), (String, i32)> {
    let target = backend.read_link(path).map_err(|e| {
        (
            format!(
                "cannot read the symlink target of {path} (errno {e}): this backend cannot expose targets yet"
            ),
            1,
        )
    })?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, dest)
        .map_err(|e| io_et(format!("cannot stage the symlink {path}"), e))?;
    #[cfg(not(unix))]
    std::fs::write(dest, target.as_bytes())
        .map_err(|e| io_et(format!("cannot stage the symlink {path}"), e))?;
    Ok(())
}

/// Best-effort permission preservation onto a staged entry.
#[cfg(unix)]
fn stage_perms(dest: &Path, perms: u32) {
    use std::os::unix::fs::PermissionsExt as _;
    let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(perms));
}

// ---------------------------------------------------------------------
// encrypt
// ---------------------------------------------------------------------

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

/// Derive a subtree key from the root DEK down an absolute path.
fn subtree_key(dek: &[u8; 32], abs_path: &str) -> [u8; 32] {
    let mut key = *dek;
    for part in abs_path.trim_matches('/').split('/') {
        if !part.is_empty() {
            key = backends_enc::derive_dir_key(&key, part);
        }
    }
    key
}

/// `tfs encrypt <src-image> -o <out.tfs> --recipient … [--subtree …]`.
pub fn cmd_encrypt(src: &Path, out: &Path, opts: &EncryptOptions) -> Result<(), (String, i32)> {
    if opts.recipients.is_empty() {
        return err("encrypt requires at least one --recipient (the root grant)");
    }
    let mount = mount_image(src)?;
    let backend = &*mount.backend;

    // The payload manifest is required: the encrypted image declares
    // its encryption state there, and its tree_hash is the PLAINTEXT
    // identity (spec 10 §2) — recomputed over the source tree so the
    // stamp never inherits a placeholder.
    let manifest_raw = read_backend_file(backend, MANIFEST_BACKEND_PATH, 1 << 20).map_err(|_| {
        et("encrypt requires a payload manifest in the source image (tfs mkimage stamps tree_hash at image creation)")
    })?;
    let manifest_text = String::from_utf8(manifest_raw)
        .map_err(|_| et("the source payload manifest is not UTF-8"))?;
    let mut manifest = tpkg::PayloadManifest::from_yaml(&manifest_text)
        .map_err(|e| et(format!("the source payload manifest is not valid: {e}")))?;
    if manifest.identity.encryption.state == tpkg::EncryptionState::Encrypted {
        return err("the source image is already encrypted (nested encapsulated images are a later milestone — spec 10 §2)");
    }
    let digest = tpkg::tree_digest(&tfs::tree_walk::BackendTree(backend))
        .map_err(|e| (format!("cannot hash the source tree (errno {e})"), 1))?;
    manifest.identity.digest.tree_hash = tpkg::render_tree_hash(&digest);

    // The key schedule: a fresh root DEK, HKDF subtree keys.
    let dek = backends_enc::generate_dek()
        .map_err(|e| (format!("cannot generate the DEK (errno {e})"), 1))?;
    let mut grants = Vec::new();
    let (root_pubs, root_keyids) = load_recipients(&opts.recipients)?;
    let root_refs: Vec<&[u8]> = root_pubs.iter().map(Vec::as_slice).collect();
    let envelope = tebako_signer::wrap_dek(&dek, &root_refs)
        .map_err(|e| et(format!("cannot wrap the root grant: {e}")))?;
    grants.push(tpkg::Grant {
        id: "/".to_string(),
        path: "/".to_string(),
        recipients: root_keyids,
        envelope: String::from_utf8(envelope)
            .map_err(|_| et("the root envelope is not UTF-8 (armored output expected)"))?,
    });

    for st in &opts.subtrees {
        if !st.path.starts_with('/') || st.path == "/" {
            return err(format!(
                "--subtree path must be absolute and not the root (the root grant is --recipient): {}",
                st.path
            ));
        }
        let rel = st.path.trim_matches('/');
        if rel == "__tpkg__" || rel.starts_with("__tpkg__/") {
            return err("--subtree grants do not apply to the /__tpkg__/ metadata directory");
        }
        match backend.stat(rel) {
            Ok(s) if s.entry_type == EntryType::Directory => {}
            Ok(_) => return err(format!("--subtree path is not a directory: {}", st.path)),
            Err(e) => {
                return errno_err(
                    format!("--subtree path {} not found in the source", st.path),
                    e,
                )
            }
        }
        let key = subtree_key(&dek, &st.path);
        let (pubs, keyids) = load_recipients(std::slice::from_ref(&st.public_key))?;
        let refs: Vec<&[u8]> = pubs.iter().map(Vec::as_slice).collect();
        let envelope = tebako_signer::wrap_dek(&key, &refs)
            .map_err(|e| et(format!("cannot wrap the {} grant: {e}", st.path)))?;
        grants.push(tpkg::Grant {
            id: st.path.clone(),
            path: st.path.clone(),
            recipients: keyids,
            envelope: String::from_utf8(envelope)
                .map_err(|_| et("a subtree envelope is not UTF-8 (armored output expected)"))?,
        });
    }

    manifest.identity.encryption = tpkg::Encryption {
        state: tpkg::EncryptionState::Encrypted,
        parts: grants
            .iter()
            .map(|g| tpkg::EncryptionPart {
                paths: vec![g.path.clone()],
                algorithm: ALGORITHM_ID.to_string(),
                envelope_refs: vec![g.id.clone()],
            })
            .collect(),
    };
    let envelopes = tpkg::EnvelopeManifest {
        schema_version: tpkg::ENVELOPES_SCHEMA_VERSION,
        suite: tpkg::Suite::Suite1,
        grants,
    };
    let manifest_text = manifest
        .to_yaml()
        .map_err(|e| et(format!("cannot serialize the manifest: {e}")))?;
    let envelopes_text = envelopes
        .to_yaml()
        .map_err(|e| et(format!("cannot serialize the envelope manifest: {e}")))?;

    // Stage the ciphertext tree (plaintext never touches disk).
    let staging = tempfile::tempdir().map_err(|e| io_et("cannot create a staging dir", e))?;
    let root = staging.path().join("tree");
    stage_encrypted(backend, &root, &dek, &manifest_text, &envelopes_text)?;
    write_image(&root, out)
}

/// Stage the encrypted transform of a plaintext source tree: metadata
/// (`/__tpkg__/`) plaintext, every other file's content replaced by its
/// per-block encrypted form. Public as the transform's write-side
/// primitive (the CLI encrypt verb drives it; tests scan its output).
pub fn stage_encrypted(
    backend: &dyn Backend,
    staging: &Path,
    dek: &[u8; 32],
    manifest_text: &str,
    envelopes_text: &str,
) -> Result<(), (String, i32)> {
    let mut entries = Vec::new();
    walk(backend, "", &mut entries)
        .map_err(|e| (format!("cannot walk the source tree (errno {e})"), 1))?;
    for (path, st) in entries {
        let dest = staging.join(&path);
        match st.entry_type {
            EntryType::Directory => {
                std::fs::create_dir_all(&dest)
                    .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
                #[cfg(unix)]
                stage_perms(&dest, st.perms);
            }
            EntryType::Symlink => stage_symlink(backend, &path, &dest)?,
            EntryType::File if path == MANIFEST_BACKEND_PATH => {
                std::fs::create_dir_all(dest.parent().unwrap_or(staging))
                    .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
                std::fs::write(&dest, manifest_text)
                    .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
            }
            EntryType::File if path.starts_with("__tpkg__/") => {
                let raw = read_backend_file(backend, &path, 1 << 20)?;
                std::fs::write(&dest, raw).map_err(|e| io_et(format!("cannot stage {path}"), e))?;
            }
            EntryType::File => encrypt_one_file(backend, &path, &dest, dek)?,
            EntryType::Other => {
                return err(format!(
                    "cannot encrypt {path}: special files are outside the ENC transform"
                ))
            }
        }
    }
    // The envelope manifest is new content (the source never has one —
    // or has a stale one, replaced here).
    let env_dest = staging.join(ENVELOPES_BACKEND_PATH);
    std::fs::create_dir_all(env_dest.parent().unwrap_or(staging))
        .map_err(|e| io_et("cannot stage the envelope manifest", e))?;
    std::fs::write(&env_dest, envelopes_text)
        .map_err(|e| io_et("cannot stage the envelope manifest", e))?;
    Ok(())
}

/// Encrypt one source file into the staging tree: header, then
/// per-block `ct || tag` (streaming — never whole-file in memory).
fn encrypt_one_file(
    backend: &dyn Backend,
    path: &str,
    dest: &Path,
    dek: &[u8; 32],
) -> Result<(), (String, i32)> {
    use std::io::Write as _;
    let st = backend
        .stat(path)
        .map_err(|e| (format!("cannot stat {path} (errno {e})"), 1))?;
    let size = st.size as u64;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| io_et(format!("cannot stage {path}"), e))?;
    }
    let fk = backends_enc::file_key_for_path(dek, path);
    let mut file =
        std::fs::File::create(dest).map_err(|e| io_et(format!("cannot stage {path}"), e))?;
    file.write_all(&backends_enc::header_for(size))
        .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
    let mut block = vec![0u8; BLOCK_SIZE as usize];
    let mut off = 0u64;
    let mut index = 0u64;
    while off < size {
        let want = (BLOCK_SIZE.min(size - off)) as usize;
        let mut got = 0usize;
        while got < want {
            let n = backend
                .pread(path, &mut block[got..want], off + got as u64)
                .map_err(|e| (format!("cannot read {path} (errno {e})"), 1))?;
            if n == 0 {
                return err(format!("short read on {path}"));
            }
            got += n;
        }
        let ct = backends_enc::encrypt_block(&fk, size, index, &block[..want]);
        // The plaintext block never reaches disk: wipe it before reuse.
        zeroize::Zeroize::zeroize(block.as_mut_slice());
        file.write_all(&ct)
            .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
        off += want as u64;
        index += 1;
    }
    #[cfg(unix)]
    stage_perms(dest, st.perms);
    Ok(())
}

// ---------------------------------------------------------------------
// decrypt (to a tar stream — plaintext only in the explicit output)
// ---------------------------------------------------------------------

/// A `Read` adapter over a backend file (streaming pread).
struct BackendReader<'a> {
    backend: &'a dyn Backend,
    path: String,
    offset: u64,
}

impl Read for BackendReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self
            .backend
            .pread(&self.path, buf, self.offset)
            .map_err(std::io::Error::from_raw_os_error)?;
        self.offset += n as u64;
        Ok(n)
    }
}

/// `tfs decrypt <enc-image> -o <out.tar> --key <secret-key-file>`.
pub fn cmd_decrypt(src: &Path, out: &Path, key_file: &Path) -> Result<(), (String, i32)> {
    let secret_key = read_key_file(key_file)?;
    let mount = mount_image(src)?;
    let enc = EncBackend::new(mount.backend, KeySource::Recipient { secret_key }).map_err(|e| {
        if e == ENOKEY {
            et("EKEY: no envelope recipient slot opens with the given key")
        } else {
            et(format!("cannot open the encrypted image (errno {e})"))
        }
    })?;

    if out.exists() {
        std::fs::remove_file(out)
            .map_err(|e| io_et(format!("cannot replace {}", out.display()), e))?;
    }
    let file = std::fs::File::create(out)
        .map_err(|e| io_et(format!("cannot create {}", out.display()), e))?;
    let mut builder = tar::Builder::new(file);
    let mut entries = Vec::new();
    walk(&enc, "", &mut entries).map_err(|e| (format!("cannot walk the tree (errno {e})"), 1))?;
    for (path, st) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_mode(st.perms);
        header.set_mtime(st.mtime.max(0) as u64);
        match st.entry_type {
            EntryType::Directory => {
                header.set_entry_type(tar::EntryType::Directory);
                header.set_size(0);
                builder
                    .append_data(&mut header, &path, std::io::empty())
                    .map_err(|e| io_et(format!("tar append {path}"), e))?;
            }
            EntryType::Symlink => {
                let target = enc.read_link(&path).map_err(|e| {
                    (
                        format!("cannot read the symlink target of {path} (errno {e})"),
                        1,
                    )
                })?;
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);
                header
                    .set_link_name(&target)
                    .map_err(|e| io_et(format!("tar link name {path}"), e))?;
                builder
                    .append_data(&mut header, &path, std::io::empty())
                    .map_err(|e| io_et(format!("tar append {path}"), e))?;
            }
            EntryType::File => {
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(st.size as u64);
                let reader = BackendReader {
                    backend: &enc,
                    path: path.clone(),
                    offset: 0,
                };
                builder
                    .append_data(&mut header, &path, reader)
                    .map_err(|e| {
                        if e.raw_os_error() == Some(ENOKEY) {
                            et(format!(
                                "EKEY: {path} is outside the granted subtree (or the key is wrong)"
                            ))
                        } else {
                            io_et(format!("tar append {path}"), e)
                        }
                    })?;
            }
            EntryType::Other => {
                return err(format!(
                    "cannot decrypt {path}: special files are not archived"
                ))
            }
        }
    }
    builder
        .finish()
        .map_err(|e| io_et("cannot finish the tar stream", e))
}

// ---------------------------------------------------------------------
// mount (the unlock/grant surface)
// ---------------------------------------------------------------------

/// `tfs mount <enc-image> --key <secret-key-file>` — unlock the image
/// with the recipient key and report the opened grant. (The persistent
/// FUSE/serve mounts are spec-11 §6 PLANNED; this is the key/grant
/// surface of spec 10 §7.)
pub fn cmd_mount_enc(image: &Path, key_file: &Path) -> Result<String, (String, i32)> {
    let secret_key = read_key_file(key_file)?;
    let mount = mount_image(image)?;
    let base_name = mount.backend.name().to_string_lossy().into_owned();
    let enc = EncBackend::new(mount.backend, KeySource::Recipient { secret_key }).map_err(|e| {
        if e == ENOKEY {
            et("EKEY: no envelope recipient slot opens with the given key")
        } else {
            et(format!("cannot open the encrypted image (errno {e})"))
        }
    })?;

    let mut out = String::new();
    out.push_str(&format!("image: {}\n", image.display()));
    out.push_str(&format!("stack: ENC over {base_name}\n"));
    out.push_str(&format!(
        "grant: {} → {}\n",
        enc.opened_grant_id().unwrap_or("?"),
        enc.grant_path()
    ));
    if let Some(envelopes) = enc.envelope_manifest() {
        out.push_str(&format!("suite: {}\n", envelopes.suite));
        if let Some(grant) = envelopes
            .grants
            .iter()
            .find(|g| Some(g.id.as_str()) == enc.opened_grant_id())
        {
            out.push_str(&format!("recipients: {}\n", grant.recipients.join(", ")));
        }
        let others: Vec<String> = envelopes
            .grants
            .iter()
            .filter(|g| Some(g.id.as_str()) != enc.opened_grant_id())
            .map(|g| format!("{} → {}", g.id, g.path))
            .collect();
        if !others.is_empty() {
            out.push_str(&format!("other grants (sealed): {}\n", others.join(", ")));
        }
    }
    // The plaintext identity (the manifest is plaintext metadata).
    if let Ok(raw) = read_backend_file(&enc, MANIFEST_BACKEND_PATH, 1 << 20) {
        if let Ok(text) = String::from_utf8(raw) {
            if let Ok(manifest) = tpkg::PayloadManifest::from_yaml(&text) {
                out.push_str(&format!(
                    "tree_hash (plaintext identity): {}\n",
                    manifest.identity.digest.tree_hash
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------
// rewrap (grant rotation: the bulk is never re-encrypted)
// ---------------------------------------------------------------------

/// `tfs encrypt <enc-image> -o <out.tfs> --rewrap --key <old-secret>
/// --recipient <new-pub>…` — unwrap every grant the presented key
/// opens, re-wrap to the new recipient set, and mirror the bulk
/// ciphertext BYTE-IDENTICAL (spec 10 §2: revocation is prospective —
/// re-issue the envelopes without the recipient).
pub fn cmd_rewrap(
    src: &Path,
    out: &Path,
    key_file: &Path,
    recipients: &[PathBuf],
) -> Result<(), (String, i32)> {
    if recipients.is_empty() {
        return err("rewrap requires at least one --recipient (the new grant set)");
    }
    let secret_key = read_key_file(key_file)?;
    let mount = mount_image(src)?;
    let backend = &*mount.backend;

    let raw = read_backend_file(backend, ENVELOPES_BACKEND_PATH, 1 << 20)
        .map_err(|_| et("not an encrypted image (no envelope manifest)"))?;
    let text = String::from_utf8(raw).map_err(|_| et("the envelope manifest is not UTF-8"))?;
    let mut envelopes = tpkg::EnvelopeManifest::from_yaml(&text)
        .map_err(|e| et(format!("the envelope manifest is not valid: {e}")))?;

    let (new_pubs, new_keyids) = load_recipients(recipients)?;
    let new_refs: Vec<&[u8]> = new_pubs.iter().map(Vec::as_slice).collect();
    let mut opened = 0usize;
    for grant in &mut envelopes.grants {
        if let Ok(dek) = tebako_signer::unwrap_dek(grant.envelope.as_bytes(), &secret_key) {
            let envelope = tebako_signer::wrap_dek(&dek, &new_refs)
                .map_err(|e| et(format!("cannot re-wrap the {} grant: {e}", grant.id)))?;
            grant.envelope = String::from_utf8(envelope)
                .map_err(|_| et("a re-wrapped envelope is not UTF-8 (armored output expected)"))?;
            grant.recipients = new_keyids.clone();
            opened += 1;
        }
        // Grants the key does not open carry over unchanged (you cannot
        // re-wrap what you cannot unwrap).
    }
    if opened == 0 {
        return ekey_err("no envelope recipient slot opens with the given key");
    }

    let staging = tempfile::tempdir().map_err(|e| io_et("cannot create a staging dir", e))?;
    let root = staging.path().join("tree");
    stage_mirrored(
        backend,
        &root,
        &envelopes.to_yaml().map_err(|e| et(e.to_string()))?,
    )?;
    write_image(&root, out)
}

/// Mirror a ciphertext tree byte-identical, replacing only the envelope
/// manifest (rewrap: the bulk is never re-encrypted).
fn stage_mirrored(
    backend: &dyn Backend,
    staging: &Path,
    envelopes_text: &str,
) -> Result<(), (String, i32)> {
    let mut entries = Vec::new();
    walk(backend, "", &mut entries)
        .map_err(|e| (format!("cannot walk the tree (errno {e})"), 1))?;
    for (path, st) in entries {
        let dest = staging.join(&path);
        match st.entry_type {
            EntryType::Directory => {
                std::fs::create_dir_all(&dest)
                    .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
                #[cfg(unix)]
                stage_perms(&dest, st.perms);
            }
            EntryType::Symlink => stage_symlink(backend, &path, &dest)?,
            EntryType::File if path == ENVELOPES_BACKEND_PATH => {
                std::fs::write(&dest, envelopes_text)
                    .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
            }
            EntryType::File => {
                let raw = read_backend_file(backend, &path, i64::MAX)?;
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| io_et(format!("cannot stage {path}"), e))?;
                }
                std::fs::write(&dest, raw).map_err(|e| io_et(format!("cannot stage {path}"), e))?;
                #[cfg(unix)]
                stage_perms(&dest, st.perms);
            }
            EntryType::Other => {
                return err(format!(
                    "cannot rewrap {path}: special files are not staged"
                ))
            }
        }
    }
    Ok(())
}

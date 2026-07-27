//! ENC: the confidentiality stacking transform (spec 10 §1 — a
//! transform like COW, not a format; it lives ONLY here in the Rust
//! TFS — backends and dwarfs-t stay ignorant of it, spec 11 §4).
//!
//! `EncBackend { base: dyn Backend, key_source }` — the base serves
//! ciphertext files; this backend serves their plaintext, decrypted
//! **per block, on demand, in memory only** (plaintext blocks never
//! touch disk; buffers are mlock'd + zeroized — see
//! [`crate::secure_buf`]). Mount requires the recipient key; a wrong
//! key is the named EKEY-class error [`ENOKEY`], never garbage.
//!
//! # The transform (encrypt AFTER compress)
//!
//! Directory structure, names, symlink targets and the `/__tpkg__/`
//! metadata directory stay plaintext (metadata hiding is spec 10's
//! deferred option); every other regular file's CONTENT is replaced by
//! its encrypted form:
//!
//! ```text
//! offset  size  field
//!      0     8  magic "tfsenc01"
//!      8     8  u64le plaintext_size
//!     16     —  per block i (block = 4096 bytes; full-block slot 4112):
//!                 ciphertext_i (block_len bytes) || GCM tag_i (16 bytes)
//! ```
//!
//! # Crypto construction (SUITE-1, spec 10 §5)
//!
//! - **Per block: AES-256-GCM** with
//!   - key = the file key `FK(path)` (below),
//!   - nonce = `u32le(0) || u64le(block_index)` (12 bytes) — unique per
//!     (key, block) because file keys are unique per file and block
//!     indexes never repeat within a file; images are immutable, so no
//!     (key, nonce) pair ever seals two different plaintexts. When a
//!     CHANGED tree is re-encrypted the DEK MUST be rotated (spec 10
//!     §2's prospective rotation), which re-keys every nonce domain.
//!   - AAD = `"tfsenc01" || u64le(plaintext_size) || u64le(block_index)`
//!     — binds each block to its file size and position (blocks cannot
//!     be relocated, truncated away silently, or transplanted between
//!     files of different sizes).
//! - **Keys: one root DEK per image** (32 random bytes), with
//!   **HKDF-SHA256 path keys** for selective disclosure (spec 10 §2):
//!
//! ```text
//! K("")            = DEK                        (the image root key)
//! K(dir/name)      = HKDF-SHA256(ikm = K(dir),  salt = none,
//!                                info = "tfs-enc-1/dir\0"  || name)
//! FK(dir/file)     = HKDF-SHA256(ikm = K(dir),  salt = none,
//!                                info = "tfs-enc-1/file\0" || name)
//! ```
//!
//!   One-way per level: K(/a/b) opens /a/b/** but not /a/c or /a.
//!   Sharing a subtree = wrapping THAT subtree's key to the recipient.
//! - **Envelopes:** the DEK (or a subtree key) is wrapped to recipients
//!   as OpenPGP PKESK packets via tebako-signer's rnp — no custom
//!   crypto anywhere — recorded in `/__tpkg__/envelopes.yaml` (tpkg's
//!   [`tpkg::EnvelopeManifest`]). The payload manifest's
//!   `encryption.parts` reference grants by id and NEVER carry keys
//!   (spec 03 §2.1).
//!
//! # Named errors
//!
//! - [`ENOKEY`] (the EKEY class): no envelope recipient slot opens with
//!   the presented key; a GCM tag fails (wrong key OR tampered
//!   ciphertext — indistinguishable, and both are "this content is not
//!   authentically yours"); a read outside the granted subtree.
//! - `EINVAL`: the image is not ENC-formatted (bad/missing magic, a
//!   missing or malformed envelope manifest when mounting by recipient,
//!   an opened envelope whose payload is not a 32-byte key).

use std::ffi::CStr;

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit};

use crate::backend::{Backend, EntryType, RawDirEntry, RawStat};
use crate::secure_buf::SecureBuf;

/// The named EKEY-class error (spec 10 §7): Linux's `ENOKEY` value,
/// defined here so the class is stable on platforms whose libc lacks it
/// (macOS has no ENOKEY). A wrong key, a failed GCM tag, or a read
/// outside the granted subtree — never garbage, always this.
pub const ENOKEY: i32 = 126;

/// The per-file magic (8 bytes; also the AAD prefix).
const MAGIC: &[u8; 8] = b"tfsenc01";
/// Header size: magic + u64le plaintext size.
const HEADER_LEN: u64 = 16;
/// The plaintext block size (4 KiB — the merkle grid, see tpkg::merkle).
pub const BLOCK_SIZE: u64 = 4096;
/// The GCM tag size appended to every block.
const TAG_LEN: u64 = 16;
/// The on-disk slot of one FULL block (ciphertext + tag).
const SLOT_LEN: u64 = BLOCK_SIZE + TAG_LEN;

const HKDF_DIR_INFO: &[u8] = b"tfs-enc-1/dir\0";
const HKDF_FILE_INFO: &[u8] = b"tfs-enc-1/file\0";

/// Sanity bound on the in-image envelope manifest.
const ENVELOPES_MAX: i64 = 1 << 20;

/// The backend-relative envelope manifest path.
const ENVELOPES_BACKEND_PATH: &str = "__tpkg__/envelopes.yaml";

/// Derive a directory's key from its parent's key (spec 10 §2 — the
/// one-way HKDF path derivation).
pub fn derive_dir_key(parent: &[u8; 32], name: &str) -> [u8; 32] {
    hkdf_expand(parent, HKDF_DIR_INFO, name.as_bytes())
}

/// Derive a file's key from its parent directory's key.
pub fn derive_file_key(parent_dir: &[u8; 32], name: &str) -> [u8; 32] {
    hkdf_expand(parent_dir, HKDF_FILE_INFO, name.as_bytes())
}

fn hkdf_expand(ikm: &[u8; 32], tag: &[u8], name: &[u8]) -> [u8; 32] {
    let hk = hkdf::Hkdf::<sha2::Sha256>::new(None, ikm);
    let mut info = Vec::with_capacity(tag.len() + name.len());
    info.extend_from_slice(tag);
    info.extend_from_slice(name);
    let mut out = [0u8; 32];
    // 32 bytes is always a valid HKDF-SHA256 output length.
    hk.expand(&info, &mut out)
        .expect("HKDF-SHA256 expand of 32 bytes cannot fail");
    out
}

/// The key of the file at `rel_path` (relative to the key's own subtree
/// root, `/`-separated): HKDF down the directory chain, then the file
/// derivation on the basename.
pub fn file_key_for_path(subtree_key: &[u8; 32], rel_path: &str) -> [u8; 32] {
    let mut key = *subtree_key;
    let mut parts = rel_path.split('/').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            key = derive_dir_key(&key, part);
        } else {
            key = derive_file_key(&key, part);
        }
    }
    key
}

/// A fresh 32-byte DEK from the OS CSPRNG.
pub fn generate_dek() -> Result<[u8; 32], i32> {
    let mut dek = [0u8; 32];
    getrandom::getrandom(&mut dek).map_err(|_| libc::EIO)?;
    Ok(dek)
}

/// The 16-byte per-file header for `size` plaintext bytes.
pub fn header_for(size: u64) -> [u8; HEADER_LEN as usize] {
    let mut out = [0u8; HEADER_LEN as usize];
    out[..8].copy_from_slice(MAGIC);
    out[8..].copy_from_slice(&size.to_le_bytes());
    out
}

fn aad_for(size: u64, index: u64) -> [u8; 24] {
    let mut aad = [0u8; 24];
    aad[..8].copy_from_slice(MAGIC);
    aad[8..16].copy_from_slice(&size.to_le_bytes());
    aad[16..].copy_from_slice(&index.to_le_bytes());
    aad
}

fn nonce_for(index: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[4..].copy_from_slice(&index.to_le_bytes());
    nonce
}

/// Encrypt one block (≤ [`BLOCK_SIZE`] bytes) of the file whose
/// plaintext size is `size`, at block `index`. Output: ciphertext
/// (block.len() bytes) || 16-byte GCM tag.
pub fn encrypt_block(file_key: &[u8; 32], size: u64, index: u64, block: &[u8]) -> Vec<u8> {
    debug_assert!(block.len() as u64 <= BLOCK_SIZE);
    let cipher = Aes256Gcm::new_from_slice(file_key).expect("AES-256 keys are 32 bytes");
    cipher
        .encrypt(
            aes_gcm::Nonce::from_slice(&nonce_for(index)),
            Payload {
                msg: block,
                aad: &aad_for(size, index),
            },
        )
        .expect("AES-GCM encryption of an in-memory block cannot fail")
}

/// Decrypt one stored block slot (`ct || tag`) — the inverse of
/// [`encrypt_block`]. A tag failure is [`ENOKEY`] (wrong key or
/// tampered ciphertext; the two are indistinguishable by construction).
pub fn decrypt_block(
    file_key: &[u8; 32],
    size: u64,
    index: u64,
    ct_tag: &[u8],
) -> Result<SecureBuf, i32> {
    let cipher = Aes256Gcm::new_from_slice(file_key).map_err(|_| ENOKEY)?;
    let plain = cipher
        .decrypt(
            aes_gcm::Nonce::from_slice(&nonce_for(index)),
            Payload {
                msg: ct_tag,
                aad: &aad_for(size, index),
            },
        )
        .map_err(|_| ENOKEY)?;
    Ok(SecureBuf::from_slice(&plain))
}

/// Where the key comes from at mount (spec 10 §1: `key_source`).
pub enum KeySource {
    /// A raw 32-byte subtree key and its subtree root (absolute path,
    /// "/" for the whole image) — agents, tests, re-wrap flows.
    SubtreeKey {
        /// The subtree root this key opens (absolute, "/" = image root).
        path: String,
        /// The 32-byte subtree key.
        key: [u8; 32],
    },
    /// Unwrap a grant from the in-image envelope manifest with a
    /// recipient secret key (armored or binary OpenPGP export).
    Recipient {
        /// The recipient's secret key export.
        secret_key: Vec<u8>,
    },
}

/// A grant opened at mount: the subtree root and its key.
struct OpenedGrant {
    /// Backend-relative subtree root ("" = image root).
    path: String,
    /// The subtree key (locked + zeroed).
    key: SecureBuf,
}

/// `EncBackend { base, key_source }` — the stacking confidentiality
/// transform (see the module docs).
pub struct EncBackend {
    base: Box<dyn Backend>,
    grant: OpenedGrant,
    /// The parsed envelope manifest, when the mount went through one
    /// (the CLI's grant display; None for raw SubtreeKey mounts).
    envelopes: Option<tpkg::EnvelopeManifest>,
    /// The id of the grant that opened (envelope mounts only).
    opened_grant_id: Option<String>,
}

/// Normalize an in-image path: no leading or trailing `/`, `""` for root.
fn normalize(path: &str) -> &str {
    path.trim_start_matches('/').trim_end_matches('/')
}

/// True for the plaintext metadata directory (root-level `__tpkg__`).
fn is_tpkg(path: &str) -> bool {
    path == "__tpkg__" || path.starts_with("__tpkg__/")
}

/// The backend-relative form of an absolute grant path (`"/a/b"` →
/// `"a/b"`, `"/"` → `""`).
fn grant_rel(absolute: &str) -> String {
    normalize(absolute).to_string()
}

/// Read a backend file fully (bounded) — small metadata reads.
fn read_whole(base: &dyn Backend, path: &str, max: i64) -> Result<Vec<u8>, i32> {
    let st = base.stat(path)?;
    if st.entry_type != EntryType::File {
        return Err(libc::EINVAL);
    }
    if st.size > max || st.size < 0 {
        return Err(libc::EINVAL);
    }
    let mut buf = vec![0u8; st.size as usize];
    let mut off = 0u64;
    while off < st.size as u64 {
        let n = base.pread(path, &mut buf[off as usize..], off)?;
        if n == 0 {
            return Err(libc::EIO);
        }
        off += n as u64;
    }
    Ok(buf)
}

/// Read and parse the in-image envelope manifest. Any absence or
/// malformation means the image is not ENC-formatted: EINVAL.
fn read_envelope_manifest(base: &dyn Backend) -> Result<tpkg::EnvelopeManifest, i32> {
    let raw = read_whole(base, ENVELOPES_BACKEND_PATH, ENVELOPES_MAX)?;
    let text = String::from_utf8(raw).map_err(|_| libc::EINVAL)?;
    tpkg::EnvelopeManifest::from_yaml(&text).map_err(|_| libc::EINVAL)
}

impl EncBackend {
    /// Stack the decrypting view over `base`. Mount REQUIRES the key:
    /// with [`KeySource::Recipient`], the envelope manifest must parse
    /// and one grant must open (else EINVAL / ENOKEY — named, never
    /// garbage).
    pub fn new(base: Box<dyn Backend>, key_source: KeySource) -> Result<EncBackend, i32> {
        match key_source {
            KeySource::SubtreeKey { path, key } => {
                if !path.starts_with('/') {
                    return Err(libc::EINVAL);
                }
                Ok(EncBackend {
                    base,
                    grant: OpenedGrant {
                        path: grant_rel(&path),
                        key: SecureBuf::from_slice(&key),
                    },
                    envelopes: None,
                    opened_grant_id: None,
                })
            }
            KeySource::Recipient { secret_key } => {
                let envelopes = read_envelope_manifest(&*base)?;
                // Try every grant in path order (deepest first is not
                // required for correctness — any grant that opens IS a
                // capability); first success wins.
                let mut opened = None;
                for (i, grant) in envelopes.grants.iter().enumerate() {
                    if let Ok(dek) =
                        tebako_signer::unwrap_dek(grant.envelope.as_bytes(), &secret_key)
                    {
                        let dek: &[u8] = &dek;
                        let key: [u8; 32] = dek.try_into().map_err(|_| libc::EINVAL)?;
                        opened = Some((i, key));
                        break;
                    }
                }
                let Some((i, key)) = opened else {
                    return Err(ENOKEY);
                };
                let grant = &envelopes.grants[i];
                let opened_grant_id = Some(grant.id.clone());
                let path = grant_rel(&grant.path);
                Ok(EncBackend {
                    base,
                    grant: OpenedGrant {
                        path,
                        key: SecureBuf::from_slice(&key),
                    },
                    envelopes: Some(envelopes),
                    opened_grant_id,
                })
            }
        }
    }

    /// The ciphertext base backend.
    pub fn base(&self) -> &dyn Backend {
        self.base.as_ref()
    }

    /// The opened grant's subtree root, absolute form (`"/"` = the
    /// whole image).
    pub fn grant_path(&self) -> String {
        if self.grant.path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.grant.path)
        }
    }

    /// The id of the grant that opened (envelope mounts only).
    pub fn opened_grant_id(&self) -> Option<&str> {
        self.opened_grant_id.as_deref()
    }

    /// The parsed envelope manifest (envelope mounts only).
    pub fn envelope_manifest(&self) -> Option<&tpkg::EnvelopeManifest> {
        self.envelopes.as_ref()
    }

    /// True when `path` (backend-relative) is inside the granted
    /// subtree.
    fn under_grant(&self, path: &str) -> bool {
        self.grant.path.is_empty()
            || path == self.grant.path
            || path
                .strip_prefix(&self.grant.path)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The file key for a backend-relative path inside the grant.
    fn file_key(&self, path: &str) -> [u8; 32] {
        let rel = path
            .strip_prefix(&self.grant.path)
            .unwrap_or(path)
            .trim_start_matches('/');
        let mut key = [0u8; 32];
        key.copy_from_slice(self.grant.key.as_slice());
        file_key_for_path(&key, rel)
    }

    /// Read and validate the per-file header: the plaintext size.
    /// Bad magic / short read → EINVAL (not an ENC file).
    fn read_header(&self, path: &str) -> Result<u64, i32> {
        let mut hdr = [0u8; HEADER_LEN as usize];
        let n = self.base.pread(path, &mut hdr, 0)?;
        if n < HEADER_LEN as usize || hdr[..8] != MAGIC[..] {
            return Err(libc::EINVAL);
        }
        Ok(u64::from_le_bytes(hdr[8..].try_into().unwrap_or([0; 8])))
    }
}

impl Backend for EncBackend {
    fn name(&self) -> &'static CStr {
        c"ENC"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        let path = normalize(path);
        if is_tpkg(path) {
            return self.base.stat(path);
        }
        let mut st = self.base.stat(path)?;
        if st.entry_type == EntryType::File {
            // The header is plaintext: sizes are visible everywhere
            // (metadata hiding is the deferred option), so stat works
            // outside the grant too — only CONTENT requires the key.
            st.size = self.read_header(path)? as i64;
        }
        Ok(st)
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        let path = normalize(path);
        if is_tpkg(path) {
            return self.base.pread(path, buf, offset);
        }
        if !self.under_grant(path) {
            return Err(ENOKEY);
        }
        // Dirs/symlinks have no content; the base answers those.
        let st = self.base.stat(path)?;
        if st.entry_type != EntryType::File {
            return self.base.pread(path, buf, offset);
        }
        let size = self.read_header(path)?;
        if offset >= size || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(size - offset);
        let file_key = self.file_key(path);
        let first = offset / BLOCK_SIZE;
        let last = (offset + want - 1) / BLOCK_SIZE;
        let mut done = 0usize;
        for index in first..=last {
            let block_len = BLOCK_SIZE.min(size - index * BLOCK_SIZE);
            let slot = HEADER_LEN + index * SLOT_LEN;
            let mut ct_tag = vec![0u8; (block_len + TAG_LEN) as usize];
            let mut got = 0usize;
            while got < ct_tag.len() {
                let n = self
                    .base
                    .pread(path, &mut ct_tag[got..], slot + got as u64)?;
                if n == 0 {
                    return Err(libc::EIO); // truncated ciphertext
                }
                got += n;
            }
            let plain = decrypt_block(&file_key, size, index, &ct_tag)?;
            let start = if index == first {
                (offset - index * BLOCK_SIZE) as usize
            } else {
                0
            };
            let end = (block_len as usize).min(start + (want as usize - done));
            buf[done..done + (end - start)].copy_from_slice(&plain.as_slice()[start..end]);
            done += end - start;
        }
        Ok(done)
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        self.base.read_dir(normalize(path))
    }

    fn read_link(&self, path: &str) -> Result<String, i32> {
        self.base.read_link(normalize(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends_hostdir::HostDirBackend;
    use std::sync::Mutex;

    // ---------------------------------------------------------------
    // Fixtures: a ciphertext HostDir tree built with the pub helpers
    // ---------------------------------------------------------------

    const ROOT_DEK: [u8; 32] = [0x11; 32];

    /// Encrypt `content` as the file `rel` under the root key.
    fn encrypt_file(rel: &str, content: &[u8]) -> Vec<u8> {
        let fk = file_key_for_path(&ROOT_DEK, rel);
        let mut out = header_for(content.len() as u64).to_vec();
        for (i, chunk) in content.chunks(BLOCK_SIZE as usize).enumerate() {
            out.extend_from_slice(&encrypt_block(&fk, content.len() as u64, i as u64, chunk));
        }
        out
    }

    /// Write a ciphertext tree: plaintext manifest + envelopes aside,
    /// every listed file encrypted under the root DEK.
    fn write_tree(root: &std::path::Path, files: &[(&str, &[u8])]) {
        for (rel, content) in files {
            let dest = root.join(rel);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(dest, encrypt_file(rel, content)).unwrap();
        }
    }

    fn host_base(root: &std::path::Path) -> Box<dyn Backend> {
        Box::new(HostDirBackend::new(root).unwrap())
    }

    fn pread_all(b: &dyn Backend, path: &str) -> Vec<u8> {
        let st = b.stat(path).unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = b.pread(path, &mut buf, 0).unwrap();
        assert_eq!(n, buf.len());
        buf
    }

    // ---------------------------------------------------------------
    // The crypto construction
    // ---------------------------------------------------------------

    #[test]
    fn block_roundtrip_and_tag_failures() {
        let fk = [0x42; 32];
        for content in [
            b"".as_slice(),
            b"x".as_slice(),
            &[0xAB; BLOCK_SIZE as usize],
            &[0xCD; BLOCK_SIZE as usize + 7],
        ] {
            for (i, chunk) in content.chunks(BLOCK_SIZE as usize).enumerate() {
                let ct = encrypt_block(&fk, content.len() as u64, i as u64, chunk);
                assert_eq!(ct.len(), chunk.len() + TAG_LEN as usize);
                let back = decrypt_block(&fk, content.len() as u64, i as u64, &ct).unwrap();
                assert_eq!(back.as_slice(), chunk);
                // Wrong key → ENOKEY.
                assert_eq!(
                    decrypt_block(&[0x43; 32], content.len() as u64, i as u64, &ct)
                        .err()
                        .unwrap(),
                    ENOKEY
                );
                // Tampered ciphertext → ENOKEY (never garbage).
                let mut tampered = ct.clone();
                tampered[0] ^= 1;
                assert_eq!(
                    decrypt_block(&fk, content.len() as u64, i as u64, &tampered)
                        .err()
                        .unwrap(),
                    ENOKEY
                );
                // Wrong block index (AAD binds position) → ENOKEY.
                assert_eq!(
                    decrypt_block(&fk, content.len() as u64, (i + 1) as u64, &ct)
                        .err()
                        .unwrap(),
                    ENOKEY
                );
                // Wrong size (AAD binds the file length) → ENOKEY.
                assert_eq!(
                    decrypt_block(&fk, content.len() as u64 + 1, i as u64, &ct)
                        .err()
                        .unwrap(),
                    ENOKEY
                );
            }
        }
    }

    #[test]
    fn hkdf_path_keys_are_one_way() {
        let k_root = ROOT_DEK;
        let k_a = derive_dir_key(&k_root, "a");
        let k_ab = derive_dir_key(&k_a, "b");
        let k_ac = derive_dir_key(&k_a, "c");
        // Distinct paths → distinct keys.
        assert_ne!(k_ab, k_ac);
        assert_ne!(k_a, k_ab);
        // file_key_for_path agrees with the per-level chain.
        assert_eq!(
            file_key_for_path(&k_root, "a/b/f.txt"),
            derive_file_key(&k_ab, "f.txt")
        );
        // The subtree key opens its own subtree identically whether
        // derived from the root or used directly.
        assert_eq!(
            file_key_for_path(&k_ab, "deep/f.txt"),
            file_key_for_path(&k_root, "a/b/deep/f.txt")
        );
    }

    #[test]
    fn generate_dek_is_random() {
        let a = generate_dek().unwrap();
        let b = generate_dek().unwrap();
        assert_ne!(a, b);
        assert_ne!(a, [0; 32]);
    }

    // ---------------------------------------------------------------
    // The backend: lazy per-block reads, wrong-key, stat/passthrough
    // ---------------------------------------------------------------

    #[test]
    fn enc_reads_decrypt_lazily_per_block() {
        let dir = tempfile::tempdir().unwrap();
        // 10 blocks + a tail, so offsets cross slots.
        let mut content = Vec::new();
        for i in 0..10u8 {
            content.extend_from_slice(&[i; BLOCK_SIZE as usize]);
        }
        content.extend_from_slice(b"tail");
        write_tree(dir.path(), &[("big.bin", &content)]);

        // A counting wrapper records every base pread.
        struct Spy {
            base: Box<dyn Backend>,
            reads: std::sync::Arc<Mutex<Vec<(u64, usize)>>>,
        }
        impl Backend for Spy {
            fn name(&self) -> &'static CStr {
                c"SPY"
            }
            fn stat(&self, path: &str) -> Result<RawStat, i32> {
                self.base.stat(path)
            }
            fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
                self.reads.lock().unwrap().push((offset, buf.len()));
                self.base.pread(path, buf, offset)
            }
            fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
                self.base.read_dir(path)
            }
        }
        let reads = std::sync::Arc::new(Mutex::new(Vec::new()));
        let enc = EncBackend::new(
            Box::new(Spy {
                base: host_base(dir.path()),
                reads: reads.clone(),
            }),
            KeySource::SubtreeKey {
                path: "/".to_string(),
                key: ROOT_DEK,
            },
        )
        .unwrap();

        // stat reports the PLAINTEXT size (one 16-byte header read).
        let st = enc.stat("big.bin").unwrap();
        assert_eq!(st.size, content.len() as i64);
        assert_eq!(reads.lock().unwrap().as_slice(), &[(0, 16)]);

        // 100 bytes in the middle of block 5: exactly ONE block slot is
        // read — lazy per-block decryption, never whole-image.
        reads.lock().unwrap().clear();
        let mut buf = [0u8; 100];
        let n = enc.pread("big.bin", &mut buf, 5 * BLOCK_SIZE + 50).unwrap();
        assert_eq!(n, 100);
        assert_eq!(&buf, &[5u8; 100]);
        assert_eq!(
            reads.lock().unwrap().as_slice(),
            &[(0, 16), (HEADER_LEN + 5 * SLOT_LEN, SLOT_LEN as usize)]
        );

        // A span crossing blocks 8..9 (plus the short tail block) reads
        // exactly those slots, in order.
        reads.lock().unwrap().clear();
        let tail_off = 8 * BLOCK_SIZE + 4090;
        let mut buf = vec![0u8; (content.len() as u64 - tail_off) as usize];
        let n = enc.pread("big.bin", &mut buf, tail_off).unwrap();
        assert_eq!(n, buf.len());
        assert_eq!(buf, content[tail_off as usize..]);
        let expected: Vec<(u64, usize)> = std::iter::once((0, 16))
            .chain((8..=10).map(|i| {
                let len = if i < 10 {
                    SLOT_LEN as usize
                } else {
                    4 + TAG_LEN as usize
                };
                (HEADER_LEN + i * SLOT_LEN, len)
            }))
            .collect();
        assert_eq!(reads.lock().unwrap().as_slice(), expected.as_slice());
    }

    #[test]
    fn enc_pread_crossing_blocks_and_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let content: Vec<u8> = (0..9000u32).map(|i| (i % 251) as u8).collect();
        write_tree(
            dir.path(),
            &[("data.bin", &content), ("empty", b""), ("small", b"hello")],
        );
        std::fs::create_dir_all(dir.path().join("__tpkg__")).unwrap();
        std::fs::write(
            dir.path().join("__tpkg__/manifest.yaml"),
            b"plaintext: metadata",
        )
        .unwrap();

        let enc = EncBackend::new(
            host_base(dir.path()),
            KeySource::SubtreeKey {
                path: "/".to_string(),
                key: ROOT_DEK,
            },
        )
        .unwrap();
        assert_eq!(enc.name().to_str().unwrap(), "ENC");
        assert_eq!(enc.grant_path(), "/");
        assert_eq!(pread_all(&enc, "data.bin"), content);
        assert_eq!(pread_all(&enc, "small"), b"hello");
        // The empty file: header only, zero content.
        assert_eq!(enc.stat("empty").unwrap().size, 0);
        assert_eq!(pread_all(&enc, "empty"), b"");
        // Reads past EOF clamp; zero-length reads succeed.
        assert_eq!(enc.pread("small", &mut [0u8; 8], 5).unwrap(), 0);
        assert_eq!(enc.pread("small", &mut [0u8; 8], 100).unwrap(), 0);
        // The metadata directory is plaintext passthrough.
        assert_eq!(
            pread_all(&enc, "__tpkg__/manifest.yaml"),
            b"plaintext: metadata"
        );
        // Directory listing and metadata pass through.
        let mut names: Vec<String> = enc
            .read_dir("")
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["__tpkg__", "data.bin", "empty", "small"]);
    }

    #[test]
    fn wrong_subtree_key_fails_every_block_read_with_enokey() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(dir.path(), &[("secret.txt", b"classified")]);
        let enc = EncBackend::new(
            host_base(dir.path()),
            KeySource::SubtreeKey {
                path: "/".to_string(),
                key: [0x99; 32], // not the DEK the tree was encrypted to
            },
        )
        .unwrap();
        // stat works (sizes are plaintext), content does not decrypt.
        assert_eq!(enc.stat("secret.txt").unwrap().size, 10);
        assert_eq!(
            enc.pread("secret.txt", &mut [0u8; 4], 0).unwrap_err(),
            ENOKEY
        );
    }

    #[test]
    fn selective_disclosure_one_subtree_only() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                ("a/secret/contract.txt", b"for legal"),
                ("a/secret/deep/more.txt", b"nested secret"),
                ("a/other/public.txt", b"for everyone else"),
                ("top.txt", b"top level"),
            ],
        );
        // Mount with the /a/secret subtree key (what a subtree recipient
        // holds after unwrapping their grant).
        let k_a = derive_dir_key(&ROOT_DEK, "a");
        let k_secret = derive_dir_key(&k_a, "secret");
        let enc = EncBackend::new(
            host_base(dir.path()),
            KeySource::SubtreeKey {
                path: "/a/secret".to_string(),
                key: k_secret,
            },
        )
        .unwrap();
        assert_eq!(enc.grant_path(), "/a/secret");
        // The granted subtree reads — including nested files.
        assert_eq!(pread_all(&enc, "a/secret/contract.txt"), b"for legal");
        assert_eq!(pread_all(&enc, "a/secret/deep/more.txt"), b"nested secret");
        // Structure and sizes stay visible (metadata hiding is deferred)...
        assert_eq!(enc.stat("a/other/public.txt").unwrap().size, 17);
        assert!(enc
            .read_dir("a/other")
            .unwrap()
            .iter()
            .any(|e| e.name == "public.txt"));
        // ...but EVERYTHING outside the granted subtree is ENOKEY.
        assert_eq!(
            enc.pread("a/other/public.txt", &mut [0u8; 4], 0)
                .unwrap_err(),
            ENOKEY
        );
        assert_eq!(enc.pread("top.txt", &mut [0u8; 4], 0).unwrap_err(), ENOKEY);
        // And the subtree key cannot be bent to a sibling or the parent.
        let bent = EncBackend::new(
            host_base(dir.path()),
            KeySource::SubtreeKey {
                path: "/a/other".to_string(),
                key: k_secret,
            },
        )
        .unwrap();
        assert_eq!(
            bent.pread("a/other/public.txt", &mut [0u8; 4], 0)
                .unwrap_err(),
            ENOKEY // derives the wrong file key: GCM tag fails
        );
    }

    #[test]
    fn unencrypted_image_is_einval_not_garbage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plain.txt"), b"not encrypted").unwrap();
        let enc = EncBackend::new(
            host_base(dir.path()),
            KeySource::SubtreeKey {
                path: "/".to_string(),
                key: ROOT_DEK,
            },
        )
        .unwrap();
        assert_eq!(enc.stat("plain.txt").unwrap_err(), libc::EINVAL);
        assert_eq!(
            enc.pread("plain.txt", &mut [0u8; 4], 0).unwrap_err(),
            libc::EINVAL
        );
    }

    #[test]
    fn recipient_mount_requires_a_slot_that_opens() {
        // Envelope manifest with one root grant to recipient A; mounting
        // with B's secret key is ENOKEY.
        let dir = tempfile::tempdir().unwrap();
        write_tree(dir.path(), &[("f.txt", b"content")]);
        let ctx = rnp::Context::new().unwrap();
        let key_a = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid("a <a@x>")
            .add_usage(rnp::KeyUsage::Sign)
            .build(&ctx)
            .unwrap();
        rnp::SubkeyBuilder::new(rnp::Algorithm::Ecdh)
            .curve(rnp::Curve::Curve25519)
            .hash(rnp::Hash::Sha256)
            .add_usage(rnp::KeyUsage::EncryptComms)
            .build(&ctx, &key_a)
            .unwrap();
        let pub_a = key_a
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::PUBLIC | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        let sec_a = key_a
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();
        let key_b = rnp::KeyBuilder::new(rnp::Algorithm::Eddsa)
            .hash(rnp::Hash::Sha256)
            .userid("b <b@x>")
            .add_usage(rnp::KeyUsage::Sign)
            .build(&ctx)
            .unwrap();
        rnp::SubkeyBuilder::new(rnp::Algorithm::Ecdh)
            .curve(rnp::Curve::Curve25519)
            .hash(rnp::Hash::Sha256)
            .add_usage(rnp::KeyUsage::EncryptComms)
            .build(&ctx, &key_b)
            .unwrap();
        let sec_b = key_b
            .export(
                rnp::ExportFlags::ARMORED | rnp::ExportFlags::SECRET | rnp::ExportFlags::SUBKEYS,
            )
            .unwrap();

        let envelope = tebako_signer::wrap_dek(&ROOT_DEK, &[&pub_a]).unwrap();
        let manifest = tpkg::EnvelopeManifest {
            schema_version: tpkg::ENVELOPES_SCHEMA_VERSION,
            suite: tpkg::Suite::Suite1,
            grants: vec![tpkg::Grant {
                id: "root".to_string(),
                path: "/".to_string(),
                recipients: vec![tebako_signer::public_key_keyid(&pub_a).unwrap()],
                envelope: String::from_utf8(envelope).unwrap(),
            }],
        };
        std::fs::create_dir_all(dir.path().join("__tpkg__")).unwrap();
        std::fs::write(
            dir.path().join(ENVELOPES_BACKEND_PATH),
            manifest.to_yaml().unwrap(),
        )
        .unwrap();

        // The recipient opens their grant and reads plaintext.
        let enc = EncBackend::new(
            host_base(dir.path()),
            KeySource::Recipient { secret_key: sec_a },
        )
        .unwrap();
        assert_eq!(enc.grant_path(), "/");
        assert_eq!(enc.opened_grant_id(), Some("root"));
        assert!(enc.envelope_manifest().is_some());
        assert_eq!(pread_all(&enc, "f.txt"), b"content");

        // A stranger's key: ENOKEY at mount, never garbage.
        let err = EncBackend::new(
            host_base(dir.path()),
            KeySource::Recipient { secret_key: sec_b },
        )
        .err()
        .unwrap();
        assert_eq!(err, ENOKEY);
    }
}

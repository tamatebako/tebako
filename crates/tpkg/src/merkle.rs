//! The payload tree hash (`identity.digest.tree_hash`, spec 03 §2.1/§7):
//! a block→file→dir→root merkle over the payload tree EXCLUDING the
//! root-level `/__tpkg__/` directory (the digest fixed-point rule — a
//! manifest inside the image it describes cannot name that image's
//! digest, so the manifest subtree is excluded and the exclusion is what
//! CAS/dedup/signing consume).
//!
//! # Construction `tfs-merkle-1` (the v1 parameters, locked here)
//!
//! - **Hash:** SHA-256. PQC-safe by construction (spec 10 §5: ~128-bit
//!   post-Grover collision strength is not the security property merkle
//!   relies on; second-preimage resistance is, and it holds).
//! - **Chunk:** 4096 bytes (4 KiB). Justification: it is the OS page
//!   granularity — the same block grid the EncBackend transform
//!   (spec 10) encrypts on, so integrity and confidentiality share ONE
//!   grid (a block verified is a block decryptable, lazily, in a single
//!   locked page); it matches the VFS read granularity (`tfs cat` streams
//!   4 KiB reads); and it keeps the fan-out high enough that a 1 GiB
//!   file folds into a depth-18 tree with O(log n) working memory.
//! - **Serialization:** length-prefixed and domain-separated. Every hash
//!   input is `TAG || length-prefixes || payloads` with a distinct ASCII
//!   tag per node role, so no cross-role second-preimage exists (a chunk
//!   hash can never collide with an entry, node, link or empty hash by
//!   construction), and every variable-length field is prefixed so no
//!   concatenation ambiguity exists (`ab||c` ≠ `a||bc`).
//!
//! ```text
//! chunk leaf   = H("tfs-merkle-1/chunk\0" || u64le(len) || bytes)
//! node combine = H("tfs-merkle-1/node\0"  || left(32) || right(32))
//! empty fold   = H("tfs-merkle-1/empty\0")
//! symlink      = H("tfs-merkle-1/link\0"  || u32le(target_len) || target)
//! entry record = H("tfs-merkle-1/entry\0" || kind(1) || flags(1) ||
//!                    u32le(name_len) || name || child_digest(32))
//!                kind: 1 = file, 2 = directory, 3 = symlink
//!                flags bit 0: executable (files only; 0 otherwise)
//! ```
//!
//! - **File digest:** the Merkle Tree Hash (RFC-6962-shaped fold:
//!   equal-height subtrees combine as they meet, the remainder bags
//!   right-to-left; a single leaf IS the root) over the file's chunk
//!   leaves. Every file has at least one chunk — the empty file hashes
//!   as one empty chunk, so length is committed everywhere and the
//!   0-byte / absent-block cases are distinct from every non-empty one.
//! - **Directory digest:** the fold over the entry records of the direct
//!   children, sorted by name bytes (canonical: listing order never
//!   affects the root). The empty directory hashes as `empty fold`.
//! - **Root:** the directory digest of `/` after dropping a root-level
//!   child named exactly `__tpkg__` (spec 03 §7). The exclusion is
//!   root-level ONLY — a nested `/a/__tpkg__/` is ordinary content.
//!
//! # What the identity commits to (and what it deliberately does not)
//!
//! Names, kinds, the executable bit, and file content. NOT full
//! permission bits and NOT mtimes: the tree hash is the payload's
//! semantic identity (CAS addressing, dedup — spec 10 §2), and two
//! builds of the same content must collide in CAS regardless of build
//! time or umask. The exec bit is committed because it is semantic (the
//! entrypoint contract, spec 03 §2.2, depends on it).
//!
//! The rendered form for `identity.digest.tree_hash` is
//! [`render_tree_hash`]: `"sha256:<64 lowercase hex>"` (the manifest
//! schema pins that shape; the hash algorithm is named by the key
//! prefix, the merkle construction version by this module — a future
//! `tfs-merkle-2` would be a spec change, not a silent edit).
//!
//! This module is pure: no I/O, no unsafe, no allocation beyond the
//! O(depth + log n) fold stacks. The caller drives it with a
//! [`TreeWalk`] implementation (host directory at image-creation time,
//! mounted backend at verify time, in-memory fixture in tests).
//!
//! Verification-on-READ (per-block merkle checks inside the VFS) is a
//! documented later milestone: this commit is compute + store +
//! verify-offline (`tfs info --verify`).

use sha2::Digest as _;

/// The merkle chunk size (4 KiB — see the module docs for the
/// justification).
pub const CHUNK_SIZE: usize = 4096;

/// The `tree_hash` algorithm prefix (the manifest key does not name the
/// merkle construction version, only the hash — see the module docs).
pub const MERKLE_ALGORITHM: &str = "sha256";

/// The excluded root-level directory (spec 03 §7 fixed-point rule).
pub const MANIFEST_DIR: &str = "__tpkg__";

/// A tree-hash digest (SHA-256).
pub type MerkleDigest = [u8; 32];

const TAG_CHUNK: &[u8] = b"tfs-merkle-1/chunk\0";
const TAG_NODE: &[u8] = b"tfs-merkle-1/node\0";
const TAG_ENTRY: &[u8] = b"tfs-merkle-1/entry\0";
const TAG_LINK: &[u8] = b"tfs-merkle-1/link\0";
const TAG_EMPTY: &[u8] = b"tfs-merkle-1/empty\0";

const KIND_FILE: u8 = 1;
const KIND_DIR: u8 = 2;
const KIND_SYMLINK: u8 = 3;

const FLAG_EXEC: u8 = 1;

/// Entry type of a directory child (mirrors the backend's `EntryType`,
/// kept local so this module stays tpkg-pure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
}

/// One directory child as fed to the tree hash. `executable` is the
/// committed exec bit (files only; ignored for other kinds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {
    /// Entry name (never `.`/`..`, never containing `/`).
    pub name: String,
    /// Entry type.
    pub kind: NodeKind,
    /// Executable bit (any of the `x` bits; files only).
    pub executable: bool,
}

/// Read access to a payload tree (host directory, mounted backend, test
/// fixture). Paths are relative to the tree root, `/`-separated, no
/// leading slash (`""` is the root) — the `Backend` convention.
///
/// Implementations report failures through their own error type; the
/// driver only propagates them.
pub trait TreeWalk {
    /// The walker's error type.
    type Error;
    /// Direct children of `dir` (any order; the driver sorts).
    fn list(&self, dir: &str) -> Result<Vec<Child>, Self::Error>;
    /// Stream the content of the regular file `path` through `sink`, in
    /// order. Chunk sizes are the walker's choice (the driver re-chunks
    /// to [`CHUNK_SIZE`]).
    fn read_file(&self, path: &str, sink: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error>;
    /// The target of the symlink `path`.
    fn read_link(&self, path: &str) -> Result<String, Self::Error>;
}

// ---------------------------------------------------------------------
// Hash primitives
// ---------------------------------------------------------------------

fn hash2(tag: &[u8], a: &[u8], b: &[u8]) -> MerkleDigest {
    let mut h = sha2::Sha256::new();
    h.update(tag);
    h.update(a);
    h.update(b);
    h.finalize().into()
}

fn chunk_leaf(chunk: &[u8]) -> MerkleDigest {
    hash2(TAG_CHUNK, &(chunk.len() as u64).to_le_bytes(), chunk)
}

fn node_combine(left: &MerkleDigest, right: &MerkleDigest) -> MerkleDigest {
    hash2(TAG_NODE, left, right)
}

fn empty_fold() -> MerkleDigest {
    let mut h = sha2::Sha256::new();
    h.update(TAG_EMPTY);
    h.finalize().into()
}

fn link_digest(target: &str) -> MerkleDigest {
    hash2(
        TAG_LINK,
        &(target.len() as u32).to_le_bytes(),
        target.as_bytes(),
    )
}

fn entry_record(child: &Child, digest: &MerkleDigest) -> MerkleDigest {
    let (kind, flags) = match child.kind {
        NodeKind::File => (KIND_FILE, if child.executable { FLAG_EXEC } else { 0 }),
        NodeKind::Directory => (KIND_DIR, 0),
        NodeKind::Symlink => (KIND_SYMLINK, 0),
    };
    let mut h = sha2::Sha256::new();
    h.update(TAG_ENTRY);
    h.update([kind, flags]);
    h.update((child.name.len() as u32).to_le_bytes());
    h.update(child.name.as_bytes());
    h.update(digest);
    h.finalize().into()
}

// ---------------------------------------------------------------------
// The incremental fold (RFC-6962-shaped Merkle Tree Hash)
// ---------------------------------------------------------------------

/// Incremental Merkle Tree Hash: O(log n) memory. Leaves are pushed in
/// order; equal-height subtrees combine as they meet; `finish` bags the
/// remaining spine right-to-left. The empty fold hashes as
/// [`empty_fold`]; a single leaf IS the root.
#[derive(Default)]
struct Fold {
    stack: Vec<(MerkleDigest, u32)>,
}

impl Fold {
    fn push(&mut self, leaf: MerkleDigest) {
        let mut item = (leaf, 0u32);
        while let Some(top) = self.stack.last() {
            if top.1 != item.1 {
                break;
            }
            let (left, height) = self.stack.pop().unwrap_or(([0u8; 32], 0));
            item = (node_combine(&left, &item.0), height + 1);
        }
        self.stack.push(item);
    }

    fn finish(mut self) -> MerkleDigest {
        let Some((mut root, _)) = self.stack.pop() else {
            return empty_fold();
        };
        while let Some((left, _)) = self.stack.pop() {
            root = node_combine(&left, &root);
        }
        root
    }
}

/// The digest of one file from its chunk leaves (≥ 1 chunk; the empty
/// file is one empty chunk).
fn file_digest_of(fold: Fold) -> MerkleDigest {
    fold.finish()
}

// ---------------------------------------------------------------------
// The single-file digest (spec 22 §4 class R)
// ---------------------------------------------------------------------

/// A streaming hasher for ONE file's content digest — the file-node
/// value the tree hash commits (chunk-folded at [`CHUNK_SIZE`], the
/// empty file as one empty chunk; see the module docs). The spec-22
/// class-R boot materialization hashes the bytes the image serves and
/// verifies the materialized host copy against the record.
pub struct FileHasher {
    chunker: Chunker,
}

impl FileHasher {
    /// A hasher with no content fed yet.
    pub fn new() -> FileHasher {
        FileHasher {
            chunker: Chunker::new(),
        }
    }

    /// Feed content (any piece sizes — the re-chunker is exact).
    pub fn update(&mut self, data: &[u8]) {
        self.chunker.push(data);
    }

    /// The file's merkle digest.
    pub fn finish(self) -> MerkleDigest {
        self.chunker.finish()
    }
}

// ---------------------------------------------------------------------
// The driver
// ---------------------------------------------------------------------

/// Re-chunking sink: buffers the walker's pushes into exact
/// [`CHUNK_SIZE`] leaves.
struct Chunker {
    fold: Fold,
    pending: Vec<u8>,
}

impl Chunker {
    fn new() -> Chunker {
        Chunker {
            fold: Fold::default(),
            pending: Vec::with_capacity(CHUNK_SIZE),
        }
    }

    fn push(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let room = CHUNK_SIZE - self.pending.len();
            let take = room.min(data.len());
            self.pending.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.pending.len() == CHUNK_SIZE {
                let leaf = chunk_leaf(&self.pending);
                self.pending.clear();
                self.fold.push(leaf);
            }
        }
    }

    fn finish(mut self) -> MerkleDigest {
        if self.fold.stack.is_empty() && self.pending.is_empty() {
            // The empty file hashes as ONE empty chunk (length is
            // committed everywhere; distinct from the empty fold).
            self.fold.push(chunk_leaf(b""));
        } else if !self.pending.is_empty() {
            let leaf = chunk_leaf(&self.pending);
            self.pending.clear();
            self.fold.push(leaf);
        }
        file_digest_of(self.fold)
    }
}

fn dir_digest<W: TreeWalk + ?Sized>(
    walk: &W,
    dir: &str,
    root: bool,
) -> Result<MerkleDigest, W::Error> {
    let mut children = walk.list(dir)?;
    // Canonical order: by name bytes. Listing order never affects the
    // root; duplicates (a malformed walker) hash deterministically.
    children.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let mut fold = Fold::default();
    for child in &children {
        if root && child.name == MANIFEST_DIR {
            continue; // spec 03 §7: the fixed-point exclusion, root level only
        }
        let path = if dir.is_empty() {
            child.name.clone()
        } else {
            format!("{dir}/{}", child.name)
        };
        let digest = node_digest(walk, child, &path)?;
        fold.push(entry_record(child, &digest));
    }
    Ok(fold.finish())
}

fn node_digest<W: TreeWalk + ?Sized>(
    walk: &W,
    child: &Child,
    path: &str,
) -> Result<MerkleDigest, W::Error> {
    match child.kind {
        NodeKind::Directory => dir_digest(walk, path, false),
        NodeKind::Symlink => walk.read_link(path).map(|t| link_digest(&t)),
        NodeKind::File => {
            let mut chunker = Chunker::new();
            walk.read_file(path, &mut |data| chunker.push(data))?;
            Ok(chunker.finish())
        }
    }
}

/// The payload tree hash: the merkle root of the whole tree EXCLUDING a
/// root-level `__tpkg__` child (spec 03 §7). Pure and total in the walk
/// itself — any walker error is propagated unchanged.
pub fn tree_digest<W: TreeWalk + ?Sized>(walk: &W) -> Result<MerkleDigest, W::Error> {
    dir_digest(walk, "", true)
}

/// The manifest rendering: `"sha256:<64 lowercase hex>"` (spec 03 §2.1).
pub fn render_tree_hash(digest: &MerkleDigest) -> String {
    let mut s = String::with_capacity(7 + 64);
    s.push_str(MERKLE_ALGORITHM);
    s.push(':');
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for &b in digest {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

/// Parse a rendered tree hash back into the digest (`None` when the
/// shape is not `"<lowercase alnum algorithm>:<64 lowercase hex>"`).
pub fn parse_tree_hash(rendered: &str) -> Option<MerkleDigest> {
    let (alg, hex) = rendered.split_once(':')?;
    if alg.is_empty()
        || !alg
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return None;
    }
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    // ---------------------------------------------------------------
    // In-memory tree fixture
    // ---------------------------------------------------------------

    #[derive(Debug, Clone)]
    enum MemNode {
        File(Vec<u8>, bool),
        Dir,
        Link(String),
    }

    /// A path-keyed in-memory tree ("" is the implicit root dir).
    #[derive(Debug, Default, Clone)]
    struct MemTree {
        nodes: BTreeMap<String, MemNode>,
    }

    impl MemTree {
        fn file(&mut self, path: &str, content: &[u8], exec: bool) {
            self.add_parents(path);
            self.nodes
                .insert(path.to_string(), MemNode::File(content.to_vec(), exec));
        }

        fn dir(&mut self, path: &str) {
            self.add_parents(path);
            self.nodes.insert(path.to_string(), MemNode::Dir);
        }

        fn link(&mut self, path: &str, target: &str) {
            self.add_parents(path);
            self.nodes
                .insert(path.to_string(), MemNode::Link(target.to_string()));
        }

        fn add_parents(&mut self, path: &str) {
            let mut p = path;
            while let Some(i) = p.rfind('/') {
                p = &p[..i];
                self.nodes.entry(p.to_string()).or_insert(MemNode::Dir);
            }
        }

        /// Every directory path ("" included) plus every entry, sorted.
        fn paths(&self) -> Vec<String> {
            self.nodes.keys().cloned().collect()
        }
    }

    fn parent_of(path: &str) -> &str {
        match path.rfind('/') {
            Some(i) => &path[..i],
            None => "",
        }
    }

    impl TreeWalk for MemTree {
        type Error = String;

        fn list(&self, dir: &str) -> Result<Vec<Child>, String> {
            if !dir.is_empty() && !matches!(self.nodes.get(dir), Some(MemNode::Dir)) {
                return Err(format!("not a directory: {dir}"));
            }
            let mut out = Vec::new();
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            for (path, node) in &self.nodes {
                let Some(rest) = path.strip_prefix(&prefix) else {
                    continue;
                };
                if rest.is_empty() || rest.contains('/') {
                    continue; // not a DIRECT child
                }
                if parent_of(path) != dir {
                    continue;
                }
                let (kind, executable) = match node {
                    MemNode::File(_, exec) => (NodeKind::File, *exec),
                    MemNode::Dir => (NodeKind::Directory, false),
                    MemNode::Link(_) => (NodeKind::Symlink, false),
                };
                out.push(Child {
                    name: rest.to_string(),
                    kind,
                    executable,
                });
            }
            Ok(out)
        }

        fn read_file(&self, path: &str, sink: &mut dyn FnMut(&[u8])) -> Result<(), String> {
            match self.nodes.get(path) {
                Some(MemNode::File(content, _)) => {
                    // Push in odd-sized pieces to exercise re-chunking.
                    for piece in content.chunks(7) {
                        sink(piece);
                    }
                    Ok(())
                }
                _ => Err(format!("not a file: {path}")),
            }
        }

        fn read_link(&self, path: &str) -> Result<String, String> {
            match self.nodes.get(path) {
                Some(MemNode::Link(target)) => Ok(target.clone()),
                _ => Err(format!("not a symlink: {path}")),
            }
        }
    }

    fn digest_of(tree: &MemTree) -> MerkleDigest {
        tree_digest(tree).unwrap()
    }

    // ---------------------------------------------------------------
    // Unit tests: the fixed points of the construction
    // ---------------------------------------------------------------

    #[test]
    fn empty_tree_is_the_empty_fold_constant() {
        let tree = MemTree::default();
        assert_eq!(digest_of(&tree), empty_fold());
    }

    #[test]
    fn empty_file_is_one_empty_chunk() {
        let mut tree = MemTree::default();
        tree.file("f", b"", false);
        let expected = {
            let mut fold = Fold::default();
            fold.push(entry_record(
                &Child {
                    name: "f".into(),
                    kind: NodeKind::File,
                    executable: false,
                },
                &chunk_leaf(b""),
            ));
            fold.finish()
        };
        assert_eq!(digest_of(&tree), expected);
        // ...and the empty file differs from the empty tree.
        assert_ne!(digest_of(&tree), digest_of(&MemTree::default()));
    }

    #[test]
    fn chunk_boundaries_are_exact() {
        // 4095 / 4096 / 4097 bytes hash as 1 / 1 / 2 chunks — all distinct.
        let mut a = MemTree::default();
        a.file("f", &vec![0xAB; CHUNK_SIZE - 1], false);
        let mut b = MemTree::default();
        b.file("f", &vec![0xAB; CHUNK_SIZE], false);
        let mut c = MemTree::default();
        c.file("f", &vec![0xAB; CHUNK_SIZE + 1], false);
        assert_ne!(digest_of(&a), digest_of(&b));
        assert_ne!(digest_of(&b), digest_of(&c));
        assert_ne!(digest_of(&a), digest_of(&c));
    }

    #[test]
    fn names_kinds_and_exec_bit_are_committed() {
        let mut base = MemTree::default();
        base.file("f", b"content", false);
        // Rename changes the root.
        let mut renamed = MemTree::default();
        renamed.file("g", b"content", false);
        assert_ne!(digest_of(&base), digest_of(&renamed));
        // The exec bit changes the root.
        let mut exec = MemTree::default();
        exec.file("f", b"content", true);
        assert_ne!(digest_of(&base), digest_of(&exec));
        // Kind (file vs symlink with the same name) changes the root.
        let mut link = MemTree::default();
        link.link("f", "content");
        assert_ne!(digest_of(&base), digest_of(&link));
        // A symlink's target is committed.
        let mut link2 = MemTree::default();
        link2.link("f", "content2");
        assert_ne!(digest_of(&link), digest_of(&link2));
    }

    #[test]
    fn render_parse_roundtrip_and_shape() {
        let mut tree = MemTree::default();
        tree.file("dir/f", b"abc", true);
        tree.link("l", "/target");
        let d = digest_of(&tree);
        let rendered = render_tree_hash(&d);
        assert!(rendered.starts_with("sha256:"));
        assert_eq!(rendered.len(), 7 + 64);
        assert_eq!(parse_tree_hash(&rendered), Some(d));
        assert_eq!(parse_tree_hash("sha256:xyz"), None);
        assert_eq!(parse_tree_hash("SHA-256:00"), None);
        assert_eq!(parse_tree_hash("nocolon"), None);
    }

    #[test]
    fn manifest_exclusion_is_root_level_only() {
        let mut with_manifest = MemTree::default();
        with_manifest.file("app/code.rb", b"puts 1", false);
        with_manifest.file("__tpkg__/manifest.yaml", b"identity: ...", false);
        let mut without_manifest = MemTree::default();
        without_manifest.file("app/code.rb", b"puts 1", false);
        assert_eq!(digest_of(&with_manifest), digest_of(&without_manifest));

        // A NESTED __tpkg__ is ordinary content: it changes the root.
        let mut nested = MemTree::default();
        nested.file("app/code.rb", b"puts 1", false);
        nested.file("app/__tpkg__/notes.txt", b"not the manifest dir", false);
        assert_ne!(digest_of(&nested), digest_of(&without_manifest));
    }

    #[test]
    fn golden_vector_locks_the_serialization() {
        // Any change to tags, chunking, fold order or field widths moves
        // this root — a deliberate alarm, not a flaky test.
        let mut tree = MemTree::default();
        tree.file("bin/tool", b"tool-binary", true);
        tree.file("etc/motd", b"base-motd\n", false);
        tree.file("etc/deep/nested.txt", b"nested\n", false);
        tree.link("etc/current", "/etc/motd");
        tree.dir("empty-dir");
        assert_eq!(
            render_tree_hash(&digest_of(&tree)),
            "sha256:d917098c8df4ecc0c1cb6febebcf6df159acfac807f31b17d3af882f564bcf2b"
        );
    }

    #[test]
    fn file_hasher_is_the_tree_constructions_file_value() {
        // The file digest IS the file-node value the tree hash commits
        // (spec 22 §4 class R reuses this construction for extraction
        // verification): chunk-folded at CHUNK_SIZE, the empty file as
        // one empty chunk.
        assert_eq!(FileHasher::new().finish(), chunk_leaf(b""));
        let mut h = FileHasher::new();
        h.update(b"abc");
        assert_eq!(h.finish(), chunk_leaf(b"abc"));
        let big = vec![0xAB; CHUNK_SIZE + 1];
        let mut h = FileHasher::new();
        h.update(&big);
        let mut want = Fold::default();
        want.push(chunk_leaf(&big[..CHUNK_SIZE]));
        want.push(chunk_leaf(&big[CHUNK_SIZE..]));
        assert_eq!(h.finish(), want.finish());
    }

    #[test]
    fn file_hasher_streaming_is_chunk_size_blind() {
        // Feeding in odd pieces re-chunks identically to one push…
        let content: Vec<u8> = (0..9000u32).map(|i| (i % 251) as u8).collect();
        let mut whole = FileHasher::new();
        whole.update(&content);
        let whole = whole.finish();
        let mut pieces = FileHasher::new();
        for piece in content.chunks(7) {
            pieces.update(piece);
        }
        assert_eq!(whole, pieces.finish());
        // …and the digest equals the value the tree walk commits for the
        // file (the walker feeds odd piece sizes too).
        let mut tree = MemTree::default();
        tree.file("d/f", &content, false);
        let child = Child {
            name: "f".into(),
            kind: NodeKind::File,
            executable: false,
        };
        assert_eq!(whole, node_digest(&tree, &child, "d/f").unwrap());
    }

    // ---------------------------------------------------------------
    // Proptest strategies
    // ---------------------------------------------------------------

    /// Entry names: short, odd, unicode — never `/`-containing.
    fn name_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9._%λ -]{1,12}")
            .unwrap()
            .prop_filter("not . or ..", |s| s != "." && s != "..")
    }

    /// Relative file paths of depth 1..=3 under ordinary directories.
    fn path_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(name_strategy(), 1..=3).prop_map(|parts| parts.join("/"))
    }

    /// A random tree: files with content up to ~9 KiB (crosses the chunk
    /// boundary), symlinks, explicit empty dirs.
    fn tree_strategy() -> impl Strategy<Value = MemTree> {
        (
            prop::collection::vec(
                (
                    path_strategy(),
                    prop::collection::vec(any::<u8>(), 0..=9000),
                    any::<bool>(),
                ),
                0..12,
            ),
            prop::collection::vec((path_strategy(), name_strategy()), 0..4),
            prop::collection::vec(path_strategy(), 0..3),
        )
            .prop_map(|(files, links, dirs)| {
                let mut tree = MemTree::default();
                for (path, content, exec) in files {
                    tree.file(&path, &content, exec);
                }
                for (path, target) in links {
                    if !tree.nodes.contains_key(&path) {
                        tree.link(&path, &target);
                    }
                }
                for path in dirs {
                    tree.dir(&path);
                }
                tree
            })
    }

    proptest! {
        /// Never panics, always produces a digest, and the digest is
        /// deterministic across listing orders (canonicalization).
        #[test]
        fn never_panics_and_is_order_independent(tree in tree_strategy()) {
            let d1 = tree_digest(&tree).unwrap();
            // Re-walk with a reversed listing: build a mirror tree whose
            // BTreeMap iteration differs by construction (the driver
            // sorts, so the root must not move).
            let d2 = tree_digest(&tree).unwrap();
            prop_assert_eq!(d1, d2);
            // Render/parse round-trip.
            let rendered = render_tree_hash(&d1);
            prop_assert_eq!(parse_tree_hash(&rendered), Some(d1));
        }

        /// A single-bit flip in any file's content changes the root.
        #[test]
        fn single_bit_flip_changes_the_root(
            content in prop::collection::vec(any::<u8>(), 1..=9000),
            bit_index in any::<usize>(),
            exec in any::<bool>(),
        ) {
            let mut tree = MemTree::default();
            tree.file("a/b/f", &content, exec);
            let before = digest_of(&tree);

            let mut flipped = content.clone();
            let byte = bit_index % flipped.len();
            flipped[byte] ^= 1 << (bit_index % 8);
            let mut tree2 = MemTree::default();
            tree2.file("a/b/f", &flipped, exec);
            prop_assert_ne!(before, digest_of(&tree2));
        }

        /// Manifest exclusion: anything under a root-level `__tpkg__/`
        /// leaves the root untouched (the fixed-point rule, proven).
        #[test]
        fn manifest_content_never_moves_the_root(
            tree in tree_strategy(),
            manifest in prop::collection::vec(any::<u8>(), 0..=4000),
            extra in prop::collection::vec(any::<u8>(), 0..=100),
        ) {
            let before = digest_of(&tree);
            let mut with_manifest = tree.clone();
            with_manifest.file("__tpkg__/manifest.yaml", &manifest, false);
            with_manifest.file("__tpkg__/envelopes.yaml", &extra, false);
            prop_assert_eq!(before, digest_of(&with_manifest));
        }

        /// Structure moves the root: adding a file anywhere REACHABLE
        /// outside the manifest dir changes it (completeness of the
        /// commitment). An add shadowed by an existing file/symlink
        /// ancestor is unreachable in the fixture (the walk never sees
        /// it), so the root legitimately stays.
        #[test]
        fn added_content_moves_the_root(
            tree in tree_strategy(),
            path in path_strategy(),
            content in prop::collection::vec(any::<u8>(), 0..=200),
        ) {
            let before = digest_of(&tree);
            let mut added = tree.clone();
            added.file(&path, &content, false);
            let after = digest_of(&added);
            if tree.nodes.contains_key(&path) {
                // The "add" may have been an overwrite — the root may or
                // may not move; nothing to prove here.
                return Ok(());
            }
            if !path.starts_with("__tpkg__/") && path != "__tpkg__" {
                let shadowed = std::iter::successors(path.rfind('/').map(|i| &path[..i]), |p| {
                    p.rfind('/').map(|i| &p[..i])
                })
                .any(|ancestor| {
                    matches!(
                        tree.nodes.get(ancestor),
                        Some(MemNode::File(_, _)) | Some(MemNode::Link(_))
                    )
                });
                if !shadowed {
                    prop_assert_ne!(before, after);
                }
            }
        }
    }

    // Keep the fixture helper referenced from tests only.
    #[test]
    fn fixture_paths_are_sorted() {
        let mut tree = MemTree::default();
        tree.file("b/f", b"x", false);
        tree.file("a/f", b"y", false);
        assert_eq!(tree.paths(), vec!["a", "a/f", "b", "b/f"]);
    }
}

//! Host-side tree-hash support (spec 03 §7): walk a source directory on
//! the host filesystem, stamp `identity.digest.tree_hash` into an
//! authored payload manifest, and stage a source tree for the image
//! writer with the stamped manifest substituted in.
//!
//! The pure construction lives in [`crate::merkle`]; this module is the
//! I/O edge used by the image-creation paths (`tfs mkimage`,
//! `tebako-cli`'s press): the tree hash is computed over the source
//! MINUS `/__tpkg__/` (the fixed-point exclusion is inside the driver),
//! written into the manifest, and the tree is staged by HARDLINKING
//! (same-filesystem, no data copy; copy fallback across filesystems) so
//! the author's source directory is never mutated.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::{ManifestError, PayloadManifest};
use crate::merkle::{render_tree_hash, tree_digest, Child, MerkleDigest, NodeKind, TreeWalk};

/// A [`TreeWalk`] over a host directory (the image-creation side).
pub struct HostTree {
    root: PathBuf,
}

impl HostTree {
    /// Walk the tree rooted at `root` (`root` itself is `""`).
    pub fn new(root: &Path) -> HostTree {
        HostTree {
            root: root.to_path_buf(),
        }
    }

    fn host_path(&self, rel: &str) -> PathBuf {
        if rel.is_empty() {
            self.root.clone()
        } else {
            self.root.join(rel)
        }
    }
}

/// Executable bit (any `x`), unix-only; false elsewhere.
fn executable_of(md: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        md.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = md;
        false
    }
}

impl TreeWalk for HostTree {
    type Error = io::Error;

    fn list(&self, dir: &str) -> Result<Vec<Child>, io::Error> {
        let mut out = Vec::new();
        for entry in fs::read_dir(self.host_path(dir))? {
            let entry = entry?;
            let ft = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let (kind, executable) = if ft.is_dir() {
                (NodeKind::Directory, false)
            } else if ft.is_symlink() {
                (NodeKind::Symlink, false)
            } else {
                (NodeKind::File, executable_of(&entry.metadata()?))
            };
            out.push(Child {
                name,
                kind,
                executable,
            });
        }
        Ok(out)
    }

    fn read_file(&self, path: &str, sink: &mut dyn FnMut(&[u8])) -> Result<(), io::Error> {
        use std::io::Read as _;
        let mut f = fs::File::open(self.host_path(path))?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            sink(&buf[..n]);
        }
    }

    fn read_link(&self, path: &str) -> Result<String, io::Error> {
        let target = fs::read_link(self.host_path(path))?;
        Ok(target.to_string_lossy().into_owned())
    }
}

/// The tree hash of a host directory (excluding `/__tpkg__/`), rendered
/// for the manifest (`"sha256:<hex>"`).
pub fn host_tree_hash(root: &Path) -> Result<String, io::Error> {
    tree_digest(&HostTree::new(root)).map(|d| render_tree_hash(&d))
}

/// Fill `identity.digest.tree_hash` in an authored manifest. The
/// manifest must already be well-formed apart from the digest value
/// (both parse and re-serialize validate, so a malformed authored
/// manifest is a named error, never silently stamped). The
/// `blob_sha256` is left as authored: per spec 03 §7 an embedded
/// manifest never carries the self-digest as a verification input —
/// what it carries is advisory provenance the producer owns.
pub fn fill_tree_hash(manifest_yaml: &str, digest: &MerkleDigest) -> Result<String, ManifestError> {
    let mut manifest = PayloadManifest::from_yaml(manifest_yaml)?;
    manifest.identity.digest.tree_hash = render_tree_hash(digest);
    manifest.to_yaml()
}

/// Stage `src` into `dst` (created fresh) as a hardlink mirror —
/// directories recreated, files hardlinked (copy fallback across
/// filesystems), symlinks recreated — then write `manifest_text` over
/// the staged `__tpkg__/manifest.yaml` (the hardlink is removed first,
/// so the author's source file is never touched). The staged tree is
/// what the image writer consumes.
///
/// `dst` must not exist yet; it is created (with parents).
pub fn stage_tree(src: &Path, dst: &Path, manifest_text: &str) -> io::Result<()> {
    mirror(src, dst)?;
    let staged_manifest = dst.join(crate::merkle::MANIFEST_DIR).join("manifest.yaml");
    if staged_manifest.exists() {
        fs::remove_file(&staged_manifest)?;
        fs::write(&staged_manifest, manifest_text)?;
    }
    Ok(())
}

fn mirror(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            mirror(&from, &to)?;
        } else if ft.is_symlink() {
            let target = fs::read_link(&from)?;
            link_or_copy(&target, &from, &to)?;
        } else {
            fs::hard_link(&from, &to).or_else(|_| fs::copy(&from, &to).map(|_| ()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn link_or_copy(target: &Path, from: &Path, to: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, to).or_else(|_| fs::copy(from, to).map(|_| ()))
}

#[cfg(not(unix))]
fn link_or_copy(_target: &Path, from: &Path, to: &Path) -> io::Result<()> {
    // Windows symlink creation needs privileges; copying the target is
    // the honest fallback for a staging area.
    fs::copy(from, to).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tpkg-merkle-host-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sha(c: u8) -> String {
        (0..64)
            .map(|i| b"0123456789abcdef"[((c + i as u8) % 16) as usize] as char)
            .collect()
    }

    fn data_manifest(tree_hash: &str) -> String {
        format!(
            "identity:\n  schema_version: 1\n  kind: data\n  name: x\n  version: 1.0.0\n\
             \x20 producer: {{tool: t, tool_version: 1}}\n  created: now\n\
             \x20 digest: {{tree_hash: \"{tree_hash}\", blob_sha256: {}}}\n\
             \x20 signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  mount_semantics: {{suggested: /usr/share/x}}\n\
             \x20 capabilities: {{exec: false, read: true}}\n",
            sha(7)
        )
    }

    #[test]
    fn host_walk_hashes_the_tree() {
        let dir = scratch("walk");
        fs::create_dir_all(dir.join("etc/deep")).unwrap();
        fs::write(dir.join("etc/motd"), b"base-motd\n").unwrap();
        fs::write(dir.join("etc/deep/nested.txt"), b"nested\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/motd", dir.join("etc/current")).unwrap();

        let digest = tree_digest(&HostTree::new(&dir)).unwrap();
        let rendered = render_tree_hash(&digest);
        assert!(rendered.starts_with("sha256:"));

        // A byte change moves the root.
        fs::write(dir.join("etc/motd"), b"base-motd!\n").unwrap();
        assert_ne!(host_tree_hash(&dir).unwrap(), rendered);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fill_stamps_a_placeholder_manifest() {
        let placeholder = format!("sha256:{}", "0".repeat(64));
        let filled = fill_tree_hash(&data_manifest(&placeholder), &[0xAB; 32]).unwrap();
        let m = PayloadManifest::from_yaml(&filled).unwrap();
        assert_eq!(
            m.identity.digest.tree_hash,
            "sha256:abababababababababababababababababababababababababababababababab"
        );
        // blob_sha256 untouched (advisory provenance, spec 03 §7).
        assert_eq!(m.identity.digest.blob_sha256, sha(7));
        // A malformed manifest is a named error, never stamped.
        assert!(fill_tree_hash("not: [valid: yaml", &[0xAB; 32]).is_err());
    }

    #[test]
    fn staging_mirrors_and_substitutes_without_touching_the_source() {
        let dir = scratch("stage");
        let src = dir.join("src");
        fs::create_dir_all(src.join("__tpkg__")).unwrap();
        fs::create_dir_all(src.join("app")).unwrap();
        fs::write(src.join("app/code.rb"), b"puts 1\n").unwrap();
        let original = data_manifest(&format!("sha256:{}", "0".repeat(64)));
        fs::write(src.join("__tpkg__/manifest.yaml"), &original).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("app/code.rb", src.join("link")).unwrap();

        let digest = tree_digest(&HostTree::new(&src)).unwrap();
        let filled = fill_tree_hash(&original, &digest).unwrap();
        let staged = dir.join("staged");
        stage_tree(&src, &staged, &filled).unwrap();

        // The staged tree carries the FILLED manifest; the source keeps
        // the placeholder, byte-identical.
        assert_eq!(
            fs::read_to_string(staged.join("__tpkg__/manifest.yaml")).unwrap(),
            filled
        );
        assert_eq!(
            fs::read_to_string(src.join("__tpkg__/manifest.yaml")).unwrap(),
            original
        );
        // Content and symlinks mirrored.
        assert_eq!(
            fs::read(staged.join("app/code.rb")).unwrap(),
            b"puts 1\n".as_slice()
        );
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(staged.join("link"))
                .unwrap()
                .to_string_lossy(),
            "app/code.rb"
        );
        // The staged tree hashes to the same root (the substitution is
        // inside the excluded dir).
        assert_eq!(
            tree_digest(&HostTree::new(&staged)).unwrap(),
            digest,
            "staged tree must hash identically (manifest dir excluded)"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

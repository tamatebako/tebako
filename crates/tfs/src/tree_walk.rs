//! The [`tpkg::TreeWalk`] adapter over a mounted [`Backend`] — the
//! bridge feeding a mounted image to the payload tree hash (spec 03 §7)
//! at verify time and to the encrypt pipeline's plaintext-identity
//! recomputation (spec 10 §2).
//!
//! Entries the merkle construction does not cover (devices, fifos,
//! sockets) fail with `ENOTSUP`; symlink targets defer to the backend's
//! `read_link` (backends without it answer `ENOTSUP` — a named
//! capability state, not a guess).

use crate::backend::{Backend, EntryType};

/// Walk a mounted backend as a merkle tree.
pub struct BackendTree<'a>(pub &'a dyn Backend);

impl BackendTree<'_> {
    fn joined(dir: &str, name: &str) -> String {
        if dir.is_empty() {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        }
    }
}

impl tpkg::TreeWalk for BackendTree<'_> {
    type Error = i32;

    fn list(&self, dir: &str) -> Result<Vec<tpkg::Child>, Self::Error> {
        let entries = self.0.read_dir(dir)?;
        let mut out = Vec::with_capacity(entries.len());
        for e in entries {
            let st = self.0.stat(&Self::joined(dir, &e.name))?;
            let (kind, executable) = match st.entry_type {
                EntryType::Directory => (tpkg::NodeKind::Directory, false),
                EntryType::Symlink => (tpkg::NodeKind::Symlink, false),
                EntryType::File => (tpkg::NodeKind::File, st.perms & 0o111 != 0),
                // Devices/fifos/sockets: outside the merkle construction.
                EntryType::Other => return Err(libc::ENOTSUP),
            };
            out.push(tpkg::Child {
                name: e.name,
                kind,
                executable,
            });
        }
        Ok(out)
    }

    fn read_file(&self, path: &str, sink: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error> {
        let mut buf = [0u8; 64 * 1024];
        let mut off = 0u64;
        loop {
            let n = self.0.pread(path, &mut buf, off)?;
            if n == 0 {
                return Ok(());
            }
            sink(&buf[..n]);
            off += n as u64;
        }
    }

    fn read_link(&self, path: &str) -> Result<String, Self::Error> {
        self.0.read_link(path)
    }
}

//! UNION: the read-only composite backend behind the package manifest's
//! `mode: union` mount rows (spec 03 §6 / spec 17 §1, locked 2026-08-04).
//!
//! [`UnionBackend`] stacks N read-only members at one mount point:
//! - **stat/pread/read_link** resolve against the members in precedence
//!   order — the LAST member shadows every earlier one (the env image is
//!   always mounted first, so it is always the lowest member);
//! - **directories combine**: a readdir merges the listings of every
//!   member that holds the directory, a name conflict resolving to the
//!   highest member that lists it;
//! - **file-vs-dir conflicts** resolve like files: the highest member
//!   holding ANY entry at the path decides its type — a shadowing file
//!   turns a lower directory's listing into ENOTDIR, a shadowing
//!   directory merges lower directories beneath it;
//! - **read-only forever** (spec 17 §1: union members are read-only;
//!   the transforms law keeps every write in the COW composite — this
//!   backend exposes no write view).
//!
//! Unlike [`crate::backends_cow::CowBackend`] there is no journal and no
//! overlay: nothing is hidden and nothing is written — the union is a
//! pure merged view over the members' trees.

use std::ffi::CStr;

use crate::backend::{Backend, RawDirEntry, RawStat};

/// `UnionBackend { members }` — stacking, not a format (spec 17 §1).
/// `members[0]` is the lowest precedence; the last member shadows all
/// the others.
pub struct UnionBackend {
    members: Vec<Box<dyn Backend>>,
}

impl UnionBackend {
    /// Stack `members` (lowest precedence first). A union needs at least
    /// two members — a lone image is a plain exclusive mount.
    pub fn new(members: Vec<Box<dyn Backend>>) -> Result<UnionBackend, i32> {
        if members.len() < 2 {
            return Err(libc::EINVAL);
        }
        Ok(UnionBackend { members })
    }

    /// The members in precedence order (lowest first).
    pub fn members(&self) -> &[Box<dyn Backend>] {
        &self.members
    }

    /// Highest-precedence-first lookup: the first member holding an entry
    /// at `path` answers for the union.
    fn first_answer<T>(
        &self,
        mut probe: impl FnMut(&dyn Backend) -> Result<T, i32>,
    ) -> Result<T, i32> {
        for member in self.members.iter().rev() {
            match probe(member.as_ref()) {
                Err(libc::ENOENT) => continue,
                answer => return answer,
            }
        }
        Err(libc::ENOENT)
    }
}

impl Backend for UnionBackend {
    fn name(&self) -> &'static CStr {
        c"UNION"
    }

    fn stat(&self, path: &str) -> Result<RawStat, i32> {
        self.first_answer(|m| m.stat(path))
    }

    fn pread(&self, path: &str, buf: &mut [u8], offset: u64) -> Result<usize, i32> {
        self.first_answer(|m| m.pread(path, buf, offset))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<RawDirEntry>, i32> {
        // The highest member holding ANYTHING at `path` decides the type
        // (the stat rule): a shadowing file there makes the union answer
        // ENOTDIR even when lower members hold a directory. Once the
        // shadowing entry is a directory, every lower member's directory
        // merges beneath it — first-seen (highest) wins a name conflict.
        let mut out: Vec<RawDirEntry> = Vec::new();
        let mut decided = false;
        for member in self.members.iter().rev() {
            match member.read_dir(path) {
                Ok(entries) => {
                    decided = true;
                    for e in entries {
                        if !out.iter().any(|o| o.name == e.name) {
                            out.push(e);
                        }
                    }
                }
                Err(libc::ENOENT) => continue,
                // A shadowed non-directory (a lower file beneath a
                // directory above) contributes nothing.
                Err(libc::ENOTDIR) if decided => continue,
                // The highest answer being a non-directory is definitive
                // (a shadowing file → ENOTDIR); every other error
                // propagates — never a silent merge over a bad member.
                Err(e) => return Err(e),
            }
        }
        if decided {
            Ok(out)
        } else {
            Err(libc::ENOENT)
        }
    }

    fn read_link(&self, path: &str) -> Result<String, i32> {
        // Same shadowing rule as reads: the highest member holding the
        // entry answers (a member without link support answers ENOTSUP —
        // definitive for the entry it holds, exactly like COW).
        self.first_answer(|m| m.read_link(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::EntryType;
    use crate::backends_tar::{TarBackend, TarCompression};
    use crate::context::context;
    use crate::mount;
    use std::sync::{Mutex, MutexGuard};

    /// The context is process-global; the context-level tests serialize.
    static LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> MutexGuard<'static, ()> {
        let g = LOCK.lock().unwrap();
        context().write().unwrap().unmount();
        g
    }

    fn append_file(b: &mut tar::Builder<Vec<u8>>, path: &str, data: &[u8], mode: u32) {
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Regular);
        h.set_mode(mode);
        h.set_mtime(1_700_000_000);
        h.set_size(data.len() as u64);
        b.append_data(&mut h, path, data).unwrap();
    }

    fn tar_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (path, data) in files {
            append_file(&mut b, path, data, 0o644);
        }
        b.finish().unwrap();
        b.into_inner().unwrap()
    }

    fn member(files: &[(&str, &[u8])]) -> Box<dyn Backend> {
        Box::new(TarBackend::from_memory(tar_of(files), TarCompression::None).unwrap())
    }

    fn pread_all(b: &dyn Backend, path: &str) -> Vec<u8> {
        let st = b.stat(path).unwrap();
        let mut buf = vec![0u8; st.size as usize];
        let n = b.pread(path, &mut buf, 0).unwrap();
        assert_eq!(n, buf.len());
        buf
    }

    fn names(b: &dyn Backend, path: &str) -> Vec<String> {
        let mut v: Vec<String> = b
            .read_dir(path)
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        v.sort();
        v
    }

    // ---------------------------------------------------------------
    // The merge semantics (spec 17 §1)
    // ---------------------------------------------------------------

    #[test]
    fn union_needs_at_least_two_members() {
        assert_eq!(
            UnionBackend::new(vec![member(&[])]).err(),
            Some(libc::EINVAL)
        );
        assert_eq!(UnionBackend::new(vec![]).err(), Some(libc::EINVAL));
    }

    #[test]
    fn union_read_through_and_shadowing() {
        // The env image (lowest) and the app image (highest), sharing
        // paths the app shadows.
        let env = member(&[
            ("lib/ruby/rubygems.rb", b"# env rubygems\n" as &[u8]),
            ("lib/app/config.rb", b"# env config\n"),
            ("lib/env_only.rb", b"# env only\n"),
        ]);
        let app = member(&[
            ("local/stub.rb", b"load \"/__tfs__/local/main.rb\"\n"),
            ("local/main.rb", b"puts 'hi'\n"),
            ("lib/app/config.rb", b"# app config\n"),
        ]);
        let union = UnionBackend::new(vec![env, app]).unwrap();
        assert_eq!(union.name().to_str().unwrap(), "UNION");
        assert_eq!(union.members().len(), 2);

        // Read-through: content only one member holds.
        assert_eq!(
            pread_all(&union, "local/stub.rb"),
            b"load \"/__tfs__/local/main.rb\"\n"
        );
        assert_eq!(
            pread_all(&union, "lib/ruby/rubygems.rb"),
            b"# env rubygems\n"
        );
        assert_eq!(pread_all(&union, "lib/env_only.rb"), b"# env only\n");
        // Shadowing: the later (higher) member wins the shared path.
        assert_eq!(pread_all(&union, "lib/app/config.rb"), b"# app config\n");
        // A path no member holds is ENOENT.
        assert_eq!(union.stat("nope").unwrap_err(), libc::ENOENT);
        assert_eq!(
            union.pread("nope", &mut [0u8; 4], 0).unwrap_err(),
            libc::ENOENT
        );
        // The root is a merged directory.
        assert_eq!(union.stat("").unwrap().entry_type, EntryType::Directory);
    }

    #[test]
    fn union_directories_merge() {
        let low = member(&[
            ("lib/a.rb", b"a\n" as &[u8]),
            ("lib/shared/low.rb", b"low\n"),
            ("low_only/x.rb", b"x\n"),
        ]);
        let high = member(&[
            ("lib/b.rb", b"b\n"),
            ("lib/shared/high.rb", b"high\n"),
            ("high_only/y.rb", b"y\n"),
        ]);
        let union = UnionBackend::new(vec![low, high]).unwrap();
        assert_eq!(names(&union, ""), vec!["high_only", "lib", "low_only"]);
        assert_eq!(names(&union, "lib"), vec!["a.rb", "b.rb", "shared"]);
        assert_eq!(names(&union, "lib/shared"), vec!["high.rb", "low.rb"]);
        assert_eq!(names(&union, "low_only"), vec!["x.rb"]);
        assert_eq!(names(&union, "high_only"), vec!["y.rb"]);
        assert_eq!(union.read_dir("nope").unwrap_err(), libc::ENOENT);
    }

    #[test]
    fn union_disjoint_members_are_a_plain_merge() {
        let low = member(&[("a/one.rb", b"1\n" as &[u8])]);
        let high = member(&[("b/two.rb", b"2\n")]);
        let union = UnionBackend::new(vec![low, high]).unwrap();
        assert_eq!(names(&union, ""), vec!["a", "b"]);
        assert_eq!(pread_all(&union, "a/one.rb"), b"1\n");
        assert_eq!(pread_all(&union, "b/two.rb"), b"2\n");
    }

    #[test]
    fn union_file_shadows_a_lower_directory_and_vice_versa() {
        // A file in the higher member over a directory in the lower:
        // the path is a file (stat), not listable (ENOTDIR).
        let low = member(&[("x/inside.rb", b"inside\n" as &[u8])]);
        let high = member(&[("x", b"i am a file\n")]);
        let union = UnionBackend::new(vec![low, high]).unwrap();
        assert_eq!(union.stat("x").unwrap().entry_type, EntryType::File);
        assert_eq!(union.read_dir("x").unwrap_err(), libc::ENOTDIR);

        // A directory in the higher member over a file in the lower:
        // the path is the merged directory; the shadowed file is gone.
        let low = member(&[("y", b"i am a file\n" as &[u8])]);
        let high = member(&[("y/inside.rb", b"inside\n")]);
        let union = UnionBackend::new(vec![low, high]).unwrap();
        assert_eq!(union.stat("y").unwrap().entry_type, EntryType::Directory);
        assert_eq!(names(&union, "y"), vec!["inside.rb"]);
    }

    #[test]
    fn union_members_stay_read_only() {
        // The transforms law: no write view on the composite (writes
        // exist only in the COW backend).
        let union = UnionBackend::new(vec![member(&[]), member(&[])]).unwrap();
        assert!(union.writable().is_none());
        assert!(union.members()[0].writable().is_none());
    }

    // ---------------------------------------------------------------
    // The context wiring (spec 17 §1): a union mount onto an occupied
    // point merges over the incumbent; the incumbent keeps its handle.
    // ---------------------------------------------------------------

    fn write_tar(dir: &std::path::Path, name: &str, files: &[(&str, &[u8])]) -> String {
        let path = dir.join(name);
        std::fs::write(&path, tar_of(files)).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn context_union_mount_merges_over_the_incumbent() {
        let _g = lock();
        let dir = tempfile::tempdir().unwrap();
        let env_image = write_tar(
            dir.path(),
            "env.tar",
            &[
                ("lib/ruby/rubygems.rb", b"# env\n" as &[u8]),
                ("lib/tebako/layout.yaml", b"layout\n"),
            ],
        );
        let app_image = write_tar(
            dir.path(),
            "app.tar",
            &[
                ("local/stub.rb", b"stub\n"),
                ("lib/tebako/layout.yaml", b"app shadows\n"),
            ],
        );

        // The env image mounts exclusively, then the app image unions
        // over it at the same point.
        let env_mount = mount::build_from_file(&env_image, "/__tfs__").unwrap();
        let env_handle = context().write().unwrap().mount_checked(env_mount).unwrap();
        let app_mount = mount::build_from_file(&app_image, "/__tfs__").unwrap();
        let union_handle = context().write().unwrap().mount_union(app_mount).unwrap();
        assert_eq!(union_handle, env_handle, "the incumbent keeps its handle");

        let mut ctx = context().write().unwrap();
        // Read-through both members; the app shadows the shared file.
        let st = ctx.stat("/__tfs__/lib/ruby/rubygems.rb").unwrap();
        assert_eq!(st.entry_type, EntryType::File);
        let st = ctx.stat("/__tfs__/local/stub.rb").unwrap();
        assert_eq!(st.entry_type, EntryType::File);
        let fd = ctx
            .open("/__tfs__/lib/tebako/layout.yaml", libc::O_RDONLY)
            .unwrap();
        let mut buf = [0u8; 64];
        let n = ctx.read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"app shadows\n");
        ctx.close(fd).unwrap();
        // readdir merges the two members' trees.
        let dir_id = ctx.opendir("/__tfs__/lib").unwrap();
        let mut seen = Vec::new();
        while ctx.readdir_abi(dir_id).unwrap() {
            let cur = ctx.dir_current(dir_id).unwrap();
            let len = cur
                .d_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(cur.d_name.len());
            seen.push(
                cur.d_name[..len]
                    .iter()
                    .map(|&c| c as u8 as char)
                    .collect::<String>(),
            );
        }
        ctx.closedir(dir_id).unwrap();
        seen.sort();
        assert_eq!(seen, vec!["ruby", "tebako"]);
        drop(ctx);

        // The union set is introspectable: two members at the point.
        let ctx = context().read().unwrap();
        let mount = ctx.mount_by_handle(env_handle).unwrap();
        assert_eq!(mount.backend.name().to_str().unwrap(), "UNION");
        assert_eq!(mount.mount_point, "/__tfs__");
        drop(ctx);

        context().write().unwrap().unmount();
    }

    #[test]
    fn context_union_mount_requires_an_occupied_point() {
        let _g = lock();
        let dir = tempfile::tempdir().unwrap();
        let image = write_tar(dir.path(), "a.tar", &[("x", b"x\n" as &[u8])]);
        let mount = mount::build_from_file(&image, "/free").unwrap();
        assert_eq!(
            context().write().unwrap().mount_union(mount).unwrap_err(),
            libc::ENODEV
        );
        let mount = mount::build_from_file(&image, "").unwrap();
        assert_eq!(
            context().write().unwrap().mount_union(mount).unwrap_err(),
            libc::EINVAL
        );
    }

    #[test]
    fn context_union_stacks_a_third_member_above() {
        let _g = lock();
        let dir = tempfile::tempdir().unwrap();
        let a = write_tar(dir.path(), "a.tar", &[("f", b"from-a\n" as &[u8])]);
        let b = write_tar(
            dir.path(),
            "b.tar",
            &[("f", b"from-b\n"), ("only-b", b"b\n")],
        );
        let c = write_tar(dir.path(), "c.tar", &[("f", b"from-c\n")]);

        let mut ctx = context().write().unwrap();
        ctx.mount_checked(mount::build_from_file(&a, "/p").unwrap())
            .unwrap();
        ctx.mount_union(mount::build_from_file(&b, "/p").unwrap())
            .unwrap();
        ctx.mount_union(mount::build_from_file(&c, "/p").unwrap())
            .unwrap();
        // The last arrival shadows everything below it; lower members
        // still serve what higher ones do not hold.
        let st = ctx.stat("/p/f").unwrap();
        assert_eq!(st.size, 7);
        let fd = ctx.open("/p/f", libc::O_RDONLY).unwrap();
        let mut buf = [0u8; 16];
        let n = ctx.read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"from-c\n");
        ctx.close(fd).unwrap();
        assert!(ctx.stat("/p/only-b").is_ok());
        drop(ctx);
        context().write().unwrap().unmount();
    }
}

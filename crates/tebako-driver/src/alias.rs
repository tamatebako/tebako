//! Windows Class L — the bare-name alias rule (spec 22 §2.1, phase W2).
//!
//! A loader call presenting a BARE library name (no path separator, no
//! drive qualifier — `ffi_lib 'user32'`, `Fiddle.dlopen 'foo'`) means
//! HOST by default, always: no VFS probe, no extension completion, no
//! search-order trickery. The ONE exception is declared: a bare name
//! matching a co-mounted image's `library_aliases:` entry EXACTLY
//! (verbatim, case-insensitive — the windows loader's own comparison;
//! `foo` does not match `foo.dll`) is payload-vendored (the grammar is
//! spec 03 §2.5; the declaration — not a heuristic — is what makes the
//! decision decidable at all). Rule L1 itself is unchanged: aliases are
//! a NAME-routing rule for path-less calls, applied before L1's check
//! runs.
//!
//! Two surfaces consume the alias union this module builds per boot:
//!
//! - **The covered surface** (the patched `dln.c` route — vendored-
//!   runtime code): the load-time check is `tebako_fs_dlalias2file` on
//!   the tfs context (the c_api export the patched `dln.c` calls); the
//!   driver feeds it by registering the union's (name → materialized
//!   host path) pairs at boot ([`register`]). A match rewrites the call
//!   to the alias target's absolute materialized path and loads it with
//!   the §2.1 binding (`LOAD_WITH_ALTERED_SEARCH_PATH`). The match rule
//!   (the bare-name byte grammar, then a verbatim case-insensitive
//!   compare) is locked and tested here on [`AliasUnion::resolve`] and
//!   re-implemented in `tfs::context` by construction — tfs stays
//!   tpkg-free and the c_api does its own grammar test per the patch's
//!   contract; the mirrored case matrices on both sides assert parity
//!   (invariant 10's assert-form — the one deliberate duplication,
//!   spec-pinned in #406).
//! - **The raw surface** (ffi's `LoadLibraryExA`, a C extension
//!   self-loading — windows has no interception surface for them, and
//!   patching third-party code is the per-gem work spec 22's law
//!   forbids): the driver materializes EVERY declared alias at boot
//!   ([`extract`]) through the exec-closure entry and PREPENDS the
//!   materialized directories to the process `PATH` ([`export_path`]),
//!   so the OS's own standard search order resolves a declared name for
//!   any caller in the process — interception-free, per-gem-code-free.
//!   EVERY co-mounted image contributes (the app payload included —
//!   unlike bin dirs — because any consumer in the process may present
//!   the name), the env image first, then the payload triples in order.
//!
//! The OS's precedence is stated honestly (spec 22 §2.1): the alias
//! guarantees AVAILABILITY on the search path, not precedence over the
//! OS's leading dirs — a declared name colliding with a System32 DLL
//! binds the host copy by OS rule, an aliasing mistake the declaration
//! surface cannot fix.
//!
//! Named errors (the spec 17 §2 slug precedent — never a silent
//! winner): one alias name declared by two co-mounted images is a named
//! boot error 65 (a duplicate WITHIN one image fails earlier, at
//! manifest parse — spec 03 §2.5); an alias whose `path` the image does
//! not hold, or that is not a regular file, is the manifest lying — a
//! named 65 (the Rule-R3 precedent), never a skipped entry; a host IO
//! failure materializing into the cache is a named 74.
//!
//! The `event=lib-load` record-mode journal (spec 23 §8's idiom — the
//! author learns the exact spelling to declare from the journal instead
//! of guessing) is owned by the PATCHED LOAD PATH, not by this module:
//! the verdict lines are emitted inside `tfs::context::dlalias2file`,
//! where the verdict is MADE (under the record policy only), and the
//! boot pass decides nothing — it materializes every declaration
//! unconditionally.

use std::path::{Path, PathBuf};

use tfs::context::context;

use crate::driver::{env_var, errno_text, join_mount, DriverError, Env};
use crate::handoff::ImageSpec;
use crate::{EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

fn manifest_err(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_MANIFEST, message.into())
}

fn io(message: impl Into<String>) -> DriverError {
    DriverError::new(EX_TEBAKO_IO, message.into())
}

/// One mounted image's contribution to the boot's alias union.
struct AliasEntry {
    /// The declared bare name (the manifest's own spelling — the match
    /// is case-insensitive, the spelling is preserved for errors).
    name: String,
    /// The alias target's VFS path (the declared in-image path joined
    /// under the declaring image's mount).
    vfs: String,
    /// Human-readable description of the declaring image, for errors.
    desc: String,
}

/// The verdict of a bare-name alias check (spec 22 §2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasVerdict<'u> {
    /// Payload-vendored: the name matches a declared alias — the value
    /// is the target's VFS path; materialize it through the exec-closure
    /// entry and load the host copy with the §2.1 binding (the covered
    /// surface's rewrite).
    Alias(&'u str),
    /// Host-by-default: no declaration matches (or the name is not
    /// bare — path surface is Rule L1's, never the alias rule's) — the
    /// name passes to the OS loader untouched.
    Host,
}

/// The boot's alias union across every co-mounted image (spec 22 §2.1):
/// insertion-ordered (the env image first, then the payload triples —
/// the PATH lead's order), keyed on the match rule's comparison
/// (verbatim, case-insensitive). Two images declaring one name is an
/// authoring ambiguity — a named 65, never a silent winner.
#[derive(Default)]
pub struct AliasUnion {
    entries: Vec<AliasEntry>,
}

impl AliasUnion {
    pub fn new() -> AliasUnion {
        AliasUnion::default()
    }

    /// Fold one mounted image's declared `library_aliases:` into the
    /// union. `desc` describes the image in errors ("env image '…'",
    /// "image '…'", "own slot N"); `mount` is the VFS point the
    /// declared in-image paths join under.
    pub fn add_image(
        &mut self,
        desc: &str,
        mount: &str,
        manifest: &tpkg::PayloadManifest,
    ) -> Result<(), DriverError> {
        for alias in &manifest.library_aliases {
            if let Some(prior) = self
                .entries
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(&alias.name))
            {
                return Err(manifest_err(format!(
                    "library alias '{}' is declared by both {} and {} — an authoring ambiguity, never a silent winner",
                    alias.name, prior.desc, desc
                )));
            }
            self.entries.push(AliasEntry {
                name: alias.name.clone(),
                vfs: join_mount(mount, &alias.path),
                desc: desc.to_string(),
            });
        }
        Ok(())
    }

    /// The bare-name decision (spec 22 §2.1): a verbatim,
    /// case-insensitive match against the union — never
    /// extension-completed (`foo` does not match `foo.dll`), never a
    /// probe. A name carrying a path separator or a drive qualifier is
    /// not bare: path surface is Rule L1's, so the answer is Host.
    ///
    /// The union-side reference of the match rule: the covered
    /// surface's load-time check is `tfs::context::dlalias2file` behind
    /// the `tebako_fs_dlalias2file` c_api export (tfs is tpkg-free, so
    /// the rule is re-implemented there — the one deliberate
    /// duplication, parity-asserted by the mirrored case matrices).
    /// The boot pass below materializes unconditionally and never
    /// resolves a presented name; the registration feeds the table the
    /// c_api answers from.
    pub fn resolve(&self, name: &str) -> AliasVerdict<'_> {
        if name.bytes().any(|b| b == b'/' || b == b'\\' || b == b':') {
            return AliasVerdict::Host;
        }
        match self
            .entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
        {
            Some(e) => AliasVerdict::Alias(&e.vfs),
            None => AliasVerdict::Host,
        }
    }
}

/// Fold one mounted image's declarations into the union. No manifest
/// declares nothing (plain images mount fine); a corrupt one is the
/// image lying about its self-description — the shared named 65
/// (`mounted_manifest_at`).
fn collect_image(union: &mut AliasUnion, desc: &str, mount: &str) -> Result<(), DriverError> {
    let Some(manifest) = crate::driver::mounted_manifest_at(mount)? else {
        return Ok(());
    };
    union.add_image(desc, mount, &manifest)
}

/// The boot pass's yield (spec 22 §2.1, phase W2): the covered
/// surface's registration table and the raw surface's PATH lead.
#[derive(Default)]
pub struct AliasBoot {
    /// (declared bare name, materialized absolute host path) per alias,
    /// in union order — the covered surface's decision table
    /// ([`register`]).
    pub pairs: Vec<(String, PathBuf)>,
    /// The materialized copies' parent directories, union order,
    /// deduped — the raw surface's PATH lead ([`export_path`]).
    pub dirs: Vec<String>,
}

/// The boot pass (spec 22 §2.1): collect the alias union of every
/// co-mounted image — the env image first, then each payload triple in
/// order — and materialize EVERY declared alias through the
/// exec-closure entry. Answers the [`AliasBoot`]: the per-alias (name,
/// host path) pairs for the covered surface's registration
/// ([`register`]) and the deduped materialized directories for the raw
/// surface's PATH lead ([`export_path`]). Called per boot after the
/// mounts, the jail, and the class-R pass, before the interpreter
/// handoff — in both boot shapes; the call sites are windows-gated (the
/// bare-name rule is a windows contract — POSIX boots never run this).
pub fn extract(
    images: &[ImageSpec],
    env: &dyn Env,
    runtime_root: &str,
) -> Result<AliasBoot, DriverError> {
    let mut union = AliasUnion::new();
    // The env image's own declarations come first (the class-R
    // precedent: the runtime's resources extract ahead of payloads).
    if let Some(image) = env_var(env, "TEBAKO_RUNTIME_IMAGE") {
        collect_image(&mut union, &format!("env image '{image}'"), runtime_root)?;
    }
    for spec in images {
        let desc = match &spec.source {
            crate::handoff::ImageSource::File(path, _) => {
                format!("image '{}'", path.display())
            }
            crate::handoff::ImageSource::OwnSlot(n) => format!("own slot {n}"),
        };
        collect_image(&mut union, &desc, &spec.mount)?;
    }
    let mut boot = AliasBoot::default();
    for entry in &union.entries {
        let host = extract_one(entry)?;
        let dir = parent_dir(&host)?;
        if !boot.dirs.contains(&dir) {
            boot.dirs.push(dir);
        }
        boot.pairs.push((entry.name.clone(), host));
    }
    Ok(boot)
}

/// Register the boot's alias table with the tfs context — the covered
/// surface's load-time decision input (spec 22 §2.1, phase W2): the
/// patched `dln.c`'s `tebako_fs_dlalias2file` answers from exactly
/// these (name → materialized host path) pairs. A direct Rust call on
/// the shared in-process context, never a c_api round-trip — the c_api
/// export exists for the runtime's patched C code. An empty union
/// registers an empty table (the context's default state made
/// explicit); the call sites are windows-gated like [`extract`]'s.
pub fn register(boot: &AliasBoot) {
    context()
        .write()
        .unwrap()
        .register_dlaliases(boot.pairs.clone());
}

/// Materialize one declared alias and answer its host path. The
/// in-image target is statted FIRST: declared-but-absent or not a
/// regular file is the manifest lying (a named 65, the Rule-R3
/// precedent), never a skipped entry.
fn extract_one(entry: &AliasEntry) -> Result<PathBuf, DriverError> {
    let stat = context().read().unwrap().stat(&entry.vfs).map_err(|e| {
        if e == libc::ENOENT {
            manifest_err(format!(
                "{} declares library alias '{}' but '{}' is absent from the image — the payload's self-description lies",
                entry.desc, entry.name, entry.vfs
            ))
        } else {
            io(format!(
                "cannot stat '{}' in the mounted image: {}",
                entry.vfs,
                errno_text(e)
            ))
        }
    })?;
    if stat.entry_type != tfs::EntryType::File {
        return Err(manifest_err(format!(
            "{} declares library alias '{}' but '{}' is not a regular file — the payload's self-description lies",
            entry.desc, entry.name, entry.vfs
        )));
    }
    // SEAM (PR #406 — the W2 PE-closure stream, crates/tfs
    // exec_closure): dlmap2file's closure walk parses Mach-O/ELF today;
    // the PE import directory joins as the third parsed format THERE.
    // This call site is unchanged when that lands — the alias's
    // sibling-import closure then materializes through the same entry
    // (one path authority, spec 22 §2.1) — and the windows
    // leave-in-place, content-keyed `dlls/<image-key>` lifecycle with
    // the Rule-R3 write-once/per-boot-rehash protocol (tamper = 70) is
    // the same stream's destination change. An alias whose closure is
    // trivial — the common case — is fully served today.
    let host = context()
        .write()
        .unwrap()
        .dlmap2file(&entry.vfs)
        .map_err(|e| {
            if e == libc::ENOENT {
                manifest_err(format!(
                    "{} declares library alias '{}' but '{}' is not held by the mounts — the payload's self-description lies",
                    entry.desc, entry.name, entry.vfs
                ))
            } else {
                io(format!(
                    "cannot materialize library alias '{}' ('{}'): {}",
                    entry.name,
                    entry.vfs,
                    errno_text(e)
                ))
            }
        })?;
    tebako_log::log!(
        tebako_log::Level::Debug,
        "driver",
        "library alias materialized name={} vfs={} at={}",
        entry.name,
        entry.vfs,
        host.to_string_lossy()
    );
    Ok(PathBuf::from(host.to_string_lossy().into_owned()))
}

/// The materialized copy's parent directory as a PATH component. The
/// split is on EITHER separator, not `Path::parent`: the only consumer
/// is a windows boot whose PATH is string-joined, and the derivation
/// must stay testable on hosts where `\` is not a separator.
fn parent_dir(host: &Path) -> Result<String, DriverError> {
    let s = host.to_string_lossy();
    let Some(cut) = s.rfind(['/', '\\']) else {
        return Err(io(format!(
            "the materialized alias '{}' has no parent directory",
            host.display()
        )));
    };
    Ok(s[..cut].to_string())
}

/// Prepend the materialized alias directories to the process `PATH`
/// (spec 22 §2.1's raw-surface wiring — the §3.2 lead's library form):
/// the OS's own standard search order then resolves a declared bare
/// name for any caller in the process. The dirs join the lead AFTER
/// the §3.2 bin dirs (the exec surface's locked lead order stays
/// byte-stable) and ahead of the inherited value — the call sites run
/// before `path_env::export`, which prepends in front. Nothing to
/// prepend leaves PATH untouched, never rewritten.
pub fn export_path(env: &dyn Env, dirs: &[String]) {
    if let Some(joined) = compose(env.var("PATH"), dirs) {
        env.set_var("PATH", &joined);
    }
}

/// The windows PATH value with `dirs` prepended ahead of the inherited
/// value (`None` when there is nothing to prepend). The separator is
/// the explicit `;` — the windows spelling — rather than
/// `std::env::join_paths`' host-shaped one: the only consumer is a
/// windows boot, and the explicit spelling keeps the windows semantics
/// testable on the legs that build this crate (the repo's windows leg
/// covers only the pure-Rust crates — tpkg among them — not the
/// driver). Empty inherited components are dropped (a trailing `;`
/// never seeds an empty search entry).
fn compose(existing: Option<String>, dirs: &[String]) -> Option<String> {
    if dirs.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = dirs.to_vec();
    if let Some(existing) = existing {
        parts.extend(
            existing
                .split(';')
                .filter(|c| !c.is_empty())
                .map(str::to_string),
        );
    }
    Some(parts.join(";"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct MapEnv(RefCell<HashMap<String, String>>);

    impl Env for MapEnv {
        fn var(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }
        fn set_var(&self, key: &str, value: &str) {
            self.0
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }
    }

    fn env_with(pairs: &[(&str, &str)]) -> MapEnv {
        MapEnv(RefCell::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ))
    }

    /// A data-kind manifest carrying the given `library_aliases:` YAML
    /// block verbatim (the union tests' vehicle — the parse path is the
    /// real one).
    fn manifest_with_aliases(aliases: &str) -> tpkg::PayloadManifest {
        let text = format!(
            "identity:\n  schema_version: 1\n  kind: data\n  name: x\n  version: \"1\"\n  \
             producer: {{tool: t, tool_version: \"1\"}}\n  created: \"2026-08-15T00:00:00Z\"\n  \
             digest: {{tree_hash: sha256:{z}, blob_sha256: {z}}}\n  \
             signing: {{state: unsigned}}\n  encryption: {{state: none}}\n\
             provides:\n  mount_semantics: {{suggested: /usr/share/x}}\n  capabilities: {{exec: false, read: true}}\n{aliases}",
            z = "0".repeat(64)
        );
        tpkg::PayloadManifest::from_yaml(&text).unwrap()
    }

    #[test]
    fn the_union_matches_verbatim_case_insensitively() {
        let mut union = AliasUnion::new();
        union
            .add_image(
                "image 'a.tfs'",
                "/__app__",
                &manifest_with_aliases(
                    "library_aliases:\n  - {name: libfoo-3.dll, path: /lib/libfoo-3.dll}\n",
                ),
            )
            .unwrap();
        // The windows loader's own comparison: case-insensitive…
        assert_eq!(
            union.resolve("LIBFOO-3.DLL"),
            AliasVerdict::Alias("/__app__/lib/libfoo-3.dll")
        );
        assert_eq!(
            union.resolve("LibFoo-3.Dll"),
            AliasVerdict::Alias("/__app__/lib/libfoo-3.dll")
        );
        // …but verbatim: never extension-completed, never a basename
        // probe.
        assert_eq!(union.resolve("libfoo-3"), AliasVerdict::Host);
        assert_eq!(union.resolve("libfoo-3.dll.dll"), AliasVerdict::Host);
        // An undeclared bare name is host-by-default — the rule's whole
        // point.
        assert_eq!(union.resolve("user32"), AliasVerdict::Host);
    }

    #[test]
    fn only_bare_names_reach_the_alias_rule() {
        let mut union = AliasUnion::new();
        union
            .add_image(
                "image 'a.tfs'",
                "/vendor",
                &manifest_with_aliases(
                    "library_aliases:\n  - {name: foo.dll, path: /lib/foo.dll}\n",
                ),
            )
            .unwrap();
        // A separator or a drive qualifier makes the name path surface
        // (Rule L1) — never an alias, even when the basename matches.
        assert_eq!(union.resolve("/vendor/lib/foo.dll"), AliasVerdict::Host);
        assert_eq!(union.resolve("lib\\foo.dll"), AliasVerdict::Host);
        assert_eq!(union.resolve("C:\\lib\\foo.dll"), AliasVerdict::Host);
    }

    #[test]
    fn one_name_declared_by_two_images_is_a_named_error() {
        // The spec 17 §2 slug precedent: an authoring ambiguity is a
        // named boot error 65, never a silent winner — on the match
        // rule's own (case-insensitive) comparison.
        let mut union = AliasUnion::new();
        union
            .add_image(
                "env image 'rt.tfs'",
                "/__tfs__",
                &manifest_with_aliases("library_aliases:\n  - {name: Foo.dll, path: /lib/a.dll}\n"),
            )
            .unwrap();
        let err = union
            .add_image(
                "image 'app.tfs'",
                "/__app__",
                &manifest_with_aliases("library_aliases:\n  - {name: foo.DLL, path: /lib/b.dll}\n"),
            )
            .unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{}", err.message);
        assert!(err.message.contains("foo.DLL"), "{}", err.message);
        assert!(
            err.message.contains("env image 'rt.tfs'"),
            "{}",
            err.message
        );
        assert!(err.message.contains("image 'app.tfs'"), "{}", err.message);
    }

    #[test]
    fn compose_prepends_with_the_windows_separator() {
        assert_eq!(
            compose(
                Some("C:\\Windows;C:\\bin".to_string()),
                &["D:\\dl\\a".to_string(), "D:\\dl\\b".to_string()],
            ),
            Some("D:\\dl\\a;D:\\dl\\b;C:\\Windows;C:\\bin".to_string())
        );
        // Prepend order is the union order — first declared leads.
        assert_eq!(
            compose(None, &["D:\\x".to_string(), "D:\\y".to_string()]),
            Some("D:\\x;D:\\y".to_string())
        );
        // Empty inherited components never seed empty search entries.
        assert_eq!(
            compose(Some("C:\\x;;C:\\y;".to_string()), &["D:\\a".to_string()],),
            Some("D:\\a;C:\\x;C:\\y".to_string())
        );
    }

    #[test]
    fn compose_without_dirs_leaves_path_untouched() {
        assert_eq!(compose(Some("C:\\Windows".to_string()), &[]), None);
        assert_eq!(compose(None, &[]), None);
    }

    #[test]
    fn export_path_leads_with_the_alias_dirs() {
        let env = env_with(&[("PATH", "C:\\Windows")]);
        export_path(&env, &["D:\\dl\\vendor".to_string()]);
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("D:\\dl\\vendor;C:\\Windows")
        );
    }

    #[test]
    fn export_path_without_inheritance_and_without_dirs() {
        let env = env_with(&[]);
        export_path(&env, &["D:\\dl\\vendor".to_string()]);
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("D:\\dl\\vendor")
        );
        // Nothing to prepend: PATH rides through untouched, never
        // rewritten.
        let env = env_with(&[("PATH", "C:\\Windows")]);
        export_path(&env, &[]);
        assert_eq!(
            env.0.borrow().get("PATH").map(String::as_str),
            Some("C:\\Windows")
        );
    }

    #[test]
    fn parent_dir_names_the_materialized_copy_home() {
        // Windows-shaped spellings split on either separator, on any
        // test host.
        assert_eq!(
            parent_dir(Path::new("D:\\cache\\dlls\\abc\\lib\\foo.dll")).unwrap(),
            "D:\\cache\\dlls\\abc\\lib"
        );
        assert_eq!(
            parent_dir(Path::new("D:/cache/dlls/abc/lib/foo.dll")).unwrap(),
            "D:/cache/dlls/abc/lib"
        );
        // A bare file name has no parent to put on PATH.
        let err = parent_dir(Path::new("foo.dll")).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_IO, "{}", err.message);
    }

    #[test]
    fn extract_without_mounted_images_extracts_nothing() {
        // No env image handed, no payload triples: the union is empty,
        // nothing materializes, no PATH dirs and no registration pairs
        // arise. (The mount-dependent paths — manifest reads and
        // dlmap2file — are the factory dogfood's surface, mirroring
        // materialize.rs's coverage shape.)
        let env = env_with(&[]);
        let boot = extract(&[], &env, "/__tfs__").unwrap();
        assert!(boot.pairs.is_empty(), "{:?}", boot.pairs);
        assert!(boot.dirs.is_empty(), "{:?}", boot.dirs);
    }

    #[test]
    fn register_installs_the_table_into_the_tfs_context() {
        // The covered surface's wiring: the boot's pairs land in the
        // process-global tfs context the `tebako_fs_dlalias2file` c_api
        // export reads. (The context is process state shared by this
        // whole test binary — restore the empty table afterwards so no
        // other test observes it.)
        let dir =
            std::env::temp_dir().join(format!("driver-alias-register-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let host = dir.join("libfoo-3.dll");
        std::fs::write(&host, b"pe").unwrap();
        let boot = AliasBoot {
            pairs: vec![("libfoo-3.dll".to_string(), host.clone())],
            dirs: Vec::new(),
        };
        register(&boot);
        let answered = context()
            .read()
            .unwrap()
            .dlalias2file("LIBFOO-3.DLL")
            .unwrap();
        assert_eq!(answered.to_str().unwrap(), host.to_string_lossy().as_ref());
        context().write().unwrap().register_dlaliases(Vec::new());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

//! The `tfs+git:` adapter: in-process git via gitoxide (spec 04 §3) —
//! never the git CLI, no shell-outs. A reference fetches as a bare clone
//! into a private temp dir; the payload blob is read straight out of the
//! pack by `#path`, no checkout, no extracted tree left behind.
//!
//! URL mapping (deterministic): the reference stores `host/path` without a
//! scheme; the adapter speaks `https://host/path`. Absolute paths and full
//! `file://` URLs are used verbatim so tests (and air-gapped sites) clone
//! local mirrors; plain `http://` is refused, mirroring tebako-http.

use std::path::{Path, PathBuf};

use crate::error::ResolveError;

/// Resolve the clone URL for a `tfs+git:` reference's `url` field.
fn transport_url(url: &str) -> Result<String, ResolveError> {
    if url.starts_with("http://") {
        return Err(ResolveError::Git {
            url: url.to_string(),
            reason: "refusing plain-http git transport (https:// and file:// are supported)"
                .to_string(),
        });
    }
    if url.starts_with("https://") || url.starts_with("file://") {
        return Ok(url.to_string());
    }
    // A Windows absolute path (`C:\…`, `\\server\…`) reads as scp-syntax
    // to gix's URL parser (host "C"): canonicalize to the file:/// form
    // with forward slashes. unix absolutes pass through as-is.
    #[cfg(windows)]
    if Path::new(url).is_absolute() {
        return Ok(format!("file:///{}", url.replace('\\', "/")));
    }
    // A leading `/` is a local path on every platform (unix absolute;
    // root-relative on Windows — Rust's is_absolute is false for the
    // latter, which is exactly why this check exists).
    if url.starts_with('/') || Path::new(url).is_absolute() {
        Ok(url.to_string())
    } else {
        Ok(format!("https://{url}"))
    }
}

/// Best-effort removal of the scratch clone directory.
struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new() -> Result<Self, ResolveError> {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("tebako-git-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| ResolveError::Git {
            url: String::new(),
            reason: format!("{e} creating {}", dir.display()),
        })?;
        Ok(ScratchDir(dir))
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Clone `url` (bare, in-process smart protocol) and read the blob at
/// `path` in `git_ref`'s tree (`None` = the remote's default branch).
pub fn fetch_blob(url: &str, git_ref: Option<&str>, path: &str) -> Result<Vec<u8>, ResolveError> {
    let transport = transport_url(url)?;
    let err = |reason: String| ResolveError::Git {
        url: transport.clone(),
        reason,
    };
    let scratch = ScratchDir::new()?;
    let parsed = gix::url::parse(gix::bstr::BStr::new(transport.as_str()))
        .map_err(|e| err(e.to_string()))?;
    let mut prepare =
        gix::prepare_clone_bare(parsed, scratch.0.join("repo")).map_err(|e| err(e.to_string()))?;
    let (repo, _outcome) = prepare
        .fetch_only(
            gix::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .map_err(|e| err(e.to_string()))?;
    // A clone keeps the default branch in refs/heads/* and every other
    // branch in refs/remotes/origin/*; resolve the spec with git's
    // standard rules first (full refs, tags, OIDs), then the
    // remote-tracking spelling for plain branch names.
    let spec = git_ref.unwrap_or("HEAD");
    let resolved = repo
        .rev_parse_single(spec)
        .or_else(|_| repo.rev_parse_single(format!("origin/{spec}").as_str()));
    let object = resolved
        .map_err(|e| err(format!("cannot resolve ref '{spec}': {e}")))?
        .object()
        .map_err(|e| err(e.to_string()))?;
    let tree = object
        .peel_to_kind(gix::object::Kind::Tree)
        .map_err(|e| err(format!("ref '{spec}' does not peel to a tree: {e}")))?
        .into_tree();
    let entry = tree
        .lookup_entry_by_path(path)
        .map_err(|e| err(e.to_string()))?
        .ok_or_else(|| err(format!("no file '{path}' at ref '{spec}'")))?;
    let blob = repo
        .find_blob(entry.object_id())
        .map_err(|e| err(format!("'{path}' at ref '{spec}' is not a file: {e}")))?;
    Ok(blob.data.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare fixture repo: `tool.tfs` on main and a second copy on a
    /// versioned branch — all through gix, no git binary involved.
    struct Fixture {
        dir: PathBuf,
    }
    impl Fixture {
        fn create(tag: &str) -> Fixture {
            let dir = std::env::temp_dir()
                .join(format!("tebako-resolve-git-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let repo = gix::init_bare(&dir).unwrap();
            drop(repo);
            // gix commit needs an identity; CI runners have no ambient
            // git config (user.name/user.email). Pin it in-repo (append —
            // init already wrote [core]) and re-open so the config is
            // re-read; never rely on the environment.
            use std::io::Write as _;
            std::fs::OpenOptions::new()
                .append(true)
                .open(dir.join("config"))
                .unwrap()
                .write_all(b"[user]\n\tname = tebako-test\n\temail = tebako-test@example.org\n")
                .unwrap();
            let repo = gix::open(&dir).unwrap();
            let empty = gix::hash::ObjectId::empty_tree(repo.object_hash());
            let tree = |bytes: &[u8]| {
                let blob = repo.write_blob(bytes).unwrap();
                let mut ed = repo.edit_tree(empty).unwrap();
                ed.upsert("images/tool.tfs", gix::object::tree::EntryKind::Blob, blob)
                    .unwrap();
                ed.write().unwrap()
            };
            let main_tree = tree(b"main-bytes");
            repo.commit(
                "refs/heads/main",
                "main",
                main_tree,
                gix::commit::NO_PARENT_IDS,
            )
            .unwrap();
            let v1_tree = tree(b"v1-bytes");
            repo.commit("refs/heads/v1", "v1", v1_tree, gix::commit::NO_PARENT_IDS)
                .unwrap();
            Fixture { dir }
        }
        fn url(&self) -> String {
            self.dir.to_string_lossy().into_owned()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn fetches_a_blob_by_ref_and_path() {
        let fx = Fixture::create("fetch");
        assert_eq!(
            fetch_blob(&fx.url(), Some("v1"), "images/tool.tfs").unwrap(),
            b"v1-bytes"
        );
        // omitted ref → the remote's default branch (HEAD)
        assert_eq!(
            fetch_blob(&fx.url(), None, "images/tool.tfs").unwrap(),
            b"main-bytes"
        );
    }

    #[test]
    fn missing_ref_and_path_are_named_errors() {
        let fx = Fixture::create("missing");
        let err = fetch_blob(&fx.url(), Some("nope"), "images/tool.tfs").unwrap_err();
        assert!(matches!(err, ResolveError::Git { .. }));
        assert!(err.to_string().contains("nope"));
        let err = fetch_blob(&fx.url(), Some("main"), "images/missing.tfs").unwrap_err();
        assert!(matches!(err, ResolveError::Git { .. }));
        assert!(err.to_string().contains("images/missing.tfs"));
        let err = fetch_blob(&fx.url(), None, "images").unwrap_err();
        assert!(matches!(err, ResolveError::Git { .. })); // a tree, not a file
    }

    #[test]
    fn url_mapping_is_deterministic() {
        assert_eq!(transport_url("h/r.git").unwrap(), "https://h/r.git");
        assert_eq!(transport_url("/mirror/r.git").unwrap(), "/mirror/r.git");
        assert_eq!(
            transport_url("file:///mirror/r.git").unwrap(),
            "file:///mirror/r.git"
        );
        assert!(transport_url("http://h/r.git").is_err());
    }
}

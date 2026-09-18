//! The fetch pipeline (spec 05 §6, locked 2026-09-18): a `FetchPlan` of
//! artifacts executed by a std::thread worker pool — concurrent artifact
//! streams, per-artifact progress through tebako-term v2, integrity
//! checked INLINE (streaming sha256 into the tmp file, one pass,
//! constant memory), the commit closure (signature verification, the
//! store's lock + rename + markers) running per artifact right after ITS
//! download completes, overlapped with the plan's remaining streams.
//!
//! The failure law: any artifact's transport/hash/commit failure cancels
//! the plan — in-flight streams abort at the next chunk, queued items
//! never start, the pool joins, every tmp file is dropped, and the FIRST
//! named error surfaces. A partial install never appears in the store
//! (tmp+rename makes the bytes invisible; the cancel path removes the
//! tmp files — test-proven, [`tests`]).
//!
//! The pipeline fetches ARTIFACTS. Release-index reads (shards,
//! monoliths, sums, registry YAML) stay small buffered GETs elsewhere.

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use sha2::Digest as _;
use tebako_http::FetchError;
use tebako_term::set::ProgressSet;

use crate::error::ResolveError;
use crate::reference::{Reference, Service};
use crate::transport::{Transport, DOWNLOAD_ATTEMPTS, RETRY_DELAY};

/// The default worker count (spec 05 §6 — docker's number).
pub const DEFAULT_FETCH_JOBS: usize = 3;
/// The `TEBAKO_FETCH_JOBS` env knob (over the config's `fetch_jobs`).
pub const FETCH_JOBS_ENV: &str = "TEBAKO_FETCH_JOBS";

/// Resolve the worker count: `TEBAKO_FETCH_JOBS` over the config's
/// `fetch_jobs` over [`DEFAULT_FETCH_JOBS`]. An unparseable or zero
/// value is a NAMED error (spec 05 §6), never a silent clamp.
pub fn resolve_fetch_jobs(
    env: Option<String>,
    config: Option<u32>,
) -> Result<usize, ResolveError> {
    let value = match (env, config) {
        (Some(v), _) => v,
        (None, Some(c)) => c.to_string(),
        (None, None) => return Ok(DEFAULT_FETCH_JOBS),
    };
    let trimmed = value.trim();
    match trimmed.parse::<usize>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err(ResolveError::InvalidFetchJobs {
            value: value.clone(),
        }),
    }
}

/// One artifact of a plan: what to fetch, the trust pin, where the tmp
/// lands, and the commit closure owning the install semantics
/// (signature verification + the store's lock/rename/markers).
pub struct FetchItem {
    /// The asset name, for progress lines and errors.
    pub display: String,
    /// What to fetch (https / file / service release / git blob).
    pub reference: Reference,
    /// The sha256 pin — verified INLINE at end-of-stream; a mismatch is
    /// the named [`ResolveError::Sha256Mismatch`] and cancels the plan.
    pub sha256_pin: Option<String>,
    /// The registry/index size hint (the plan header's total; the live
    /// bar rides the transport's content-length).
    pub size_hint: Option<u64>,
    /// The directory the in-flight `<display>.<pid>.<idx>.part` lands in
    /// (the entry's store tmp dir — rename stays on one filesystem).
    pub tmp_dir: PathBuf,
    /// Install the verified staged bytes; runs on the worker right after
    /// THIS artifact's download (overlapped with the other streams).
    /// Receives the tmp path + the computed sha256 + the concrete
    /// origin; owns signature verification and the atomic place, and
    /// CONSUMES the tmp file on success (rename into place — on its
    /// error the pipeline removes the tmp).
    pub commit: Box<dyn FnOnce(&StagedArtifact) -> Result<CommitReport, ResolveError> + Send>,
}

/// The commit closure's view of a staged artifact.
pub struct StagedArtifact<'a> {
    /// [`FetchItem::display`].
    pub display: &'a str,
    /// [`FetchItem::reference`].
    pub reference: &'a Reference,
    /// The staged tmp file (verified against the pin already).
    pub tmp: &'a Path,
    /// Lowercase sha256 of the staged bytes (the streaming hash).
    pub sha256: &'a str,
    /// The concrete origin (download URL / file path / git coordinates)
    /// — the `.origin` marker's content.
    pub origin: &'a str,
    /// The staged byte count.
    pub size: u64,
}

/// What a commit closure reports back.
pub struct CommitReport {
    /// The plain-mode done line (None → `installed <asset> (<size>)`).
    pub line: Option<String>,
}

/// A fetch plan: the subject line (the progress header) plus the
/// artifacts, in plan order (slot indexes are this order).
pub struct FetchPlan {
    /// The header's subject — e.g. `runtime ruby 3.3.12`.
    pub title: String,
    pub items: Vec<FetchItem>,
}

impl FetchPlan {
    pub fn new(title: impl Into<String>, items: Vec<FetchItem>) -> FetchPlan {
        FetchPlan {
            title: title.into(),
            items,
        }
    }

    /// The header's total: the sum of the known size hints (None when
    /// no item knows its size).
    pub fn total_hint(&self) -> Option<u64> {
        let mut sum = 0u64;
        let mut any = false;
        for item in &self.items {
            if let Some(hint) = item.size_hint {
                sum = sum.saturating_add(hint);
                any = true;
            }
        }
        any.then_some(sum)
    }
}

/// A writer that hashes what flows through it (the inline integrity
/// pass): the response body streams to the tmp file AND the sha256 in
/// one pass, constant memory.
struct HashWriter {
    file: std::fs::File,
    hasher: sha2::Sha256,
}

impl std::io::Write for HashWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// A per-item failure: a transport answer (retryable per the schedule)
/// or a terminal named error.
enum ItemFail {
    Transport(FetchError),
    Named(ResolveError),
}

/// The tmp path of item `idx` (unique per plan execution — a retry
/// truncates the same path; concurrent plans of one process ride the
/// sequence, foreign processes the pid).
fn tmp_path(item: &FetchItem, idx: usize, plan_seq: usize) -> PathBuf {
    let safe: String = item
        .display
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    item.tmp_dir.join(format!(
        "{safe}.{}.{plan_seq}.{idx}.part",
        std::process::id()
    ))
}

/// Per-process plan counter for tmp-name uniqueness.
static PLAN_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Resolve a service reference to its asset descriptor (the same
/// selection rule as [`crate::fetch::Fetcher::fetch_service`], spec 04
/// §1) so the artifact BYTES stream; the index reads stay buffered.
fn select_service_asset<T: Transport>(
    transport: &T,
    service: Service,
    owner: &str,
    repo: &str,
    version: &str,
    artifact: Option<&str>,
) -> Result<crate::adapters::Asset, ResolveError> {
    let adapter = crate::adapters::adapter_for(service);
    match artifact {
        Some(name) => adapter
            .asset_named(transport, owner, repo, version, name)?
            .ok_or_else(|| ResolveError::AssetNotFound {
                service,
                owner: owner.to_string(),
                repo: repo.to_string(),
                version: version.to_string(),
                artifact: Some(name.to_string()),
            }),
        None => {
            let assets = adapter.assets(transport, owner, repo, version)?;
            crate::adapters::select_candidate(service, owner, repo, version, assets)
        }
    }
}

/// One stream attempt: the reference's bytes flow into the tmp file
/// through the hashing writer, ticking progress per chunk and aborting
/// on plan cancel. Returns the sha256, the byte count, and the concrete
/// origin.
fn stream_once<T: Transport, W: std::io::Write + Send>(
    transport: &T,
    item: &FetchItem,
    tmp: &Path,
    idx: usize,
    progress: Option<&ProgressSet<W>>,
    cancel: &AtomicBool,
) -> Result<(String, u64, String), ItemFail> {
    let mut out = HashWriter {
        file: std::fs::File::create(tmp).map_err(|e| {
            ItemFail::Named(ResolveError::CacheIo {
                op: "creating",
                path: tmp.to_path_buf(),
                reason: e.to_string(),
            })
        })?,
        hasher: sha2::Sha256::new(),
    };
    let mut tick = |so_far: u64, total: Option<u64>| {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        if let Some(p) = progress {
            p.download_tick(idx, so_far, total);
        }
        true
    };
    let written = match &item.reference {
        Reference::Https { url, .. } => {
            let r = transport.stream(url, &mut out, Some(&mut tick));
            (r, url.clone())
        }
        Reference::File { path, .. } => {
            let url = tebako_http::file_url(Path::new(path));
            let r = transport.stream(&url, &mut out, Some(&mut tick));
            (r, url)
        }
        Reference::Service {
            service,
            owner,
            repo,
            version,
            artifact,
            ..
        } => {
            let asset = select_service_asset(
                transport,
                *service,
                owner,
                repo,
                version,
                artifact.as_deref(),
            )
            .map_err(ItemFail::Named)?;
            let r = transport.stream_asset(
                &asset.url,
                asset.accept.as_deref(),
                asset.authenticate,
                &mut out,
                Some(&mut tick),
            );
            (r, asset.url)
        }
        Reference::Git {
            url,
            git_ref,
            path,
            ..
        } => {
            // The git adapter yields blobs in memory (small payloads);
            // honor the pipeline's shape by writing them as one chunk.
            let Some(path) = path else {
                return Err(ItemFail::Named(ResolveError::GitPathRequired { url: url.clone() }));
            };
            #[cfg(feature = "git")]
            {
                let bytes = crate::git::fetch_blob(url, git_ref.as_deref(), path)
                    .map_err(ItemFail::Named)?;
                let len = bytes.len() as u64;
                let origin = item.reference.to_string();
                (
                    out.write_all(&bytes)
                        .map(|_| len)
                        .map_err(|e| FetchError::DownloadFailed(format!("{e} writing {url}"))),
                    origin,
                )
            }
            #[cfg(not(feature = "git"))]
            {
                let _ = (git_ref, path);
                return Err(ItemFail::Named(ResolveError::GitAdapterDisabled { url: url.clone() }));
            }
        }
    };
    let (result, origin) = written;
    let size = result.map_err(ItemFail::Transport)?;
    let sha256 = crate::fetch::hex_digest(&out.hasher.finalize());
    Ok((sha256, size, origin))
}

/// The per-item pipeline: stream (with the per-worker retry/throttle
/// discipline) → inline pin check → commit. Errors clean the tmp.
#[allow(clippy::too_many_arguments)]
fn run_item<T: Transport, W: std::io::Write + Send>(
    transport: &T,
    item: FetchItem,
    idx: usize,
    progress: Option<&ProgressSet<W>>,
    cancel: &AtomicBool,
    offline: bool,
    plan_seq: usize,
) -> Result<(), ResolveError> {
    if let Some(p) = progress {
        p.download_begin(idx, &item.display);
    }
    let tmp = tmp_path(&item, idx, plan_seq);
    if let Some(dir) = tmp.parent() {
        std::fs::create_dir_all(dir).map_err(|e| ResolveError::CacheIo {
            op: "creating",
            path: dir.to_path_buf(),
            reason: e.to_string(),
        })?;
    }
    if offline {
        return Err(ResolveError::Offline {
            what: item.display.clone(),
        });
    }
    let mut attempts = 0;
    let mut throttles = 0;
    let (sha256, size, origin) = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&tmp);
            return Ok(());
        }
        match stream_once(transport, &item, &tmp, idx, progress, cancel) {
            Ok(done) => break done,
            Err(ItemFail::Named(e)) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(e);
            }
            Err(ItemFail::Transport(FetchError::Cancelled(_))) => {
                let _ = std::fs::remove_file(&tmp);
                return Ok(());
            }
            Err(ItemFail::Transport(FetchError::IndexUnavailable(msg))) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(ResolveError::NotFound { origin: msg });
            }
            Err(ItemFail::Transport(FetchError::Throttled {
                retry_after,
                status,
                ..
            })) => {
                throttles += 1;
                if throttles >= tebako_http::THROTTLE_ROUNDS {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(ResolveError::DownloadFailed {
                        origin: item.display.clone(),
                        reason: format!(
                            "still throttled after {} backoff rounds ({status})",
                            tebako_http::THROTTLE_ROUNDS
                        ),
                    });
                }
                std::thread::sleep(tebako_http::throttle_backoff(throttles, retry_after));
            }
            Err(ItemFail::Transport(FetchError::DownloadFailed(msg))) => {
                attempts += 1;
                if attempts >= DOWNLOAD_ATTEMPTS {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(ResolveError::DownloadFailed {
                        origin: item.reference.to_string(),
                        reason: format!(
                            "failed to download after {DOWNLOAD_ATTEMPTS} attempts: {msg}"
                        ),
                    });
                }
                std::thread::sleep(RETRY_DELAY);
            }
            Err(
                ItemFail::Transport(
                    e @ (FetchError::ProxyAuthRequired(_) | FetchError::NetworkingCompiledOut(_)),
                ),
            ) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(ResolveError::DownloadFailed {
                    origin: item.reference.to_string(),
                    reason: e.to_string(),
                });
            }
            #[cfg(feature = "network")]
            Err(ItemFail::Transport(e @ FetchError::NetConfig(_))) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(ResolveError::DownloadFailed {
                    origin: item.reference.to_string(),
                    reason: e.to_string(),
                });
            }
        }
        // A mid-stream failure retries FROM ZERO (spec 05 §6): the tmp
        // truncates on the next attempt's create, the hasher is fresh.
        if let Some(p) = progress {
            p.download_begin(idx, &item.display);
        }
    };

    // The inline pin check (spec 05 §6): a mismatch deletes the
    // download, caches nothing, cancels the plan.
    if let Some(expected) = &item.sha256_pin {
        let expected = expected.to_ascii_lowercase();
        if sha256 != expected {
            let _ = std::fs::remove_file(&tmp);
            return Err(ResolveError::Sha256Mismatch {
                origin,
                expected,
                actual: sha256,
            });
        }
    }

    if let Some(p) = progress {
        p.verifying(idx);
    }
    let staged = StagedArtifact {
        display: &item.display,
        reference: &item.reference,
        tmp: &tmp,
        sha256: &sha256,
        origin: &origin,
        size,
    };
    match (item.commit)(&staged) {
        Ok(report) => {
            if let Some(p) = progress {
                p.installed(idx, size, report.line.as_deref());
            }
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Execute a plan over the worker pool (spec 05 §6): `jobs` concurrent
/// artifact streams, first failure cancels, results identical to a
/// sequential run. `jobs` clamps to the item count; `0`/garbage was
/// refused at [`resolve_fetch_jobs`].
pub fn execute_plan<T, W>(
    transport: &T,
    plan: FetchPlan,
    jobs: usize,
    progress: Option<&ProgressSet<W>>,
) -> Result<(), ResolveError>
where
    T: Transport + Sync,
    W: std::io::Write + Send,
{
    let n = plan.items.len();
    if n == 0 {
        return Ok(());
    }
    if let Some(p) = progress {
        p.begin(&plan.title, n, plan.total_hint());
    }
    let offline = crate::cache::offline();
    let queue: Mutex<VecDeque<(usize, FetchItem)>> =
        Mutex::new(plan.items.into_iter().enumerate().collect());
    let cancel = AtomicBool::new(false);
    let failure: Mutex<Option<ResolveError>> = Mutex::new(None);
    let workers = jobs.clamp(1, n);
    let plan_seq = PLAN_SEQ.fetch_add(1, Ordering::Relaxed);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let queue = &queue;
            let cancel = &cancel;
            let failure = &failure;
            scope.spawn(move || loop {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                let next = queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
                let Some((idx, item)) = next else { return };
                match run_item(transport, item, idx, progress, cancel, offline, plan_seq) {
                    Ok(()) => {
                        if cancel.load(Ordering::Relaxed) {
                            // This item aborted mid-stream because the
                            // plan is already failing.
                            if let Some(p) = progress {
                                p.failed(idx);
                            }
                        }
                    }
                    Err(e) => {
                        cancel.store(true, Ordering::Relaxed);
                        let mut slot = failure.lock().unwrap_or_else(|e| e.into_inner());
                        if slot.is_none() {
                            *slot = Some(e);
                        }
                        drop(slot);
                        if let Some(p) = progress {
                            p.failed(idx);
                        }
                    }
                }
            });
        }
    });

    let failed = failure.lock().unwrap_or_else(|e| e.into_inner()).take();
    match failed {
        Some(e) => {
            if let Some(p) = progress {
                p.abort();
            }
            Err(e)
        }
        None => {
            if let Some(p) = progress {
                p.finish();
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn scratch(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tebako-resolve-plan-{}-{}-{}",
            tag,
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_item(dir: &Path, name: &str, bytes: &[u8], pinned: bool) -> (FetchItem, PathBuf) {
        let src = dir.join(format!("src-{name}"));
        std::fs::write(&src, bytes).unwrap();
        let dest = dir.join(format!("out-{name}"));
        let dest2 = dest.clone();
        let origin_note = dir.join(format!("origin-{name}"));
        (
            FetchItem {
                display: name.to_string(),
                reference: Reference::File {
                    path: src.to_string_lossy().into_owned(),
                    sha256: None,
                },
                sha256_pin: pinned.then(|| crate::fetch::sha256_hex(bytes)),
                size_hint: Some(bytes.len() as u64),
                tmp_dir: dir.join("tmp"),
                commit: Box::new(move |staged| {
                    std::fs::rename(staged.tmp, &dest2).unwrap();
                    std::fs::write(&origin_note, staged.origin).unwrap();
                    Ok(CommitReport { line: None })
                }),
            },
            dest,
        )
    }

    fn run_plan(
        dir: &Path,
        items: Vec<FetchItem>,
        jobs: usize,
    ) -> Result<(), ResolveError> {
        let t = crate::transport::HttpTransport;
        let sink = ProgressSet::new(Vec::new(), false);
        execute_plan(&t, FetchPlan::new("test plan", items), jobs, Some(&sink))
    }

    /// The engine reads the process-wide TEBAKO_OFFLINE gate; every plan
    /// test serializes on the crate-wide env mutex so the offline test
    /// never races a sibling's fetch.
    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn streaming_hash_matches_the_post_hoc_hash() {
        let _guard = env_guard();
        let dir = scratch("hash");
        // property shape over a spread of sizes incl. > one 64 KiB chunk
        for (i, size) in [0usize, 1, 65_536, 150_001, 1_048_576].iter().enumerate() {
            let bytes: Vec<u8> = (0..*size as u32).map(|j| (j % 251) as u8).collect();
            let (item, dest) = file_item(&dir, &format!("f{i}"), &bytes, true);
            run_plan(&dir, vec![item], 1).unwrap();
            assert_eq!(std::fs::read(&dest).unwrap(), bytes, "size {size}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parallel_matches_sequential_and_overlaps() {
        let _guard = env_guard();
        let dir = scratch("par");
        let bytes: Vec<Vec<u8>> = (0..4)
            .map(|i| vec![i as u8; 200_000 + i * 13])
            .collect();
        let mk = |dir: &Path, tag: &str| {
            bytes
                .iter()
                .enumerate()
                .map(|(i, b)| file_item(dir, &format!("{tag}{i}"), b, true))
                .collect::<Vec<_>>()
        };
        // sequential
        let (items, dests_seq): (Vec<_>, Vec<_>) = mk(&dir, "s").into_iter().unzip();
        run_plan(&dir, items, 1).unwrap();
        // parallel
        let (items, dests_par): (Vec<_>, Vec<_>) = mk(&dir, "p").into_iter().unzip();
        run_plan(&dir, items, 4).unwrap();
        for ((b, s), p) in bytes.iter().zip(dests_seq).zip(dests_par) {
            assert_eq!(std::fs::read(s).unwrap(), *b);
            assert_eq!(std::fs::read(p).unwrap(), *b);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A transport whose streams stall until a sibling is in flight —
    /// the overlap proof: with jobs ≥ 2 both must be in flight together
    /// or the rendezvous deadlocks (bounded by a timeout).
    struct RendezvousTransport {
        bytes: HashMap<String, Vec<u8>>,
        in_flight: std::sync::atomic::AtomicUsize,
        high_water: std::sync::atomic::AtomicUsize,
    }

    impl Transport for RendezvousTransport {
        fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            self.bytes
                .get(url)
                .cloned()
                .ok_or_else(|| FetchError::IndexUnavailable(url.to_string()))
        }

        fn stream(
            &self,
            url: &str,
            writer: &mut dyn std::io::Write,
            on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
        ) -> Result<u64, FetchError> {
            let bytes = self
                .bytes
                .get(url)
                .cloned()
                .ok_or_else(|| FetchError::IndexUnavailable(url.to_string()))?;
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.high_water.fetch_max(now, Ordering::SeqCst);
            // rendezvous: wait (bounded) for a second stream to arrive
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while self.in_flight.load(Ordering::SeqCst) < 2
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            writer
                .write_all(&bytes)
                .map_err(|e| FetchError::DownloadFailed(e.to_string()))?;
            if let Some(cb) = on_progress {
                if !cb(bytes.len() as u64, Some(bytes.len() as u64)) {
                    self.in_flight.fetch_sub(1, Ordering::SeqCst);
                    return Err(FetchError::Cancelled(url.to_string()));
                }
            }
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(bytes.len() as u64)
        }
    }

    fn https_item(dir: &Path, name: &str, url: &str, pin: Option<String>) -> FetchItem {
        let dest = dir.join(format!("out-{name}"));
        FetchItem {
            display: name.to_string(),
            reference: Reference::Https {
                url: url.to_string(),
                sha256: None,
            },
            sha256_pin: pin,
            size_hint: None,
            tmp_dir: dir.join("tmp"),
            commit: Box::new(move |staged| {
                std::fs::rename(staged.tmp, &dest).unwrap();
                Ok(CommitReport { line: None })
            }),
        }
    }

    #[test]
    fn two_workers_really_run_concurrently() {
        let _guard = env_guard();
        let dir = scratch("rendezvous");
        let t = RendezvousTransport {
            bytes: [
                ("https://cdn/a".to_string(), vec![b'a'; 1000]),
                ("https://cdn/b".to_string(), vec![b'b'; 2000]),
            ]
            .into_iter()
            .collect(),
            in_flight: std::sync::atomic::AtomicUsize::new(0),
            high_water: std::sync::atomic::AtomicUsize::new(0),
        };
        let items = vec![
            https_item(&dir, "a", "https://cdn/a", Some(crate::fetch::sha256_hex(&vec![b'a'; 1000]))),
            https_item(&dir, "b", "https://cdn/b", Some(crate::fetch::sha256_hex(&vec![b'b'; 2000]))),
        ];
        let sink = ProgressSet::new(Vec::new(), false);
        execute_plan(&t, FetchPlan::new("rendezvous", items), 2, Some(&sink)).unwrap();
        assert!(t.high_water.load(Ordering::SeqCst) >= 2, "no overlap");
        assert_eq!(std::fs::read(dir.join("out-a")).unwrap(), vec![b'a'; 1000]);
        assert_eq!(std::fs::read(dir.join("out-b")).unwrap(), vec![b'b'; 2000]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pin_mismatch_cancels_the_plan_and_cleans_tmps() {
        let _guard = env_guard();
        let dir = scratch("cancel");
        let t = RendezvousTransport {
            bytes: [
                ("https://cdn/good".to_string(), vec![b'g'; 500]),
                ("https://cdn/bad".to_string(), vec![b'x'; 500]),
            ]
            .into_iter()
            .collect(),
            in_flight: std::sync::atomic::AtomicUsize::new(0),
            high_water: std::sync::atomic::AtomicUsize::new(0),
        };
        let items = vec![
            https_item(&dir, "good", "https://cdn/good", Some(crate::fetch::sha256_hex(&vec![b'g'; 500]))),
            // the pin lies — the streamed hash mismatches
            https_item(&dir, "bad", "https://cdn/bad", Some("0".repeat(64))),
        ];
        let sink = ProgressSet::new(Vec::new(), false);
        let err = execute_plan(&t, FetchPlan::new("cancel", items), 2, Some(&sink)).unwrap_err();
        assert!(matches!(err, ResolveError::Sha256Mismatch { .. }), "{err:?}");
        // every tmp of the plan is gone
        let tmp_dir = dir.join("tmp");
        let leftovers = std::fs::read_dir(&tmp_dir).map(|d| d.count()).unwrap_or(0);
        assert_eq!(leftovers, 0, "tmp files left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_artifact_is_the_named_not_found() {
        let _guard = env_guard();
        let dir = scratch("nf");
        let t = RendezvousTransport {
            bytes: HashMap::new(),
            in_flight: std::sync::atomic::AtomicUsize::new(0),
            high_water: std::sync::atomic::AtomicUsize::new(0),
        };
        let items = vec![https_item(&dir, "gone", "https://cdn/gone", None)];
        let sink = ProgressSet::new(Vec::new(), false);
        let err = execute_plan(&t, FetchPlan::new("nf", items), 1, Some(&sink)).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound { .. }), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_commit_failure_cancels_and_drops_the_tmp() {
        let _guard = env_guard();
        let dir = scratch("commit");
        let bytes = b"payload".to_vec();
        let (mut item, _dest) = file_item(&dir, "c", &bytes, true);
        item.commit = Box::new(|_staged| {
            Err(ResolveError::DownloadFailed {
                origin: "test".to_string(),
                reason: "commit boom".to_string(),
            })
        });
        let err = run_plan(&dir, vec![item], 1).unwrap_err();
        assert!(err.to_string().contains("commit boom"), "{err}");
        let leftovers = std::fs::read_dir(dir.join("tmp")).map(|d| d.count()).unwrap_or(0);
        assert_eq!(leftovers, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fetch_jobs_resolution() {
        assert_eq!(resolve_fetch_jobs(None, None).unwrap(), 3);
        assert_eq!(resolve_fetch_jobs(Some("5".into()), None).unwrap(), 5);
        // env over config
        assert_eq!(resolve_fetch_jobs(Some("2".into()), Some(8)).unwrap(), 2);
        assert_eq!(resolve_fetch_jobs(None, Some(1)).unwrap(), 1);
        assert!(matches!(
            resolve_fetch_jobs(Some("lots".into()), None),
            Err(ResolveError::InvalidFetchJobs { .. })
        ));
        assert!(matches!(
            resolve_fetch_jobs(Some("0".into()), None),
            Err(ResolveError::InvalidFetchJobs { .. })
        ));
    }

    #[test]
    fn offline_items_are_the_named_error() {
        let _guard = env_guard();
        std::env::set_var("TEBAKO_OFFLINE", "1");
        let dir = scratch("offline");
        let (item, _dest) = file_item(&dir, "o", b"bytes", true);
        let err = run_plan(&dir, vec![item], 1).unwrap_err();
        std::env::remove_var("TEBAKO_OFFLINE");
        assert!(matches!(err, ResolveError::Offline { .. }), "{err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

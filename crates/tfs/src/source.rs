//! The byte-source seam (spec 39 §2): positioned byte reads below the
//! format backend. The mount-source kinds of spec 11 §5 (host file,
//! memory, file region, VFS region) all serve one `&[u8]` core; the
//! lazy mount adds a FIFTH kind — remote-range — whose bytes are NOT
//! resident. This trait is that kind's abstraction: transport-shaped,
//! not URL-shaped (the OCI closure arm of spec 39 §8 rides it
//! unchanged), and read-only forever (the transforms law: COW/ENC
//! stack ABOVE the format backend, never inside a source).
//!
//! The seam itself is unconditional; the caching REMOTE source lives
//! in `source_remote.rs` behind the `backend-remote` feature (the
//! compiled-out rule, spec 39 §2: a lazy mount on a build without the
//! feature is the named ENOTSUP).

/// What went wrong at the byte layer. The errno channel is coarse
/// (spec 39 §6: EIO on the touching read); the KIND keeps the spec's
/// exit-class information for consumers that hold the concrete error
/// (the driver's mount-open mapping, spec 39 §6's 65/69/70/74 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceErrorKind {
    /// A range fetch failed (transport, throttle exhaustion, offline
    /// miss) — the 69 class.
    Fetch,
    /// A digest mismatch: the group against its sidecar digest (2nd
    /// fetch) or the whole body against the pin — the 70 class.
    Integrity,
    /// The local block cache failed (scan, write, rename, read) —
    /// the 74 class.
    Io,
    /// The source cannot serve this at all (a compiled-out feature,
    /// a server without range support where fallback is refused).
    Unsupported,
}

/// A byte-layer failure: the kind (exit-class information) plus the
/// named detail the log carries (URL, group index, transport cause —
/// spec 39 §6's naming law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError {
    /// The failure class.
    pub kind: SourceErrorKind,
    /// The named detail (log-bound; the errno channel carries the code).
    pub detail: String,
}

impl SourceError {
    /// A fetch failure (69 class).
    pub fn fetch(detail: impl Into<String>) -> SourceError {
        SourceError {
            kind: SourceErrorKind::Fetch,
            detail: detail.into(),
        }
    }

    /// An integrity failure (70 class).
    pub fn integrity(detail: impl Into<String>) -> SourceError {
        SourceError {
            kind: SourceErrorKind::Integrity,
            detail: detail.into(),
        }
    }

    /// A local cache IO failure (74 class).
    pub fn io(detail: impl Into<String>) -> SourceError {
        SourceError {
            kind: SourceErrorKind::Io,
            detail: detail.into(),
        }
    }

    /// An unsupported-source refusal.
    pub fn unsupported(detail: impl Into<String>) -> SourceError {
        SourceError {
            kind: SourceErrorKind::Unsupported,
            detail: detail.into(),
        }
    }

    /// The errno this surfaces as on the VFS channel (spec 39 §6: EIO
    /// on the touching read; the named detail rides the log).
    pub fn errno(&self) -> i32 {
        match self.kind {
            SourceErrorKind::Unsupported => libc::ENOTSUP,
            SourceErrorKind::Fetch | SourceErrorKind::Integrity | SourceErrorKind::Io => libc::EIO,
        }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.kind {
            SourceErrorKind::Fetch => "LazyUnavailable",
            SourceErrorKind::Integrity => "Sha256Mismatch",
            SourceErrorKind::Io => "LazyStoreIo",
            SourceErrorKind::Unsupported => "LazyUnsupported",
        };
        write!(f, "{name}: {}", self.detail)
    }
}

impl std::error::Error for SourceError {}

/// A positioned byte source under a format backend (spec 39 §2).
///
/// `pread` semantics, exactly as limnifs-core's `PositionedReader`
/// spells them: `read_at(offset, len)` returns UP TO `len` bytes; a
/// short return means the object ended (EOF), an empty return means
/// `offset` is at or past the end. Implementations MUST be
/// deterministic (the same range always yields the same bytes — the
/// digest verification law depends on it) and MUST NOT return more
/// than `len` bytes.
pub trait ByteSource: Send + Sync {
    /// Read up to `len` bytes at `offset` (short only at EOF).
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, SourceError>;

    /// The object's total byte length, when the source knows it (the
    /// caching remote source always does — the descriptor's
    /// `size_bytes`). Used for the mount-open trailing-garbage check
    /// where a plain EOF probe is cheaper than a fetch.
    fn len(&self) -> Option<u64> {
        None
    }

    /// `len` present ⇔ not empty, mirroring `Option`'s convenience.
    fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }
}

/// Read EXACTLY `len` bytes at `offset` (the mount-open prefix walk's
/// primitive): loops over the source's short reads, and a shortfall
/// before `len` is a truncated object — the named fetch failure,
/// never a zero-fill, never a silent short serve (spec 39 §6).
pub fn read_exact_at(
    source: &dyn ByteSource,
    offset: u64,
    len: usize,
) -> Result<Vec<u8>, SourceError> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let chunk = source.read_at(offset + out.len() as u64, len - out.len())?;
        if chunk.is_empty() {
            return Err(SourceError::fetch(format!(
                "the byte source ended at {} of {len} bytes requested at offset {offset} (a truncated image)",
                offset + out.len() as u64,
            )));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

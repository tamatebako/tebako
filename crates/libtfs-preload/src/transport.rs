//! The preload's lazy transport (spec 39 §3/§8): `file://` reads from
//! disk (the test/airgap spelling), everything else rides tebako-http's
//! one agent (https + the loopback carve-out — the transport's
//! enforcement, one layer down). A non-200 document GET is a named
//! failure, never a guess.
//!
//! The interposition shim rebuilds the runtime's mounts in the
//! interpreter's own engine instance (and in every exec'd child), so it
//! must speak the same transport the driver used: a LAZY_SEEDING env
//! image's groups are fetched on demand here exactly as they are in the
//! driver (tebako#848).

#![forbid(unsafe_code)]

/// The driver-shaped transport over one source URL.
pub struct HttpTransport {
    url: String,
}

impl HttpTransport {
    /// The transport for an image served at `url`.
    pub fn new(url: impl Into<String>) -> HttpTransport {
        HttpTransport { url: url.into() }
    }
}

impl tfs::lazy_entry::LazyTransport for HttpTransport {
    fn range(
        &self,
        offset: u64,
        len: usize,
        if_range: Option<&str>,
    ) -> Result<tfs::source_remote::RangeFetchAnswer, String> {
        let range = tebako_http::ByteRange {
            offset,
            len: len as u64,
        };
        // tebako#851: the fetch's own machinery (the resolver's settings
        // reads) must never re-enter the engine while the VFS read lock
        // is held — the guard makes every interposed call pass through.
        let _g = crate::route::TransportGuard::enter();
        match tebako_http::get_range(&self.url, range, if_range, None) {
            Ok(tebako_http::RangeAnswer::Partial(body)) => {
                Ok(tfs::source_remote::RangeFetchAnswer::Partial {
                    bytes: body.bytes,
                    etag: body.etag,
                })
            }
            Ok(tebako_http::RangeAnswer::Full(body)) => {
                Ok(tfs::source_remote::RangeFetchAnswer::Full {
                    bytes: body.bytes,
                    etag: body.etag,
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn document(&self, url: &str) -> Result<Vec<u8>, String> {
        if let Some(path) = url.strip_prefix("file://") {
            return std::fs::read(path).map_err(|e| format!("{url}: {e}"));
        }
        let _g = crate::route::TransportGuard::enter();
        let response = tebako_http::get_raw(url, Some("application/json"), None)
            .map_err(|e| format!("{url}: {e}"))?;
        if response.status != 200 {
            return Err(format!(
                "{url}: status {} — the blksum sidecar must be a 200",
                response.status
            ));
        }
        Ok(response.body)
    }
}

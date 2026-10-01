//! The spec 39 §8 contract test: a limnifs env image served as an OCI
//! blob by a loopback distribution fixture (the spec 38 §8 carve-out —
//! in-process CI tooling, never shipped), its blksum sidecar resolved
//! through the sibling digest-tag (spec 38 §3), and the image mounted
//! through the caching remote byte source over tebako-oci's
//! range-capable blob fetch — the closure arm, Bearer dance and
//! confinement included. Reads are byte-compared against the same image
//! file-mounted (the parity property, the OCI-transport twin of
//! lazy_remote.rs), and the fixture counts every image-blob body byte
//! it serves: the image-size-vs-fetched-bytes ratio IS the proof that
//! a lazy env image streams 4 MiB groups from an OCI blob instead of
//! downloading whole.
#![cfg(feature = "backend-remote")]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use tebako_oci::{
    Annotations, ArtifactClass, Client, CredentialSource, Descriptor, Manifest, RepoRef, Selector,
    ShapeExpectation,
};
use tfs::backend::Backend;
use tfs::backends_limnifs::LimnifsBackend;
use tfs::source_remote::{RangeFetchAnswer, RemoteByteSource};
use tpkg::lazy::Blksum;

const REPO: &str = "tebako/runtimes";
const IMAGE_TAG: &str = "test";

/// The fixture's blob-serving mode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Honor `Range` (206), whole body (200) without it.
    Ranges,
    /// Answer every blob GET with the whole body (the 200-fallback leg).
    WholeOnly,
}

/// What the fixture needs to serve one runtime pair over the
/// distribution API: the manifests by tag (body + own digest), the
/// image blob, and the sidecar blob.
struct Served {
    manifests: HashMap<String, (Vec<u8>, String)>,
    image: Vec<u8>,
    image_blob_path: String,
    sidecar: Vec<u8>,
    sidecar_blob_path: String,
}

/// A distribution-shaped loopback fixture: one request per connection,
/// the image blob's body bytes and requests counted, an optional
/// Bearer-challenge mode (the spec 38 §6 dance against `/token`).
struct DistFixture {
    port: u16,
    stop: Arc<AtomicBool>,
    blob_served: Arc<AtomicU64>,
    blob_requests: Arc<AtomicUsize>,
    token_requests: Arc<AtomicUsize>,
    saw_bearer: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl DistFixture {
    /// Stage one runtime pair and serve it. `serve_sidecar: false`
    /// publishes the image without its blksum sibling (the
    /// blksum-missing leg).
    fn serve(image: Vec<u8>, mode: Mode, auth: bool, serve_sidecar: bool) -> (DistFixture, Served) {
        let image_hex = tpkg::lazy::sha256_hex(&image);
        let blksum = Blksum::from_image_bytes(&image);
        let sidecar = blksum.render().into_bytes();
        let sidecar_hex = tpkg::lazy::sha256_hex(&sidecar);
        let image_layer = Descriptor {
            media_type: ArtifactClass::Payload.layer_media_type().to_string(),
            digest: format!("sha256:{image_hex}"),
            size: image.len() as u64,
        };
        let image_manifest = Manifest::render(
            ArtifactClass::Payload,
            &image_layer,
            &Annotations {
                title: Some("image.tfs".to_string()),
                ..Annotations::default()
            },
        )
        .expect("the image manifest renders");
        let image_manifest_hex = tpkg::lazy::sha256_hex(&image_manifest);
        let mut manifests = HashMap::new();
        manifests.insert(
            IMAGE_TAG.to_string(),
            (image_manifest, image_manifest_hex.clone()),
        );
        manifests.insert(
            image_manifest_hex.clone(),
            (
                Manifest::render(
                    ArtifactClass::Payload,
                    &image_layer,
                    &Annotations {
                        title: Some("image.tfs".to_string()),
                        ..Annotations::default()
                    },
                )
                .expect("the image manifest renders"),
                image_manifest_hex,
            ),
        );
        if serve_sidecar {
            let sidecar_layer = Descriptor {
                media_type: ArtifactClass::Blksum.layer_media_type().to_string(),
                digest: format!("sha256:{sidecar_hex}"),
                size: sidecar.len() as u64,
            };
            let tag = tebako_oci::blksum_tag(&image_hex);
            let sidecar_manifest = Manifest::render(
                ArtifactClass::Blksum,
                &sidecar_layer,
                &Annotations {
                    title: Some(tag.clone()),
                    blksum_subject: Some(format!("sha256:{image_hex}")),
                    ..Annotations::default()
                },
            )
            .expect("the sidecar manifest renders");
            let sidecar_manifest_hex = tpkg::lazy::sha256_hex(&sidecar_manifest);
            manifests.insert(tag, (sidecar_manifest, sidecar_manifest_hex));
        }
        let served = Served {
            manifests,
            image_blob_path: format!("/v2/{REPO}/blobs/sha256:{image_hex}"),
            sidecar_blob_path: format!("/v2/{REPO}/blobs/sha256:{sidecar_hex}"),
            image,
            sidecar,
        };

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let stop = Arc::new(AtomicBool::new(false));
        let blob_served = Arc::new(AtomicU64::new(0));
        let blob_requests = Arc::new(AtomicUsize::new(0));
        let token_requests = Arc::new(AtomicUsize::new(0));
        let saw_bearer = Arc::new(AtomicBool::new(false));
        let handle = {
            let stop = Arc::clone(&stop);
            let blob_served = Arc::clone(&blob_served);
            let blob_requests = Arc::clone(&blob_requests);
            let token_requests = Arc::clone(&token_requests);
            let saw_bearer = Arc::clone(&saw_bearer);
            let manifests = served.manifests.clone();
            let image = served.image.clone();
            let image_blob_path = served.image_blob_path.clone();
            let sidecar = served.sidecar.clone();
            let sidecar_blob_path = served.sidecar_blob_path.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            // macOS/BSD accepted sockets inherit O_NONBLOCK
                            // from the nonblocking listener (Linux does
                            // not): without this the request reads race
                            // WouldBlock.
                            let _ = stream.set_nonblocking(false);
                            Self::answer(AnswerCtx {
                                stream,
                                port,
                                auth,
                                mode,
                                manifests: &manifests,
                                image: &image,
                                image_blob_path: &image_blob_path,
                                sidecar: &sidecar,
                                sidecar_blob_path: &sidecar_blob_path,
                                blob_served: &blob_served,
                                blob_requests: &blob_requests,
                                token_requests: &token_requests,
                                saw_bearer: &saw_bearer,
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        (
            DistFixture {
                port,
                stop,
                blob_served,
                blob_requests,
                token_requests,
                saw_bearer,
                thread: Some(handle),
            },
            served,
        )
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn blob_served(&self) -> u64 {
        self.blob_served.load(Ordering::Relaxed)
    }

    fn blob_requests(&self) -> usize {
        self.blob_requests.load(Ordering::Relaxed)
    }

    fn token_requests(&self) -> usize {
        self.token_requests.load(Ordering::Relaxed)
    }

    fn saw_bearer(&self) -> bool {
        self.saw_bearer.load(Ordering::Relaxed)
    }

    fn answer(ctx: AnswerCtx<'_>) {
        let AnswerCtx {
            mut stream,
            port,
            auth,
            mode,
            manifests,
            image,
            image_blob_path,
            sidecar,
            sidecar_blob_path,
            blob_served,
            blob_requests,
            token_requests,
            saw_bearer,
        } = ctx;
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => return,
        });
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return;
        }
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_string();
        let mut range: Option<(u64, u64)> = None; // inclusive end, as the wire spells it
        let mut authorization: Option<String> = None;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some((name, value)) = trimmed.split_once(':') {
                let value = value.trim();
                if name.eq_ignore_ascii_case("range") {
                    if let Some(span) = value.strip_prefix("bytes=") {
                        if let Some((start, end)) = span.split_once('-') {
                            if let (Ok(start), Ok(end)) =
                                (start.trim().parse::<u64>(), end.trim().parse::<u64>())
                            {
                                range = Some((start, end));
                            }
                        }
                    }
                } else if name.eq_ignore_ascii_case("authorization") {
                    authorization = Some(value.to_string());
                }
            }
        }
        let (status, headers, body): (u16, String, Vec<u8>) = if path.starts_with("/token") {
            token_requests.fetch_add(1, Ordering::Relaxed);
            (200, String::new(), br#"{"token":"fixture-token"}"#.to_vec())
        } else if auth && authorization.as_deref() != Some("Bearer fixture-token") {
            (
                401,
                format!(
                    "WWW-Authenticate: Bearer realm=\"http://127.0.0.1:{port}/token\",service=\"fixture\"\r\n"
                ),
                br#"{"errors":[{"code":"UNAUTHORIZED"}]}"#.to_vec(),
            )
        } else if let Some(tag) = path.strip_prefix(&format!("/v2/{REPO}/manifests/")) {
            match manifests.get(tag) {
                Some((body, hex)) => (
                    200,
                    format!("Docker-Content-Digest: sha256:{hex}\r\n"),
                    body.clone(),
                ),
                None => (
                    404,
                    String::new(),
                    br#"{"errors":[{"code":"MANIFEST_UNKNOWN"}]}"#.to_vec(),
                ),
            }
        } else if path == *image_blob_path {
            blob_requests.fetch_add(1, Ordering::Relaxed);
            if authorization.as_deref() == Some("Bearer fixture-token") {
                saw_bearer.store(true, Ordering::Relaxed);
            }
            let total = image.len() as u64;
            let (status, headers, body) = match (mode, range) {
                (Mode::Ranges, Some((start, end))) => {
                    let end = end.min(total.saturating_sub(1));
                    let body = image[start as usize..=(end as usize)].to_vec();
                    (
                        206,
                        format!("Content-Range: bytes {start}-{end}/{total}\r\nETag: \"v1\"\r\n"),
                        body,
                    )
                }
                _ => (200, "ETag: \"v1\"\r\n".to_string(), image.to_vec()),
            };
            blob_served.fetch_add(body.len() as u64, Ordering::Relaxed);
            (status, headers, body)
        } else if path == *sidecar_blob_path {
            (200, String::new(), sidecar.to_vec())
        } else {
            (
                404,
                String::new(),
                br#"{"errors":[{"code":"NAME_UNKNOWN"}]}"#.to_vec(),
            )
        };
        let response = format!(
            "HTTP/1.1 {status} X\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        if stream.write_all(response.as_bytes()).is_err() {
            return;
        }
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    }
}

struct AnswerCtx<'a> {
    stream: TcpStream,
    port: u16,
    auth: bool,
    mode: Mode,
    manifests: &'a HashMap<String, (Vec<u8>, String)>,
    image: &'a [u8],
    image_blob_path: &'a str,
    sidecar: &'a [u8],
    sidecar_blob_path: &'a str,
    blob_served: &'a Arc<AtomicU64>,
    blob_requests: &'a Arc<AtomicUsize>,
    token_requests: &'a Arc<AtomicUsize>,
    saw_bearer: &'a Arc<AtomicBool>,
}

impl Drop for DistFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// [`tebako_oci::Http`] over tebako-http's raw distribution surfaces —
/// the test-local twin of tebako-resolve's OciTransport (the decided
/// credential header attaches verbatim).
struct FixtureHttp;

impl tebako_oci::Http for FixtureHttp {
    fn get(
        &self,
        url: &str,
        accept: Option<&str>,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, tebako_http::FetchError> {
        let header = tebako_oci::authorization_header(auth);
        tebako_http::get_raw(url, accept, header.as_ref().map(|(k, v)| (*k, v.as_str())))
    }

    fn post(
        &self,
        _url: &str,
        _body: &[u8],
        _content_type: Option<&str>,
        _auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, tebako_http::FetchError> {
        unreachable!("the pull path never posts")
    }

    fn put(
        &self,
        _url: &str,
        _body: &[u8],
        _content_type: &str,
        _auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, tebako_http::FetchError> {
        unreachable!("the pull path never puts")
    }

    fn stream(
        &self,
        url: &str,
        auth: tebako_oci::Auth<'_>,
        writer: &mut dyn Write,
        on_progress: Option<&mut dyn FnMut(u64, Option<u64>) -> bool>,
    ) -> Result<u64, tebako_http::FetchError> {
        let header = tebako_oci::authorization_header(auth);
        tebako_http::stream_raw(
            url,
            None,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
            writer,
            on_progress,
        )
    }

    fn get_range(
        &self,
        url: &str,
        range: tebako_http::ByteRange,
        if_range: Option<&str>,
        auth: tebako_oci::Auth<'_>,
    ) -> Result<tebako_http::RawResponse, tebako_http::FetchError> {
        let header = tebako_oci::authorization_header(auth);
        tebako_http::get_range_raw(
            url,
            range,
            if_range,
            header.as_ref().map(|(k, v)| (*k, v.as_str())),
        )
    }
}

/// The anonymous credential ride (the fixture's dance mints anonymous
/// tokens; confinement is the client's, exercised in its own tier).
struct FixtureCreds;

impl CredentialSource for FixtureCreds {
    fn registry_credential(&self) -> Result<Option<tebako_oci::BasicCred>, tebako_oci::OciError> {
        Ok(None)
    }

    fn realm_credential(
        &self,
        _realm_host: &str,
    ) -> Result<Option<tebako_oci::BasicCred>, tebako_oci::OciError> {
        Ok(None)
    }
}

/// Resolve the image manifest and return the one layer's descriptor.
fn resolve_image_layer(host: &str) -> Descriptor {
    let http = FixtureHttp;
    let creds = FixtureCreds;
    let client = Client::new(&http, &creds);
    let r = RepoRef {
        host,
        repo: REPO,
        selector: Selector::Tag(IMAGE_TAG),
    };
    client
        .resolve(&r, ShapeExpectation::Class(ArtifactClass::Payload))
        .expect("the image manifest resolves")
        .layer
}

/// The mount-open's sidecar fetch (spec 39 §8): the sibling digest-tag
/// resolves, the one layer fetches digest-verified, the document
/// parses and cross-checks against the image it names.
fn fetch_sidecar(host: &str, image_hex: &str) -> Blksum {
    let http = FixtureHttp;
    let creds = FixtureCreds;
    let client = Client::new(&http, &creds);
    let tag = tebako_oci::blksum_tag(image_hex);
    let r = RepoRef {
        host,
        repo: REPO,
        selector: Selector::Tag(&tag),
    };
    let artifact = client
        .resolve(&r, ShapeExpectation::Class(ArtifactClass::Blksum))
        .expect("the sidecar manifest resolves");
    let bytes = client
        .fetch_blob(&r, &artifact.layer)
        .expect("the sidecar blob fetches");
    let blksum = Blksum::parse(std::str::from_utf8(&bytes).expect("the sidecar is UTF-8"))
        .expect("the sidecar parses");
    assert_eq!(
        blksum.sha256, image_hex,
        "the sidecar pins the image its sibling tag names"
    );
    blksum
}

/// The spec 39 §8 closure, OCI-shaped: the byte source's fetch closure
/// backed by the OCI client's range-capable blob fetch (the caller-side
/// adapter — what the driver's mount-open arm will do).
fn oci_closure(host: String, layer: Descriptor) -> Arc<dyn tfs::source_remote::RangeFetch> {
    Arc::new(move |offset: u64, len: usize, if_range: Option<&str>| {
        let http = FixtureHttp;
        let creds = FixtureCreds;
        let client = Client::new(&http, &creds);
        let r = RepoRef {
            host: &host,
            repo: REPO,
            selector: Selector::Tag(IMAGE_TAG),
        };
        match client.fetch_blob_range(&r, &layer, offset, len as u64, if_range) {
            Ok(tebako_http::RangeAnswer::Partial(body)) => Ok(RangeFetchAnswer::Partial {
                bytes: body.bytes,
                etag: body.etag,
            }),
            Ok(tebako_http::RangeAnswer::Full(body)) => Ok(RangeFetchAnswer::Full {
                bytes: body.bytes,
                etag: body.etag,
            }),
            Err(e) => Err(e.to_string()),
        }
    })
}

/// The fixture tree: inline files plus a big slab-backed payload so
/// the IMAGE spans several 4 MiB groups (the lazy_remote.rs shape).
fn fixture_image() -> Vec<u8> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("hello.txt"), b"hello, lazy limnifs\n").unwrap();
    std::fs::write(root.join("sub").join("nested.txt"), b"nested content here").unwrap();
    // ~24 MiB of INCOMPRESSIBLE bytes (seeded xorshift64*): the image
    // spans 7 4 MiB groups even after lz4, so a sparse touch provably
    // fetches a fraction of it.
    let mut big = vec![0u8; 24 * 1024 * 1024];
    let mut state = 0x9E3779B97F4A7C15u64;
    for chunk in big.chunks_mut(8) {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let bytes = state.wrapping_mul(0x2545F4914F6CDD1D).to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    std::fs::write(root.join("big.bin"), &big).unwrap();
    let mut config = limnifs_write::WriteConfig::default_v0_1();
    config.dictionaries.enabled = false;
    let artifact =
        limnifs_write::write_directory_with_config(root, &config).expect("write succeeds");
    assert!(
        artifact.metadata_sidecar.is_none(),
        "the fixture tree must keep its metadata inline"
    );
    let mut image = artifact.bytes;
    for slab in &artifact.slabs {
        image.extend_from_slice(&slab.bytes);
    }
    image
}

/// Mount the fixture image lazily over the OCI distribution fixture:
/// the manifest resolve, the sidecar sibling-tag fetch, then the byte
/// source over the OCI range closure.
fn lazy_oci_mount(
    fixture: &DistFixture,
    image_hex: &str,
    blocks: &std::path::Path,
) -> tfs::context::Mount {
    let host = fixture.host();
    let layer = resolve_image_layer(&host);
    let blksum = fetch_sidecar(&host, image_hex);
    let origin = format!("http://{host}/v2/{REPO}/blobs/sha256:{image_hex}");
    let source = RemoteByteSource::new(oci_closure(host, layer), blksum, blocks, &origin)
        .expect("the remote source opens");
    tfs::mount::build_from_source(Arc::new(source), "/lazy-oci").expect("the lazy mount opens")
}

#[test]
fn lazy_mount_over_oci_streams_groups_and_matches_the_golden() {
    let image = fixture_image();
    let image_hex = tpkg::lazy::sha256_hex(&image);
    let golden = LimnifsBackend::from_image(image.clone()).expect("the golden mount opens");
    let (fixture, _served) = DistFixture::serve(image.clone(), Mode::Ranges, false, true);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_oci_mount(&fixture, &image_hex, blocks.path());
    let served_after_open = fixture.blob_served();

    // The touched working set: two small files + one window of the big
    // file — a FRACTION of the image must cross the wire.
    let mut buf = [0u8; 64];
    let n = mount.backend.pread("hello.txt", &mut buf, 0).expect("read");
    assert_eq!(&buf[..n], b"hello, lazy limnifs\n");
    let n = mount
        .backend
        .pread("sub/nested.txt", &mut buf, 0)
        .expect("read");
    assert_eq!(&buf[..n], b"nested content here");

    // A 64 KiB window deep in the big file, byte-compared against the
    // golden (file-mounted) answer — the parity property.
    let mut lazy_win = vec![0u8; 65536];
    let mut golden_win = vec![0u8; 65536];
    let at = 5 * 1024 * 1024 + 12345;
    let n = mount
        .backend
        .pread("big.bin", &mut lazy_win, at)
        .expect("lazy window");
    let g = golden
        .pread("big.bin", &mut golden_win, at)
        .expect("golden window");
    assert_eq!(n, g);
    assert_eq!(lazy_win, golden_win);

    // THE PROOF: image size vs blob bytes actually fetched over OCI.
    let served = fixture.blob_served();
    eprintln!(
        "lazy OCI e2e: image {} bytes; blob-fetched on open {served_after_open} bytes; \
         after the touched working set {served} bytes in {} blob requests \
         ({:.1}% of the image)",
        image.len(),
        fixture.blob_requests(),
        served as f64 / image.len() as f64 * 100.0,
    );
    assert!(
        served < image.len() as u64 / 2,
        "a sparse touch must stream a fraction of the blob: {served} of {}",
        image.len()
    );
    // One blob request per touched 4 MiB group: group 0 at open, the
    // window's group on the touch — nothing else crossed the wire.
    assert_eq!(fixture.blob_requests(), 2, "one request per touched group");
    assert_eq!(fixture.token_requests(), 0, "an anonymous registry never dances");
}

#[test]
fn the_bearer_dance_rides_the_oci_closure() {
    let image = fixture_image();
    let image_hex = tpkg::lazy::sha256_hex(&image);
    let (fixture, _served) = DistFixture::serve(image.clone(), Mode::Ranges, true, true);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_oci_mount(&fixture, &image_hex, blocks.path());

    let mut buf = [0u8; 64];
    let n = mount.backend.pread("hello.txt", &mut buf, 0).expect("read");
    assert_eq!(&buf[..n], b"hello, lazy limnifs\n");
    // The dance happened ONCE (the challenge's token then rode every
    // request preemptively — the per-process token cache), and the blob
    // fetches presented it.
    assert_eq!(fixture.token_requests(), 1, "one mint, then the cached token");
    assert!(fixture.saw_bearer(), "the blob requests carried the Bearer token");
}

#[test]
fn the_200_fallback_seeds_eagerly_over_oci() {
    let image = fixture_image();
    let image_hex = tpkg::lazy::sha256_hex(&image);
    let (fixture, _served) = DistFixture::serve(image.clone(), Mode::WholeOnly, false, true);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_oci_mount(&fixture, &image_hex, blocks.path());

    let mut buf = [0u8; 32];
    let n = mount.backend.pread("hello.txt", &mut buf, 0).expect("read");
    assert_eq!(&buf[..n], b"hello, lazy limnifs\n");
    // The whole body crossed once (equal trust: the blksum's group
    // digests verified it) — the spec 39 §3 loud eager path.
    assert_eq!(fixture.blob_served(), image.len() as u64);
}

#[test]
fn a_missing_sidecar_is_the_blksum_missing_signal_not_an_error() {
    let image = fixture_image();
    let image_hex = tpkg::lazy::sha256_hex(&image);
    let (fixture, _served) = DistFixture::serve(image.clone(), Mode::Ranges, false, false);
    let http = FixtureHttp;
    let creds = FixtureCreds;
    let client = Client::new(&http, &creds);
    let host = fixture.host();
    let tag = tebako_oci::blksum_tag(&image_hex);
    let r = RepoRef {
        host: &host,
        repo: REPO,
        selector: Selector::Tag(&tag),
    };
    let err = client
        .resolve(&r, ShapeExpectation::Class(ArtifactClass::Blksum))
        .expect_err("an unpublished sibling tag names nothing");
    assert!(
        matches!(err, tebako_oci::OciError::ManifestNotFound { .. }),
        "the spec 39 §3 loud-fallback signal: {err:?}"
    );
}

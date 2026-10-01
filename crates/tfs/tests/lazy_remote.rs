//! The spec 39 §10 loopback e2e: a limnifs image served by a real
//! Range-capable HTTP fixture, mounted through the caching remote byte
//! source over tebako-http's `get_range` (the spec 38 §8 loopback
//! carve-out — in-process CI tooling, never shipped), read lazily, and
//! byte-compared against the same image file-mounted (the parity
//! property). The fixture counts every body byte it serves: the
//! image-size-vs-fetched-bytes ratio IS the feature's proof.
#![cfg(feature = "backend-remote")]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use tfs::backend::Backend;
use tfs::backends_limnifs::LimnifsBackend;
use tfs::source_remote::{RangeFetchAnswer, RemoteByteSource};
use tpkg::lazy::Blksum;

/// The fixture's serving mode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Honor `Range` (206), whole body (200) without it.
    Ranges,
    /// Answer every GET with the whole body (the 200-fallback leg).
    WholeOnly,
}

/// A Range-capable loopback HTTP fixture: one request per connection,
/// body bytes and requests counted.
struct Fixture {
    port: u16,
    stop: Arc<AtomicBool>,
    served_bytes: Arc<AtomicU64>,
    requests: Arc<AtomicUsize>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Fixture {
    fn serve(image: Vec<u8>, mode: Mode) -> Fixture {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("addr").port();
        let stop = Arc::new(AtomicBool::new(false));
        let served_bytes = Arc::new(AtomicU64::new(0));
        let requests = Arc::new(AtomicUsize::new(0));
        let handle = {
            let stop = Arc::clone(&stop);
            let served_bytes = Arc::clone(&served_bytes);
            let requests = Arc::clone(&requests);
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            requests.fetch_add(1, Ordering::Relaxed);
                            // macOS/BSD accepted sockets inherit O_NONBLOCK
                            // from the nonblocking listener (Linux does not):
                            // without this the request reads race WouldBlock.
                            let _ = stream.set_nonblocking(false);
                            Self::answer(stream, &image, mode, &served_bytes);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Fixture {
            port,
            stop,
            served_bytes,
            requests,
            thread: Some(handle),
        }
    }

    fn answer(mut stream: TcpStream, image: &[u8], mode: Mode, served_bytes: &Arc<AtomicU64>) {
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => return,
        });
        // Read the request line + headers (until the blank line).
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            return;
        }
        let mut range: Option<(u64, u64)> = None; // inclusive end, as the wire spells it
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
            if let Some(value) = trimmed
                .strip_prefix("Range:")
                .or_else(|| trimmed.strip_prefix("range:"))
            {
                // bytes=<start>-<end> (the transport's only spelling)
                if let Some(span) = value.trim().strip_prefix("bytes=") {
                    if let Some((start, end)) = span.split_once('-') {
                        if let (Ok(start), Ok(end)) =
                            (start.trim().parse::<u64>(), end.trim().parse::<u64>())
                        {
                            range = Some((start, end));
                        }
                    }
                }
            }
        }
        let total = image.len() as u64;
        let (status, headers, body): (u16, String, Vec<u8>) = match (mode, range) {
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
        served_bytes.fetch_add(body.len() as u64, Ordering::Relaxed);
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

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/image.tfs", self.port)
    }

    fn served(&self) -> u64 {
        self.served_bytes.load(Ordering::Relaxed)
    }

    fn requests(&self) -> usize {
        self.requests.load(Ordering::Relaxed)
    }

    fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Adapt the fixture's URL + tebako-http's `get_range` into the byte
/// source's fetch closure (the caller-side adapter of spec 39 §8's
/// closure shape — this is exactly what the driver will do).
fn fetch_closure(url: String) -> Arc<dyn tfs::source_remote::RangeFetch> {
    Arc::new(move |offset: u64, len: usize, if_range: Option<&str>| {
        let range = tebako_http::ByteRange {
            offset,
            len: len as u64,
        };
        match tebako_http::get_range(&url, range, if_range, None) {
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
/// the IMAGE spans several 4 MiB groups.
fn fixture_image() -> (tempfile::TempDir, Vec<u8>) {
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
    (tmp, image)
}

/// Mount the fixture image lazily over the fixture server.
fn lazy_mount(fixture: &Fixture, image: &[u8], blocks: &std::path::Path) -> tfs::context::Mount {
    let blksum = Blksum::from_image_bytes(image);
    let source = RemoteByteSource::new(fetch_closure(fixture.url()), blksum, blocks, fixture.url())
        .expect("the remote source opens");
    tfs::mount::build_from_source(Arc::new(source), "/lazy").expect("the lazy mount opens")
}

#[test]
fn lazy_mount_reads_a_fraction_of_the_image_and_matches_the_golden() {
    let (_tmp, image) = fixture_image();
    let golden = LimnifsBackend::from_image(image.clone()).expect("the golden mount opens");
    let fixture = Fixture::serve(image.clone(), Mode::Ranges);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_mount(&fixture, &image, blocks.path());
    let served_after_open = fixture.served();

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

    // THE PROOF: image size vs bytes actually fetched.
    let served = fixture.served();
    eprintln!(
        "lazy e2e: image {} bytes; fetched on open {served_after_open} bytes; \
         fetched after the touched working set {served} bytes in {} requests \
         ({:.1}% of the image)",
        image.len(),
        fixture.requests(),
        served as f64 / image.len() as f64 * 100.0,
    );
    assert!(
        served < image.len() as u64 / 2,
        "a sparse touch must fetch a fraction of the image: {served} of {}",
        image.len()
    );
    // One request per 4 MiB group: open fetched group 0, the window
    // touch fetched group 1 — nothing else crossed the wire.
    assert_eq!(fixture.requests(), 2, "one request per touched group");

    // The directory tree answers identically too (stat/read_dir over
    // the same metadata).
    let st = mount.backend.stat("big.bin").expect("stat");
    assert_eq!(st.size, 24 * 1024 * 1024);
    let mut root = mount.backend.read_dir("").expect("root lists");
    root.sort_by(|a, b| a.name.cmp(&b.name));
    let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["big.bin", "hello.txt", "sub"]);
}

#[test]
fn cached_groups_serve_with_the_server_down_a_miss_is_eio() {
    let (_tmp, image) = fixture_image();
    let fixture = Fixture::serve(image.clone(), Mode::Ranges);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_mount(&fixture, &image, blocks.path());

    // Touch one window of the big file, then KILL the server.
    let mut touched = vec![0u8; 8192];
    let n = mount
        .backend
        .pread("big.bin", &mut touched, 1024)
        .expect("touch");
    assert_eq!(n, 8192);
    let served_before = fixture.served();
    fixture.stop();
    std::thread::sleep(std::time::Duration::from_millis(50));

    // The touched set serves from the block cache — no network, no
    // new byte over the wire (spec 39 §6: TEBAKO_OFFLINE semantics at
    // group granularity, here proven by a dead server).
    let mut again = vec![0u8; 8192];
    let n = mount
        .backend
        .pread("big.bin", &mut again, 1024)
        .expect("the cache serves");
    assert_eq!(n, 8192);
    assert_eq!(again, touched);
    assert_eq!(
        fixture.served(),
        served_before,
        "no refetch of a cached group"
    );

    // An UNtouched region: the touching read is EIO — never a
    // fabricated zero-fill, never a silent short read (spec 39 §6).
    let mut miss = vec![0u8; 8192];
    let err = mount
        .backend
        .pread("big.bin", &mut miss, 8 * 1024 * 1024)
        .expect_err("a miss with the server down is EIO");
    assert_eq!(err, libc::EIO);
}

#[test]
fn the_200_fallback_seeds_eagerly_and_the_mount_outlives_the_server() {
    let (_tmp, image) = fixture_image();
    let fixture = Fixture::serve(image.clone(), Mode::WholeOnly);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let mount = lazy_mount(&fixture, &image, blocks.path());

    let mut buf = [0u8; 32];
    let n = mount.backend.pread("hello.txt", &mut buf, 0).expect("read");
    assert_eq!(&buf[..n], b"hello, lazy limnifs\n");
    // The whole body crossed once (equal trust: digest-verified).
    assert_eq!(fixture.served(), image.len() as u64);
    fixture.stop();
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Every group is seeded: the whole image now reads with the
    // server DOWN.
    let mut win = vec![0u8; 4096];
    for at in [0u64, 4 * 1024 * 1024, 8 * 1024 * 1024] {
        let n = mount
            .backend
            .pread("big.bin", &mut win, at)
            .expect("eager-seeded read");
        assert!(n > 0);
    }
}

#[test]
fn mount_open_against_a_dead_server_is_a_named_failure() {
    let (_tmp, image) = fixture_image();
    // Bind and immediately release a port: nothing serves there.
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let url = format!("http://127.0.0.1:{port}/image.tfs");
    let blksum = Blksum::from_image_bytes(&image);
    let blocks = tempfile::tempdir().expect("blocks dir");
    let source =
        RemoteByteSource::new(fetch_closure(url), blksum, blocks.path(), "dead").expect("open");
    // Mount-open needs the prefix: the fetch fails, the mount refuses
    // (EIO on the errno channel — never a partial mount, spec 39 §6).
    let err = tfs::mount::build_from_source(Arc::new(source), "/lazy")
        .map(|_| ())
        .expect_err("a dead server fails the mount-open");
    assert_eq!(err, libc::EIO);
}

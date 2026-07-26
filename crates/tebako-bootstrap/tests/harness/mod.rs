//! End-to-end self-test: a port of the C++ `test/self-test.sh` scenarios
//! against the Rust bootstrap, plus a direct parity run of the same
//! fixtures against the C++ tebako-bootstrap binary (the oracle).
//!
//! Layout mirrors the shell test: a fake release mirror holding a fake
//! runtime (a shell script printing its argv), manifest.json and
//! SHA256SUMS.txt; packages stitched with crates/tpkg exactly the way
//! tpkg-stitch does.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A temporary directory that cleans itself up (unique per instance).
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> TempDir {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tebako-rs-boot-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct Harness {
    pub tmp: TempDir,
    pub bootstrap: PathBuf,
    pub fake_runtime: PathBuf,
    pub mirror_root: PathBuf,
    pub asset: String,
    pub entry: String,
    pub runtime_ref: String,
    pub sha: String,
    pub image_asset: String,
    pub image_sha: String,
}

pub const TEBAKO_VER: &str = "9.9.9";
pub const RUBY_VER: &str = "3.3.7";

pub fn platform() -> &'static str {
    tebako_bootstrap::platform::platform_string()
}

pub fn rust_bootstrap() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target")
                .canonicalize()
                .unwrap()
        });
    for profile in ["debug", "release"] {
        let cand = target.join(profile).join("tebako-bootstrap");
        if cand.is_file() {
            return cand;
        }
    }
    panic!("tebako-bootstrap binary not built (run `cargo build -p tebako-bootstrap`)")
}

pub fn cpp_bootstrap() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEBAKO_CPP_BOOTSTRAP") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let well_known =
        PathBuf::from("/Users/mulgogi/src/tamatebako/tebako-bootstrap/build/tebako-bootstrap");
    if well_known.is_file() {
        return Some(well_known);
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let cand = PathBuf::from(dir).join("tebako-bootstrap");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

pub fn sha256_of(path: &Path) -> String {
    tebako_bootstrap::sha::sha256_file_hex(path).unwrap()
}

pub fn write_fake_runtime(path: &Path) {
    std::fs::write(
        path,
        "#!/bin/sh\necho FAKE-RUNTIME\necho \"TEBAKO_RUNTIME_IMAGE=$TEBAKO_RUNTIME_IMAGE\"\ni=0\nfor a in \"$@\"; do\n  echo \"argv[$i]=$a\"\n  i=$((i+1))\ndone\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

impl Harness {
    pub fn new(bootstrap: PathBuf) -> Harness {
        let tmp = TempDir::new("boot");
        let fake_runtime = tmp.0.join("fake-runtime");
        write_fake_runtime(&fake_runtime);

        let plat = platform();
        let exe = tebako_bootstrap::platform::exe_suffix();
        let asset = format!("tebako-runtime-{TEBAKO_VER}-{RUBY_VER}-{plat}{exe}");
        let entry = format!("ruby-{RUBY_VER}-{TEBAKO_VER}-{plat}");
        let runtime_ref = format!("ruby@{RUBY_VER};tebako={TEBAKO_VER}");

        // Fake release mirror: v<tebako>/<asset> + SHA256SUMS.txt + manifest.json
        let mirror_root = tmp.0.join("mirror");
        let mirror = mirror_root.join(format!("v{TEBAKO_VER}"));
        std::fs::create_dir_all(&mirror).unwrap();
        std::fs::copy(&fake_runtime, mirror.join(&asset)).unwrap();
        let sha = sha256_of(&mirror.join(&asset));

        // item 30b: the image-era sibling (<asset>.tfs) + its index entries
        let image_asset = asset.strip_suffix(exe).unwrap_or(&asset).to_string() + ".tfs";
        let fake_runtime_image = tmp.0.join("fake-runtime-image");
        std::fs::write(&fake_runtime_image, b"FAKE TFS RUNTIME IMAGE PAYLOAD").unwrap();
        std::fs::copy(&fake_runtime_image, mirror.join(&image_asset)).unwrap();
        let image_sha = sha256_of(&mirror.join(&image_asset));

        std::fs::write(
            mirror.join("SHA256SUMS.txt"),
            format!("{sha}  {asset}\n{image_sha}  {image_asset}\n"),
        )
        .unwrap();
        std::fs::write(
            mirror.join("manifest.json"),
            format!(
                "[\n  {{\n    \"tebako_version\": \"{TEBAKO_VER}\",\n    \"ruby_version\": \"{RUBY_VER}\",\n    \"platform\": \"{plat}\",\n    \"filename\": \"{asset}\",\n    \"sha256\": \"{sha}\",\n    \"size_bytes\": 12345,\n    \"image\": {{\"filename\": \"{image_asset}\", \"sha256\": \"{image_sha}\", \"size_bytes\": 6789}}\n  }}\n]\n"
            ),
        )
        .unwrap();

        Harness {
            tmp,
            bootstrap,
            fake_runtime,
            mirror_root,
            asset,
            entry,
            runtime_ref,
            sha,
            image_asset,
            image_sha,
        }
    }

    /// Stitch a package exactly like tpkg-stitch: base bytes + parts in
    /// order + trailer (slots in part order).
    pub fn stitch(
        &self,
        base: &Path,
        parts: &[(PathBuf, u32, &str)],
        runtime_ref: &str,
        launcher_abi: u32,
        out: &Path,
    ) {
        let mut m = tpkg::Manifest {
            package_flags: 0,
            launcher_abi,
            ..Default::default()
        };
        m.set_runtime_ref(runtime_ref.as_bytes());

        let mut pos = std::fs::metadata(base).unwrap().len();
        for (path, format_id, mount) in parts {
            let size = std::fs::metadata(path).unwrap().len();
            let mut slot = tpkg::Slot::new(pos, size, *format_id, mount);
            slot.flags = 0;
            m.slots.push(slot);
            pos += size;
        }

        let mut outf = std::fs::File::create(out).unwrap();
        let mut copy = |p: &Path| {
            let data = std::fs::read(p).unwrap();
            use std::io::Write as _;
            outf.write_all(&data).unwrap();
        };
        copy(base);
        for (p, _, _) in parts {
            copy(p);
        }
        tpkg::write_to(&mut outf, &m).unwrap();
        drop(outf);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    /// A fake image slot file.
    pub fn fake_image(&self) -> PathBuf {
        let p = self.tmp.0.join("app.tfs");
        std::fs::write(&p, b"FAKE TFS IMAGE PAYLOAD").unwrap();
        p
    }

    pub fn lean_pkg(&self, name: &str) -> PathBuf {
        let out = self.tmp.0.join(name);
        let img = self.fake_image();
        self.stitch(
            &self.bootstrap,
            &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__")],
            &self.runtime_ref,
            0,
            &out,
        );
        out
    }

    /// An image-era lean package: runtime_ref carries the `;image` flag
    /// (item 30b).
    pub fn lean_pkg_image(&self, name: &str) -> PathBuf {
        let out = self.tmp.0.join(name);
        let img = self.fake_image();
        self.stitch(
            &self.bootstrap,
            &[(img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__")],
            &format!("{};image", self.runtime_ref),
            0,
            &out,
        );
        out
    }

    /// The cached runtime image path for a home.
    pub fn cache_image(&self, home: &Path) -> PathBuf {
        home.join("runtimes")
            .join(&self.entry)
            .join(&self.image_asset)
    }

    pub fn fat_pkg(&self, name: &str, payload: &Path) -> PathBuf {
        let out = self.tmp.0.join(name);
        let img = self.fake_image();
        let ref_sha = format!("{};sha256={}", self.runtime_ref, sha256_of(payload));
        self.stitch(
            &self.bootstrap,
            &[
                (img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__"),
                (payload.to_path_buf(), tpkg::TPKG_FORMAT_RUNTIME, ""),
            ],
            &ref_sha,
            0,
            &out,
        );
        out
    }

    /// A fat package whose runtime_ref carries a DIFFERENT sha than the
    /// payload bytes (the C++ "tampered payload" fixture: ref has the
    /// original sha, payload is tampered → must be refused).
    pub fn fat_pkg_mismatched(&self, name: &str, payload: &Path) -> PathBuf {
        let out = self.tmp.0.join(name);
        let img = self.fake_image();
        let ref_sha = format!("{};sha256={}", self.runtime_ref, self.sha);
        self.stitch(
            &self.bootstrap,
            &[
                (img, tpkg::TPKG_FORMAT_DWARFS, "/__tebako_memfs__"),
                (payload.to_path_buf(), tpkg::TPKG_FORMAT_RUNTIME, ""),
            ],
            &ref_sha,
            0,
            &out,
        );
        out
    }

    pub fn run(
        &self,
        pkg: &Path,
        home: &Path,
        extra_env: &[(&str, &str)],
        args: &[&str],
    ) -> (i32, String, String) {
        let mut cmd = Command::new(pkg);
        cmd.args(args)
            .env("TEBAKO_HOME", home)
            .env("TEBAKO_RUNTIME_MIRROR", &self.mirror_root)
            // Deterministic env: no ambient knobs leaking in.
            .env_remove("TEBAKO_OFFLINE");
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        let out = {
            // Linux ETXTBSY race: a freshly stitched fixture binary can
            // transiently be reported busy when spawned while parallel
            // tests execute their own fixture binaries. Retry bounded.
            let mut attempt = 0;
            loop {
                match cmd.output() {
                    Ok(o) => break o,
                    Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && attempt < 20 => {
                        attempt += 1;
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(e) => panic!("run package {}: {e}", pkg.display()),
                }
            }
        };
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    pub fn home(&self, name: &str) -> PathBuf {
        self.tmp.0.join(name)
    }

    pub fn cache_exe(&self, home: &Path) -> PathBuf {
        home.join("runtimes").join(&self.entry).join(&self.asset)
    }
}

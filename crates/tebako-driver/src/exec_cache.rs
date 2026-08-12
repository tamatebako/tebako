//! The exec-cache contract (spec 22 §6): per boot the driver names the
//! directory this process's materialized binaries and libraries live
//! under and exports it to the interpreter's environment as
//! `TEBAKO_EXEC_CACHE` (read-only to payloads). The closure walk
//! (`tebako_fs_dlmap2file`) writes under it — the variable and the
//! extraction root are the same place by construction.
//!
//! The root is `<temp>/tebako-exec-<key>`: `<key>` segregates per
//! runtime image (Rule L3 — a rebuilt runtime never reads a stale
//! extraction). The key's source, in order:
//!
//! 1. the image's `<path>.sha256` store sidecar (the trust anchor every
//!    resolved runtime and payload carries — the store layout; reading
//!    it is free, and verification stays at install, never per run),
//!    else
//! 2. a key of the image's path string (a dev boot without the store —
//!    honest about WHAT it keys: the path, not the content).
//!
//! Extractions today land in a per-process leaf under the root
//! (`tebako-dl-<hex>`, cleaned at exit — process-lifetime semantics);
//! the keyed root is the segregation the contract fixes and the
//! namespace the persistent write-once form upgrades into.

use std::path::{Path, PathBuf};

use crate::driver::Env;

/// The exported variable (spec 22 §6).
pub const VAR: &str = "TEBAKO_EXEC_CACHE";

/// The no-runtime-image key: a bare boot's materializations (payload
/// mounts) are keyed by their memfs paths inside a process-lifetime
/// cache; there is no runtime image to name the namespace.
const HOST_KEY: &str = "host";

/// Lowercase hex sha256 — the tebako-resolve `sha256_hex` idiom (the
/// same copy lives in `ffi::interpose` for the macOS content key;
/// layering forbids importing it from tebako-resolve).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// The 16-hex identity of an image file: the store sidecar when present
/// (the content key), else a key of the path string (a path key — a
/// no-store dev boot, documented above).
pub fn image_key(image: &Path) -> String {
    if let Some(hex) = sidecar_hex(image) {
        return hex;
    }
    sha256_hex(image.to_string_lossy().as_bytes())[..16].to_string()
}

/// First 16 chars of the sidecar's leading 64-hex token, when the
/// sidecar exists and carries one. A sidecar that does not parse is no
/// key source (this is cache NAMING, never a verification path: trust
/// anchors are verified at install, not per run).
fn sidecar_hex(image: &Path) -> Option<String> {
    let mut sidecar = image.as_os_str().to_os_string();
    sidecar.push(".sha256");
    let text = std::fs::read_to_string(PathBuf::from(sidecar)).ok()?;
    let token = text.split_whitespace().next()?;
    if token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(token[..16].to_string())
    } else {
        None
    }
}

/// The root this boot exports: `<temp>/tebako-exec-<key>`.
pub fn root_for(temp: &Path, key: &str) -> PathBuf {
    temp.join(format!("tebako-exec-{key}"))
}

/// Name and export this boot's exec-cache root (spec 22 §6). Called
/// once per boot before the interpreter handoff; the interpreter and
/// payloads read it, never write it.
pub fn export(env: &dyn Env) {
    let key = env
        .var("TEBAKO_RUNTIME_IMAGE")
        .filter(|s| !s.is_empty())
        .map(|p| image_key(Path::new(&p)))
        .unwrap_or_else(|| HOST_KEY.to_string());
    let root = root_for(&std::env::temp_dir(), &key);
    env.set_var(VAR, &root.to_string_lossy());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn temp(tag: &str) -> PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tebako-exec-cache-{tag}-{}-{uniq}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

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

    #[test]
    fn the_sidecar_is_the_content_key() {
        let dir = temp("sidecar");
        let img = dir.join("runtime.tfs");
        std::fs::write(&img, b"image").unwrap();
        std::fs::write(
            dir.join("runtime.tfs.sha256"),
            format!("{}  runtime.tfs\n", "ab".repeat(32)),
        )
        .unwrap();
        assert_eq!(image_key(&img), "abababababababab");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_sidecar_keys_the_path_not_the_content() {
        let dir = temp("pathkey");
        let img = dir.join("runtime.tfs");
        std::fs::write(&img, b"image").unwrap();
        let k1 = image_key(&img);
        assert_eq!(k1.len(), 16);
        // A path key: rewriting the file at the same path keeps the key
        // (documented — a no-store dev boot), a different path keys
        // differently.
        std::fs::write(&img, b"different bytes").unwrap();
        assert_eq!(image_key(&img), k1);
        let other = dir.join("other.tfs");
        std::fs::write(&other, b"different bytes").unwrap();
        assert_ne!(image_key(&other), k1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_malformed_sidecar_falls_back_to_the_path_key() {
        let dir = temp("badsidecar");
        let img = dir.join("runtime.tfs");
        std::fs::write(&img, b"image").unwrap();
        std::fs::write(dir.join("runtime.tfs.sha256"), b"not a checksum\n").unwrap();
        let with_bad = image_key(&img);
        std::fs::remove_file(dir.join("runtime.tfs.sha256")).unwrap();
        assert_eq!(with_bad, image_key(&img));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_root_is_tebako_namespaced_under_temp() {
        assert_eq!(
            root_for(Path::new("/tmp"), "0123456789abcdef"),
            Path::new("/tmp/tebako-exec-0123456789abcdef")
        );
    }

    #[test]
    fn export_names_the_env_image_keyed_root() {
        let dir = temp("export");
        let img = dir.join("runtime.tfs");
        std::fs::write(&img, b"image").unwrap();
        std::fs::write(
            dir.join("runtime.tfs.sha256"),
            format!("{}  runtime.tfs\n", "cd".repeat(32)),
        )
        .unwrap();
        let env = env_with(&[("TEBAKO_RUNTIME_IMAGE", &img.to_string_lossy())]);
        export(&env);
        let got = env.0.borrow().get(VAR).cloned().unwrap();
        assert_eq!(
            got,
            std::env::temp_dir()
                .join("tebako-exec-cdcdcdcdcdcdcdcd")
                .to_string_lossy()
                .as_ref()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_without_a_runtime_image_uses_the_host_key() {
        let env = env_with(&[]);
        export(&env);
        let got = env.0.borrow().get(VAR).cloned().unwrap();
        assert_eq!(
            got,
            std::env::temp_dir()
                .join("tebako-exec-host")
                .to_string_lossy()
                .as_ref()
        );
    }
}

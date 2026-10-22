//! The node press end to end, opt-in: stage → vendor → image → stitch →
//! RUN. The fixture needs an installed node pair and nothing else — the
//! closure is zero-dependency, so the vendor step is offline.
//!
//!   TEBAKO_NODE_PAIR=<store>/runtimes/node-<lv>-<ver>-<triplet>/<exe> \
//!   TEBAKO_NODE_IMAGE=<…>/<…>.tfs \
//!   cargo test -p tebako-cli --test press_node -- --ignored
//!
//! The store is synthesized from the pair's own directory, so the press
//! resolves from cache (no registry, no network) exactly as a real user's
//! store would after `tebako run node` has run once.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn pair() -> Option<(PathBuf, PathBuf)> {
    let exe = std::env::var_os("TEBAKO_NODE_PAIR")?;
    let image = std::env::var_os("TEBAKO_NODE_IMAGE")?;
    Some((PathBuf::from(exe), PathBuf::from(image)))
}

/// A scratch store carrying the pair under its canonical entry name.
fn seeded_store(pair_exe: &Path, pair_image: &Path) -> PathBuf {
    let store =
        std::env::temp_dir().join(format!("tebako-press-node-store-{}", std::process::id()));
    let _ = fs::remove_dir_all(&store);
    let entry = pair_exe
        .parent()
        .expect("the pair exe lives in its entry directory");
    let name = entry
        .file_name()
        .expect("the entry directory has a name")
        .to_owned();
    let dst = store.join("runtimes").join(&name);
    fs::create_dir_all(&dst).unwrap();
    for item in fs::read_dir(entry).unwrap() {
        let item = item.unwrap();
        let target = dst.join(item.file_name());
        if item.path().is_dir() {
            continue;
        }
        fs::copy(item.path(), target).unwrap();
    }
    // The pair's markers ride along; make sure the image the caller named
    // exists in the entry (a store entry owns both artifacts).
    let image_name = pair_image
        .file_name()
        .expect("the image has a name")
        .to_owned();
    if !dst.join(&image_name).is_file() {
        fs::copy(pair_image, dst.join(&image_name)).unwrap();
    }
    fs::write(store.join("config.yaml"), "registries: []\n").unwrap();
    store
}

#[test]
#[ignore = "needs TEBAKO_NODE_PAIR and TEBAKO_NODE_IMAGE (a real installed node pair)"]
fn a_node_project_presses_into_a_runnable_package() {
    let Some((exe, image)) = pair() else {
        panic!("set TEBAKO_NODE_PAIR and TEBAKO_NODE_IMAGE to a real installed node pair");
    };
    let store = seeded_store(&exe, &image);

    let project =
        std::env::temp_dir().join(format!("tebako-press-node-proj-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"name":"hello-node-press","version":"1.0.0","bin":{"hello-node-press":"cli.js"},"engines":{"node":">=20"}}"#,
    )
    .unwrap();
    fs::write(
        project.join("package-lock.json"),
        r#"{"name":"hello-node-press","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"hello-node-press","version":"1.0.0"}}}"#,
    )
    .unwrap();
    fs::write(project.join("cli.js"), "console.log('node press ok')\n").unwrap();

    let press = Command::new(tebako_bin())
        .arg("press")
        .current_dir(&project)
        .env("TEBAKO_HOME", &store)
        .env("TEBAKO_NO_PROGRESS", "1")
        .output()
        .expect("the CLI must run");
    assert!(
        press.status.success(),
        "press failed: {}{}",
        String::from_utf8_lossy(&press.stdout),
        String::from_utf8_lossy(&press.stderr)
    );
    let package = project.join("cli");
    assert!(
        package.is_file(),
        "the package must land next to the project"
    );

    let run = Command::new(&package)
        .env("TEBAKO_HOME", &store)
        .env("TEBAKO_NO_PROGRESS", "1")
        .output()
        .expect("the package must run");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "package failed: {stdout}{stderr}");
    assert!(
        stdout.contains("node press ok"),
        "the payload's script must run: {stdout}{stderr}"
    );

    let _ = fs::remove_dir_all(&project);
    let _ = fs::remove_dir_all(&store);
}

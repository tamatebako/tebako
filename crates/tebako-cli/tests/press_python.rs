//! The python press end to end, opt-in: stage → vendor → flatten → image
//! → stitch → RUN. The fixture needs an installed python pair and network
//! (pip's build isolation fetches the project's build backend — the
//! wheel-only closure contract is about the DEPENDENCIES, and this
//! fixture declares none).
//!
//!   TEBAKO_PYTHON_PAIR=<store>/runtimes/python-<lv>-<ver>-<triplet>/<exe> \
//!   TEBAKO_PYTHON_IMAGE=<…>/<…>.tfs \
//!   cargo test -p tebako-cli --test press_python -- --ignored
//!
//! The store is synthesized from the pair's own directory, so the press
//! resolves from cache (no registry) exactly as a real user's store would
//! after `tebako run python` has run once.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn pair() -> Option<(PathBuf, PathBuf)> {
    let exe = std::env::var_os("TEBAKO_PYTHON_PAIR")?;
    let image = std::env::var_os("TEBAKO_PYTHON_IMAGE")?;
    Some((PathBuf::from(exe), PathBuf::from(image)))
}

/// A scratch store carrying the pair under its canonical entry name.
fn seeded_store(pair_exe: &Path, pair_image: &Path) -> PathBuf {
    let store =
        std::env::temp_dir().join(format!("tebako-press-python-store-{}", std::process::id()));
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
#[ignore = "needs TEBAKO_PYTHON_PAIR and TEBAKO_PYTHON_IMAGE (a real installed python pair)"]
fn a_python_project_presses_into_a_runnable_package() {
    let Some((exe, image)) = pair() else {
        panic!("set TEBAKO_PYTHON_PAIR and TEBAKO_PYTHON_IMAGE to a real installed python pair");
    };
    let store = seeded_store(&exe, &image);

    let project =
        std::env::temp_dir().join(format!("tebako-press-python-proj-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(project.join("src").join("hello_py")).unwrap();
    fs::write(
        project.join("pyproject.toml"),
        "[build-system]\nrequires = [\"setuptools>=61\"]\nbuild-backend = \"setuptools.build_meta\"\n\n[project]\nname = \"hello-py-press\"\nversion = \"0.1.0\"\nrequires-python = \">=3.11\"\n\n[project.scripts]\nhello-py-press = \"hello_py.cli:main\"\n",
    )
    .unwrap();
    // The lockfile-true gate: the file must exist; an empty closure is a
    // comment away.
    fs::write(project.join("requirements.txt"), "# empty closure\n").unwrap();
    fs::write(project.join("src").join("hello_py").join("__init__.py"), "").unwrap();
    fs::write(
        project.join("src").join("hello_py").join("cli.py"),
        "def main():\n    print('python press ok')\n",
    )
    .unwrap();

    let press = Command::new(tebako_bin())
        .arg("press")
        .current_dir(&project)
        .env("TEBAKO_HOME", &store)
        .env("TEBAKO_NO_PROGRESS", "1")
        .output()
        .expect("the CLI must run");
    let stdout = String::from_utf8_lossy(&press.stdout);
    let stderr = String::from_utf8_lossy(&press.stderr);
    assert!(press.status.success(), "press failed: {stdout}{stderr}");
    // The package lands where the press said it landed.
    let created = stdout
        .lines()
        .find_map(|l| {
            l.strip_prefix("Created tebako package at \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .unwrap_or_else(|| panic!("the press names its package: {stdout}{stderr}"));
    let package = PathBuf::from(created);
    assert!(package.is_file(), "the package must exist: {created}");

    let run = Command::new(&package)
        .env("TEBAKO_HOME", &store)
        .env("TEBAKO_NO_PROGRESS", "1")
        .output()
        .expect("the package must run");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "package failed: {stdout}{stderr}");
    assert!(
        stdout.contains("python press ok"),
        "the payload's console script must run: {stdout}{stderr}"
    );

    let _ = fs::remove_dir_all(&project);
    let _ = fs::remove_dir_all(&store);
}

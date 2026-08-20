//! `tebako check` e2e (spec 26 §2 — the check engine): structural checks
//! against bare images, exec checks against a GIVEN runtime (the
//! press-time gate) with the driver-contract argv/env observed end to
//! end, the store-backed name form (zero-runtime materialized exec), the
//! pressed-package form (argv0 entry selection against the type-2 entry
//! table), and the composition-document form (the slice union). The
//! fixture "runtimes"/"packages" are shebang scripts — the kernel execs
//! them and they report what they saw. Unix only (shebang exec), like
//! tests/run.rs and tests/trace_run.rs.

#![cfg(unix)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn tebako_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tebako"))
}

fn workdir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tebako-check-e2e-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Run {
    rc: i32,
    stdout: String,
    stderr: String,
}

/// `tebako check <args>` with TEBAko_HOME pinned at `home` (never the
/// operator's real store) plus any per-leg env.
fn tebako_check(home: &Path, args: &[&Path], env: &[(&str, String)]) -> Run {
    let mut cmd = Command::new(tebako_bin());
    cmd.arg("check")
        .args(args)
        .env("TEBAKO_HOME", home)
        .env("TEBAKO_OFFLINE", "1");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    Run {
        rc: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// A mountable payload image (the tfs ZIP backend is pure Rust) with
/// explicit directory entries, so a backend `stat` on a directory sees
/// one.
fn zip_image(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let mut dirs = std::collections::BTreeSet::new();
    for (name, _) in files {
        let mut prefix = String::new();
        for component in Path::new(name).parent().into_iter().flat_map(|p| p.iter()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(&component.to_string_lossy());
            dirs.insert(format!("{prefix}/"));
        }
    }
    for dir in dirs {
        writer.add_directory(dir, options).unwrap();
    }
    for (name, bytes) in files {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

const TREE_HASH: &str = "sha256:650f8ad9527c28dbb8ae43270215e4ef64c884cea06bec289918b060f3b69ee3";
const BLOB: &str = "7a5eb4446074d0193468f1a24cf5a94e4748cf1f033b0fdfcb8bfbaa901a81e1";

fn identity(kind: &str, name: &str, version: &str) -> String {
    [
        "identity:\n  schema_version: 1\n  kind: ",
        kind,
        "\n  name: ",
        name,
        "\n  version: \"",
        version,
        "\"\n  producer: {tool: t, tool_version: \"1\"}\n  created: \"2026-08-19T00:00:00Z\"\n",
        "  digest:\n    tree_hash: \"",
        TREE_HASH,
        "\"\n    blob_sha256: ",
        BLOB,
        "\n  signing: {state: unsigned}\n  encryption: {state: none}\n",
    ]
    .concat()
}

/// A kind:app manifest for payload `name` with one entrypoint
/// `acme` → `/bin/acme` and the given checks block.
fn app_manifest(name: &str, checks: &str) -> String {
    [
        identity("app", name, "1.0"),
        "provides:\n  entrypoints: [{name: acme, path: /bin/acme}]\n  platforms: [aarch64-macos]\n  capabilities: {exec: true, read: true}\nchecks:\n"
            .to_string(),
        checks.to_string(),
    ]
    .concat()
}

/// A kind:data manifest with the given checks block.
fn data_manifest(name: &str, suggested: &str, checks: &str) -> String {
    [
        identity("data", name, "1.0"),
        "provides:\n  mount_semantics: {suggested: ".to_string(),
        suggested.to_string(),
        "}\n  capabilities: {exec: false, read: true}\nchecks:\n".to_string(),
        checks.to_string(),
    ]
    .concat()
}

/// The fixture app image: an entrypoint, a fixtures dir, a data file,
/// and the six-check block the Given-form legs select from.
fn app_image(dir: &Path) -> PathBuf {
    let checks = "  layout:\n    expect: {image_files: [/data/file.txt]}\n\
        \x20 boot:\n    entry: /bin/acme\n    argv: [\"{scratch}/made.txt\"]\n    fixtures: /fixtures\n    expect: {exit: 0, files: [made.txt], stdout: \"CONTRACT OK\"}\n\
        \x20 bail:\n    entry: /bin/acme\n\
        \x20 slow:\n    entry: /bin/acme\n    timeout: 1\n\
        \x20 win-only:\n    entry: /bin/acme\n    when: [windows]\n\
        \x20 needs-jvm:\n    entry: /bin/acme\n    requires: {provides: [jvm]}\n";
    let manifest = app_manifest("acme-app", checks);
    let bytes = zip_image(&[
        ("__tpkg__/manifest.yaml", manifest.as_bytes()),
        ("bin/acme", b"#!/bin/sh\n"),
        ("fixtures/hello.txt", b"hello fixture\n"),
        ("data/file.txt", b"x"),
    ]);
    let path = dir.join("acme-app.tfs");
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The contract probe: report the driver-contract argv/env (spec 17 §1),
/// prove the fixtures landed at the scratch root (the cwd), write the
/// `{scratch}`-named file, and print the regex anchor.
const CONTRACT: &str = "#!/bin/sh\n\
echo \"IMG=$2\"\n\
echo \"ENTRY=$4\"\n\
echo \"RTIMG=${TEBAKO_RUNTIME_IMAGE:-UNSET}\"\n\
cat hello.txt\n\
echo data > \"$5\"\n\
echo \"CONTRACT OK\"\n";

/// A bare "env image" stand-in (the engine only hands its path to the
/// runtime as TEBAKO_RUNTIME_IMAGE).
fn env_image(dir: &Path) -> PathBuf {
    let bytes = zip_image(&[("lib/stub", b"x")]);
    let path = dir.join("env.tfs");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn structural_checks_pass_and_fail_on_bare_images() {
    let dir = workdir("structural");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();

    // PASS: the asserted file exists and is non-empty.
    let pass = dir.join("data-pass.tfs");
    std::fs::write(
        &pass,
        zip_image(&[
            (
                "__tpkg__/manifest.yaml",
                data_manifest(
                    "acme-templates",
                    "/templates/acme",
                    "  layout:\n    expect: {image_files: [/templates/acme/cover.adoc]}\n",
                )
                .as_bytes(),
            ),
            ("templates/acme/cover.adoc", b"= Cover\n"),
        ]),
    )
    .unwrap();
    let run = tebako_check(&home, &[&pass], &[]);
    assert_eq!(
        run.rc, 0,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(run.stdout.contains("check layout PASS"), "{}", run.stdout);

    // FAIL: the asserted file is absent — the aggregate exit is
    // EX_TEBAKO_CHECK (79; spec 26 §2's 72 collides with EX_TEBAKO_TRUST).
    let fail = dir.join("data-fail.tfs");
    std::fs::write(
        &fail,
        zip_image(&[
            (
                "__tpkg__/manifest.yaml",
                data_manifest(
                    "acme-templates",
                    "/templates/acme",
                    "  layout:\n    expect: {image_files: [/templates/acme/cover.adoc, /templates/acme/missing.html]}\n",
                )
                .as_bytes(),
            ),
            ("templates/acme/cover.adoc", b"= Cover\n"),
        ]),
    )
    .unwrap();
    let run = tebako_check(&home, &[&fail], &[]);
    assert_eq!(
        run.rc, 79,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stdout.contains(
            "check layout FAIL (expected image file missing: /templates/acme/missing.html)"
        ),
        "{}",
        run.stdout
    );
}

#[test]
fn exec_check_against_a_given_runtime_observes_the_driver_contract() {
    let dir = workdir("given");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);
    let runtime = script(&dir, "fake-ruby", CONTRACT);
    let env = env_image(&dir);

    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("boot"),
            &PathBuf::from("--runtime"),
            &runtime,
            &PathBuf::from("--runtime-image"),
            &env,
        ],
        &[],
    );
    assert_eq!(
        run.rc, 0,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    // The driver contract, observed by the payload side: the checked
    // image mounted whole at /, the entry path, the env image.
    assert!(
        run.stdout.contains(&format!("IMG={}:0:/", image.display())),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("ENTRY=/bin/acme"), "{}", run.stdout);
    assert!(
        run.stdout.contains(&format!("RTIMG={}", env.display())),
        "{}",
        run.stdout
    );
    // The in-image fixtures materialized at the scratch root (the cwd)…
    assert!(run.stdout.contains("hello fixture"), "{}", run.stdout);
    // …and the verdict line.
    assert!(run.stdout.contains("check boot PASS"), "{}", run.stdout);
}

#[test]
fn exec_check_fail_exit_mismatch_and_timeout() {
    let dir = workdir("givenfail");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);
    let env = env_image(&dir);

    // exit mismatch → FAIL with the reason named, aggregate 79
    let bail = script(&dir, "bail-ruby", "#!/bin/sh\nexit 3\n");
    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("bail"),
            &PathBuf::from("--runtime"),
            &bail,
            &PathBuf::from("--runtime-image"),
            &env,
        ],
        &[],
    );
    assert_eq!(run.rc, 79, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout
            .contains("check bail FAIL (exit code 3 (expected 0))"),
        "{}",
        run.stdout
    );

    // timeout → FAIL naming the timeout
    let sleepy = script(&dir, "sleepy-ruby", "#!/bin/sh\nsleep 30\n");
    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("slow"),
            &PathBuf::from("--runtime"),
            &sleepy,
            &PathBuf::from("--runtime-image"),
            &env,
        ],
        &[],
    );
    assert_eq!(run.rc, 79, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains("check slow FAIL (timeout after 1s)"),
        "{}",
        run.stdout
    );
}

#[test]
fn skips_are_loud_and_exit_zero() {
    let dir = workdir("skips");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);
    let runtime = script(&dir, "fake-ruby", CONTRACT);
    let env = env_image(&dir);

    // The platform filter: this host is never `windows` (the file is
    // unix-only), so win-only SKIPs.
    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("win-only"),
            &PathBuf::from("--runtime"),
            &runtime,
            &PathBuf::from("--runtime-image"),
            &env,
        ],
        &[],
    );
    assert_eq!(run.rc, 0, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains("check win-only SKIP (not for "),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("(when: windows))"), "{}", run.stdout);

    // An unmet composition prerequisite SKIPs, never FAILs.
    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("needs-jvm"),
            &PathBuf::from("--runtime"),
            &runtime,
            &PathBuf::from("--runtime-image"),
            &env,
        ],
        &[],
    );
    assert_eq!(run.rc, 0, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout
            .contains("check needs-jvm SKIP (no jvm in the composition)"),
        "{}",
        run.stdout
    );
}

#[test]
fn list_prints_declaration_order_and_shape() {
    let dir = workdir("list");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);

    let run = tebako_check(&home, &[&image, &PathBuf::from("--list")], &[]);
    assert_eq!(run.rc, 0, "stdout:\n{}", run.stdout);
    let lines: Vec<&str> = run
        .stdout
        .lines()
        .filter(|l| l.starts_with("check "))
        .collect();
    assert_eq!(
        lines,
        vec![
            "check layout (structural)",
            "check boot (exec)",
            "check bail (exec)",
            "check slow (exec)",
            "check win-only (exec)",
            "check needs-jvm (exec)",
        ],
        "{}",
        run.stdout
    );
}

#[test]
fn unknown_check_name_is_a_named_usage_error() {
    let dir = workdir("unknown");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);

    let run = tebako_check(
        &home,
        &[&image, &PathBuf::from("--check"), &PathBuf::from("nope")],
        &[],
    );
    assert_eq!(run.rc, 64, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains("no check named \"nope\""),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("declared:"), "{}", run.stdout);
}

#[test]
fn keep_scratch_preserves_and_names_the_dir() {
    let dir = workdir("keep");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let image = app_image(&dir);
    let runtime = script(&dir, "fake-ruby", CONTRACT);
    let env = env_image(&dir);

    let run = tebako_check(
        &home,
        &[
            &image,
            &PathBuf::from("--check"),
            &PathBuf::from("boot"),
            &PathBuf::from("--runtime"),
            &runtime,
            &PathBuf::from("--runtime-image"),
            &env,
            &PathBuf::from("--keep-scratch"),
        ],
        &[],
    );
    assert_eq!(run.rc, 0, "stdout:\n{}", run.stdout);
    let path = run
        .stdout
        .lines()
        .find_map(|l| l.strip_prefix("scratch kept: "))
        .unwrap_or_else(|| panic!("scratch kept: missing from stdout:\n{}", run.stdout));
    assert!(Path::new(path).is_dir(), "--keep-scratch preserves {path}");
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn usage_errors_exit_1() {
    let dir = workdir("usage");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();

    // no target
    let run = tebako_check(&home, &[], &[]);
    assert_eq!(
        run.rc, 1,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(run.stderr.contains("usage: tebako check"), "{}", run.stderr);

    // --runtime-image without --runtime
    let run = tebako_check(
        &home,
        &[
            &PathBuf::from("x.tfs"),
            &PathBuf::from("--runtime-image"),
            &PathBuf::from("y"),
        ],
        &[],
    );
    assert_eq!(run.rc, 1, "stderr:\n{}", run.stderr);
    assert!(
        run.stderr.contains("--runtime-image needs --runtime"),
        "{}",
        run.stderr
    );
}

// ---------------------------------------------------------------------
// The pressed-package form
// ---------------------------------------------------------------------

/// A "package": a probe script + one zip slot carrying the checks-bearing
/// payload manifest + the tpkg trailer with a type-2 package manifest
/// (tests/run.rs's fixture shape).
fn package_with_checks(dir: &Path) -> PathBuf {
    let pkg = dir.join("acme-pkg");
    std::fs::write(&pkg, "#!/bin/sh\necho \"ARGS=$*\"\nexit 0\n").unwrap();

    let checks = "  boot:\n    entry: /bin/acme\n    argv: [hello]\n    expect: {exit: 0, stdout: \"ARGS=hello\"}\n\
        \x20 layout:\n    expect: {image_files: [/data/file.txt]}\n\
        \x20 nope:\n    entry: /bin/other\n";
    let slot = zip_image(&[
        (
            "__tpkg__/manifest.yaml",
            app_manifest("acme-app", checks).as_bytes(),
        ),
        ("data/file.txt", b"x"),
    ]);
    let payload = dir.join("acme-pkg.payload");
    std::fs::write(&payload, &slot).unwrap();

    let mut m = tpkg::Manifest {
        package_flags: 0,
        launcher_abi: 0,
        ..Default::default()
    };
    m.set_runtime_ref(b"ruby@9.9.9;tebako=9.9.9");
    let base = std::fs::metadata(&pkg).unwrap().len();
    let size = std::fs::metadata(&payload).unwrap().len();
    m.slots.push(tpkg::Slot::new(
        base,
        size,
        tpkg::TPKG_FORMAT_DWARFS,
        "/__tfs__",
    ));
    m.set_package_manifest(&tpkg::PackageManifest {
        schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
        package: tpkg::PackageIdentity {
            name: "acme-pkg".to_string(),
            version: "1.0.0".to_string(),
            producer: tpkg::Producer {
                tool: "tebako-cli".to_string(),
                tool_version: "0.16.0".to_string(),
            },
            created: "2026-08-20T00:00:00Z".to_string(),
        },
        entries: vec![tpkg::PackageEntry {
            name: "acme".to_string(),
            slot: 0,
            // The entry table names the PROVIDES entrypoint BY NAME; the
            // engine resolves it to the in-image path through the slot
            // manifest.
            entrypoint: "acme".to_string(),
            runtime_ref: "ruby@9.9.9;tebako=9.9.9".to_string(),
        }],
        jail: None,
        env: Default::default(),
        mounts: Vec::new(),
    })
    .unwrap();
    {
        let mut f = std::fs::OpenOptions::new().append(true).open(&pkg).unwrap();
        f.write_all(&std::fs::read(&payload).unwrap()).unwrap();
        tpkg::write_to(&mut f, &m).unwrap();
    }
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&pkg, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    pkg
}

#[test]
fn package_form_runs_slot_checks_and_names_entry_mismatches() {
    let dir = workdir("package");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    let pkg = package_with_checks(&dir);

    let run = tebako_check(&home, &[&pkg], &[]);
    assert_eq!(
        run.rc, 79,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    // exec: the package itself runs with the check's argv…
    assert!(run.stdout.contains("check boot PASS"), "{}", run.stdout);
    // …structural reads the slot region…
    assert!(run.stdout.contains("check layout PASS"), "{}", run.stdout);
    // …and an entry path no package entry maps to is a named FAIL (the
    // declared table is listed).
    assert!(
        run.stdout.contains(
            "check nope FAIL (check entry /bin/other matches no declared package entry for slot 0"
        ),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("acme→slot 0:acme"), "{}", run.stdout);
}

// ---------------------------------------------------------------------
// The store-backed name form
// ---------------------------------------------------------------------

/// Install a fake payload record: the image, its manifest mirror, and
/// (for the zero-runtime exec leg) the materialized tree.
fn store_payload(home: &Path, name: &str, version: &str, manifest: &str, files: &[(&str, &[u8])]) {
    let dir = home.join("payloads").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let mut image_files = vec![("__tpkg__/manifest.yaml", manifest.as_bytes())];
    image_files.extend_from_slice(files);
    std::fs::write(dir.join(format!("{version}.tfs")), zip_image(&image_files)).unwrap();
    std::fs::write(dir.join(format!("{version}.manifest.yaml")), manifest).unwrap();
}

#[test]
fn name_form_resolves_the_store_and_execs_zero_runtime() {
    let dir = workdir("name");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();

    let checks = "  boot:\n    entry: /bin/acme\n    argv: [--out, \"{scratch}/made.txt\"]\n    expect: {exit: 0, files: [made.txt], stdout: \"ZERO OK\"}\n";
    let manifest = app_manifest("acme-tool", checks);
    store_payload(
        &home,
        "acme-tool",
        "1.0",
        &manifest,
        &[("bin/acme", b"#!/bin/sh\n")],
    );

    // The zero-runtime entrypoint's install-time materialization (the
    // dispatch rule — a run never materializes): a probe script that
    // writes the {scratch}-named file ($1 = --out, $2 = the path).
    let tree = home.join("payloads/acme-tool/1.0.tree");
    std::fs::create_dir_all(tree.join("bin")).unwrap();
    script(
        &tree,
        "bin/acme",
        "#!/bin/sh\necho \"ZERO OK\"\necho x > \"$2\"\n",
    );

    let run = tebako_check(
        &home,
        &[&PathBuf::from("acme-tool")],
        &[("TEBAKO_ACME_TOOL_VERSION", "1.0".to_string())],
    );
    assert_eq!(
        run.rc, 0,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(run.stdout.contains("ZERO OK"), "{}", run.stdout);
    assert!(run.stdout.contains("check boot PASS"), "{}", run.stdout);
}

// ---------------------------------------------------------------------
// The composition-document form (spec 26 §2.1)
// ---------------------------------------------------------------------

#[test]
fn composition_checks_assert_the_slice_union() {
    let dir = workdir("composition");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    store_payload(
        &home,
        "acme-data",
        "1.0",
        &data_manifest("acme-data", "/data", ""),
        &[("templates/acme/cover.adoc", b"= Cover\n")],
    );

    // PASS: the asserted file is in the mounted slice union.
    let doc = dir.join("tebako.yaml");
    std::fs::write(
        &doc,
        "version: 1\nslices:\n  - {name: acme-data, mount: /data}\nchecks:\n  layout:\n    expect: {image_files: [/templates/acme/cover.adoc]}\n",
    )
    .unwrap();
    let run = tebako_check(&home, &[&doc], &[]);
    assert_eq!(
        run.rc, 0,
        "stdout:\n{}\nstderr:\n{}",
        run.stdout, run.stderr
    );
    assert!(
        run.stdout.contains("composition: layout PASS"),
        "{}",
        run.stdout
    );

    // FAIL: absent from every slice — the union misses.
    std::fs::write(
        &doc,
        "version: 1\nslices:\n  - {name: acme-data, mount: /data}\nchecks:\n  layout:\n    expect: {image_files: [/templates/acme/missing.html]}\n",
    )
    .unwrap();
    let run = tebako_check(&home, &[&doc], &[]);
    assert_eq!(run.rc, 79, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains(
            "composition: layout FAIL (expected image file missing: /templates/acme/missing.html)"
        ),
        "{}",
        run.stdout
    );
}

#[test]
fn composition_documents_are_validated_with_named_errors() {
    let dir = workdir("composition-invalid");
    let home = dir.join("home");
    std::fs::create_dir(&home).unwrap();
    store_payload(
        &home,
        "acme-data",
        "1.0",
        &data_manifest("acme-data", "/data", ""),
        &[("templates/acme/cover.adoc", b"= Cover\n")],
    );
    let doc = dir.join("tebako.yaml");

    // Only version 1 exists.
    std::fs::write(&doc, "version: 2\n").unwrap();
    let run = tebako_check(&home, &[&doc], &[]);
    assert_eq!(run.rc, 65, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains("version must be 1 (got 2)"),
        "{}",
        run.stdout
    );

    // An entrypoint-less composition's slices must declare their mounts
    // (a collision at / would be a silent order — refused).
    std::fs::write(&doc, "version: 1\nslices:\n  - {name: acme-data}\n").unwrap();
    let run = tebako_check(&home, &[&doc], &[]);
    assert_eq!(run.rc, 65, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout.contains("needs a declared mount"),
        "{}",
        run.stdout
    );

    // An unknown policy word is named (record is the engine's --record,
    // never authored).
    std::fs::write(
        &doc,
        "version: 1\npolicy: record\nslices:\n  - {name: acme-data, mount: /data}\n",
    )
    .unwrap();
    let run = tebako_check(&home, &[&doc], &[]);
    assert_eq!(run.rc, 65, "stdout:\n{}", run.stdout);
    assert!(
        run.stdout
            .contains("unknown policy \"record\" (want open|deny"),
        "{}",
        run.stdout
    );
}

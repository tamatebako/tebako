//! spec 06 §5 (locked) progress UX, asserted against the real bootstrap
//! binary with piped stderr — i.e. the non-TTY contract: exactly the
//! start + done single lines per fetched artifact, one quiet line on a
//! cache hit, the benefit wording with the real size, opt-out envs keeping
//! the plain mode, and stdout left entirely to the payload.
//!
//! The TTY-mode frames (bar geometry, ≤ 10 redraws/s throttle, spinner,
//! phase lines, `\r\x1b[K` redraws) are asserted exactly in tebako-term's
//! unit tests through its Writer/IsTty injection seam — no pty, no global
//! state.

// The harness is shared with selftest.rs; this suite uses run_raw while
// that one uses run — silence per-binary unused-function warnings.
#[allow(dead_code)]
mod harness;

use harness::{rust_bootstrap, strip_legacy_warning, Harness, TEBAKO_VER};

fn h() -> Harness {
    Harness::new(rust_bootstrap())
}

/// stderr minus the orthogonal item-29 legacy warning: the fixtures are
/// v1-unsigned BY DESIGN, and that warning is asserted in tests/chain.rs.
/// What remains is the spec 06 §5 transcript this suite pins.
fn clean(err: String) -> String {
    strip_legacy_warning(err)
}

/// The exact non-TTY transcript for one artifact: start + done, nothing
/// else (phases and the bar are TTY-only).
fn expected_fetch_lines(h: &Harness, home: &std::path::Path, size: u64) -> String {
    let entry_dir = home.join("runtimes").join(&h.entry);
    format!(
        "downloading {} ({})\ninstalled {} ({}) — cached at {} and shared by every tebako app on this machine\n",
        h.asset,
        tebako_term::human_bytes(size),
        h.entry,
        tebako_term::human_bytes(size),
        entry_dir.display()
    )
}

#[test]
fn non_tty_download_is_exactly_start_plus_done() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    let (rc, out, err) = h.run_raw(&pkg, &home, &[], &["hello"]);
    assert_eq!(rc, 0, "{err}");
    let err = clean(err);

    // the benefit line carries the REAL installed size
    let size = std::fs::metadata(h.cache_exe(&home)).unwrap().len();
    assert!(size > 0, "the fixture runtime must have bytes");
    assert_eq!(err, expected_fetch_lines(&h, &home, size), "{err}");

    // stdout belongs to the payload: no progress leaks into it
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
    assert!(!out.contains("downloading"), "{out}");
    assert!(!out.contains("installed"), "{out}");
    assert!(!out.contains("resolving"), "{out}");
}

#[test]
fn cache_hit_is_one_quiet_line() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home");
    assert_eq!(h.run_raw(&pkg, &home, &[], &[]).0, 0, "warm-up install");

    let (rc, out, err) = h.run_raw(&pkg, &home, &[], &["hello"]);
    assert_eq!(rc, 0, "{err}");
    let err = clean(err);
    assert_eq!(err, "runtime ruby-3.3.7 (cached)\n", "{err}");
    assert!(out.contains("FAKE-RUNTIME"), "{out}");
}

#[test]
fn image_era_fetches_report_per_artifact() {
    let h = h();
    let pkg = h.lean_pkg_image("imgapp");
    let home = h.home("home");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 0, "{err}");
    let err = clean(err);

    let entry_dir = home.join("runtimes").join(&h.entry);
    let exe_size = std::fs::metadata(h.cache_exe(&home)).unwrap().len();
    let image_size = std::fs::metadata(h.cache_image(&home)).unwrap().len();
    let image_lines = format!(
        "downloading {} ({})\ninstalled {} ({}) — cached at {} and shared by every tebako app on this machine\n",
        h.image_asset,
        tebako_term::human_bytes(image_size),
        h.image_asset,
        tebako_term::human_bytes(image_size),
        entry_dir.display()
    );
    let expected = format!(
        "{}{}",
        expected_fetch_lines(&h, &home, exe_size),
        image_lines
    );
    assert_eq!(err, expected, "{err}");
}

#[test]
fn opt_out_envs_keep_the_plain_two_line_mode() {
    for (k, v) in [
        ("TEBAKO_NO_PROGRESS", "1"),
        ("NO_COLOR", "1"),
        ("TERM", "dumb"),
    ] {
        let h = h();
        let pkg = h.lean_pkg("myapp");
        let home = h.home("home");
        let (rc, _, err) = h.run_raw(&pkg, &home, &[(k, v)], &[]);
        assert_eq!(rc, 0, "{k}={v}: {err}");
        let err = clean(err);
        let size = std::fs::metadata(h.cache_exe(&home)).unwrap().len();
        assert_eq!(err, expected_fetch_lines(&h, &home, size), "{k}={v}: {err}");
    }
}

#[test]
fn error_bodies_are_golden_with_progress_present() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    // Poison the mirror's manifest: the download succeeds, the sha check fails.
    let badsha = "0".repeat(64);
    let manifest = h
        .mirror_root
        .join(format!("v{TEBAKO_VER}"))
        .join("manifest.json");
    let text = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace(&h.sha, &badsha);
    std::fs::write(&manifest, text).unwrap();

    let home = h.home("home3");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[], &[]);
    assert_eq!(rc, 70, "{err}");
    let err = clean(err);
    // the start line printed; the done line did not
    let size = std::fs::metadata(h.mirror_root.join(format!("v{TEBAKO_VER}")).join(&h.asset))
        .unwrap()
        .len();
    assert!(
        err.starts_with(&format!(
            "downloading {} ({})\n",
            h.asset,
            tebako_term::human_bytes(size)
        )),
        "{err}"
    );
    assert!(!err.contains("installed "), "{err}");
    // the golden error body is byte-stable (C++ parity)
    assert!(
        err.contains(&format!(
            "tebako-bootstrap: SHA256 mismatch for downloaded runtime {} — refusing to install or execute\n  expected: {badsha} (from ",
            h.asset
        )),
        "{err}"
    );
    assert!(
        err.contains("  the download was deleted; the cache was not touched\n"),
        "{err}"
    );
}

#[test]
fn offline_miss_has_no_progress_output() {
    let h = h();
    let pkg = h.lean_pkg("myapp");
    let home = h.home("home2");
    let (rc, _, err) = h.run_raw(&pkg, &home, &[("TEBAKO_OFFLINE", "1")], &[]);
    assert_eq!(rc, 69, "{err}");
    let err = clean(err);
    // nothing fetched → not even the start line; the error body is the
    // whole stderr (golden parity for errors).
    assert!(!err.contains("downloading"), "{err}");
    assert!(
        err.starts_with("tebako-bootstrap: cannot resolve runtime"),
        "{err}"
    );
}

//! The real-bytes instrument for delta transfer: what a genuine runtime
//! update costs on the wire, measured through the shipping assembler on
//! two published artifacts rather than on a fixture.
//!
//! The arc's acceptance asks whether a version bump can move less than a
//! tenth of the bytes. The unit tests pin the mechanics; this file answers
//! the question on artifacts a user actually updates between, and it
//! reports the numerator and denominator beside the share, because a
//! percentage without its bytes proves nothing.
//!
//! It runs only when pointed at a pair, so an ordinary `cargo test` stays
//! green on a machine with no store:
//!
//! ```text
//! TEBAKO_DELTA_BASE=<older .tfs> TEBAKO_DELTA_TARGET=<newer .tfs> \
//!   cargo test -p tebako-resolve --test delta_real_bytes -- --ignored --nocapture
//! ```
//!
//! `TEBAKO_DELTA_BUDGET` (a share such as `0.10`) turns the measurement
//! into an assertion; without it the test reports and passes.

use std::path::{Path, PathBuf};

use tpkg::lazy::Blksum;

fn pair() -> Option<(PathBuf, PathBuf)> {
    let base = std::env::var_os("TEBAKO_DELTA_BASE")?;
    let target = std::env::var_os("TEBAKO_DELTA_TARGET")?;
    Some((PathBuf::from(base), PathBuf::from(target)))
}

fn scratch(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "tebako-delta-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    path
}

#[test]
#[ignore = "measures a pair named by the environment; run it deliberately"]
fn a_real_update_moves_its_measured_share() {
    let Some((donor, target)) = pair() else {
        panic!("TEBAKO_DELTA_BASE and TEBAKO_DELTA_TARGET must name two published images");
    };
    let target_bytes = std::fs::read(&target).expect("the target artifact is readable");
    let sidecar = Blksum::from_image_bytes(&target_bytes);
    let out = scratch("assembly.tfs");
    let mut wire = 0u64;
    let mut ranges = 0u64;
    let mut fetch = |offset: u64, len: u64| {
        wire += len;
        ranges += 1;
        Ok(target_bytes[offset as usize..(offset + len) as usize].to_vec())
    };
    let stats = tebako_resolve::delta::assemble(
        &donor,
        &sidecar,
        &sidecar.sha256,
        &out,
        &mut fetch,
    )
    .expect("the assembler produces the published image");

    // The share describes a correct artifact only when the artifact it
    // produced is the published one.
    let assembled = std::fs::read(&out).expect("the assembled image is readable");
    let _ = std::fs::remove_file(&out);
    assert_eq!(assembled, target_bytes, "the assembled image is byte-identical");
    assert_eq!(wire, stats.bytes_fetched, "the wire carried the planned bytes");

    println!(
        "donor {} bytes -> target {} bytes ({} groups, group size locked at 4 MiB)",
        std::fs::metadata(Path::new(&donor)).map(|m| m.len()).unwrap_or(0),
        sidecar.size_bytes,
        sidecar.group_count()
    );
    println!(
        "copied {} groups / {} bytes; fetched {} groups / {} bytes in {} range read(s)",
        stats.groups_copied, stats.bytes_copied, stats.groups_fetched, stats.bytes_fetched, ranges
    );
    println!(
        "share {:.2}% of the image crossed the wire",
        stats.fetched_fraction() * 100.0
    );
    println!("journal line: event=delta-fetch {}", stats.journal_fields());

    if let Ok(budget) = std::env::var("TEBAKO_DELTA_BUDGET") {
        let budget: f64 = budget
            .parse()
            .expect("TEBAKO_DELTA_BUDGET is a share such as 0.10");
        assert!(
            stats.fetched_fraction() <= budget,
            "the update moved {:.2}% of the bytes against a {:.2}% budget",
            stats.fetched_fraction() * 100.0,
            budget * 100.0
        );
    }
}

#[test]
#[ignore = "measures a pair named by the environment; run it deliberately"]
fn a_donor_of_a_different_family_still_assembles() {
    // The donor arm must never cost correctness: a donor sharing nothing
    // still yields the published image, with every group fetched.
    let Some((_donor, target)) = pair() else {
        panic!("TEBAKO_DELTA_BASE and TEBAKO_DELTA_TARGET must name two published images");
    };
    let target_bytes = std::fs::read(&target).expect("the target artifact is readable");
    let sidecar = Blksum::from_image_bytes(&target_bytes);
    let out = scratch("no-donor.tfs");
    let empty = scratch("empty-donor.tfs");
    std::fs::write(&empty, b"").expect("an empty donor is writable");
    let mut fetch = |offset: u64, len: u64| {
        Ok(target_bytes[offset as usize..(offset + len) as usize].to_vec())
    };
    let stats =
        tebako_resolve::delta::assemble(&empty, &sidecar, &sidecar.sha256, &out, &mut fetch)
            .expect("an empty donor assembles the whole image");
    let assembled = std::fs::read(&out).expect("the assembled image is readable");
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&empty);
    assert_eq!(assembled, target_bytes, "the assembled image is byte-identical");
    assert_eq!(stats.groups_copied, 0, "an empty donor copies nothing");
    assert_eq!(stats.fetched_fraction(), 1.0, "and the wire carries everything");
}

//! Test fixture: the fake runtime the bootstrap selftests exec.
//!
//! Prints a marker line, the image env handoff, and its argv — the
//! byte-exact contract `tests/harness` asserts on. This was a `/bin/sh`
//! script; a compiled stub is what CreateProcess can run on Windows, and
//! it is closer to a real runtime binary on every platform (exec of a
//! binary, never of a script through a shell).

fn main() {
    println!("FAKE-RUNTIME");
    println!(
        "TEBAKO_RUNTIME_IMAGE={}",
        std::env::var("TEBAKO_RUNTIME_IMAGE").unwrap_or_default()
    );
    for (i, arg) in std::env::args().skip(1).enumerate() {
        println!("argv[{i}]={arg}");
    }
}

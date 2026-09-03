//! Test fixture: the fake runtime the bootstrap selftests exec.
//!
//! Prints a marker line, the image env handoff, the jail policy env
//! (the jail tests' probe), and its argv — the byte-exact contract
//! `tests/harness` asserts on. This was a `/bin/sh` script; a compiled
//! stub is what CreateProcess can run on Windows, and it is closer to a
//! real runtime binary on every platform (exec of a binary, never of a
//! script through a shell).

fn env_or_unset(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| "UNSET".to_string())
}

fn main() {
    println!("FAKE-RUNTIME");
    println!(
        "TEBAKO_RUNTIME_IMAGE={}",
        std::env::var("TEBAKO_RUNTIME_IMAGE").unwrap_or_default()
    );
    // The jail probe's contract (tests/jail.rs): UNSET when absent.
    println!("JAIL={}", env_or_unset("TEBAKO_JAIL"));
    println!("JAIL-SOURCE={}", env_or_unset("TEBAKO_JAIL_SOURCE"));
    println!("JAIL-JOURNAL={}", env_or_unset("TEBAKO_JAIL_JOURNAL"));
    // The spawned-edge probe's contract (tests/compose.rs): UNSET when
    // the loader exported no spawn lock (spec 30 §3).
    println!("SPAWN-LOCK={}", env_or_unset("TEBAKO_SPAWN_LOCK"));
    for (i, arg) in std::env::args().skip(1).enumerate() {
        println!("argv[{i}]={arg}");
    }
}

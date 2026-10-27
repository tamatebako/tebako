//! tebako-bench-child — the golden-test measured child for the spec 27
//! sampler/run-engine tests. TEST TOOLING inside the never-shipped
//! tebako-bench crate (the crate-root boundary comment applies): it exists
//! so the golden tests exercise one identical child on every triplet —
//! no `sleep`/`busybox`/PowerShell flavor divergence, no platform tool at
//! all (the harness's no-shell-out uniformity rule, spec 27 §0).
//!
//! ```text
//! tebako-bench-child [--sleep-ms N] [--busy-ms N] [--alloc-mb N]
//!                    [--spawn-busy-ms N] [--spawn-alloc-mb N]
//!                    [--touch PATH]... [--print TEXT] [--print-env VAR]
//!                    [--exit N]
//! ```
//!
//! Order of operations: spawn-launcher → alloc → busy → sleep →
//! touch/print → exit.

use std::time::{Duration, Instant};

fn take_value(args: &[String], i: &mut usize, flag: &str) -> String {
    match args.get(*i + 1) {
        Some(v) => {
            *i += 1;
            v.clone()
        }
        None => {
            eprintln!("tebako-bench-child: missing value for {flag}");
            std::process::exit(64);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut sleep_ms = 0u64;
    let mut busy_ms = 0u64;
    let mut alloc_mb = 0usize;
    let mut touches: Vec<String> = Vec::new();
    let mut prints: Vec<String> = Vec::new();
    let mut print_envs: Vec<String> = Vec::new();
    let mut exit = 0i32;
    let mut spawn_busy_ms: Option<u64> = None;
    let mut spawn_alloc_mb = 0usize;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sleep-ms" => sleep_ms = take_value(&args, &mut i, "--sleep-ms").parse().unwrap_or(0),
            "--busy-ms" => busy_ms = take_value(&args, &mut i, "--busy-ms").parse().unwrap_or(0),
            "--alloc-mb" => alloc_mb = take_value(&args, &mut i, "--alloc-mb").parse().unwrap_or(0),
            "--spawn-busy-ms" => {
                spawn_busy_ms = Some(
                    take_value(&args, &mut i, "--spawn-busy-ms")
                        .parse()
                        .unwrap_or(0),
                )
            }
            "--spawn-alloc-mb" => {
                spawn_alloc_mb = take_value(&args, &mut i, "--spawn-alloc-mb")
                    .parse()
                    .unwrap_or(0)
            }
            "--touch" => touches.push(take_value(&args, &mut i, "--touch")),
            "--print" => prints.push(take_value(&args, &mut i, "--print")),
            "--print-env" => print_envs.push(take_value(&args, &mut i, "--print-env")),
            "--exit" => exit = take_value(&args, &mut i, "--exit").parse().unwrap_or(0),
            other => {
                eprintln!("tebako-bench-child: unknown flag {other}");
                std::process::exit(64);
            }
        }
        i += 1;
    }

    // Launcher mode (tebako#927's shape): do none of the work here —
    // spawn a grandchild that does it, wait, and mirror its status. A
    // sampler that measures this process alone reads milliseconds.
    if let Some(ms) = spawn_busy_ms {
        let exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("tebako-bench-child: cannot locate itself to spawn: {e}");
                std::process::exit(74);
            }
        };
        let status = std::process::Command::new(exe)
            .arg("--busy-ms")
            .arg(ms.to_string())
            .arg("--alloc-mb")
            .arg(spawn_alloc_mb.to_string())
            .status();
        match status {
            Ok(s) => std::process::exit(s.code().unwrap_or(-1)),
            Err(e) => {
                eprintln!("tebako-bench-child: cannot spawn its grandchild: {e}");
                std::process::exit(74);
            }
        }
    }

    // Allocated and page-touched, then HELD until the end (the peak-RSS
    // golden test reads ru_maxrss / PeakWorkingSetSize — a freed-before-
    // exit allocation still counts against the high-water mark, but
    // holding it keeps the intent obvious).
    let mut held: Vec<Vec<u8>> = Vec::new();
    for _ in 0..alloc_mb {
        let mut block = vec![0u8; 1024 * 1024];
        for page in block.chunks_mut(4096) {
            page[0] = 1;
        }
        held.push(block);
    }

    if busy_ms > 0 {
        let deadline = Instant::now() + Duration::from_millis(busy_ms);
        let mut acc = 0u64;
        while Instant::now() < deadline {
            acc = acc.wrapping_add(1);
        }
        std::hint::black_box(acc);
    }

    if sleep_ms > 0 {
        std::thread::sleep(Duration::from_millis(sleep_ms));
    }

    for path in &touches {
        if let Err(e) = std::fs::write(path, b"tebako-bench-child\n") {
            eprintln!("tebako-bench-child: cannot touch {path}: {e}");
            std::process::exit(74);
        }
    }
    for text in &prints {
        println!("{text}");
        eprintln!("{text}");
    }
    for var in &print_envs {
        println!("{}={}", var, std::env::var(var).unwrap_or_default());
    }

    std::hint::black_box(&held);
    std::process::exit(exit);
}

// rust-tool: the dynamic-Rust-binary proof (roadmap 39). std::fs calls
// ride the interposed libc family on linux-gnu (stat/open64/readdir
// wrappers) and libSystem (macOS) — the shim must serve in-image paths
// with no extraction, exactly like the C fixtures.
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: rust-tool <read|metadata|read_dir> <path>");
        exit(64);
    }
    match args[1].as_str() {
        "read" => match std::fs::read_to_string(&args[2]) {
            Ok(text) => print!("{text}"),
            Err(e) => {
                eprintln!("read {}: {e}", args[2]);
                exit(e.raw_os_error().unwrap_or(1));
            }
        },
        "metadata" => match std::fs::metadata(&args[2]) {
            Ok(md) => println!("SIZE:{}", md.len()),
            Err(e) => {
                eprintln!("metadata {}: {e}", args[2]);
                exit(e.raw_os_error().unwrap_or(1));
            }
        },
        "read_dir" => match std::fs::read_dir(&args[2]) {
            Ok(rd) => {
                let mut names: Vec<String> = rd
                    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                println!("{}", names.join(" "));
            }
            Err(e) => {
                eprintln!("read_dir {}: {e}", args[2]);
                exit(e.raw_os_error().unwrap_or(1));
            }
        },
        other => {
            eprintln!("rust-tool: unknown command {other}");
            exit(64);
        }
    }
}

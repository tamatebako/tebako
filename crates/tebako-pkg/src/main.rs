//! tebako-pkg — the tebako package (tpkg) trailer surgery CLI.
//!
//! Subcommands (the C++ tebakofs package ops, item 25's scoping: TPKG
//! TRAILER operations ONLY — generic image ops belong to the future
//! tfs-cli):
//!
//! ```text
//! tebako-pkg info <archive>
//! tebako-pkg bundle --bootstrap <exe> --image <img[:mountpoint]>... -o <file>
//!                    [--runtime-ref <ref>] [--lean] [--launcher-abi <n>]
//! tebako-pkg unbundle <binary> -o <dir>
//! tebako-pkg reassemble <dir> -o <file>
//! tebako-pkg insert-image <binary> <img[:mountpoint]>
//! tebako-pkg remove-image <binary> <slot>
//! tebako-pkg set-runtime <binary> <runtime-file>
//! ```
//!
//! Exit codes: 0 success, 1 any error. Errors print
//! `Error: <cmd> failed: <message>` to stderr (matching the C++ tool).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tebako_pkg::{
    bundle, default_mount, info, insert_image, parse_image_spec, reassemble, remove_image,
    set_runtime, unbundle, PackageImage, PackageOptions,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return ExitCode::SUCCESS;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];
    match cmd {
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        "info" => cmd_info(rest),
        "bundle" => cmd_bundle(rest),
        "unbundle" => cmd_unbundle(rest),
        "reassemble" => cmd_reassemble(rest),
        "insert-image" => cmd_insert_image(rest),
        "remove-image" => cmd_remove_image(rest),
        "set-runtime" => cmd_set_runtime(rest),
        other => {
            eprintln!("Error: Unknown command: {other}");
            eprintln!("Use 'tebako-pkg help' for usage information");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------
// Tiny flag parser (argtable-compatible for the flags we support:
// "--name value", "--name=value", "-o value", "-o=value", "-v")
// ---------------------------------------------------------------------

#[derive(Default)]
struct Args {
    positional: Vec<String>,
    bootstrap: Option<String>,
    images: Vec<String>,
    output: Option<String>,
    runtime_ref: Option<String>,
    lean: bool,
    launcher_abi: Option<i64>,
    verbose: bool,
}

impl Args {
    fn parse(rest: &[String]) -> Result<Args, String> {
        let mut a = Args::default();
        let mut i = 0;
        while i < rest.len() {
            let arg = rest[i].as_str();
            let (name, mut value): (&str, Option<String>) = match arg.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (arg, None),
            };
            let mut take_value = |i: &mut usize| -> Result<String, String> {
                if let Some(v) = value.take() {
                    return Ok(v);
                }
                *i += 1;
                rest.get(*i)
                    .cloned()
                    .ok_or_else(|| format!("missing value for {name}"))
            };
            match name {
                "-v" | "--verbose" => a.verbose = true,
                "--lean" => a.lean = true,
                "--bootstrap" => a.bootstrap = Some(take_value(&mut i)?),
                "--image" => a.images.push(take_value(&mut i)?),
                "-o" | "--output" => a.output = Some(take_value(&mut i)?),
                "--runtime-ref" => a.runtime_ref = Some(take_value(&mut i)?),
                "--launcher-abi" => {
                    let v = take_value(&mut i)?;
                    a.launcher_abi = Some(
                        v.parse()
                            .map_err(|_| format!("invalid --launcher-abi value: {v}"))?,
                    );
                }
                _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
                _ => a.positional.push(arg.to_string()),
            }
            i += 1;
        }
        Ok(a)
    }

    fn need_positional(&self, n: usize, usage: &str) -> Result<(), String> {
        if self.positional.len() != n {
            return Err(format!("wrong number of arguments\nusage: {usage}"));
        }
        Ok(())
    }

    fn need_output(&self) -> Result<&String, String> {
        self.output
            .as_ref()
            .ok_or_else(|| "missing required option --output".to_string())
    }
}

fn fail(cmd: &str, msg: &str) -> ExitCode {
    eprintln!("Error: {cmd} failed: {msg}");
    ExitCode::FAILURE
}

// ---------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------

fn cmd_info(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("info", &e),
    };
    if let Err(e) = a.need_positional(1, "tebako-pkg info <archive>") {
        return fail("info", &e);
    }
    match info(Path::new(&a.positional[0])) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_bundle(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("bundle", &e),
    };
    if !a.positional.is_empty() {
        return fail("bundle", "unexpected positional arguments");
    }
    let output = match a.need_output() {
        Ok(o) => o.clone(),
        Err(e) => return fail("bundle", &e),
    };
    if a.images.is_empty() {
        return fail("bundle", "missing required option --image");
    }
    let Some(bootstrap) = a.bootstrap else {
        return fail("bundle", "missing required option --bootstrap");
    };
    let images: Vec<PackageImage> = a.images.iter().map(|s| parse_image_spec(s)).collect();
    let opts = PackageOptions {
        runtime_ref: a.runtime_ref.unwrap_or_default(),
        package_flags: if a.lean { tpkg::TPKG_FLAG_LEAN } else { 0 },
        launcher_abi: a
            .launcher_abi
            .map_or(0, |n| if n < 0 { 0 } else { n as u32 }),
    };
    match bundle(Path::new(&bootstrap), &images, Path::new(&output), &opts) {
        Ok(()) => {
            if a.verbose {
                println!("Wrote package: {output} ({} image slot(s))", images.len());
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("bundle", &e),
    }
}

fn cmd_unbundle(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("unbundle", &e),
    };
    if let Err(e) = a.need_positional(1, "tebako-pkg unbundle <binary> --output <dir>") {
        return fail("unbundle", &e);
    }
    let output = match a.need_output() {
        Ok(o) => o,
        Err(e) => return fail("unbundle", &e),
    };
    let binary = &a.positional[0];
    match unbundle(Path::new(binary), Path::new(output)) {
        Ok(()) => {
            if a.verbose {
                println!("Unbundled {binary} into: {output}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("unbundle", &e),
    }
}

fn cmd_reassemble(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("reassemble", &e),
    };
    if let Err(e) = a.need_positional(1, "tebako-pkg reassemble <dir> --output <file>") {
        return fail("reassemble", &e);
    }
    let output = match a.need_output() {
        Ok(o) => o,
        Err(e) => return fail("reassemble", &e),
    };
    let dir = &a.positional[0];
    match reassemble(Path::new(dir), Path::new(output)) {
        Ok(()) => {
            if a.verbose {
                println!("Reassembled {dir} into: {output}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("reassemble", &e),
    }
}

fn cmd_insert_image(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("insert-image", &e),
    };
    if let Err(e) = a.need_positional(2, "tebako-pkg insert-image <binary> <img[:mountpoint]>") {
        return fail("insert-image", &e);
    }
    let binary = &a.positional[0];
    let image = parse_image_spec(&a.positional[1]);
    match insert_image(Path::new(binary), &image.path, &image.mount_point) {
        Ok(()) => {
            if a.verbose {
                println!("Inserted {} into: {binary}", image.path.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("insert-image", &e),
    }
}

fn cmd_remove_image(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("remove-image", &e),
    };
    if let Err(e) = a.need_positional(2, "tebako-pkg remove-image <binary> <slot>") {
        return fail("remove-image", &e);
    }
    let binary = &a.positional[0];
    let slot: u32 = match a.positional[1].parse() {
        Ok(s) => s,
        Err(_) => {
            return fail(
                "remove-image",
                &format!("invalid slot index: {}", a.positional[1]),
            )
        }
    };
    match remove_image(Path::new(binary), slot) {
        Ok(()) => {
            if a.verbose {
                println!("Removed slot {slot} from: {binary}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("remove-image", &e),
    }
}

fn cmd_set_runtime(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("set-runtime", &e),
    };
    if let Err(e) = a.need_positional(2, "tebako-pkg set-runtime <binary> <runtime-file>") {
        return fail("set-runtime", &e);
    }
    let binary = &a.positional[0];
    let runtime = &a.positional[1];
    match set_runtime(Path::new(binary), Path::new(runtime)) {
        Ok(()) => {
            if a.verbose {
                println!("Replaced the bootstrap portion of: {binary}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail("set-runtime", &e),
    }
}

fn print_help() {
    println!("tebako-pkg - tebako package (tpkg) trailer surgery\n");
    println!("Usage: tebako-pkg <command> [options]\n");
    println!("Commands:");
    println!("  info          Dump a three-part package trailer (or archive summary)");
    println!("  bundle        Assemble a three-part package (bootstrap + images + trailer)");
    println!("  unbundle      Decompose a three-part package into a directory");
    println!("  reassemble    Rebuild a binary from an unbundled directory");
    println!("  insert-image  Append an image slot to a package (in place)");
    println!("  remove-image  Remove an image slot from a package (in place)");
    println!("  set-runtime   Replace the bootstrap portion of a package (in place)");
    println!("  help          Show this help\n");
    println!("Options vary per command; the default mountpoint for image slot 0 is");
    println!("{} (slot N: {}).", default_mount(0), default_mount(1));
}

// Keep PathBuf import used (some signatures may evolve).
#[allow(unused)]
fn _unused(_: PathBuf) {}

//! tfs — the generic VFS image tool (item 25's tfs : libtfs ::
//! sqlite3 : libsqlite3).
//!
//! ```text
//! tfs info [-v] [--json] <image>
//! tfs ls [-r|--recursive] [-l|--long] [-v] [-q|--quiet] <image> [path]
//! tfs tree [-v] <image> [path]
//! tfs cat [-v] <image> <file>
//! tfs stat [-v] <image> <path>
//! tfs extract [-v] [-q|--quiet] [-d|--dest <dir>] <image> [files...]
//! tfs find [-v] <image> <pattern>
//! tfs mkimage --format dwarfs <srcdir> -o <img> [-v]
//! ```
//!
//! Exit codes: 0 success, 1 any error. Package (tpkg trailer) operations
//! live in tebako-pkg, not here.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tfs_cli::{
    cmd_cat, cmd_extract, cmd_find, cmd_info, cmd_info_json, cmd_ls, cmd_mkimage, cmd_stat,
    cmd_tree, ExtractOptions, ListOptions,
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
        "info" => cmd_info_main(rest),
        "ls" => cmd_ls_main(rest),
        "tree" => cmd_tree_main(rest),
        "cat" => cmd_cat_main(rest),
        "stat" => cmd_stat_main(rest),
        "extract" => cmd_extract_main(rest),
        "find" => cmd_find_main(rest),
        "mkimage" => cmd_mkimage_main(rest),
        other => {
            eprintln!("Error: Unknown command: {other}");
            eprintln!("Use 'tfs help' for usage information");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------
// Tiny flag parser (same shapes as tebako-pkg's)
// ---------------------------------------------------------------------

#[derive(Default)]
struct Args {
    positional: Vec<String>,
    recursive: bool,
    long_format: bool,
    verbose: bool,
    quiet: bool,
    json: bool,
    dest: Option<String>,
    output: Option<String>,
    format: Option<String>,
    mkdwarfs: Option<String>,
}

impl Args {
    fn parse(rest: &[String]) -> Result<Args, String> {
        let mut a = Args::default();
        let mut i = 0;
        while i < rest.len() {
            let arg = rest[i].as_str();
            // Combined single-char flags: "-rl" == "-r -l" (argtable style).
            if arg.len() > 2 && arg.starts_with('-') && !arg.starts_with("--") {
                let mut chars = arg[1..].chars().peekable();
                let mut ok = true;
                while let Some(c) = chars.next() {
                    match c {
                        'r' => a.recursive = true,
                        'l' => a.long_format = true,
                        'v' => a.verbose = true,
                        'q' => a.quiet = true,
                        'd' | 'o' => {
                            // value-taking: the remainder of the token, else
                            // the next argument.
                            let rem: String = chars.collect();
                            let v = if !rem.is_empty() {
                                rem
                            } else {
                                i += 1;
                                rest.get(i).cloned().ok_or("missing value")?
                            };
                            if c == 'd' {
                                a.dest = Some(v);
                            } else {
                                a.output = Some(v);
                            }
                            break;
                        }
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    return Err(format!("unknown option: {arg}"));
                }
                i += 1;
                continue;
            }
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
                "-r" | "--recursive" => a.recursive = true,
                "-l" | "--long" => a.long_format = true,
                "-v" | "--verbose" => a.verbose = true,
                "-q" | "--quiet" => a.quiet = true,
                "--json" => a.json = true,
                "-d" | "--dest" => a.dest = Some(take_value(&mut i)?),
                "-o" | "--output" => a.output = Some(take_value(&mut i)?),
                "--format" => a.format = Some(take_value(&mut i)?),
                "--mkdwarfs" => a.mkdwarfs = Some(take_value(&mut i)?),
                _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
                _ => a.positional.push(arg.to_string()),
            }
            i += 1;
        }
        Ok(a)
    }

    fn positional_count(&self, min: usize, max: usize, usage: &str) -> Result<(), String> {
        if self.positional.len() < min || self.positional.len() > max {
            return Err(format!("wrong number of arguments\nusage: {usage}"));
        }
        Ok(())
    }
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::FAILURE
}

// ---------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------

fn cmd_info_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 1, "tfs info [--json] <image>") {
        return fail(&format!("Error: {e}"));
    }
    let image = Path::new(&a.positional[0]);
    let result = if a.json {
        cmd_info_json(image)
    } else {
        cmd_info(image)
    };
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_ls_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 2, "tfs ls [options] <image> [path]") {
        return fail(&format!("Error: {e}"));
    }
    let opts = ListOptions {
        recursive: a.recursive,
        long_format: a.long_format,
        verbose: a.verbose,
        quiet: a.quiet,
    };
    let path = a.positional.get(1).map_or("/", String::as_str);
    match cmd_ls(Path::new(&a.positional[0]), path, &opts) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_tree_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 2, "tfs tree [-v] <image> [path]") {
        return fail(&format!("Error: {e}"));
    }
    let path = a.positional.get(1).map_or("/", String::as_str);
    match cmd_tree(Path::new(&a.positional[0]), path) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_cat_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(2, 2, "tfs cat [-v] <image> <file>") {
        return fail(&format!("Error: {e}"));
    }
    let mut out = std::io::stdout().lock();
    match cmd_cat(Path::new(&a.positional[0]), &a.positional[1], &mut out) {
        Ok(()) => ExitCode::SUCCESS,
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_stat_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(2, 2, "tfs stat [-v] <image> <path>") {
        return fail(&format!("Error: {e}"));
    }
    match cmd_stat(Path::new(&a.positional[0]), &a.positional[1]) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_extract_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if a.positional.is_empty() {
        return fail(
            "Error: wrong number of arguments\nusage: tfs extract [options] <image> [files...]",
        );
    }
    let opts = ExtractOptions {
        dest_dir: a.dest.map_or_else(|| PathBuf::from("."), PathBuf::from),
        verbose: a.verbose,
        quiet: a.quiet,
    };
    let files: Vec<String> = a.positional[1..].to_vec();
    let (out, err, rc) = cmd_extract(Path::new(&a.positional[0]), &files, &opts);
    print!("{out}");
    eprint!("{err}");
    ExitCode::from(rc as u8)
}

fn cmd_find_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(2, 2, "tfs find [-v] <image> <pattern>") {
        return fail(&format!("Error: {e}"));
    }
    match cmd_find(Path::new(&a.positional[0]), &a.positional[1]) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_mkimage_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 1, "tfs mkimage --format dwarfs <srcdir> --output <img>")
    {
        return fail(&format!("Error: {e}"));
    }
    let Some(format) = a.format else {
        return fail("Error: missing required option --format");
    };
    let Some(output) = a.output else {
        return fail("Error: missing required option --output");
    };
    match cmd_mkimage(
        &format,
        Path::new(&a.positional[0]),
        Path::new(&output),
        a.mkdwarfs.as_deref(),
    ) {
        Ok(()) => {
            if a.verbose {
                println!("Wrote {} image: {output}", format.to_lowercase());
            }
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("Error: mkimage failed: {msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn print_help() {
    println!("tfs - generic VFS image tool (tebako)\n");
    println!("Usage: tfs <command> [options]\n");
    println!("Commands:");
    println!("  info     Show image information (--json for backend metadata JSON)");
    println!("  ls       List directory contents (-r recursive, -l long)");
    println!("  tree     Show directory tree");
    println!("  cat      Display file contents");
    println!("  stat     Show file/directory metadata");
    println!("  extract  Extract archive contents (-d dest, default .)");
    println!("  find     Search for files by name glob");
    println!("  mkimage  Create a dwarfs image from a directory (wraps mkdwarfs)");
    println!("  help     Show help\n");
    println!("Package (tpkg trailer) operations live in tebako-pkg.");
}

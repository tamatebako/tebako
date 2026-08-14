//! tfs — the generic VFS image tool (item 25's tfs : libtfs ::
//! sqlite3 : libsqlite3).
//!
//! ```text
//! tfs info [-v] [--manifest] [--provides] [--requires] [--platforms]
//!          [--json] [--backend-json] [--verify] [--require-signed] <image>
//! tfs ls [-r|--recursive] [-l|--long] [-v] [-q|--quiet] <image> [path]
//! tfs tree [-v] <image> [path]
//! tfs cat [-v] <image> <file>
//! tfs stat [-v] <image> <path>
//! tfs extract [-v] [-q|--quiet] [-d|--dest <dir>] <image> [files...]
//! tfs find [-v] <image> <pattern>
//! tfs mkimage --format dwarfs|limnifs <srcdir> -o <img> [-v]
//! tfs exec <image>[:mount] [--image <image:mount>]...
//!          [--jail <spec> | --compose <file.yaml>] -- <cmd> [args...]
//! tfs needs --from-journal <journal.log>
//! tfs encrypt <image> -o <img> --recipient <pubkey>... [--subtree <path>=<pubkey>]...
//! tfs encrypt <image> -o <img> --rewrap --key <secret> --recipient <pubkey>...
//! tfs decrypt <image> -o <out.tar> --key <secret>
//! tfs mount <image> --key <secret>
//! ```
//!
//! The flag-less `tfs info` output is the C++ parity summary (unchanged);
//! every richer view is an explicit spec-15 flag. Exit codes: 0 success,
//! 1 any error, and the spec-15 §5 codes (65/70/71/72) under --verify.
//! Package (tpkg trailer) operations live in tebako-pkg, not here.
//! Encryption (spec 10) is opt-in: it happens only through the explicit
//! encrypt/decrypt/mount verbs.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tfs_cli::enc::{
    cmd_decrypt, cmd_encrypt, cmd_mount_enc, cmd_rewrap, EncryptOptions, SubtreeGrant,
};
use tfs_cli::{
    cmd_cat, cmd_exec, cmd_extract, cmd_find, cmd_info, cmd_info_json, cmd_info_rich, cmd_ls,
    cmd_mkimage, cmd_needs_from_journal, cmd_stat, cmd_tree, ExecOptions, ExtractOptions,
    InfoOptions, ListOptions,
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
        "exec" => cmd_exec_main(rest),
        "needs" => cmd_needs_main(rest),
        "encrypt" => cmd_encrypt_main(rest),
        "decrypt" => cmd_decrypt_main(rest),
        "mount" => cmd_mount_main(rest),
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
    manifest: bool,
    provides: bool,
    requires: bool,
    platforms: bool,
    backend_json: bool,
    verify: bool,
    require_signed: bool,
    recipients: Vec<String>,
    key: Option<String>,
    subtrees: Vec<String>,
    rewrap: bool,
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
                "--manifest" => a.manifest = true,
                "--provides" => a.provides = true,
                "--requires" => a.requires = true,
                "--platforms" => a.platforms = true,
                "--backend-json" => a.backend_json = true,
                "--verify" => a.verify = true,
                "--require-signed" => a.require_signed = true,
                "--rewrap" => a.rewrap = true,
                "--recipient" => a.recipients.push(take_value(&mut i)?),
                "--key" => a.key = Some(take_value(&mut i)?),
                "--subtree" => a.subtrees.push(take_value(&mut i)?),
                "-d" | "--dest" => a.dest = Some(take_value(&mut i)?),
                "-o" | "--output" => a.output = Some(take_value(&mut i)?),
                "--format" => a.format = Some(take_value(&mut i)?),
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
    if let Err(e) = a.positional_count(
        1,
        1,
        "tfs info [--manifest|--provides|--requires|--platforms|--json|--backend-json|--verify] <image>",
    ) {
        return fail(&format!("Error: {e}"));
    }
    let image = Path::new(&a.positional[0]);
    let opts = InfoOptions {
        manifest: a.manifest,
        provides: a.provides,
        requires: a.requires,
        platforms: a.platforms,
        json: a.json,
        backend_json: a.backend_json,
        verify: a.verify,
        require_signed: a.require_signed,
    };
    if opts.require_signed && !opts.verify {
        return fail("Error: --require-signed only applies with --verify");
    }
    // --backend-json alone is the legacy backend metadata dump (pre-spec-15
    // `--json`); every richer view goes through the spec-15 surface.
    if opts.backend_json && !opts.any_rich() {
        return match cmd_info_json(image) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err((msg, rc)) => {
                eprintln!("{msg}");
                ExitCode::from(rc as u8)
            }
        };
    }
    if !opts.any_rich() {
        return match cmd_info(image) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err((msg, rc)) => {
                eprintln!("{msg}");
                ExitCode::from(rc as u8)
            }
        };
    }
    match cmd_info_rich(image, &opts) {
        Ok((text, code)) => {
            print!("{text}");
            ExitCode::from(code as u8)
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
    if let Err(e) = a.positional_count(
        1,
        1,
        "tfs mkimage --format dwarfs|limnifs <srcdir> --output <img>",
    ) {
        return fail(&format!("Error: {e}"));
    }
    let Some(format) = a.format else {
        return fail("Error: missing required option --format");
    };
    let Some(output) = a.output else {
        return fail("Error: missing required option --output");
    };
    match cmd_mkimage(&format, Path::new(&a.positional[0]), Path::new(&output)) {
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

// ---------------------------------------------------------------------
// exec (spec 07 §8 tier 1)
// ---------------------------------------------------------------------

/// `tfs exec <image>[:mount] [--image <image:mount>]... [--jail <spec> | --compose <file.yaml>] --
/// <cmd> [args...]` — everything after `--` is the command, verbatim (the
/// generic flag parser must never see the command's own flags).
fn cmd_exec_main(rest: &[String]) -> ExitCode {
    const USAGE: &str =
        "tfs exec <image>[:mount] [--image <image:mount>]... [--jail <spec> | --compose <file.yaml>] -- <cmd> [args...]";
    let Some(sep) = rest.iter().position(|a| a == "--") else {
        return fail(&format!(
            "Error: tfs exec requires `--` before the command\nusage: {USAGE}"
        ));
    };
    let (ours, cmd) = (&rest[..sep], &rest[sep + 1..]);
    if cmd.is_empty() {
        return fail(&format!(
            "Error: missing command after `--`\nusage: {USAGE}"
        ));
    }
    let mut images: Vec<String> = Vec::new();
    let mut jail: Option<String> = None;
    let mut compose: Option<String> = None;
    let mut i = 0;
    while i < ours.len() {
        let arg = ours[i].as_str();
        let (name, mut value) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg, None),
        };
        let mut take_value = |i: &mut usize| -> Result<String, String> {
            if let Some(v) = value.take() {
                return Ok(v);
            }
            *i += 1;
            ours.get(*i)
                .cloned()
                .ok_or_else(|| format!("missing value for {name}"))
        };
        match name {
            "--image" => images.push(match take_value(&mut i) {
                Ok(v) => v,
                Err(e) => return fail(&format!("Error: {e}")),
            }),
            "--jail" => match take_value(&mut i) {
                Ok(v) => jail = Some(v),
                Err(e) => return fail(&format!("Error: {e}")),
            },
            "--compose" => match take_value(&mut i) {
                Ok(v) => compose = Some(v),
                Err(e) => return fail(&format!("Error: {e}")),
            },
            _ if arg.starts_with('-') => {
                return fail(&format!("Error: unknown option: {arg}\nusage: {USAGE}"));
            }
            _ => images.push(arg.to_string()),
        }
        i += 1;
    }
    if images.is_empty() && compose.is_none() {
        return fail(&format!("Error: missing image\nusage: {USAGE}"));
    }
    let opts = ExecOptions {
        images,
        jail,
        compose,
        cmd: cmd.to_vec(),
    };
    match cmd_exec(&opts) {
        // Unreachable on unix (exec replaces us); the not-unix port errors.
        Ok(()) => ExitCode::SUCCESS,
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_encrypt_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(
        1,
        1,
        "tfs encrypt <image> -o <img> --recipient <pubkey>... [--subtree <path>=<pubkey>]...\n       tfs encrypt <image> -o <img> --rewrap --key <secret> --recipient <pubkey>...",
    ) {
        return fail(&format!("Error: {e}"));
    }
    let Some(output) = a.output else {
        return fail("Error: missing required option --output");
    };
    let src = Path::new(&a.positional[0]);
    let out = Path::new(&output);
    let recipients: Vec<PathBuf> = a.recipients.iter().map(PathBuf::from).collect();
    let result = if a.rewrap {
        if !a.subtrees.is_empty() {
            return fail("Error: --subtree does not apply to --rewrap");
        }
        let Some(key) = a.key else {
            return fail("Error: --rewrap requires --key <secret-key-file>");
        };
        cmd_rewrap(src, out, Path::new(&key), &recipients)
    } else {
        if a.key.is_some() {
            return fail("Error: --key only applies to --rewrap / decrypt / mount");
        }
        let mut subtrees = Vec::new();
        for s in &a.subtrees {
            let Some((path, pubkey)) = s.split_once('=') else {
                return fail("Error: --subtree expects <absolute-path>=<pubkey-file>");
            };
            subtrees.push(SubtreeGrant {
                path: path.to_string(),
                public_key: PathBuf::from(pubkey),
            });
        }
        cmd_encrypt(
            src,
            out,
            &EncryptOptions {
                recipients,
                subtrees,
            },
        )
    };
    match result {
        Ok(()) => {
            if a.verbose {
                println!("Wrote encrypted image: {output}");
            }
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("Error: {msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_decrypt_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 1, "tfs decrypt <image> -o <out.tar> --key <secret>") {
        return fail(&format!("Error: {e}"));
    }
    let Some(output) = a.output else {
        return fail("Error: missing required option --output");
    };
    let Some(key) = a.key else {
        return fail("Error: decrypt requires --key <secret-key-file>");
    };
    match cmd_decrypt(
        Path::new(&a.positional[0]),
        Path::new(&output),
        Path::new(&key),
    ) {
        Ok(()) => {
            if a.verbose {
                println!("Wrote plaintext tar: {output}");
            }
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("Error: {msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn cmd_mount_main(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail(&format!("Error: {e}")),
    };
    if let Err(e) = a.positional_count(1, 1, "tfs mount <image> --key <secret>") {
        return fail(&format!("Error: {e}"));
    }
    let Some(key) = a.key else {
        return fail("Error: mount requires --key <secret-key-file> (encrypted images open only with a recipient key)");
    };
    match cmd_mount_enc(Path::new(&a.positional[0]), Path::new(&key)) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("Error: {msg}");
            ExitCode::from(rc as u8)
        }
    }
}

/// `tfs needs --from-journal <journal.log>` — draft a payload `needs:`
/// block from a record-mode journal (spec 23 §8: the "perm all and
/// monitor" half of the workflow).
fn cmd_needs_main(rest: &[String]) -> ExitCode {
    const USAGE: &str = "tfs needs --from-journal <journal.log>";
    let mut journal: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i].as_str();
        let (name, mut value) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg, None),
        };
        match name {
            "--from-journal" => {
                journal = match value.take() {
                    Some(v) => Some(v),
                    None => {
                        i += 1;
                        match rest.get(i) {
                            Some(v) => Some(v.clone()),
                            None => return fail("Error: missing value for --from-journal"),
                        }
                    }
                };
            }
            _ => return fail(&format!("Error: unknown option: {arg}\nusage: {USAGE}")),
        }
        i += 1;
    }
    let Some(journal) = journal else {
        return fail(&format!("Error: missing --from-journal\nusage: {USAGE}"));
    };
    match cmd_needs_from_journal(&journal) {
        Ok(yaml) => {
            print!("{yaml}");
            ExitCode::SUCCESS
        }
        Err((msg, rc)) => {
            eprintln!("{msg}");
            ExitCode::from(rc as u8)
        }
    }
}

fn print_help() {
    println!("tfs - generic VFS image tool (tebako)\n");
    println!("Usage: tfs <command> [options]\n");
    println!("Commands:");
    println!("  info     Show image information (--json for the info document;");
    println!("           --manifest/--provides/--requires/--platforms for sections,");
    println!("           --backend-json for backend metadata, --verify to validate)");
    println!("  ls       List directory contents (-r recursive, -l long)");
    println!("  tree     Show directory tree");
    println!("  cat      Display file contents");
    println!("  stat     Show file/directory metadata");
    println!("  extract  Extract archive contents (-d dest, default .)");
    println!("  find     Search for files by name glob");
    println!(
        "  mkimage  Create a dwarfs or limnifs (.tfs) image from a directory (in-process writer)"
    );
    println!("  exec     Run a dynamic native command with the VFS injected (preload shim;");
    println!("           --compose <file.yaml> takes the whole composition, spec 23 §9)");
    println!("  needs    Draft a payload needs: block from a record-mode journal");
    println!("           (--from-journal <log>; spec 23 §8)");
    println!("  encrypt  Encrypt an image to recipients (-o, --recipient, --subtree;");
    println!("           --rewrap --key rotates grants without touching the bulk)");
    println!("  decrypt  Decrypt an image to a plaintext tar (-o, --key)");
    println!("  mount    Unlock an encrypted image with the recipient key (--key)");
    println!("  help     Show help\n");
    println!("Package (tpkg trailer) operations live in tebako-pkg.");
}

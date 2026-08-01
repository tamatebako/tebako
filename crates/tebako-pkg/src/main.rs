//! tebako-pkg — the tebako package (tpkg) trailer surgery CLI.
//!
//! Subcommands (the C++ tebakofs package ops, item 25's scoping: TPKG
//! TRAILER operations ONLY — generic image ops belong to the future
//! tfs-cli):
//!
//! ```text
//! tebako-pkg info [--full|--slot N|--json|--verify|--depth N|--require-signed] <archive>
//! tebako-pkg validate [--require-signed] <binary>
//! tebako-pkg bundle --bootstrap <exe> --image <img[:mountpoint]>... -o <file>
//!                    [--runtime-ref <ref>] [--lean] [--launcher-abi <n>]
//!                    [--package-manifest <file.yaml>]
//! tebako-pkg unbundle <binary> -o <dir>
//! tebako-pkg reassemble <dir> -o <file>
//! tebako-pkg insert-image <binary> <img[:mountpoint]>
//! tebako-pkg remove-image <binary> <slot>
//! tebako-pkg set-runtime <binary> <runtime-file>
//! ```
//!
//! Exit codes: 0 success, 1 any error; `info --verify` and `validate`
//! exit with the spec-15 §5 codes (0/65/70/71/72) plus 77 for the
//! spec-18 C6 contract gate (era-1 or era-mismatch refusal). Errors print
//! `Error: <cmd> failed: <message>` to stderr (matching the C++ tool).
//! The flag-less `info` output keeps byte-parity with the C++ oracle.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tebako_pkg::{
    bundle, default_mount, info, info_rich, insert_image, parse_image_spec, reassemble,
    remove_image, set_runtime, unbundle, validate, InfoOptions, PackageImage, PackageOptions,
    SignRequest,
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
        "validate" => cmd_validate(rest),
        "bundle" => cmd_bundle(rest),
        "unbundle" => cmd_unbundle(rest),
        "reassemble" => cmd_reassemble(rest),
        "insert-image" => cmd_insert_image(rest),
        "remove-image" => cmd_remove_image(rest),
        "set-runtime" => cmd_set_runtime(rest),
        "sign" => cmd_sign(rest),
        "verify" => cmd_verify(rest),
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
    package_manifest: Option<String>,
    lean: bool,
    launcher_abi: Option<i64>,
    verbose: bool,
    sign: Option<SignRequest>,
    key_file: Option<String>,
    no_sums: bool,
    full: bool,
    slot: Option<String>,
    json: bool,
    verify: bool,
    require_signed: bool,
    depth: Option<String>,
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
                "--full" => a.full = true,
                "--json" => a.json = true,
                "--verify" => a.verify = true,
                "--require-signed" => a.require_signed = true,
                "--slot" => a.slot = Some(take_value(&mut i)?),
                "--depth" => a.depth = Some(take_value(&mut i)?),
                "--sign" => {
                    // --sign (press-local key) or --sign=<keyid> (a secret
                    // key from $TEBAKO_HOME/keys). The space form is not
                    // consumed: a bare --sign never eats a positional.
                    a.sign = Some(match value.take() {
                        Some(keyid) => SignRequest::Keyid(keyid),
                        None => SignRequest::PressLocal,
                    });
                }
                "--bootstrap" => a.bootstrap = Some(take_value(&mut i)?),
                "--key-file" => a.key_file = Some(take_value(&mut i)?),
                "--no-sums" => a.no_sums = true,
                "--key" => {
                    let v = take_value(&mut i)?;
                    a.sign = Some(SignRequest::Keyid(v));
                }
                "--image" => a.images.push(take_value(&mut i)?),
                "-o" | "--output" => a.output = Some(take_value(&mut i)?),
                "--runtime-ref" => a.runtime_ref = Some(take_value(&mut i)?),
                "--package-manifest" => a.package_manifest = Some(take_value(&mut i)?),
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
    if let Err(e) = a.need_positional(
        1,
        "tebako-pkg info [--full|--slot N|--json|--verify|--depth N|--require-signed] <archive>",
    ) {
        return fail("info", &e);
    }
    let binary = Path::new(&a.positional[0]);
    let opts = match info_options(&a) {
        Ok(o) => o,
        Err(e) => return fail("info", &e),
    };
    // The spec-18 C6 contract gate rides the strict path (exit 77).
    if opts.verify {
        match tebako_pkg::check_contract(binary) {
            Ok(Some(e)) => {
                eprintln!("Error: {e}");
                return ExitCode::from(e.exit_code() as u8);
            }
            Ok(None) => {}
            Err(e) => return fail("info", &e),
        }
    }
    if opts.any_rich() {
        return match info_rich(binary, &opts) {
            Ok((text, code)) => {
                print!("{text}");
                ExitCode::from(code as u8)
            }
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::FAILURE
            }
        };
    }
    match info(binary) {
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

fn info_options(a: &Args) -> Result<InfoOptions, String> {
    if a.require_signed && !a.verify {
        return Err("--require-signed only applies with --verify".to_string());
    }
    let slot = match &a.slot {
        Some(text) => Some(
            text.parse::<u32>()
                .map_err(|_| format!("invalid --slot value: {text}"))?,
        ),
        None => None,
    };
    let depth = match &a.depth {
        Some(text) => {
            let d = text
                .parse::<u8>()
                .map_err(|_| format!("invalid --depth value: {text} (want 0, 1 or 2)"))?;
            if d > 2 {
                return Err(format!("invalid --depth value: {text} (want 0, 1 or 2)"));
            }
            Some(d)
        }
        None => None,
    };
    Ok(InfoOptions {
        full: a.full,
        slot,
        json: a.json,
        verify: a.verify,
        require_signed: a.require_signed,
        depth,
    })
}

fn cmd_validate(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("validate", &e),
    };
    if let Err(e) = a.need_positional(1, "tebako-pkg validate [--require-signed] <binary>") {
        return fail("validate", &e);
    }
    // The spec-18 C6 contract gate (fail-closed, exit 77): era-1 and
    // era-mismatch refusals are the typed ContractError's distinct paths.
    match tebako_pkg::check_contract(Path::new(&a.positional[0])) {
        Ok(Some(e)) => {
            eprintln!("Error: {e}");
            return ExitCode::from(e.exit_code() as u8);
        }
        Ok(None) => {}
        Err(e) => return fail("validate", &e),
    }
    match validate(Path::new(&a.positional[0]), a.require_signed) {
        Ok((text, code)) => {
            print!("{text}");
            ExitCode::from(code as u8)
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
    // The L2 package manifest (spec 03 §6): authored YAML, embedded as
    // extension block type 2 (spec 02 §5b).
    let package_manifest = match &a.package_manifest {
        Some(path) => {
            let text = match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    return fail(
                        "bundle",
                        &format!("cannot read the package manifest {path}: {e}"),
                    )
                }
            };
            match tpkg::PackageManifest::from_yaml(&text) {
                Ok(pm) => Some(pm),
                Err(e) => return fail("bundle", &format!("invalid package manifest {path}: {e}")),
            }
        }
        None => None,
    };
    let opts = PackageOptions {
        runtime_ref: a.runtime_ref.unwrap_or_default(),
        package_flags: if a.lean { tpkg::TPKG_FLAG_LEAN } else { 0 },
        launcher_abi: a
            .launcher_abi
            .map_or(0, |n| if n < 0 { 0 } else { n as u32 }),
        sign: a.sign.unwrap_or_default(),
        package_manifest,
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
    println!("  info          Dump a three-part package trailer (or archive summary);");
    println!("                --full container report, --slot N payload, --json document,");
    println!("                --verify strict checks, --depth 0|1|2 (spec 15)");
    println!("  validate      Strict package verification (exit 0/65/70/71/72/77)");
    println!("  bundle        Assemble a three-part package (bootstrap + images + trailer)");
    println!("  unbundle      Decompose a three-part package into a directory");
    println!("  reassemble    Rebuild a binary from an unbundled directory");
    println!("  insert-image  Append an image slot to a package (in place)");
    println!("  remove-image  Remove an image slot from a package (in place)");
    println!("  set-runtime   Replace the bootstrap portion of a package (in place)");
    println!("  sign          Sign artifacts (detached .asc per artifact + signed SHA256SUMS)");
    println!("  verify        Verify artifacts against the trusted keyring");
    println!("  help          Show this help\n");
    println!("Signing is OPT-IN: packages are unsigned unless `bundle --sign[=keyid]`");
    println!("is given (--sign uses the press-local key, generated on first use;");
    println!("--sign=<keyid> selects a secret key from $TEBAKO_HOME/keys). Rewrite");
    println!("operations preserve the input's signing state. Verification of signed");
    println!("packages at run time is always strict.");
    println!("`bundle --package-manifest <file.yaml>` embeds the L2 package manifest");
    println!("(ext block type 2, spec 02 §5b / spec 03 §6) — the press adds the spec-18");
    println!("contract declaration (contract_era/pressed_by/reader_era) to it; rewrites");
    println!("preserve extension blocks, and `info --full` prints the package section");
    println!("when present. `validate` / `info --verify` enforce the contract gate");
    println!("(exit 77: pre-era or era-mismatch refusal, spec 18 C6).");
    println!("Options vary per command; the default mountpoint for image slot 0 is");
    println!("{} (slot N: {}).", default_mount(0), default_mount(1));
}

// ---------------------------------------------------------------------
// sign / verify (release tooling: detached .asc per artifact + signed
// SHA256SUMS; consumers verify against the trusted keyring)
// ---------------------------------------------------------------------

fn resolve_signing_key(a: &Args, home: &Path) -> Result<tebako_signer::PressKey, String> {
    if let Some(path) = &a.key_file {
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read the key file {path}: {e}"))?;
        return tebako_signer::press_key_from_secret_bytes(&bytes).map_err(|e| e.to_string());
    }
    match &a.sign {
        Some(SignRequest::Keyid(keyid)) => tebako_signer::secret_key_by_keyid(home, keyid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!(
                    "no secret key with keyid {keyid} under {}",
                    home.join("keys").display()
                )
            }),
        _ => tebako_signer::press_local_key(home).map_err(|e| e.to_string()),
    }
}

fn sign_artifact(artifact: &Path, press: &tebako_signer::PressKey) -> Result<String, String> {
    let data = std::fs::read(artifact)
        .map_err(|_| format!("cannot read artifact: {}", artifact.display()))?;
    let sig = tebako_signer::sign_detached(&data, &press.secret_key, &press.fingerprint)
        .map_err(|e| e.to_string())?;
    let armored = rnp::armor_bytes(&sig, rnp::ops::ArmorType::Signature)
        .map_err(|e| format!("cannot armor the signature: {e}"))?;
    let asc = artifact.with_file_name(format!(
        "{}.asc",
        artifact
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    std::fs::write(&asc, &armored).map_err(|e| format!("cannot write {}: {e}", asc.display()))?;
    let digest = {
        use sha2::Digest;
        sha2::Sha256::digest(&data)
    };
    Ok(tebako_signer::hex_lower(&digest))
}

fn cmd_sign(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("sign", &e),
    };
    if a.positional.is_empty() {
        return fail(
            "sign",
            "usage: tebako-pkg sign [--key <keyid>|--key-file <path>] [--no-sums] <artifact...>",
        );
    }
    let home = match tebako_signer::default_home() {
        Ok(h) => h,
        Err(e) => return fail("sign", &e.to_string()),
    };
    let press = match resolve_signing_key(&a, &home) {
        Ok(k) => k,
        Err(e) => return fail("sign", &e),
    };
    if let Err(e) = tebako_signer::register_trusted(&home, &press.public_key) {
        return fail("sign", &e.to_string());
    }

    let mut entries = Vec::new();
    for artifact in &a.positional {
        let path = Path::new(artifact);
        match sign_artifact(path, &press) {
            Ok(digest) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                entries.push(format!("{digest}  {name}"));
            }
            Err(e) => return fail("sign", &e),
        }
    }

    if !a.no_sums {
        let sums = entries.join("\n") + "\n";
        let sums_path = Path::new("SHA256SUMS");
        if let Err(e) = std::fs::write(sums_path, &sums) {
            return fail("sign", &format!("cannot write SHA256SUMS: {e}"));
        }
        let sig = match tebako_signer::sign_detached(
            sums.as_bytes(),
            &press.secret_key,
            &press.fingerprint,
        ) {
            Ok(s) => s,
            Err(e) => return fail("sign", &e.to_string()),
        };
        let armored = match rnp::armor_bytes(&sig, rnp::ops::ArmorType::Signature) {
            Ok(s) => s,
            Err(e) => return fail("sign", &format!("cannot armor the signature: {e}")),
        };
        if let Err(e) = std::fs::write("SHA256SUMS.asc", &armored) {
            return fail("sign", &format!("cannot write SHA256SUMS.asc: {e}"));
        }
    }

    println!(
        "signed {} artifact(s) with key {}",
        a.positional.len(),
        press.keyid_hex()
    );
    ExitCode::SUCCESS
}

fn cmd_verify(rest: &[String]) -> ExitCode {
    let a = match Args::parse(rest) {
        Ok(a) => a,
        Err(e) => return fail("verify", &e),
    };
    if a.positional.is_empty() {
        return fail(
            "verify",
            "usage: tebako-pkg verify [--keyring <path>] <artifact...>",
        );
    }
    let keyring = match &a.key_file {
        Some(path) => match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return fail("verify", &format!("cannot read the keyring {path}: {e}")),
        },
        None => {
            let home = match tebako_signer::default_home() {
                Ok(h) => h,
                Err(e) => return fail("verify", &e.to_string()),
            };
            match tebako_signer::trusted_keyring_bytes(&home) {
                Ok(b) => b,
                Err(e) => return fail("verify", &e.to_string()),
            }
        }
    };

    let mut all_trusted = true;
    for artifact in &a.positional {
        let path = Path::new(artifact);
        let asc = path.with_file_name(format!(
            "{}.asc",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ));
        let (data, sig) = match (std::fs::read(path), std::fs::read(&asc)) {
            (Ok(d), Ok(s)) => (d, s),
            (Err(e), _) => return fail("verify", &format!("cannot read {}: {e}", path.display())),
            (_, Err(e)) => return fail("verify", &format!("cannot read {}: {e}", asc.display())),
        };
        let sig = match rnp::dearmor_bytes(&sig) {
            Ok(s) => s,
            Err(e) => return fail("verify", &format!("cannot dearmor {}: {e}", asc.display())),
        };
        match tebako_signer::verify_detached_full(&keyring, &data, &sig) {
            Ok(tebako_signer::VerifyOutcome::Trusted(_)) => {
                println!("{artifact}: trusted");
            }
            Ok(tebako_signer::VerifyOutcome::Untrusted(keyid)) => {
                all_trusted = false;
                let signer = tebako_signer::signature_issuer_fingerprint(&sig)
                    .unwrap_or_else(|_| keyid.clone());
                println!("{artifact}: UNTRUSTED (signer {signer} not in the keyring)");
            }
            Ok(tebako_signer::VerifyOutcome::Invalid(_)) => {
                all_trusted = false;
                println!("{artifact}: INVALID SIGNATURE");
            }
            Err(e) => return fail("verify", &e.to_string()),
        }
    }

    if all_trusted {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// Keep PathBuf import used (some signatures may evolve).
#[allow(unused)]
fn _unused(_: PathBuf) {}

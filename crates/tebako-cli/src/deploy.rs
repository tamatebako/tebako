//! Port of the gem's RuntimeDeployer (lib/tebako/runtime_deployer.rb):
//! executes deploy operations (gem/bundler installs) inside the resolved
//! prebuilt runtime.
//!
//! The prebuilt runtime packages are pressed in 'runtime' mode: their
//! compiled-in entry point is /local/stub.rb and their image carries a
//! full Ruby environment but no bin/ tooling. Deploy serializes the
//! operations into a driver script placed at /local/stub.rb of a
//! throwaway image, stitches it onto an empty base and execs the runtime
//! itself with that image (--tebako-image, the launcher ABI handoff).
//! The driver runs with the runtime's own Ruby and installs into the
//! packaging environment through absolute host paths.
//!
//! Native-extension deploy: when the runtime SDK was resolved (POSIX,
//! ops present — src/sdk.rs), the driver's build_overrides also point
//! rubyhdrdir/rubyarchhdrdir at the SDK header tree and LIBRUBYARG at
//! the SDK's symbol stub, and the cc_override re-resolves the recorded
//! toolchain against the press host — mkmf-driven extension builds
//! inside the driver then compile against the runtime's own headers,
//! exactly like the gem. Without the SDK the overrides carry only the
//! bindir override (the gem's no-SDK branch).
//!
//! Windows deploy: the re-entry vehicles exist on every platform
//! (tebako#753). POSIX keeps the `#!/bin/sh` ruby shim; Windows gets
//! `ruby.cmd` (the batch re-entry, PATHEXT's answer to the override's
//! extensionless `bin/ruby`) AND `ruby.exe` — a real re-entry
//! executable, because mkmf/rubygems compute the spawn path as
//! `bindir + ruby_install_name + EXEEXT` and CreateProcess runs only a
//! PE at the explicit `.exe` spelling. The exe is the architecture's
//! own vehicle: the press's bootstrap bytes stitched with the deploy
//! driver image (the trailer's `;image` ref resolves the exact runtime
//! pair from the store; the handoff's `--tebako-entry /local/stub.rb`
//! runs the stub, which applies the RbConfig overrides in the child —
//! the same script mode the shims ride). Windows paths crossing into
//! Ruby sources and spawned command lines are normalized to forward
//! slashes (`fwd`): cmd.exe/sh.exe layers strip backslashes (the
//! leg's `C:Usersrunneradmin…/ruby.exe` evidence). Script mode's named
//! refusal on Windows is retired with the shim's arrival; the SDK
//! (header overrides) stays POSIX-only.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::TebakoError;
use crate::runner::run_with_capture;

const DRIVER_IMAGE: &str = "deploy-driver.tfs";
const DRIVER_PACKAGE: &str = "deploy-driver.pkg";
const EMPTY_BASE: &str = "deploy-driver.base";
const BUNDLE_EXEC_SCRIPT_NAME: &str = "bundle_exec.rb";

/// Deploy directives (DeployHelper ops), executed in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Dir.chdir(dir)
    Chdir(String),
    /// Gem::GemRunner.run(argv)
    Gem(Vec<String>),
    /// activate bundler (pinned when Some) and run its CLI
    Bundle(Option<String>, Vec<String>),
    /// Bundler.setup + the gem command in a fresh re-exec
    BundleExec(Option<String>, Vec<String>),
    /// gem install every *.gem in dir
    InstallAll(String, Vec<String>),
}

pub struct RuntimeDeployer {
    pub runtime_path: PathBuf,
    pub staging_bin_dir: PathBuf,
    pub fs_mount_point: String,
    pub ruby_version: String,
    pub tebako_version: String,
    pub verbose: bool,
    /// The provisioned runtime SDK (native-extension deploy); None keeps
    /// the gem's no-SDK build_overrides branch (bindir override only).
    pub sdk: Option<crate::sdk::SdkPaths>,
    /// The press's resolved bootstrap (the base of the shipped package);
    /// on Windows the deploy reuses those bytes as the base of the
    /// `ruby.exe` re-entry vehicle (tebako#753). POSIX never reads it.
    pub bootstrap_path: PathBuf,
}

impl RuntimeDeployer {
    /// env: GEM_HOME/GEM_PATH/GEM_SPEC_CACHE/SSL_CERT_* for the deploy; it
    /// travels in the process environment because Gem::PathSupport
    /// snapshots it at interpreter boot. The deploy installs no jail, so
    /// the driver writes the packaging environment on the host directly
    /// (v1's TEBAKO_PASS_THROUGH memfs-filter escape is retired — no v2
    /// runtime patch reads it).
    pub fn execute(
        &self,
        ops: &[Op],
        env: &[(String, String)],
        seed_dir: &Path,
    ) -> Result<(), TebakoError> {
        self.write_driver(seed_dir, ops);
        // The deploy driver image is PRESS-INTERNAL machinery: the
        // resolved runtime mounts it to run the deploy ops, and released
        // runtimes predate the limnifs backend — it stays dwarfs
        // regardless of --format (which governs the shipped app payload
        // only). A limnifs-less runtime therefore presses a
        // `--format limnifs` package fine; running that package needs a
        // limnifs-capable runtime (spec 20 §5's fail-closed rule).
        // The staging tree carries the extracted runtime layout — its
        // `__tpkg__/` block (the runtime's own in-image manifest,
        // library_aliases included) must NOT ride the deploy image: the
        // env image declares the same aliases from its own bytes, and
        // two different byte sources declaring one alias is the driver's
        // authoring-ambiguity named error (tebako#753's windows leg:
        // 'libwinpthread-1.dll' declared by both). The deploy image
        // declares no manifest of its own (spec 03 §7 already excludes
        // the block from the digest).
        let deploy_excludes = tpkg::exclude::ExcludeSet::parse(&["__tpkg__".to_string()])
            .map_err(|e| crate::error::plain_error(format!("{e:?}")))?;
        crate::image::build_image(
            &self.driver_image(),
            seed_dir,
            crate::options::PressImageFormat::Dwarfs,
            &deploy_excludes,
        )?;
        self.stitch_driver_package()?;
        self.write_bundle_exec_script()?;
        self.write_ruby_shim()?;
        self.write_ruby_exe()?;
        let mut full_env = self.toolchain_env();
        full_env.extend(env.iter().cloned());
        // The deploy bindir in the process env: a contract variable (the
        // scrub keeps it), so every re-entered generation's stub reads the
        // same forward-slashed value — the ruby.exe vehicle sets nothing
        // itself, and the shims' own assignment is belt-and-braces over
        // this inheritance.
        full_env.push((
            "TEBAKO_DEPLOY_BINDIR".to_string(),
            fwd(&self.staging_bin_dir.to_string_lossy()),
        ));
        // The v2 handoff needs the entry explicitly (spec 17 §1): with no
        // --tebako-entry the boot is the smoke form — the interpreter
        // starts with its own args and the driver script never runs (the
        // v1 runtime ran its compiled-in /local/stub.rb unconditionally).
        // Same re-entry form as the ruby shim below.
        let out = run_with_capture(
            &self.runtime_path,
            &[
                "--tebako-image".to_string(),
                self.driver_image_ref(),
                "--tebako-entry".to_string(),
                "/local/stub.rb".to_string(),
            ],
            &full_env,
        )?;
        if self.verbose {
            print!("{out}");
        }
        Ok(())
    }

    fn driver_image(&self) -> PathBuf {
        self.staging_bin_dir.join(DRIVER_IMAGE)
    }

    fn driver_package(&self) -> PathBuf {
        self.staging_bin_dir.join(DRIVER_PACKAGE)
    }

    fn driver_image_ref(&self) -> String {
        format!(
            "{}:0:{}",
            self.driver_package().display(),
            crate::declared_mount(&self.fs_mount_point)
        )
    }

    fn bundle_exec_script(&self) -> PathBuf {
        self.staging_bin_dir.join(BUNDLE_EXEC_SCRIPT_NAME)
    }

    fn write_driver(&self, seed_dir: &Path, ops: &[Op]) {
        let local = seed_dir.join("local");
        let _ = fs::create_dir_all(&local);
        let _ = fs::write(local.join("stub.rb"), self.driver_source(ops));
    }

    /// The runtime reads the slot region referenced by the file's tpkg
    /// trailer; the base bytes are irrelevant to the mount, so the package
    /// is stitched onto an empty base. The slot's mount point is the
    /// DECLARED root (the driver qualifies it onto the VFS drive at boot,
    /// spec 17 §1) and the L2 `mounts:` row declares the union over the
    /// env image — the deploy driver image lands at the runtime root the
    /// env image already owns.
    fn stitch_driver_package(&self) -> Result<(), TebakoError> {
        let empty_base = self.staging_bin_dir.join(EMPTY_BASE);
        fs::write(&empty_base, b"").map_err(|e| {
            crate::error::plain_error(format!("{e} writing {}", empty_base.display()))
        })?;
        // Press-internal machinery, never loader-booted: the ref carries
        // no `;image` flag (write_ruby_exe's loader-booted trailer does).
        let runtime_ref = format!("ruby@{};tebako={}", self.ruby_version, self.tebako_version);
        self.stitch_deploy_package(
            &empty_base,
            &self.driver_package(),
            "tebako-deploy-driver",
            runtime_ref,
        )
    }

    /// The deploy driver image stitched onto `base` at `out`, under the
    /// package identity `name` — the shared assembly of the driver
    /// package (empty base, runtime-exec reads the trailer) and the
    /// Windows ruby.exe vehicle (bootstrap base, the bootstrap reads the
    /// trailer).
    fn stitch_deploy_package(
        &self,
        base: &Path,
        out: &Path,
        name: &str,
        runtime_ref: String,
    ) -> Result<(), TebakoError> {
        let declared = crate::declared_mount(&self.fs_mount_point);
        let images = [tebako_pkg::PackageImage {
            path: self.driver_image(),
            mount_point: declared.to_string(),
            format_id: tpkg::TPKG_FORMAT_DWARFS,
        }];
        let options = tebako_pkg::PackageOptions {
            runtime_ref: runtime_ref.clone(),
            package_flags: tpkg::TPKG_FLAG_LEAN,
            launcher_abi: crate::LAUNCHER_ABI,
            package_manifest: Some(tpkg::PackageManifest {
                schema_version: tpkg::PACKAGE_SCHEMA_VERSION,
                package: tpkg::PackageIdentity {
                    name: name.to_string(),
                    version: self.tebako_version.clone(),
                    producer: tpkg::Producer {
                        tool: "tebako-cli".to_string(),
                        tool_version: self.tebako_version.clone(),
                    },
                    created: crate::install::rfc3339_utc(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    ),
                },
                entries: vec![tpkg::PackageEntry {
                    name: name.to_string(),
                    slot: Some(0),
                    entrypoint: "/local/stub.rb".to_string(),
                    runtime_ref,
                    interp_env: Default::default(),
                }],
                jail: None,
                env: Default::default(),
                lock: None,
                mounts: vec![tpkg::PackageMount {
                    slot: 0,
                    point: declared.to_string(),
                    mode: tpkg::MountMode::Union,
                    precedence: Some(tpkg::Precedence::AfterEnv),
                }],
            }),
            ..Default::default()
        };
        tebako_pkg::bundle_exact(base, &images, out, &options).map_err(crate::error::plain_error)
    }

    fn ruby_exe_path(&self) -> PathBuf {
        self.staging_bin_dir.join("ruby.exe")
    }

    /// tebako#753 (the spawn-name layer): mkmf/rubygems compute the
    /// re-entry path as `bindir + ruby_install_name + EXEEXT` —
    /// `ruby.exe` on Windows, an explicit suffix PATHEXT never rewrites
    /// to the `ruby.cmd` script, and CreateProcess runs only a real PE at
    /// that name. The vehicle is the architecture's own entry point: the
    /// press's bootstrap bytes stitched with this same deploy driver
    /// image. A spawn of ruby.exe reads the trailer, resolves the exact
    /// pinned runtime pair from the store (the `;image` flag — the
    /// deployer resolved the pair into this store moments earlier, so
    /// this is a cache hit, and TEBAKO_HOME rides the contract env), and
    /// hands off `--tebako-entry /local/stub.rb <user args>` — the stub
    /// applies the overrides in the child, exactly as the shims do.
    /// POSIX never builds it (its spawn spellings resolve the exec-shim
    /// directly).
    fn write_ruby_exe(&self) -> Result<(), TebakoError> {
        self.write_ruby_exe_for(cfg!(windows))
    }

    fn write_ruby_exe_for(&self, windows: bool) -> Result<(), TebakoError> {
        if !windows {
            return Ok(());
        }
        let runtime_ref = format!(
            "ruby@{};tebako={};image",
            self.ruby_version, self.tebako_version
        );
        self.stitch_deploy_package(
            &self.bootstrap_path,
            &self.ruby_exe_path(),
            "tebako-deploy-ruby",
            runtime_ref,
        )
    }

    fn write_bundle_exec_script(&self) -> Result<(), TebakoError> {
        fs::write(self.bundle_exec_script(), BUNDLE_EXEC_SCRIPT)
            .map_err(|e| crate::error::plain_error(format!("{e} writing bundle_exec script")))
    }

    /// mkmf-driven native extension builds spawn RbConfig.ruby / Gem.ruby
    /// as a subprocess (the extconf.rb run). The shim re-enters the driver
    /// image at its stub (the script mode applies the RbConfig overrides
    /// and loads the script in-process — the interpreter keyword would
    /// skip them).
    ///
    /// tebako#753: the shim has a WINDOWS shape — `ruby.cmd` beside the
    /// POSIX `ruby` (the override's `File.join(bindir, "ruby")` keeps the
    /// extensionless spelling; CreateProcess resolves it through PATHEXT).
    /// Without it the child's spawn fell to the runtime's baked
    /// CONFIG["ruby"] (A:/t/bin/ruby.exe — the mount-root world, naming
    /// nothing on the host) and the press died on the first native gem.
    fn write_ruby_shim(&self) -> Result<(), TebakoError> {
        let (path, shim) = ruby_shim(
            &self.staging_bin_dir,
            &self.runtime_path.display().to_string(),
            &self.driver_image_ref(),
            cfg!(windows),
        );
        fs::write(&path, shim)
            .map_err(|e| crate::error::plain_error(format!("{e} writing {}", path.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)
                .map_err(|e| crate::error::plain_error(format!("{e}")))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)
                .map_err(|e| crate::error::plain_error(format!("{e}")))?;
        }
        Ok(())
    }

    /// Toolchain for the driver process environment: the first available
    /// candidate per key, but only for keys the user has not already set
    /// (explicit user CC/CXX/... wins). POSIX-only.
    fn toolchain_env(&self) -> Vec<(String, String)> {
        // POSIX-only by platform, never by the shim's reach (the shim now
        // exists everywhere — tebako#753; the candidate list is the POSIX
        // toolchain's spellings).
        if cfg!(windows) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (key, candidates) in toolchain_candidates() {
            let set = std::env::var(key).map(|v| !v.is_empty()).unwrap_or(false);
            if set {
                continue;
            }
            if let Some(tool) = candidates.iter().find(|c| on_path(c)) {
                out.push((key.to_string(), tool.to_string()));
            }
        }
        out
    }

    fn driver_source(&self, ops: &[Op]) -> String {
        DRIVER_TEMPLATE
            .replace(
                "@BUNDLE_EXEC_SCRIPT@",
                &rb_str(&self.bundle_exec_script().to_string_lossy()),
            )
            .replace("@BUILD_OVERRIDES@", &self.build_overrides())
            .replace("@OP_LINES@", &op_lines(ops))
    }

    /// Without the RuntimeSdk this is the gem's no-SDK branch: bindir
    /// override only (no header/library overrides, no cc_override). With
    /// it, extconf/make recipes spawn RbConfig.ruby / Gem.ruby (the shim)
    /// and compile against rubyhdrdir; the overrides point bindir at the
    /// host shim, the header dirs at the runtime SDK, and LIBRUBYARG at
    /// the SDK's symbol stub — mkmf's link probes expand $(LIBRUBYARG)
    /// only for throwaway executables, so they get true yes/no
    /// resolution while the shipped extension .so never links the stub
    /// and resolves against the runtime executable at load time. mkmf
    /// reads MAKEFILE_CONFIG, rubygems reads CONFIG — both take the
    /// overrides.
    fn build_overrides(&self) -> String {
        let bindir = fwd(&self.staging_bin_dir.to_string_lossy());
        let mut out =
            String::from("[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n");
        out.push_str(&format!(
            "  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", {})\n",
            rb_str(&bindir)
        ));
        // tebako#753: the factory bakes CONFIG[\"ruby\"] as the BUILD
        // container's absolute path (A:/t/bin/ruby.exe — the mount-root
        // world); mkmf and Gem.ruby invoke it verbatim for extconf and
        // make's $(RUBY), so the bindir override alone never reaches
        // them. Point the absolute spellings at the host shim. The values
        // are forward-slashed (`fwd`): cmd.exe/sh.exe layers strip
        // backslashes from spawn strings (the rails-windows leg's
        // `C:Usersrunneradmin…/ruby.exe` evidence).
        out.push_str(&format!(
            "  tg_ruby = File.join(ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", {}), \"ruby\")\n  \
             tg_config[\"ruby\"] = tg_ruby\n  \
             tg_config[\"RUBY\"] = tg_ruby\n",
            rb_str(&bindir)
        ));
        let Some(sdk) = &self.sdk else {
            return format!("{out}end\n");
        };
        out.push_str(&format!(
            "  tg_config[\"rubyhdrdir\"] = {}\n",
            rb_str(&sdk.include.to_string_lossy())
        ));
        out.push_str(&format!(
            "  tg_config[\"rubyarchhdrdir\"] = {}\n",
            rb_str(&sdk.archhdr.to_string_lossy())
        ));
        out.push_str(&format!(
            "  tg_config[\"LIBRUBYARG\"] = {}\n",
            rb_str(&sdk.stub.to_string_lossy())
        ));
        out.push_str("  tg_config[\"EXTDLDFLAGS\"] = \"\"\n");
        out.push_str(&cc_override());
        out.push_str("end\n");
        out
    }
}

// ---------------------------------------------------------------------
// op serialization (RuntimeDeployer#op_line)
// ---------------------------------------------------------------------

/// The ruby shim's (path, content) per platform — a pure function so the
/// windows form is testable on any host (the `cfg!(windows)` code path is
/// invisible to POSIX test runs otherwise). POSIX writes `ruby` (a
/// #!/bin/sh re-entry); windows writes `ruby.cmd` (the batch re-entry —
/// PATHEXT resolves the override's extensionless `bin\ruby` to it, behind
/// `ruby.exe`'s earlier PATHEXT slot when both exist). The batch
/// normalizes its self-located bindir to forward slashes: the value
/// re-enters Ruby sources and spawned command lines, where cmd.exe/sh.exe
/// layers strip backslashes (tebako#753).
fn ruby_shim(bin_dir: &Path, runtime: &str, image_ref: &str, windows: bool) -> (PathBuf, String) {
    if windows {
        (
            bin_dir.join("ruby.cmd"),
            format!(
                "@echo off\r\nset \"TEBAKO_DEPLOY_BINDIR=%~dp0\"\r\nset \"TEBAKO_DEPLOY_BINDIR=%TEBAKO_DEPLOY_BINDIR:\\=/%\"\r\n\"{runtime}\" --tebako-image \"{image_ref}\" --tebako-entry /local/stub.rb %*\r\nexit /b %ERRORLEVEL%\r\n"
            ),
        )
    } else {
        (
            bin_dir.join("ruby"),
            format!(
                "#!/bin/sh\nTEBAKO_DEPLOY_BINDIR=\"$(dirname \"$0\")\"; export TEBAKO_DEPLOY_BINDIR\nexec \"{runtime}\" --tebako-image \"{image_ref}\" --tebako-entry /local/stub.rb \"$@\"\n"
            ),
        )
    }
}

/// Windows path values flowing into Ruby sources and spawned command
/// lines are forward-slashed: Ruby-win32, CreateProcess, cmd.exe and the
/// msys2 sh/make layers all accept them, while backslashes die at the
/// shell layers (tebako#753's `C:Usersrunneradmin…` evidence). A no-op
/// for POSIX spellings (no backslashes to rewrite).
fn fwd(s: &str) -> String {
    s.replace('\\', "/")
}

fn op_lines(ops: &[Op]) -> String {
    ops.iter().map(op_line).collect::<Vec<_>>().join("\n")
}

fn op_line(op: &Op) -> String {
    match op {
        Op::Chdir(dir) => format!("Dir.chdir({})", rb_str(dir)),
        Op::Gem(argv) => format!("tg_run_gem({})", rb_arr(argv)),
        Op::Bundle(version, argv) => format!(
            "tg_run_bundle({}, {})",
            rb_opt_str(version.as_deref()),
            rb_arr(argv)
        ),
        Op::BundleExec(version, argv) => format!(
            "tg_bundle_exec({}, {})",
            rb_opt_str(version.as_deref()),
            rb_arr(argv)
        ),
        Op::InstallAll(dir, argv) => {
            format!("tg_install_all({}, {})", rb_str(dir), rb_arr(argv))
        }
    }
}

/// Ruby String#inspect for the path/argv shapes tebako emits.
pub fn rb_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn rb_opt_str(s: Option<&str>) -> String {
    match s {
        Some(v) => rb_str(v),
        None => "nil".to_string(),
    }
}

fn rb_arr(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|i| rb_str(i)).collect();
    format!("[{}]", inner.join(", "))
}

// ---------------------------------------------------------------------
// toolchain candidates (RuntimeDeployer::TOOLCHAIN_FALLBACKS)
// ---------------------------------------------------------------------

const CLANG_VERSIONS: &[&str] = &["20", "19", "18", "17", "16", "15", "14", "13", "12", "11"];

fn toolchain_candidates() -> Vec<(&'static str, Vec<String>)> {
    let versioned = |name: &str| -> Vec<String> {
        CLANG_VERSIONS
            .iter()
            .map(|v| format!("{name}-{v}"))
            .collect()
    };
    let mut cc = vec!["clang".to_string()];
    cc.extend(versioned("clang"));
    cc.extend(["cc".to_string(), "gcc".to_string()]);

    let mut cxx = vec!["clang++".to_string()];
    cxx.extend(versioned("clang++"));
    cxx.extend(["c++".to_string(), "g++".to_string()]);

    let simple = |first: &str, llvm: &str| -> Vec<String> {
        let mut v = vec![first.to_string()];
        v.extend(versioned(llvm));
        v
    };

    vec![
        ("CC", cc),
        ("CXX", cxx),
        ("AR", simple("ar", "llvm-ar")),
        ("RANLIB", simple("ranlib", "llvm-ranlib")),
        ("NM", simple("nm", "llvm-nm")),
        ("OBJDUMP", simple("objdump", "llvm-objdump")),
        ("OBJCOPY", simple("objcopy", "llvm-objcopy")),
    ]
}

fn on_path(tool: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    for dir in path_var.split(if cfg!(windows) { ';' } else { ':' }) {
        let candidate = Path::new(dir).join(tool);
        if is_executable(&candidate) {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

// ---------------------------------------------------------------------
// driver-side toolchain override (RuntimeDeployer#cc_override)
// ---------------------------------------------------------------------

/// The recorded toolchain comes from the runtime's build machine (an
/// LLVM release); when it is not installed on the press host, mkmf probes
/// and bundled-library links die at shell level ("The compiler failed to
/// generate an executable file", "command not found"). The emitted driver
/// code falls back to the first available equivalent: newer/older clang
/// for the compilers (the recorded flags are clang-flavored), binutils
/// for the llvm tools. Each candidate list starts with the recorded tool
/// as a Ruby code reference (it reads the runtime's rbconfig inside the
/// driver), followed by the literal fallbacks.
///
/// The tool re-resolution alone is not enough (tebako-runtime-ruby#22):
/// the recorded flag STRINGS stay clang-flavored, and on a gcc-only press
/// host mkmf's first probe dies on clang-only flags (-fdeclspec & co).
/// The emitted tg_filter_flags replays each compile-flag value against
/// the re-resolved compiler and drops what it rejects, loudly. Deliberate
/// deviation from the gem (which replayed the recorded flags verbatim —
/// correct only while every runtime and every press host were clang).
fn cc_override() -> String {
    let mut out = String::from(
        "def tg_first_tool(*candidates)\n  candidates.find { |tg_c| !tg_c.to_s.empty? && system(\"command -v #{tg_c} >/dev/null 2>&1\") }\nend\n\n{\n",
    );
    for (key, fallbacks) in toolchain_candidates() {
        out.push_str(&format!(
            "  {} => [{}],\n",
            rb_str(key),
            override_candidates(key, &fallbacks)
        ));
    }
    out.push_str(
        "}.each do |tg_key, tg_candidates|\n  tg_tool = tg_first_tool(*tg_candidates)\n  next if tg_tool.nil?\n  [RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each { |tg_config| tg_config[tg_key] = tg_tool }\nend\n",
    );
    out.push_str(&flag_filter());
    out
}

/// [recorded tool as a Ruby code reference, literal fallbacks...] — the
/// gem's override_candidates; NM's recorded value carries flags
/// (`nm --no-codesign` & co), hence the `.to_s.split.first`.
fn override_candidates(key: &str, fallbacks: &[String]) -> String {
    let recorded = if key == "NM" {
        format!("RbConfig::CONFIG[{}].to_s.split.first", rb_str(key))
    } else {
        format!("RbConfig::CONFIG[{}]", rb_str(key))
    };
    let mut parts = vec![recorded];
    parts.extend(fallbacks.iter().map(|c| rb_str(c)));
    parts.join(", ")
}

/// The compile-flag keys mkmf assembles into its probes, as
/// (rbconfig key, tool key, probe -x language). Link flags (LDFLAGS,
/// DLDFLAGS) are not filtered: they reach the SAME system linker through
/// either compiler driver, and EXTLDFLAGS/LIBRUBYARG are already
/// overridden above.
const FLAG_FILTER_KEYS: &[(&str, &str, &str)] = &[
    ("CFLAGS", "CC", "c"),
    ("cflags", "CC", "c"),
    ("CPPFLAGS", "CC", "c"),
    ("cppflags", "CC", "c"),
    ("warnflags", "CC", "c"),
    ("ARCH_FLAG", "CC", "c"),
    ("CXXFLAGS", "CXX", "c++"),
    ("cxxflags", "CXX", "c++"),
];

/// The emitted flag filter (tg_probe_ok / TG_FLAG_PAIRS /
/// tg_filter_flags + the per-key application). Probe semantics: the fast
/// path replays the whole value at once and keeps it verbatim when the
/// re-resolved compiler accepts everything (same family as the runtime's
/// build compiler — zero drift, one probe); only a rejection pays the
/// per-token pass. Pair flags (-arch arm64, -isysroot <path>) probe with
/// their value — gcc rejects a bare -arch just the same, but a bare
/// -isystem would spuriously pass while the pair fails. Make-variable
/// references ($(cflags) in MAKEFILE_CONFIG's unexpanded compositions)
/// are never probe subjects. Every drop is logged: the drop lines in the
/// press log are the audit trail of the deviation.
fn flag_filter() -> String {
    let mut out = String::from(
        r#"def tg_probe_ok(tg_cc, tg_lang, tg_flags)
  system(tg_cc, "-Werror", "-x", tg_lang, "/dev/null", "-fsyntax-only", *tg_flags, out: File::NULL, err: File::NULL) ? true : false
end

TG_FLAG_PAIRS = %w[-arch -isysroot -isystem -iquote -idirafter -include -imacros -target --sysroot -gcc-toolchain -Xclang -Xpreprocessor].freeze

def tg_filter_flags(tg_cc, tg_lang, tg_recorded)
  tg_tokens = tg_recorded.to_s.split
  return tg_recorded if tg_tokens.empty? || tg_probe_ok(tg_cc, tg_lang, tg_tokens)

  tg_kept = []
  tg_idx = 0
  while tg_idx < tg_tokens.length
    tg_tok = tg_tokens[tg_idx]
    if tg_tok.include?("$(")
      tg_kept << tg_tok
      tg_idx += 1
      next
    end
    tg_pair = TG_FLAG_PAIRS.include?(tg_tok) && tg_idx + 1 < tg_tokens.length ? [tg_tok, tg_tokens[tg_idx + 1]] : [tg_tok]
    if tg_probe_ok(tg_cc, tg_lang, tg_pair)
      tg_kept.concat(tg_pair)
    else
      warn "tebako deploy: dropped compiler flag '#{tg_pair.join(" ")}' (rejected by #{tg_cc}; recorded for the runtime's build compiler)"
    end
    tg_idx += tg_pair.length
  end
  tg_kept.join(" ")
end

{
"#,
    );
    for (key, tool_key, lang) in FLAG_FILTER_KEYS {
        out.push_str(&format!(
            "  {} => [{}, {}],\n",
            rb_str(key),
            rb_str(tool_key),
            rb_str(lang)
        ));
    }
    out.push_str(
        r#"}.each do |tg_key, (tg_tool_key, tg_lang)|
  tg_cc = RbConfig::CONFIG[tg_tool_key].to_s
  next if tg_cc.empty? || !system("command -v #{tg_cc} >/dev/null 2>&1")
  [RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|
    tg_recorded = tg_config[tg_key]
    next if tg_recorded.to_s.empty?
    tg_config[tg_key] = tg_filter_flags(tg_cc, tg_lang, tg_recorded)
  end
end
"#,
    );
    out
}

// ---------------------------------------------------------------------
// driver templates (verbatim ports of the gem's heredocs)
// ---------------------------------------------------------------------

const DRIVER_TEMPLATE: &str = r##"# THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE
require "rubygems"
require "rubygems/gem_runner"
require "rubygems/request"
require "fileutils"
require "tmpdir"

BUNDLE_EXEC_SCRIPT = @BUNDLE_EXEC_SCRIPT@

# tebako#119: git-source gems must materialize INTO the image. Bundler
# keeps a git gem's checkout under the bundle path (the default is the
# host-absolute ~/.bundle — empty at run time, so Bundler.require died
# with 'not yet checked out'). Pin the bundle path RELATIVE to the
# Gemfile's app root: the press world's paths ARE the image's VFS
# paths, so the checkout lands in-image, and the relative spelling
# resolves identically when the packaged app runs.
if (tg_gemfile = ENV["BUNDLE_GEMFILE"]) && File.file?(tg_gemfile)
  tg_bundle_root = File.expand_path("vendor/bundle", File.dirname(tg_gemfile))
  ENV["BUNDLE_PATH"] = tg_bundle_root
  ENV["BUNDLE_DISABLE_SHARED_GEMS"] = "false"
end

@BUILD_OVERRIDES@if ARGV.any?
  # Script mode: mkmf-driven extension builds spawn the ruby at
  # RbConfig's bindir (the host shim); the shim re-enters this image
  # with the spawn's full argv. Emulate the ruby command line: consume
  # the usual interpreter switches first (-r/-I/-e/--), then run the
  # script -- mkmf derives srcdir from $0, so the script takes over
  # the program name before it is loaded.
  tg_ran_eval = false
  while ARGV.any?
    case ARGV.first
    when "-r"
      ARGV.shift
      require ARGV.shift
    when /\A-r(.+)/
      require Regexp.last_match(1)
      ARGV.shift
    when "-I"
      ARGV.shift
      $LOAD_PATH.unshift ARGV.shift
    when /\A-I(.+)/
      $LOAD_PATH.unshift Regexp.last_match(1)
      ARGV.shift
    when "-e"
      ARGV.shift
      eval(ARGV.shift, TOPLEVEL_BINDING, "-e")
      tg_ran_eval = true
    when /\A-e(.+)/
      eval(Regexp.last_match(1), TOPLEVEL_BINDING, "-e")
      tg_ran_eval = true
    when "--"
      ARGV.shift
      break
    else
      break
    end
  end
  exit 0 if tg_ran_eval

  if ARGV.any?
    $0 = ARGV.first
    load ARGV.shift
  end
else
# OpenSSL reads certificate files at the C level, where the memfs is
# invisible; give rubygems and bundler host-side copies of the CA
# certs vendored in the image
TG_DEPLOY_CERT_DIR = File.join(ENV.fetch("GEM_SPEC_CACHE", Dir.mktmpdir), "ssl_certs")
FileUtils.mkdir_p(TG_DEPLOY_CERT_DIR)

module TebakoDeployCerts
  def get_cert_files
    super.map do |src|
      dst = File.join(TG_DEPLOY_CERT_DIR, File.basename(src))
      FileUtils.cp(src, dst) unless File.exist?(dst)
      dst
    end
  end
end
Gem::Request.singleton_class.prepend(TebakoDeployCerts)

# rubygems commands end with terminate_interaction(0) on success,
# which raises Gem::SystemExitException (< SystemExit) and would end
# this process before the remaining operations; bundler exits the
# same way on failure. Guard both and re-raise on non-zero status.
def tg_run_gem(args)
  puts "   ... @ gem #{args.join(" ")}"
  begin
    Gem::GemRunner.new.run(args)
  rescue SystemExit => e
    raise "gem #{args.first} failed (exit #{e.status})" unless e.status.zero?
  end
  # Gems this operation installed must be visible to the following
  # ones (rubygems caches the spec index at interpreter boot)
  Gem::Specification.reset
end

def tg_run_bundle(version, args)
  puts "   ... @ bundle #{args.join(" ")}"
  gem "bundler", version unless version.nil?
  ARGV.replace(args)
  begin
    load Gem.bin_path("bundler", "bundle")
  rescue SystemExit => e
    raise "bundle #{args.first} failed (exit #{e.status})" unless e.status.zero?
  end
end

# 'bundle exec' needs a fresh process: the driver itself may already
# have activated a default gem at another version (openssl for the
# fetch above), and a gem cannot be re-activated at the bundle's
# version. The shim re-enters this image with the companion script
# in a clean interpreter.
def tg_bundle_exec(version, argv)
  puts "   ... @ bundle exec #{argv.join(" ")}"
  raise "bundle exec #{argv.first} failed" unless system(RbConfig.ruby, BUNDLE_EXEC_SCRIPT, version.to_s, *argv)
end

def tg_install_all(dir, args)
  gems = Dir.glob(File.join(dir, "*.gem"))
  raise "No gem files found after build" if gems.empty?

  gems.each { |gem_file| tg_run_gem(["install", gem_file] + args) }
end

@OP_LINES@
end
"##;

const BUNDLE_EXEC_SCRIPT: &str = r##"# THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE
version = ARGV.shift
gem "bundler", version unless version.nil? || version.empty?
# tebako#119: the bundle exec re-entry runs in a FRESH interpreter —
# the driver's BUNDLE_PATH pin must hold here too, or Bundler falls
# back to the host-absolute ~/.bundle and the in-image git checkout
# the press materialized is invisible.
if (tg_gemfile = ENV["BUNDLE_GEMFILE"]) && File.file?(tg_gemfile)
  ENV["BUNDLE_PATH"] ||= File.expand_path("vendor/bundle", File.dirname(tg_gemfile))
end
require "bundler"
Bundler.setup
require "rubygems"
require "rubygems/gem_runner"
begin
  Gem::GemRunner.new.run(ARGV)
rescue SystemExit => e
  exit(e.status)
end
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ruby_shim_has_a_windows_shape() {
        // tebako#753: the windows shim is a batch re-entry beside the
        // POSIX script — PATHEXT resolves the override's extensionless
        // `bin\ruby` to it (behind ruby.exe's earlier PATHEXT slot);
        // the batch sets TEBAKO_DEPLOY_BINDIR from its own dir (%~dp0),
        // normalized to forward slashes for the Ruby-source and spawn
        // consumers, and re-enters the runtime at the stub with the exit
        // code propagated.
        let (path, content) =
            ruby_shim(Path::new("/x/bin"), "/rt/ruby.exe", "/p/pkg.tfs:0:/t", true);
        assert_eq!(path, Path::new("/x/bin").join("ruby.cmd"));
        assert!(content.contains("TEBAKO_DEPLOY_BINDIR=%~dp0"), "{content}");
        assert!(
            content.contains("set \"TEBAKO_DEPLOY_BINDIR=%TEBAKO_DEPLOY_BINDIR:\\=/%\""),
            "{content}"
        );
        assert!(content.contains("\"/rt/ruby.exe\" --tebako-image \"/p/pkg.tfs:0:/t\" --tebako-entry /local/stub.rb %*"), "{content}");
        assert!(content.contains("exit /b %ERRORLEVEL%"), "{content}");

        // POSIX keeps the shell re-entry byte shape (the override's
        // File.join(bindir, "ruby") names it directly).
        let (path, content) = ruby_shim(Path::new("/x/bin"), "/rt/ruby", "/p/pkg.tfs:0:/t", false);
        assert_eq!(path, Path::new("/x/bin").join("ruby"));
        assert!(content.starts_with("#!/bin/sh\n"), "{content}");
        assert!(content.contains("exec \"/rt/ruby\" --tebako-image \"/p/pkg.tfs:0:/t\" --tebako-entry /local/stub.rb \"$@\""), "{content}");
    }

    #[test]
    fn the_ruby_exe_is_the_bootstrap_stitched_with_the_deploy_image() {
        // tebako#753 (the spawn-name layer): mkmf/rubygems spawn
        // `bindir + ruby_install_name + EXEEXT` — `ruby.exe` on Windows —
        // and CreateProcess runs only a real PE at that explicit suffix.
        // The vehicle is the press's bootstrap bytes stitched with the
        // deploy driver image: the trailer carries the `;image` ref (the
        // bootstrap resolves the exact runtime pair + exports
        // TEBAKO_RUNTIME_IMAGE) and the /local/stub.rb entry (the
        // argv0-fallback entry — "ruby.exe" matches no declared name).
        let tmp = tempfile::tempdir().unwrap();
        let bootstrap = tmp.path().join("bootstrap.exe");
        fs::write(&bootstrap, b"MZ-fake-bootstrap-bytes").unwrap();
        let d = RuntimeDeployer {
            staging_bin_dir: tmp.path().to_path_buf(),
            bootstrap_path: bootstrap.clone(),
            ..deployer(None)
        };
        // The driver image must exist for the stitch (the press builds it
        // first); the bytes are irrelevant to the trailer assertions.
        fs::write(d.driver_image(), b"image-bytes").unwrap();
        d.write_ruby_exe_for(true).unwrap();

        let exe = d.ruby_exe_path();
        let bytes = fs::read(&exe).unwrap();
        assert!(
            bytes.starts_with(b"MZ-fake-bootstrap-bytes"),
            "ruby.exe must carry the bootstrap bytes as its base"
        );
        let mut f = fs::File::open(&exe).unwrap();
        let m = tpkg::read_from(&mut f).expect("ruby.exe must carry a readable tpkg trailer");
        assert_eq!(
            m.runtime_ref_str(),
            Some("ruby@3.3.7;tebako=0.15.9;image"),
            "the bootstrap resolves the exact pinned pair via the ;image ref"
        );
        let pm = m.package_manifest().unwrap().expect("package manifest");
        assert_eq!(pm.package.name, "tebako-deploy-ruby");
        assert_eq!(pm.entries[0].entrypoint, "/local/stub.rb");
        assert_eq!(pm.entries[0].runtime_ref, "ruby@3.3.7;tebako=0.15.9;image");

        // The POSIX arm is a no-op.
        let tmp2 = tempfile::tempdir().unwrap();
        let d2 = RuntimeDeployer {
            staging_bin_dir: tmp2.path().to_path_buf(),
            ..deployer(None)
        };
        d2.write_ruby_exe_for(false).unwrap();
        assert!(!d2.ruby_exe_path().exists());
    }

    #[test]
    fn forward_slash_normalization() {
        assert_eq!(
            fwd("C:\\Users\\runneradmin\\.tebako\\op"),
            "C:/Users/runneradmin/.tebako/op"
        );
        assert_eq!(fwd("/tmp/o/p"), "/tmp/o/p");
    }

    #[test]
    fn the_deploy_image_drops_the_runtime_manifest_block() {
        // tebako#753 (the alias-ambiguity layer): the staging tree carries
        // the extracted runtime layout; its __tpkg__/ manifest declares
        // the runtime's library_aliases, and mounted beside the env
        // image's own declaration the driver fails closed on the
        // authoring ambiguity ('libwinpthread-1.dll' declared by both).
        // The deploy image excludes the block; the stub's own tree is
        // untouched.
        let ex = tpkg::exclude::ExcludeSet::parse(&["__tpkg__".to_string()]).unwrap();
        // The walker prunes a matched directory with its whole subtree
        // (merkle_host::mirror_filtered's continue) — the matcher answers
        // for the directory itself, not for paths beneath it.
        assert!(ex.matches("__tpkg__"));
        assert!(!ex.matches("local/stub.rb"));
        assert!(!ex.matches("bin/bundle_exec.rb"));
    }

    #[test]
    fn build_overrides_normalize_the_bindir_to_forward_slashes() {
        // tebako#753: a backslashed windows staging dir must not reach the
        // Ruby sources verbatim — the shell layers strip the backslashes.
        let d = RuntimeDeployer {
            staging_bin_dir: PathBuf::from("C:\\Users\\runneradmin\\.tebako\\op"),
            ..deployer(None)
        };
        let out = d.build_overrides();
        assert!(
            out.contains(
                "ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"C:/Users/runneradmin/.tebako/op\")"
            ),
            "{out}"
        );
        assert!(!out.contains("runneradmin\\"), "{out}");
    }

    #[test]
    fn op_lines_match_gem_rendering() {
        let ops = vec![
            Op::Chdir("/tmp/o/s/local".to_string()),
            Op::Bundle(
                Some("2.5.6".to_string()),
                vec![
                    "config".to_string(),
                    "set".to_string(),
                    "--local".to_string(),
                    "force_ruby_platform".to_string(),
                    "true".to_string(),
                ],
            ),
            Op::Bundle(None, vec!["install".to_string(), "--jobs=8".to_string()]),
            Op::Gem(vec!["install".to_string(), "bundler".to_string()]),
        ];
        let rendered = op_lines(&ops);
        assert_eq!(
            rendered,
            "Dir.chdir(\"/tmp/o/s/local\")\n\
             tg_run_bundle(\"2.5.6\", [\"config\", \"set\", \"--local\", \"force_ruby_platform\", \"true\"])\n\
             tg_run_bundle(nil, [\"install\", \"--jobs=8\"])\n\
             tg_run_gem([\"install\", \"bundler\"])"
        );
    }

    #[test]
    fn rb_str_escapes() {
        assert_eq!(rb_str("a\"b"), "\"a\\\"b\"");
        assert_eq!(rb_str("a\\b"), "\"a\\\\b\"");
        assert_eq!(rb_str("plain"), "\"plain\"");
    }

    fn deployer(sdk: Option<crate::sdk::SdkPaths>) -> RuntimeDeployer {
        RuntimeDeployer {
            runtime_path: PathBuf::from("/tmp/runtime"),
            staging_bin_dir: PathBuf::from("/tmp/o/p"),
            fs_mount_point: "/__tfs__".to_string(),
            ruby_version: "3.3.7".to_string(),
            tebako_version: "0.15.9".to_string(),
            verbose: false,
            sdk,
            bootstrap_path: PathBuf::from("/tmp/bootstrap"),
        }
    }

    fn sdk_paths() -> crate::sdk::SdkPaths {
        crate::sdk::SdkPaths {
            root: PathBuf::from("/tmp/deps/sdk/3.3.7-v0.2.1-test"),
            include: PathBuf::from("/tmp/deps/sdk/3.3.7-v0.2.1-test/include"),
            archhdr: PathBuf::from("/tmp/deps/sdk/3.3.7-v0.2.1-test/archhdr"),
            stub: PathBuf::from("/tmp/deps/sdk/3.3.7-v0.2.1-test/lib/libruby-stub.a"),
        }
    }

    #[test]
    fn build_overrides_without_sdk_is_the_gem_no_sdk_branch() {
        // tebako#753: the overrides emit on every platform — the shim's
        // windows shape (ruby.cmd) carries the re-entry there too.
        assert_eq!(
            deployer(None).build_overrides(),
            "[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\")\n  tg_ruby = File.join(ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\"), \"ruby\")\n  tg_config[\"ruby\"] = tg_ruby\n  tg_config[\"RUBY\"] = tg_ruby\nend\n"
        );
    }

    #[test]
    fn build_overrides_with_sdk_matches_the_gem_rendering() {
        if cfg!(windows) {
            return;
        }
        let out = deployer(Some(sdk_paths())).build_overrides();
        let head = "[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\")\n  tg_ruby = File.join(ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\"), \"ruby\")\n  tg_config[\"ruby\"] = tg_ruby\n  tg_config[\"RUBY\"] = tg_ruby\n  tg_config[\"rubyhdrdir\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/include\"\n  tg_config[\"rubyarchhdrdir\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/archhdr\"\n  tg_config[\"LIBRUBYARG\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/lib/libruby-stub.a\"\n  tg_config[\"EXTDLDFLAGS\"] = \"\"\n";
        assert!(out.starts_with(head), "unexpected overrides head:\n{out}");
        assert!(
            out.ends_with("end\n"),
            "overrides must close the each block"
        );
        assert!(
            out.contains("def tg_first_tool(*candidates)\n"),
            "cc_override helper missing:\n{out}"
        );
        assert!(
            out.contains("\"CC\" => [RbConfig::CONFIG[\"CC\"], \"clang\", \"clang-20\""),
            "CC candidates missing the recorded tool + fallbacks:\n{out}"
        );
        assert!(
            out.contains("\"NM\" => [RbConfig::CONFIG[\"NM\"].to_s.split.first, \"nm\""),
            "NM candidates must split the recorded value:\n{out}"
        );
        assert!(
            out.contains("\"AR\" => [RbConfig::CONFIG[\"AR\"], \"ar\", \"llvm-ar-20\""),
            "AR candidates:\n{out}"
        );
    }

    #[test]
    fn cc_override_candidate_lists_match_the_gem() {
        let cc = override_candidates(
            "CC",
            &[
                "clang".to_string(),
                "clang-20".to_string(),
                "cc".to_string(),
            ],
        );
        assert_eq!(
            cc,
            "RbConfig::CONFIG[\"CC\"], \"clang\", \"clang-20\", \"cc\""
        );
        let nm = override_candidates("NM", &["nm".to_string()]);
        assert_eq!(nm, "RbConfig::CONFIG[\"NM\"].to_s.split.first, \"nm\"");
    }

    #[test]
    fn flag_filter_replays_against_the_resolved_compiler() {
        // tebako-runtime-ruby#22: the recorded flags are the runtime build
        // machine's (clang on the CI legs); a gcc press host dies on
        // clang-only flags inside mkmf's first probe. The emitted driver
        // code probes the RE-RESOLVED compiler and drops what it rejects.
        let out = flag_filter();
        assert!(
            out.contains("def tg_probe_ok(tg_cc, tg_lang, tg_flags)"),
            "probe helper missing:\n{out}"
        );
        // Fast path: the whole value in one probe, kept verbatim when the
        // compiler accepts it (same family as the build — zero drift).
        assert!(
            out.contains(
                "return tg_recorded if tg_tokens.empty? || tg_probe_ok(tg_cc, tg_lang, tg_tokens)"
            ),
            "fast path missing:\n{out}"
        );
        // Pair flags probe with their value (a bare -arch would be a
        // different question than -arch arm64).
        assert!(
            out.contains("TG_FLAG_PAIRS = %w["),
            "pair-flag table missing:\n{out}"
        );
        for pair in ["-arch", "-isysroot", "-isystem", "-target", "--sysroot"] {
            assert!(out.contains(pair), "TG_FLAG_PAIRS misses {pair}:\n{out}");
        }
        // Make-var references ($(cflags) in MAKEFILE_CONFIG's unexpanded
        // compositions) are never probe subjects.
        assert!(
            out.contains("tg_tok.include?(\"$(\")"),
            "make-var skip missing:\n{out}"
        );
        // Every drop is loud — the press log's drop lines are the audit
        // trail of the deviation from the gem's verbatim replay.
        assert!(
            out.contains("tebako deploy: dropped compiler flag"),
            "drop log missing:\n{out}"
        );
        // The mkmf compile surface: C flags against CC as c, C++ flags
        // against CXX as c++.
        for entry in [
            "\"CFLAGS\" => [\"CC\", \"c\"]",
            "\"cflags\" => [\"CC\", \"c\"]",
            "\"CPPFLAGS\" => [\"CC\", \"c\"]",
            "\"warnflags\" => [\"CC\", \"c\"]",
            "\"ARCH_FLAG\" => [\"CC\", \"c\"]",
            "\"CXXFLAGS\" => [\"CXX\", \"c++\"]",
            "\"cxxflags\" => [\"CXX\", \"c++\"]",
        ] {
            assert!(out.contains(entry), "flag key table misses {entry}:\n{out}");
        }
        // No resolvable compiler -> no filtering (the build dies on its
        // own terms; stripping flags would only obscure that).
        assert!(
            out.contains(
                "next if tg_cc.empty? || !system(\"command -v #{tg_cc} >/dev/null 2>&1\")"
            ),
            "unresolvable-cc guard missing:\n{out}"
        );
    }

    #[test]
    fn build_overrides_with_sdk_carries_the_flag_filter() {
        if cfg!(windows) {
            return;
        }
        let out = deployer(Some(sdk_paths())).build_overrides();
        assert!(
            out.contains("def tg_filter_flags(tg_cc, tg_lang, tg_recorded)"),
            "the SDK overrides must emit the flag filter:\n{out}"
        );
        // The tool re-resolution still lands ahead of the filter (the
        // filter probes the re-resolved compiler).
        let tools = out.find("}.each do |tg_key, tg_candidates|").unwrap();
        let filter = out.find("\"CFLAGS\" => [\"CC\", \"c\"]").unwrap();
        assert!(
            tools < filter,
            "filter must follow the tool re-resolution:\n{out}"
        );
    }

    #[test]
    fn driver_script_mode_emulates_the_ruby_command_line() {
        // mkmf/make spawn RbConfig.ruby with switches (-r/-I/-e/--); the
        // gem's script mode consumes them before loading the script.
        let src = DRIVER_TEMPLATE;
        assert!(src.contains("tg_ran_eval = false"));
        assert!(src.contains("when \"-I\""));
        assert!(src.contains("when /\\A-r(.+)/"));
        assert!(src.contains("exit 0 if tg_ran_eval"));
        assert!(src.contains("$0 = ARGV.first"));
    }
}

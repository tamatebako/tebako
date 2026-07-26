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
}

impl RuntimeDeployer {
    /// env: GEM_HOME/GEM_PATH/GEM_SPEC_CACHE/SSL_CERT_* for the deploy; it
    /// travels in the process environment because Gem::PathSupport
    /// snapshots it at interpreter boot. TEBAKO_PASS_THROUGH joins it: the
    /// tebako-patched rubygems filters gem paths to the memfs mount point
    /// unless it is set, and the driver installs into the packaging
    /// environment on the host.
    pub fn execute(
        &self,
        ops: &[Op],
        env: &[(String, String)],
        seed_dir: &Path,
    ) -> Result<(), TebakoError> {
        self.write_driver(seed_dir, ops);
        crate::image::build_image(&self.driver_image(), seed_dir)?;
        self.stitch_driver_package()?;
        if self.shim_supported() {
            self.write_bundle_exec_script()?;
            self.write_ruby_shim()?;
        }
        let mut full_env = self.toolchain_env();
        full_env.extend(env.iter().cloned());
        full_env.push(("TEBAKO_PASS_THROUGH".to_string(), "1".to_string()));
        let out = run_with_capture(
            &self.runtime_path,
            &["--tebako-image".to_string(), self.driver_image_ref()],
            &full_env,
        )?;
        if self.verbose {
            print!("{out}");
        }
        Ok(())
    }

    fn shim_supported(&self) -> bool {
        !cfg!(windows)
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
            self.fs_mount_point
        )
    }

    fn bundle_exec_script(&self) -> PathBuf {
        self.staging_bin_dir.join(BUNDLE_EXEC_SCRIPT_NAME)
    }

    fn ruby_shim_path(&self) -> PathBuf {
        self.staging_bin_dir.join("ruby")
    }

    fn write_driver(&self, seed_dir: &Path, ops: &[Op]) {
        let local = seed_dir.join("local");
        let _ = fs::create_dir_all(&local);
        let _ = fs::write(local.join("stub.rb"), self.driver_source(ops));
    }

    /// The runtime reads the slot region referenced by the file's tpkg
    /// trailer; the base bytes are irrelevant to the mount, so the package
    /// is stitched onto an empty base.
    fn stitch_driver_package(&self) -> Result<(), TebakoError> {
        let empty_base = self.staging_bin_dir.join(EMPTY_BASE);
        fs::write(&empty_base, b"").map_err(|e| {
            crate::error::plain_error(format!("{e} writing {}", empty_base.display()))
        })?;
        let images = [tebako_pkg::PackageImage {
            path: self.driver_image(),
            mount_point: self.fs_mount_point.clone(),
            format_id: tpkg::TPKG_FORMAT_DWARFS,
        }];
        let options = tebako_pkg::PackageOptions {
            runtime_ref: format!("ruby@{};tebako={}", self.ruby_version, self.tebako_version),
            package_flags: tpkg::TPKG_FLAG_LEAN,
            launcher_abi: crate::LAUNCHER_ABI,
            ..Default::default()
        };
        tebako_pkg::bundle_exact(&empty_base, &images, &self.driver_package(), &options)
            .map_err(crate::error::plain_error)
    }

    fn write_bundle_exec_script(&self) -> Result<(), TebakoError> {
        fs::write(self.bundle_exec_script(), BUNDLE_EXEC_SCRIPT)
            .map_err(|e| crate::error::plain_error(format!("{e} writing bundle_exec script")))
    }

    /// mkmf-driven native extension builds spawn RbConfig.ruby / Gem.ruby
    /// as a subprocess (the extconf.rb run). The shim re-enters the driver
    /// image with the script as argument (the stub's script mode).
    fn write_ruby_shim(&self) -> Result<(), TebakoError> {
        let shim = format!(
            "#!/bin/sh\nTEBAKO_DEPLOY_BINDIR=\"$(dirname \"$0\")\"; export TEBAKO_DEPLOY_BINDIR\nexec \"{}\" --tebako-image \"{}\" --tebako-entry ruby \"$@\"\n",
            self.runtime_path.display(),
            self.driver_image_ref()
        );
        let path = self.ruby_shim_path();
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
        if !self.shim_supported() {
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
        if !self.shim_supported() {
            return String::new();
        }
        let mut out =
            String::from("[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n");
        out.push_str(&format!(
            "  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", {})\n",
            rb_str(&self.staging_bin_dir.to_string_lossy())
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
            fs_mount_point: "/__tebako_memfs__".to_string(),
            ruby_version: "3.3.7".to_string(),
            tebako_version: "0.15.9".to_string(),
            verbose: false,
            sdk,
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
        if cfg!(windows) {
            assert_eq!(deployer(None).build_overrides(), "");
            return;
        }
        assert_eq!(
            deployer(None).build_overrides(),
            "[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\")\nend\n"
        );
    }

    #[test]
    fn build_overrides_with_sdk_matches_the_gem_rendering() {
        if cfg!(windows) {
            return;
        }
        let out = deployer(Some(sdk_paths())).build_overrides();
        let head = "[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", \"/tmp/o/p\")\n  tg_config[\"rubyhdrdir\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/include\"\n  tg_config[\"rubyarchhdrdir\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/archhdr\"\n  tg_config[\"LIBRUBYARG\"] = \"/tmp/deps/sdk/3.3.7-v0.2.1-test/lib/libruby-stub.a\"\n  tg_config[\"EXTDLDFLAGS\"] = \"\"\n";
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

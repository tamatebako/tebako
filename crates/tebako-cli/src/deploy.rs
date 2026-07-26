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
//! Simplification vs the gem (documented in the crate README): the
//! RuntimeSdk / src-release subsystem is not ported — native-extension
//! builds inside deploy (mkmf/cmake) are out of this milestone's scope.
//! The driver's build_overrides therefore carry only the bindir override,
//! which is what the gem emits when no SDK was resolved.

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
    /// override only (no header/library overrides, no cc_override).
    fn build_overrides(&self) -> String {
        if !self.shim_supported() {
            return String::new();
        }
        format!(
            "[RbConfig::CONFIG, RbConfig::MAKEFILE_CONFIG].each do |tg_config|\n  tg_config[\"bindir\"] = ENV.fetch(\"TEBAKO_DEPLOY_BINDIR\", {})\nend\n",
            rb_str(&self.staging_bin_dir.to_string_lossy())
        )
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
  # with the script as argument. mkmf derives srcdir from $0, so the
  # script takes over the program name before it is loaded.
  $0 = ARGV.first
  load ARGV.shift
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
}

//! Port of the gem's Packager + DeployHelper (lib/tebako/packager.rb,
//! lib/tebako/deploy_helper.rb): seed the packaging environment from the
//! resolved runtime's extracted layout, stage the application, run the
//! deploy ops under the runtime, strip, align the arch layout, write the
//! entry dispatcher and build the application image in-process (the
//! dwarfs-t Writer — no mkdwarfs binary anywhere).

use std::fs;
use std::path::{Path, PathBuf};

use crate::deploy::{Op, RuntimeDeployer};
use crate::error::{packaging_error, plain_error, TebakoError};
use crate::image::build_image;
use crate::options::PressOptions;
use crate::resolve::Resolved;
use crate::scenario::{api_version, Scenario, ScenarioManager};

/// Deploy the application and build its DwarFS image for stitching;
/// returns the image path (fs.tfs).
pub fn build_app_image(
    opts: &PressOptions,
    scenario: &mut ScenarioManager,
    resolved: &Resolved,
    ruby_ver: &str,
) -> Result<PathBuf, TebakoError> {
    let runtime_path = &resolved.executable;
    // Layout source (item 30b): the runtime image when the release is
    // image-era (extracted in-process into the packaging environment —
    // no extracted tree in the cache); otherwise the runtime's own
    // extracted layout (v1 flow, byte-identical to the gem).
    let image_path = resolved.image.as_ref().map(|img| {
        resolved
            .executable
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&img.filename)
    });
    let layout_dir = if image_path.is_none() {
        let resolver = crate::resolve::Resolver::new(crate::resolve::Flavor::Runtime);
        Some(resolver.layout(runtime_path, opts.verbose)?)
    } else {
        None
    };
    init(layout_dir.as_deref(), image_path.as_deref(), opts)?;
    // RuntimeSdk provenance (native-extension deploy): the runtime's
    // rbconfig comes from the mounted image (image-era — no extraction)
    // or the extracted layout (v1, the gem's flow).
    let rbconfig_source = match (image_path.as_deref(), layout_dir.as_deref()) {
        (Some(image), _) => Some(crate::sdk::RbconfigSource::Image(image.to_path_buf())),
        (None, Some(layout)) => Some(crate::sdk::RbconfigSource::Layout(layout.to_path_buf())),
        (None, None) => None,
    };
    deploy(opts, scenario, runtime_path, ruby_ver, rbconfig_source)?;
    if let Some(layout_dir) = &layout_dir {
        // Image-era seeds come from the runtime's own image, so the arch
        // layout matches by construction (alignment is a no-op).
        align_layout_to_runtime(&opts.data_src_dir(), layout_dir, ruby_ver);
    }
    write_entry_dispatcher(&opts.data_src_dir(), scenario, opts.cwd.as_deref());
    build_image(&opts.data_bundle_file(), &opts.data_src_dir())?;
    Ok(opts.data_bundle_file())
}

/// Init: recreate o/{s,r,p} and seed s/ from the runtime image
/// (image-era) or the extracted layout (v1).
fn init(
    layout_dir: Option<&Path>,
    image_path: Option<&Path>,
    opts: &PressOptions,
) -> Result<(), TebakoError> {
    println!("-- Running init script");
    let src = opts.data_src_dir();
    println!("   ... creating packaging environment at {}", src.display());
    for dir in [src.clone(), opts.data_pre_dir(), opts.data_bin_dir()] {
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)
            .map_err(|e| plain_error(format!("{e} creating {}", dir.display())))?;
    }
    match (image_path, layout_dir) {
        (Some(image), _) => {
            println!("   ... extracting the runtime image {}", image.display());
            extract_runtime_image(image, &src)
        }
        (None, Some(layout)) => cp_r_contents(layout, &src)
            .map_err(|e| plain_error(format!("{e} seeding {}", src.display()))),
        (None, None) => Err(plain_error("internal error: no layout source")),
    }
}

/// Mount the runtime image through the tfs C ABI and extract it into the
/// packaging environment (in-process; the image itself stays immutable
/// in the cache).
fn extract_runtime_image(image: &Path, dest: &Path) -> Result<(), TebakoError> {
    use tfs::c_api::*;

    let path = std::ffi::CString::new(image.to_string_lossy().as_bytes())
        .map_err(|_| plain_error(format!("invalid image path: {}", image.display())))?;
    let mount = std::ffi::CString::new("/mnt").unwrap();
    let rc = unsafe { tebako_fs_init_from_file(path.as_ptr(), mount.as_ptr()) };
    if rc != 0 {
        return Err(plain_error(format!(
            "cannot mount the runtime image {}",
            image.display()
        )));
    }
    struct Unmount;
    impl Drop for Unmount {
        fn drop(&mut self) {
            unsafe { tebako_fs_unmount() };
        }
    }
    let _guard = Unmount;

    let dest_c = std::ffi::CString::new(dest.to_string_lossy().as_bytes())
        .map_err(|_| plain_error(format!("invalid path: {}", dest.display())))?;
    let rc = unsafe { tebako_fs_extract_all(dest_c.as_ptr()) };
    if rc != 0 {
        let errno = unsafe { tebako_get_errno() };
        let message = unsafe {
            let ptr = tebako_strerror(errno);
            if ptr.is_null() {
                format!("errno {errno}")
            } else {
                std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        return Err(plain_error(format!(
            "cannot extract the runtime image {}: {message}",
            image.display()
        )));
    }
    Ok(())
}

/// Read the runtime's rbconfig.rb from the runtime image (image-era
/// RuntimeSdk provenance): the image is mounted in-process through the
/// tfs C ABI, the first `lib/ruby/<ver>/<arch>/rbconfig.rb` is read
/// straight from the mount, and the image is unmounted — no extraction
/// anywhere (item 30b; the cache holds the immutable artifact only).
/// `Ok(None)` when the image carries no rbconfig (the SDK maps that to
/// the named 135 error).
pub(crate) fn read_image_rbconfig(image: &Path) -> Result<Option<String>, TebakoError> {
    use tfs::c_api::*;

    let path = std::ffi::CString::new(image.to_string_lossy().as_bytes())
        .map_err(|_| plain_error(format!("invalid image path: {}", image.display())))?;
    let mount = std::ffi::CString::new("/mnt").unwrap();
    let rc = unsafe { tebako_fs_init_from_file(path.as_ptr(), mount.as_ptr()) };
    if rc != 0 {
        return Err(plain_error(format!(
            "cannot mount the runtime image {}",
            image.display()
        )));
    }
    struct Unmount;
    impl Drop for Unmount {
        fn drop(&mut self) {
            unsafe { tebako_fs_unmount() };
        }
    }
    let _guard = Unmount;

    // lib/ruby/<ver>/<arch>/rbconfig.rb, first lexical match (the gem's
    // sorted Dir.glob(...).first).
    for ver in image_dir_entries("/mnt/lib/ruby") {
        for arch in image_dir_entries(&format!("/mnt/lib/ruby/{ver}")) {
            let rel = format!("/mnt/lib/ruby/{ver}/{arch}/rbconfig.rb");
            if let Some(content) = image_read_file(&rel) {
                return Ok(Some(content));
            }
        }
    }
    Ok(None)
}

/// Sorted subdirectory names of a directory inside the mounted image.
fn image_dir_entries(path: &str) -> Vec<String> {
    use tfs::c_api::*;
    let Ok(c_path) = std::ffi::CString::new(path) else {
        return Vec::new();
    };
    let dir = unsafe { tebako_fs_opendir(c_path.as_ptr()) };
    if dir.is_null() {
        return Vec::new();
    }
    struct CloseDir(*mut std::ffi::c_void);
    impl Drop for CloseDir {
        fn drop(&mut self) {
            unsafe { tebako_fs_closedir(self.0) };
        }
    }
    let _guard = CloseDir(dir);
    let mut out = Vec::new();
    loop {
        let entry = unsafe { tebako_fs_readdir(dir) };
        if entry.is_null() {
            break;
        }
        let (name, d_type) = unsafe {
            let e = &*entry;
            let name = std::ffi::CStr::from_ptr(e.d_name.as_ptr())
                .to_string_lossy()
                .into_owned();
            (name, e.d_type)
        };
        if d_type == tfs::DT_DIR && name != "." && name != ".." {
            out.push(name);
        }
    }
    out.sort();
    out
}

/// Read a whole file from the mounted image (pread-chunked at 1 MiB).
fn image_read_file(path: &str) -> Option<String> {
    use tfs::c_api::*;
    let c_path = std::ffi::CString::new(path).ok()?;
    let fd = unsafe { tebako_fs_open(c_path.as_ptr(), 0) };
    if fd < 0 {
        return None;
    }
    struct CloseFd(libc::c_int);
    impl Drop for CloseFd {
        fn drop(&mut self) {
            unsafe { tebako_fs_close(self.0) };
        }
    }
    let _guard = CloseFd(fd);
    let mut out = Vec::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = unsafe { tebako_fs_read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    String::from_utf8(out).ok()
}

/// Deploy: stage the app, build and run the deploy ops, check, strip.
fn deploy(
    opts: &PressOptions,
    scenario: &mut ScenarioManager,
    runtime_path: &Path,
    ruby_ver: &str,
    rbconfig_source: Option<crate::sdk::RbconfigSource>,
) -> Result<(), TebakoError> {
    println!("-- Running deploy script");
    let target = opts.data_src_dir();
    let api = api_version(ruby_ver);
    let tbd = target.join("bin");
    let tgd = target.join("lib").join("ruby").join("gems").join(&api);
    let tld = target.join("local");

    // Bundler resolution happens here (ScenarioManagerWithBundler).
    scenario.resolve_bundler()?;

    verify_runtime_gem(&tgd)?;

    let mut ops: Vec<Op> = Vec::new();
    if scenario.needs_bundler {
        println!(
            "   ... installing bundler gem version {}",
            scenario.bundler_version
        );
        ops.push(Op::Gem(install_gem_argv(
            "bundler",
            Some(&scenario.bundler_version),
            &tgd,
            &tbd,
            opts.verbose,
        )));
    }
    if opts.verbose {
        ops.push(Op::Gem(vec!["env".to_string()]));
    }

    match scenario.scenario {
        Scenario::SimpleScript => {
            // DeployHelper prints OptionsManager#root (trailing slash kept)
            println!("   ... collecting simple Ruby script from {}", opts.root());
            copy_app_files(&scenario.fs_root, &tld)?;
        }
        Scenario::Gemfile => {
            println!("   ... deploying Gemfile");
            copy_app_files(&scenario.fs_root, &tld)?;
            ops.push(Op::Chdir(tld.to_string_lossy().into_owned()));
            let activation = bundler_activation(scenario);
            for opt in bundle_config_options(opts) {
                let mut argv = vec![
                    "config".to_string(),
                    "set".to_string(),
                    "--local".to_string(),
                ];
                argv.extend(opt);
                ops.push(Op::Bundle(activation.clone(), argv));
            }
            println!(
                "   *** It may take a long time for a big project. It takes REALLY long time on Windows ***"
            );
            ops.push(Op::Bundle(activation, bundle_install_argv(opts)));
        }
        // gem/gemspec scenarios need the bundle_exec op and the RuntimeSdk
        // (native builds) — a later milestone.
        Scenario::Gem | Scenario::Gemspec | Scenario::GemspecAndGemfile => {
            return Err(packaging_error(
                130,
                Some("gem/gemspec scenarios are not supported by this milestone (use a Gemfile root)"),
            ));
        }
    }

    if !ops.is_empty() {
        // Native-extension deploy (the gem's semantics): whenever deploy
        // ops run on POSIX, provision the runtime SDK so mkmf-driven
        // extension builds inside the driver compile against the
        // runtime's own header tree. Windows stays on the gem's no-SDK
        // branch (the ruby shim has no exec path there).
        let sdk = if cfg!(windows) {
            None
        } else {
            rbconfig_source
                .as_ref()
                .map(|source| {
                    crate::sdk::RuntimeSdk::resolve(
                        runtime_path,
                        source.clone(),
                        &opts.deps(),
                        ruby_ver,
                    )
                })
                .transpose()?
        };
        let deployer = RuntimeDeployer {
            runtime_path: runtime_path.to_path_buf(),
            staging_bin_dir: opts.data_bin_dir(),
            fs_mount_point: scenario.fs_mount_point.clone(),
            ruby_version: ruby_ver.to_string(),
            tebako_version: opts.tebako_version.clone(),
            verbose: opts.verbose,
            sdk,
        };
        deployer.execute(&ops, &deploy_env(&target, &api), &target)?;
    }

    check_solution(&target, scenario)?;
    check_cwd(&target, opts.cwd.as_deref())?;
    crate::strip::strip(&target, &scenario.exe_suffix);
    Ok(())
}

/// DeployHelper#verify_runtime_gem!: the runtime layout carries the
/// tebako-runtime gem pre-installed (error 129 otherwise).
fn verify_runtime_gem(tgd: &Path) -> Result<(), TebakoError> {
    let specs = tgd.join("specifications");
    let found = fs::read_dir(&specs)
        .map(|rd| {
            rd.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("tebako-runtime-")
                    && e.file_name().to_string_lossy().ends_with(".gemspec")
            })
        })
        .unwrap_or(false);
    if found {
        Ok(())
    } else {
        Err(packaging_error(129, Some(&specs.to_string_lossy())))
    }
}

/// DeployHelper#install_gem_op (+ install_argv_tail).
fn install_gem_argv(
    name: &str,
    version: Option<&str>,
    tgd: &Path,
    tbd: &Path,
    verbose: bool,
) -> Vec<String> {
    let mut argv = vec!["install".to_string(), name.to_string()];
    if let Some(v) = version {
        argv.push("-v".to_string());
        argv.push(v.to_string());
    }
    argv.push("--no-document".to_string());
    argv.push("--install-dir".to_string());
    argv.push(tgd.to_string_lossy().into_owned());
    argv.push("--bindir".to_string());
    argv.push(tbd.to_string_lossy().into_owned());
    if verbose {
        argv.push("--verbose".to_string());
    }
    if cfg!(windows) {
        argv.push("--platform".to_string());
        argv.push("ruby".to_string());
    }
    argv
}

/// The version the bundle ops activate ('_x.y.z_' when pinned, the
/// runtime's default otherwise).
fn bundler_activation(scenario: &ScenarioManager) -> Option<String> {
    if scenario.needs_bundler {
        Some(scenario.bundler_version.clone())
    } else {
        None
    }
}

/// DeployHelper#bundle_install_op. The gem passes --prefer-local
/// unconditionally (resolution then prefers the runtime's own gems: the
/// statically linked default extensions own their namespaces, and gems
/// the image already carries are used in place). Under --prefer-local a
/// remote (re)resolution restricts candidates to runtime-local gems and
/// backtracks to dependency-free versions — fontist 3.0.10 came out as
/// 0.1.0 — and the fetch layer can additionally fall back to the retired
/// rubygems dependency API (404 "The dependency API has gone away").
/// The default is therefore the modern compact-index resolution;
/// --prefer-local stays available as a press flag for apps whose native
/// dependencies are the runtime's bundled/default gems (used in place —
/// the only way those install without the RuntimeSdk). With a complete
/// lockfile the flag is a no-op: locked specs are installed as resolved.
fn bundle_install_argv(opts: &PressOptions) -> Vec<String> {
    let mut argv = vec!["install".to_string(), format!("--jobs={}", ncores())];
    if opts.prefer_local {
        argv.push("--prefer-local".to_string());
    }
    argv
}

/// DeployHelper#bundle_config_ops: ffi/nokogiri build hints (applied
/// when a gem without a precompiled platform variant is built from
/// source — the RuntimeSdk path), plus the openssl build config when a
/// libtfs-deps vcpkg tree is provisioned.
///
/// The gem additionally forces `force_ruby_platform true` on every
/// platform (tebako#343: precompiled variants can link against shared
/// system libraries the memfs does not carry). In the gem that is
/// viable because the RuntimeSdk ships the headers those source builds
/// need; the SDK is not ported here, so the forced setting only broke
/// precompiled platform gems (nokogiri attempted a source build with no
/// headers). Precompiled variants are the default again; bundler falls
/// back to the ruby (source) platform on its own for gems without one.
fn bundle_config_options(opts: &PressOptions) -> Vec<Vec<String>> {
    let nokogiri = if cfg!(windows) {
        "--use-system-libraries"
    } else {
        "--no-use-system-libraries"
    };
    let mut out = vec![
        vec![
            "build.ffi".to_string(),
            "--disable-system-libffi".to_string(),
        ],
        vec!["build.nokogiri".to_string(), nokogiri.to_string()],
    ];
    if let Some(dir) = openssl_dir(&opts.deps()) {
        out.push(vec![
            "build.openssl".to_string(),
            format!(
                "--with-openssl-dir={} --with-ldflags=-ldl -lz",
                dir.display()
            ),
        ]);
    }
    out
}

/// The libtfs-deps package provisioned by 'tebako setup' carries the
/// OpenSSL headers and static libraries the runtime itself was built with.
fn openssl_dir(deps: &Path) -> Option<PathBuf> {
    let vcpkg = deps.join("vcpkg_installed");
    let children = fs::read_dir(vcpkg).ok()?;
    for child in children.filter_map(|c| c.ok()) {
        let dir = child.path();
        if dir.join("include").join("openssl").join("ssl.h").is_file() {
            return Some(dir);
        }
    }
    None
}

/// DeployHelper#deploy_env: GEM_HOME/GEM_PATH/GEM_SPEC_CACHE plus the
/// press host's certificate store (the runtime's OpenSSL carries the
/// build machine's certificate paths).
fn deploy_env(target: &Path, api: &str) -> Vec<(String, String)> {
    let gem_home = target
        .join("lib")
        .join("ruby")
        .join("gems")
        .join(api)
        .to_string_lossy()
        .into_owned();
    let mut env = vec![
        ("GEM_HOME".to_string(), gem_home.clone()),
        ("GEM_PATH".to_string(), gem_home),
        (
            "GEM_SPEC_CACHE".to_string(),
            target.join("spec_cache").to_string_lossy().into_owned(),
        ),
    ];
    if let Some(cert_file) = default_cert_file() {
        env.push(("SSL_CERT_FILE".to_string(), cert_file));
    }
    if let Some(cert_dir) = default_cert_dir() {
        env.push(("SSL_CERT_DIR".to_string(), cert_dir));
    }
    env
}

/// OpenSSL::X509::DEFAULT_CERT_FILE of the press host: honor an explicit
/// setting, then probe the well-known locations.
fn default_cert_file() -> Option<String> {
    if let Ok(v) = std::env::var("SSL_CERT_FILE") {
        if !v.is_empty() && Path::new(&v).is_file() {
            return Some(v);
        }
    }
    const CANDIDATES: &[&str] = &[
        "/etc/ssl/certs/ca-certificates.crt",   // Debian/Ubuntu/Alpine
        "/etc/pki/tls/certs/ca-bundle.crt",     // Fedora/RHEL
        "/etc/ssl/ca-bundle.pem",               // openSUSE
        "/etc/ssl/cert.pem",                    // macOS (LibreSSL)
        "/opt/homebrew/etc/openssl@3/cert.pem", // Homebrew arm64
        "/usr/local/etc/openssl@3/cert.pem",    // Homebrew Intel
    ];
    CANDIDATES
        .iter()
        .find(|p| Path::new(p).is_file())
        .map(|s| s.to_string())
}

fn default_cert_dir() -> Option<String> {
    if let Ok(v) = std::env::var("SSL_CERT_DIR") {
        if !v.is_empty() && Path::new(&v).is_dir() {
            return Some(v);
        }
    }
    const CANDIDATES: &[&str] = &[
        "/etc/ssl/certs",
        "/opt/homebrew/etc/openssl@3/certs",
        "/usr/local/etc/openssl@3/certs",
        "/System/Library/OpenSSL/certs",
    ];
    CANDIDATES
        .iter()
        .find(|p| Path::new(p).is_dir())
        .map(|s| s.to_string())
}

/// DeployHelper#copy_files: cp -r <root>/. <dest> (error 107 when the
/// root is not a readable directory).
fn copy_app_files(fs_root: &str, dest: &Path) -> Result<(), TebakoError> {
    fs::create_dir_all(dest).map_err(|e| plain_error(format!("{e}")))?;
    let root = Path::new(fs_root);
    if !(root.is_dir() && fs::metadata(root).is_ok()) {
        return Err(TebakoError::new(
            format!("{fs_root} is not accessible or is not a directory."),
            107,
        ));
    }
    cp_r_contents(root, dest).map_err(|_| {
        TebakoError::new(
            format!("{fs_root} does not exist or is not accessible."),
            107,
        )
    })
}

/// cp_r "src/." dest: copy the CONTENTS (including dotfiles), preserving
/// symlinks and permission bits, like FileUtils.cp_r.
pub fn cp_r_contents(src: &Path, dest: &Path) -> std::io::Result<()> {
    for child in fs::read_dir(src)? {
        let child = child?;
        let target = dest.join(child.file_name());
        copy_entry(&child.path(), &target)?;
    }
    Ok(())
}

fn copy_entry(src: &Path, dest: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        let link = fs::read_link(src)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&link, dest)?;
        #[cfg(windows)]
        {
            if link.is_dir() {
                std::os::windows::fs::symlink_dir(&link, dest)?;
            } else {
                std::os::windows::fs::symlink_file(&link, dest)?;
            }
        }
        return Ok(());
    }
    if meta.is_dir() {
        fs::create_dir_all(dest)?;
        for child in fs::read_dir(src)? {
            let child = child?;
            copy_entry(&child.path(), &dest.join(child.file_name()))?;
        }
        return Ok(());
    }
    fs::copy(src, dest)?;
    fs::set_permissions(dest, meta.permissions())?;
    Ok(())
}

/// DeployHelper#check_solution: the entry point must exist post-deploy.
fn check_solution(target: &Path, scenario: &ScenarioManager) -> Result<(), TebakoError> {
    let root = match scenario.scenario {
        Scenario::SimpleScript | Scenario::Gemfile => "local",
        Scenario::Gem | Scenario::Gemspec | Scenario::GemspecAndGemfile => "bin",
    };
    let fs_entry = format!("{root}/{}", scenario.fs_entrance);
    println!(
        "   ... target entry point will be at {}/{}",
        scenario.fs_mount_point, fs_entry
    );
    if target.join(&fs_entry).exists() {
        Ok(())
    } else {
        Err(TebakoError::new(
            format!("Entry point {fs_entry} does not exist or is not accessible"),
            106,
        ))
    }
}

fn check_cwd(target: &Path, cwd: Option<&str>) -> Result<(), TebakoError> {
    let Some(cwd) = cwd else { return Ok(()) };
    if target.join(cwd).is_dir() {
        Ok(())
    } else {
        Err(TebakoError::new(
            format!("Package working directory {cwd} does not exist"),
            108,
        ))
    }
}

/// Packager.align_layout_to_runtime!: rename the image's arch directories
/// to the runtime's names and drop in the runtime's own rbconfig.rb.
fn align_layout_to_runtime(data_src_dir: &Path, layout_dir: &Path, ruby_ver: &str) {
    let api = api_version(ruby_ver);
    align_stdlib_arch(data_src_dir, layout_dir, &api);
    align_gem_ext_arch(data_src_dir, layout_dir, &api);
}

fn align_stdlib_arch(data_src_dir: &Path, layout_dir: &Path, api: &str) {
    let runtime_arch = arch_dir_of(
        &layout_dir.join("lib").join("ruby").join(api),
        "rbconfig.rb",
    );
    let image_arch = arch_dir_of(
        &data_src_dir.join("lib").join("ruby").join(api),
        "rbconfig.rb",
    );
    let (Some(runtime_arch), Some(image_arch)) = (runtime_arch, image_arch) else {
        return;
    };
    if runtime_arch == image_arch {
        return;
    }
    println!("   ... aligning app image layout to the runtime ({image_arch} -> {runtime_arch})");
    let base = data_src_dir.join("lib").join("ruby").join(api);
    let _ = fs::rename(base.join(&image_arch), base.join(&runtime_arch));
    let _ = fs::copy(
        layout_dir
            .join("lib")
            .join("ruby")
            .join(api)
            .join(&runtime_arch)
            .join("rbconfig.rb"),
        base.join(&runtime_arch).join("rbconfig.rb"),
    );
}

fn align_gem_ext_arch(data_src_dir: &Path, layout_dir: &Path, api: &str) {
    let img_ext = data_src_dir
        .join("lib")
        .join("ruby")
        .join("gems")
        .join(api)
        .join("extensions");
    let rt_ext = layout_dir
        .join("lib")
        .join("ruby")
        .join("gems")
        .join(api)
        .join("extensions");
    let Some(runtime_ext) = first_dir(&rt_ext) else {
        return;
    };
    if !img_ext.is_dir() {
        return;
    }
    let Ok(children) = fs::read_dir(&img_ext) else {
        return;
    };
    for child in children.filter_map(|c| c.ok()) {
        let name = child.file_name().to_string_lossy().into_owned();
        if name != runtime_ext && child.path().is_dir() {
            let _ = fs::rename(child.path(), img_ext.join(&runtime_ext));
        }
    }
}

fn arch_dir_of(dir: &Path, marker: &str) -> Option<String> {
    let children = fs::read_dir(dir).ok()?;
    for child in children.filter_map(|c| c.ok()) {
        let name = child.file_name().to_string_lossy().into_owned();
        if dir.join(&name).join(marker).is_file() {
            return Some(name);
        }
    }
    None
}

fn first_dir(dir: &Path) -> Option<String> {
    let children = fs::read_dir(dir).ok()?;
    for child in children.filter_map(|c| c.ok()) {
        if child.path().is_dir() {
            return Some(child.file_name().to_string_lossy().into_owned());
        }
    }
    None
}

/// Packager.write_entry_dispatcher: the runtime's compiled-in entry point
/// is /local/stub.rb; the dispatcher receives control and loads the real
/// entry point (replicating the bundle-mode working directory when --cwd
/// was given).
fn write_entry_dispatcher(data_src_dir: &Path, scenario: &ScenarioManager, cwd: Option<&str>) {
    let mut dispatcher = String::new();
    if let Some(cwd) = cwd {
        dispatcher.push_str(&format!(
            "Dir.chdir(\"{}/{cwd}\")\n",
            scenario.fs_mount_point
        ));
    }
    dispatcher.push_str(&format!(
        "load \"{}{}\"\n",
        scenario.fs_mount_point, scenario.fs_entry_point
    ));
    let local = data_src_dir.join("local");
    let _ = fs::create_dir_all(&local);
    let _ = fs::write(local.join("stub.rb"), dispatcher);
}

/// ScenarioManagerBase#ncores (sysctl/nproc, 4 on failure).
fn ncores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::PressMode;

    fn test_opts(prefer_local: bool) -> PressOptions {
        PressOptions {
            root_arg: "/tmp/root".to_string(),
            entrance: "main.rb".to_string(),
            output: None,
            prefix: PathBuf::from("/tmp/prefix"),
            cwd: None,
            ruby_requested: None,
            mode: PressMode::Lean,
            log_level: "error".to_string(),
            image_specs: Vec::new(),
            bootstrap: None,
            suite: None,
            tebako_version: crate::DEFAULT_TEBAKO_VERSION.to_string(),
            prefer_local,
            verbose: false,
            devmode: false,
            fs_current: "/tmp".to_string(),
        }
    }

    #[test]
    fn bundle_install_uses_the_compact_index_by_default() {
        // The retired rubygems dependency API is only reachable through
        // --prefer-local; the default resolution must not pass it
        // (fontist resolved to the dependency-free 0.1.0 otherwise).
        let argv = bundle_install_argv(&test_opts(false));
        assert_eq!(argv[0], "install");
        assert!(argv[1].starts_with("--jobs="), "{argv:?}");
        assert!(!argv.iter().any(|a| a == "--prefer-local"), "{argv:?}");
    }

    #[test]
    fn bundle_install_prefer_local_is_opt_in() {
        // The gem-era behavior stays available behind the press flag
        // (the runtime's default gems own their namespaces).
        let argv = bundle_install_argv(&test_opts(true));
        assert!(argv.iter().any(|a| a == "--prefer-local"), "{argv:?}");
    }

    #[test]
    fn bundle_config_does_not_force_the_ruby_platform() {
        // force_ruby_platform=true made every native gem attempt a
        // source build; the RuntimeSdk that supplied the headers is not
        // ported, so precompiled platform gems must be the default.
        let opts = bundle_config_options(&test_opts(false));
        assert!(
            !opts.iter().any(|kv| kv[0] == "force_ruby_platform"),
            "{opts:?}"
        );
        // The build hints for the SDK source-build path stay.
        assert!(opts.iter().any(|kv| kv[0] == "build.ffi"));
        assert!(opts.iter().any(|kv| kv[0] == "build.nokogiri"));
    }
}

//! Port of the gem's Stripper (lib/tebako/stripper.rb): removes build
//! artefacts from the packaging environment and strips shared objects.

use std::path::Path;
use std::process::Command;

const DELETE_EXTENSIONS: [&str; 6] = ["o", "lo", "obj", "a", "la", "lib"];
const BIN_FILES: [&str; 15] = [
    "bundle",
    "bundler",
    "rbs",
    "erb",
    "gem",
    "irb",
    "racc",
    "racc2y",
    "rake",
    "rdoc",
    "ri",
    "y2racc",
    "rdbg",
    "syntax_suggest",
    "typeprof",
];

pub fn strip(src_dir: &Path, exe_suffix: &str) {
    println!("   ... stripping the output");
    strip_bs(src_dir);
    strip_fi(src_dir, exe_suffix);
    strip_li(src_dir);
}

fn strip_file(file: &Path) {
    let out = Command::new("strip").args(["-S"]).arg(file).output();
    match out {
        Ok(o) if o.status.success() => {
            resign_after_strip(file);
        }
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout).into_owned()
                + &String::from_utf8_lossy(&o.stderr);
            println!(
                "Warning: could not strip {}:\n {}",
                file.display(),
                text.trim_end()
            );
        }
        Err(e) => {
            println!("Warning: could not strip {}:\n {}", file.display(), e);
        }
    }
}

/// strip -S rewrites the file and thereby invalidates any embedded code
/// signature ("changes being made to the file will invalidate the code
/// signature"). On macOS arm64 every mapped page must be signed — the
/// kernel kills the dlopen of a modified-after-sign .bundle (AMFI
/// cs_invalid_page), which breaks precompiled platform gems (nokogiri &
/// co.) at package runtime. Re-sign ad-hoc, best-effort, like the
/// stitcher's package re-sign.
#[cfg(target_os = "macos")]
fn resign_after_strip(file: &Path) {
    let ok = Command::new("codesign")
        .args(["--sign", "-", "--force"])
        .arg(file)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        println!(
            "Warning: could not ad-hoc re-sign {} after stripping",
            file.display()
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn resign_after_strip(_file: &Path) {}

fn strip_bs(src_dir: &Path) {
    let _ = std::fs::remove_dir_all(src_dir.join("share"));
    let _ = std::fs::remove_dir_all(src_dir.join("include"));
    let _ = std::fs::remove_dir_all(src_dir.join("lib").join("pkgconfig"));
}

fn strip_fi(src_dir: &Path, exe_suffix: &str) {
    let bin = src_dir.join("bin");
    for f in BIN_FILES {
        for name in [f.to_string(), format!("{f}.cmd"), format!("{f}.bat")] {
            let _ = std::fs::remove_file(bin.join(name));
        }
    }
    let _ = std::fs::remove_file(bin.join(format!("ruby{exe_suffix}")));
    let _ = std::fs::remove_file(bin.join(format!("rubyw{exe_suffix}")));
}

fn strip_li(src_dir: &Path) {
    let strip_exts: Vec<&str> = if cfg!(target_os = "macos") {
        vec!["so", "dylib", "bundle"]
    } else if cfg!(windows) {
        vec!["so", "dll"]
    } else {
        vec!["so"]
    };
    walk(src_dir, &mut |file| {
        let ext = file
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if DELETE_EXTENSIONS.contains(&ext.as_str()) {
            let _ = std::fs::remove_file(file);
        } else if strip_exts.contains(&ext.as_str()) {
            strip_file(file);
        }
    });
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(children) = std::fs::read_dir(dir) else {
        return;
    };
    for child in children.filter_map(|c| c.ok()) {
        let path = child.path();
        if path.is_dir() {
            walk(&path, f);
        } else {
            f(&path);
        }
    }
}

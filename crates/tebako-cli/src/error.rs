//! Tebako error type mirroring the gem's `Tebako::Error` (message + exit
//! code) and the PACKAGING_ERRORS table of lib/tebako.rb.

use std::fmt;

#[derive(Debug)]
pub struct TebakoError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for TebakoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TebakoError {}

impl TebakoError {
    pub fn new(message: impl Into<String>, code: i32) -> Self {
        TebakoError {
            message: message.into(),
            code,
        }
    }
}

/// The gem's PACKAGING_ERRORS table (lib/tebako.rb).
pub fn packaging_message(code: i32) -> Option<&'static str> {
    Some(match code {
        101 => "'tebako setup' configure step failed",
        102 => "'tebako setup' build step failed",
        105 => "Failed to map MSys path to Windows",
        106 => "Entry point does not exist or is not accessible",
        107 => "Project root does not exist or is not accessible",
        108 => "Package working directory does not exist",
        109 => "Invalid Ruby version format",
        110 => "Ruby version is not supported",
        111 => "Ruby version is not supported on Windows",
        112 => "OS is not supported",
        113 => "Path to root shall be absolute. Relative path is not allowed",
        114 => "Entry point is not within the project root",
        115 => "Failed to load Gemfile",
        116 => "Ruby version does not satify Gemfile requirements",
        117 => "Failed to load Gemfile.lock",
        118 => "Bundler version in Gemfile.lock does satisfy minimal Tebako version requirememnts",
        119 => "Failed to find compatible bundler version",
        120 => "No prebuilt tebako runtime package for the requested Ruby/platform combination",
        121 => "SHA256 checksum mismatch for downloaded tebako runtime package",
        122 => "Failed to download tebako runtime package",
        123 => "TEBAKO_OFFLINE is set and the requested tebako runtime package is not cached",
        124 => "Runtime package release carries no usable package index",
        125 => "Timed out waiting for the runtime package cache lock",
        126 => "Invalid stitch specification",
        127 => "Stitch input file is not accessible",
        128 => "Prebuilt runtime press requires the packaging environment (run 'tebako setup' first)",
        129 => "The resolved runtime package does not carry the tebako-runtime gem",
        130 => "Option combination is not supported",
        131 => "No tebako-bootstrap package for the requested platform",
        132 => "TEBAKO_OFFLINE is set and the requested tebako-bootstrap package is not cached",
        133 => "The 'runtime' press mode was removed: runtime packages are produced by the \
                 tebako-runtime-ruby pipeline and resolved automatically by the lean/fat/classic modes",
        134 => "Fat mode requires a payload-capable tebako-bootstrap release",
        135 => "Failed to provision the runtime SDK for native extension builds",
        201 => "Warning. Could not create cache version file",
        _ => return None,
    })
}

/// Mirror of `Tebako.packaging_error(code, extm)`: the table message plus
/// an optional ": <extm>" suffix.
pub fn packaging_error(code: i32, extm: Option<&str>) -> TebakoError {
    let mut msg = packaging_message(code)
        .unwrap_or("Unknown packaging error")
        .to_string();
    if let Some(ext) = extm {
        msg.push_str(": ");
        msg.push_str(ext);
    }
    TebakoError::new(msg, code)
}

/// Plain error with the gem's default exit code (255).
pub fn plain_error(message: impl Into<String>) -> TebakoError {
    TebakoError::new(message, 255)
}

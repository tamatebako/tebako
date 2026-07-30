//! The handoff argv grammar (spec 17 §1).
//!
//! ```text
//! --tebako-image <self|image-path>:<slot|->:<mount>   (repeatable)
//! --tebako-entry <argv0> <user args...>               (terminates scanning)
//! ```
//!
//! Both `--flag value` and `--flag=value` forms are accepted (the v1
//! driver's `match_launcher_arg` took both). Values split on the LAST TWO
//! colons so the file component may itself contain colons (Windows drive
//! prefixes). Anything before the first `--tebako-*` belongs to the
//! interpreter; everything after `--tebako-entry` belongs to the user,
//! verbatim.

use std::path::PathBuf;

use crate::driver::DriverError;
use crate::EX_TEBAKO_MANIFEST;

/// The slot component of a triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRef {
    /// `-` — the whole file (bare images only; a packaged file is a
    /// named error at mount time).
    Whole,
    /// A numeric slot. On a bare file `0` ≡ [`SlotRef::Whole`] (spec 07
    /// §0: registry payloads mount whole); on a packaged file it names
    /// the trailer-described region.
    Slot(u32),
}

/// Where an image's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    /// `<self>:<slot>` — a slot of the running executable's own package.
    OwnSlot(u32),
    /// `<image-path>:<slot|->` — a standalone image file.
    File(PathBuf, SlotRef),
}

/// One `--tebako-image` triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSpec {
    pub source: ImageSource,
    /// The mount point inside the jail namespace (never empty).
    pub mount: String,
}

/// The parsed handoff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Handoff {
    /// Image triples in argv order; the FIRST is the app payload the
    /// entry resolves against.
    pub images: Vec<ImageSpec>,
    /// The `--tebako-entry` value (the entrypoint inside the mounted
    /// tree), when given.
    pub entry: Option<String>,
    /// Everything after `--tebako-entry`, verbatim.
    pub user_args: Vec<String>,
}

fn malformed(value: &str) -> DriverError {
    DriverError::new(
        EX_TEBAKO_MANIFEST,
        format!(
            "malformed --tebako-image value '{value}' — expected <self|image-path>:<slot|->:<mount-point>"
        ),
    )
}

/// Split `<file>:<slot>:<mount>` on the last two colons.
fn split_triple(value: &str) -> Result<ImageSpec, DriverError> {
    let Some(last) = value.rfind(':') else {
        return Err(malformed(value));
    };
    if last == 0 {
        return Err(malformed(value));
    }
    let Some(prev) = value[..last].rfind(':') else {
        return Err(malformed(value));
    };
    let file = &value[..prev];
    let slot = &value[prev + 1..last];
    let mount = &value[last + 1..];
    if file.is_empty() || mount.is_empty() {
        return Err(malformed(value));
    }
    if file == "self" {
        let n: u32 = slot.parse().map_err(|_| malformed(value))?;
        return Ok(ImageSpec {
            source: ImageSource::OwnSlot(n),
            mount: mount.to_string(),
        });
    }
    let slot = if slot == "-" {
        SlotRef::Whole
    } else {
        match slot.parse::<u32>() {
            Ok(n) => SlotRef::Slot(n),
            Err(_) => return Err(malformed(value)),
        }
    };
    Ok(ImageSpec {
        source: ImageSource::File(PathBuf::from(file), slot),
        mount: mount.to_string(),
    })
}

impl Handoff {
    /// Parse the loader-consumed prefix of `argv` (spec 17 §1). `argv[0]`
    /// is the program name and is always skipped. Loader flags are
    /// recognized wherever they appear before `--tebako-entry` (the
    /// interpreter's own args are skipped); `--tebako-entry` terminates
    /// the scan — everything after it belongs to the user, verbatim.
    /// `--tebako-extract` belongs to the interpreter (spec 06 §1) and is
    /// skipped like any other non-loader arg; an unknown `--tebako-*`
    /// flag is a named error, never silently ignored.
    pub fn parse(argv: &[String]) -> Result<Handoff, DriverError> {
        let mut h = Handoff::default();
        let mut i = 1;
        while i < argv.len() {
            let arg = &argv[i];
            let (flag, inline) = match arg.split_once('=') {
                Some((f, v)) => (f, Some(v.to_string())),
                None => (arg.as_str(), None),
            };
            match flag {
                "--tebako-image" => {
                    let value = take_value(flag, inline, argv, &mut i)?;
                    h.images.push(split_triple(&value)?);
                    i += 1;
                }
                "--tebako-entry" => {
                    let value = take_value(flag, inline, argv, &mut i)?;
                    if value.is_empty() {
                        return Err(DriverError::new(
                            EX_TEBAKO_MANIFEST,
                            "--tebako-entry shall be followed by the entrypoint path".to_string(),
                        ));
                    }
                    h.entry = Some(value);
                    h.user_args.extend(argv[i + 1..].iter().cloned());
                    break;
                }
                "--tebako-extract" => {
                    // The runtime-side option (spec 06 §1): the
                    // interpreter handles it; the driver never does.
                    i += 1;
                }
                _ if flag.starts_with("--tebako-") => {
                    return Err(DriverError::new(
                        EX_TEBAKO_MANIFEST,
                        format!("unknown loader option '{flag}' — the handoff grammar is --tebako-image/--tebako-entry (spec 17 §1)"),
                    ));
                }
                _ => {
                    // The interpreter's own args — not the loader's.
                    i += 1;
                }
            }
        }
        Ok(h)
    }
}

fn take_value(
    flag: &str,
    inline: Option<String>,
    argv: &[String],
    i: &mut usize,
) -> Result<String, DriverError> {
    match inline {
        Some(v) => Ok(v),
        None => {
            *i += 1;
            argv.get(*i).cloned().ok_or_else(|| {
                DriverError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("{flag} shall be followed by a value"),
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plain_argv_is_an_empty_handoff() {
        let h = Handoff::parse(&argv(&["ruby", "--version"])).unwrap();
        assert_eq!(h, Handoff::default());
    }

    #[test]
    fn one_triple_plus_entry_and_user_args() {
        let h = Handoff::parse(&argv(&[
            "ruby",
            "--tebako-image",
            "/cache/payloads/mn/1.0.tfs:0:/",
            "--tebako-entry",
            "/bin/metanorma",
            "--version",
        ]))
        .unwrap();
        assert_eq!(h.images.len(), 1);
        assert_eq!(
            h.images[0].source,
            ImageSource::File(
                PathBuf::from("/cache/payloads/mn/1.0.tfs"),
                SlotRef::Slot(0)
            )
        );
        assert_eq!(h.images[0].mount, "/");
        assert_eq!(h.entry.as_deref(), Some("/bin/metanorma"));
        assert_eq!(h.user_args, argv(&["--version"]));
    }

    #[test]
    fn dash_slot_and_inline_form() {
        let h = Handoff::parse(&argv(&["ruby", "--tebako-image=/x/y.tfs:-:/opt/x"])).unwrap();
        assert_eq!(
            h.images[0].source,
            ImageSource::File(PathBuf::from("/x/y.tfs"), SlotRef::Whole)
        );
        assert_eq!(h.images[0].mount, "/opt/x");
    }

    #[test]
    fn windows_drive_colon_survives() {
        let h = Handoff::parse(&argv(&[
            "ruby.exe",
            "--tebako-image",
            "C:/img/a.tfs:0:/app",
        ]))
        .unwrap();
        assert_eq!(
            h.images[0].source,
            ImageSource::File(PathBuf::from("C:/img/a.tfs"), SlotRef::Slot(0))
        );
    }

    #[test]
    fn self_slot() {
        let h = Handoff::parse(&argv(&["ruby", "--tebako-image", "self:2:/data"])).unwrap();
        assert_eq!(h.images[0].source, ImageSource::OwnSlot(2));
    }

    #[test]
    fn tebako_extract_is_the_interpreters_not_the_drivers() {
        let h = Handoff::parse(&argv(&["ruby", "--tebako-extract", "dest"])).unwrap();
        assert_eq!(h, Handoff::default());
    }

    #[test]
    fn unknown_tebako_flag_is_named_65() {
        let err = Handoff::parse(&argv(&["ruby", "--tebako-imag", "x:0:/"])).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
        assert!(
            err.message.contains("unknown loader option"),
            "{}",
            err.message
        );
    }

    #[test]
    fn malformed_values_are_named_65() {
        for bad in [
            "/x/y.tfs:/",   // too few components
            "/x/y.tfs:q:/", // non-numeric slot
            ":/",           // empty everything
            "/x/y.tfs::/",  // empty slot
            "/x/y.tfs:0:",  // empty mount
            "self:-:/",     // self needs a numeric slot
        ] {
            let err = Handoff::parse(&argv(&["ruby", "--tebako-image", bad])).unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{bad}");
        }
    }

    #[test]
    fn missing_value_is_named_65() {
        let err = Handoff::parse(&argv(&["ruby", "--tebako-image"])).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
        let err = Handoff::parse(&argv(&["ruby", "--tebako-entry"])).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST);
    }
}

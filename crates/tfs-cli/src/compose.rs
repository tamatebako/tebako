//! The compose file (spec 23 §9): declarative composition for
//! `tfs exec --compose` — the post-bake swap channel at the image layer.
//!
//! A compose file declares exactly what `tfs exec` otherwise takes on the
//! command line, plus the spec 23 `needs:` spelling:
//!
//! ```yaml
//! images:                       # the slices to mount
//!   - /img/app.tfs:/app         # string form: image[:mount]
//!   - image: /img/data.tfs      # map form
//!     mount: /data
//! policy: deny                  # open | deny | record (default: open;
//!                               # deny when needs/mounts appear alone)
//! mounts:                       # extra host grants (docker -v)
//!   - host: /srv/cache
//!     access: rw                # mount point defaults to the host path
//! needs:                        # the payload's declared host needs
//!   host:
//!     - path: "$HOME/.config/app"
//!       access: ro
//!       why: "reads its config" # documentation, ignored by the engine
//! ```
//!
//! MECE at this layer: only `images`, `policy`, `mounts`, `needs` are
//! known keys — slice NAMES and entrypoints resolve at the shim layer, so
//! a compose file naming `runtime:`, `slices:`, or `entrypoint:` earns a
//! named error pointing there. `$HOME`/`$TMPDIR`/`$CWD` atoms in host and
//! image paths expand at compose time. Needs entries lower to identity
//! host-mount grants (the host path exposed at itself). `record` carries
//! no grants (the same rule as the env grammar — inert configuration is a
//! named error).
//!
//! The compose file is the whole composition: combining `--compose` with
//! `--image`/`--jail` is a named error (one source of truth per run).

use tfs::policy::{HostAccess, HostMountSpec, JailSpec, PolicyDefault};

/// What a compose file declares, lowered to the `tfs exec` operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeSpec {
    /// `image[:mount]` tokens (the same form `tfs exec` positionals take;
    /// atoms already expanded).
    pub images: Vec<String>,
    /// The jail to bind, when the file declares `policy`, `mounts`, or
    /// `needs`; `None` = no policy installed (today's behavior).
    pub jail: Option<JailSpec>,
}

/// The serde model. `deny_unknown_fields` is the MECE gate: a key this
/// layer does not own is a named error, never silently ignored.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ComposeFile {
    images: Option<Vec<ImageEntry>>,
    policy: Option<String>,
    mounts: Option<Vec<MountEntry>>,
    needs: Option<Needs>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ImageEntry {
    /// The string form: `image[:mount]`.
    Token(String),
    /// The map form; a missing mount lowers to the bare image token.
    Map {
        image: String,
        mount: Option<String>,
    },
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MountEntry {
    host: String,
    /// Defaults to the host path itself (the identity mount).
    mount: Option<String>,
    access: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Needs {
    host: Option<Vec<NeedEntry>>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NeedEntry {
    path: String,
    access: String,
    /// `true` = the grant applies only where the path exists at compose
    /// time (probe artifacts — the generator marks them; the floor's
    /// courtesy-surface rule). Absent without it: a named error.
    optional: Option<bool>,
    /// Documentation for the reviewer (spec 23 §3) — the engine ignores it.
    #[allow(dead_code)]
    why: Option<String>,
}

/// Parse a compose file. `expand` resolves the symbolic atoms (`$HOME`,
/// `$TMPDIR`, `$CWD`) — the caller binds it to the process environment so
/// tests never mutate it. `exists` probes the host at compose time (the
/// CLI binds it to canonicalize): absent hosts named by `optional: true`
/// needs are skipped; absent hosts otherwise are named errors.
pub fn parse_compose(
    yaml: &str,
    expand: &dyn Fn(&str) -> Option<String>,
    exists: &dyn Fn(&str) -> bool,
) -> Result<ComposeSpec, String> {
    let file: ComposeFile = serde_yaml::from_str(yaml).map_err(|e| {
        let msg = format!("cannot parse the compose file: {e}");
        if msg.contains("unknown field") {
            format!(
                "{msg} — slice names, runtimes, and entrypoints resolve at the shim layer; tfs exec composes images"
            )
        } else {
            msg
        }
    })?;

    let mut images = Vec::new();
    for entry in file.images.unwrap_or_default() {
        match entry {
            ImageEntry::Token(t) => images.push(expand_atoms(&t, expand)?),
            ImageEntry::Map { image, mount } => {
                let image = expand_atoms(&image, expand)?;
                images.push(match mount {
                    Some(m) => format!("{image}:{m}"),
                    None => image,
                });
            }
        }
    }

    let policy = match file.policy.as_deref() {
        None => None,
        Some("open") => Some(PolicyDefault::Open),
        Some("deny") => Some(PolicyDefault::Deny),
        Some("record") => Some(PolicyDefault::Record),
        Some(other) => {
            return Err(format!(
                "unknown policy {other:?} in the compose file (want open|deny|record)"
            ))
        }
    };

    let mut grants: Vec<HostMountSpec> = Vec::new();
    for m in file.mounts.unwrap_or_default() {
        let host = expand_atoms(&m.host, expand)?;
        if !exists(&host) {
            return Err(format!(
                "mount host {host:?} does not resolve at compose time (a probe-only path belongs in needs with `optional: true`)"
            ));
        }
        grants.push(HostMountSpec {
            mount: m.mount.unwrap_or_else(|| host.clone()),
            host: host.into(),
            access: parse_access(&m.access)?,
        });
    }
    for n in file.needs.and_then(|n| n.host).unwrap_or_default() {
        let host = expand_atoms(&n.path, expand)?;
        if !exists(&host) {
            if n.optional == Some(true) {
                continue; // a probe artifact, absent here — courtesy skip
            }
            return Err(format!(
                "needs entry {host:?} does not resolve at compose time (mark it `optional: true` if the path is a probe)"
            ));
        }
        grants.push(HostMountSpec {
            mount: host.clone(),
            host: host.into(),
            access: parse_access(&n.access)?,
        });
    }

    if policy == Some(PolicyDefault::Record) && !grants.is_empty() {
        return Err(
            "record carries no grants: mounts and needs are inert under it — drop them".to_string(),
        );
    }
    let jail = if policy.is_some() || !grants.is_empty() {
        Some(JailSpec {
            // Jailed-safe by default (spec 23 §2): needs/mounts without an
            // explicit policy mean the declared grants are the ONLY
            // openings.
            default: policy.unwrap_or(PolicyDefault::Deny),
            mounts: grants,
            arg_files: Vec::new(),
        })
    } else {
        None
    };
    Ok(ComposeSpec { images, jail })
}

/// Expand a leading `$ATOM` (the manifest grammar's atoms, spec 23 §4)
/// using `expand`; an unknown atom is a named error. Atoms are path
/// prefixes: bare `$HOME` or `$HOME/…`.
fn expand_atoms(path: &str, expand: &dyn Fn(&str) -> Option<String>) -> Result<String, String> {
    let Some(rest) = path.strip_prefix('$') else {
        return Ok(path.to_string());
    };
    let (atom, tail) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    match expand(atom) {
        Some(base) => Ok(format!("{base}{tail}")),
        None => Err(format!(
            "unknown atom ${atom} in {path:?} (the compose file speaks $HOME, $TMPDIR, $CWD)"
        )),
    }
}

/// The `ro|rw` grant bit, as the compose file spells it.
fn parse_access(access: &str) -> Result<HostAccess, String> {
    match access {
        "ro" => Ok(HostAccess::Ro),
        "rw" => Ok(HostAccess::Rw),
        other => Err(format!("unknown access {other:?} (want ro|rw)")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The atom lookup seam: HOME/TMPDIR/CWD from a fixed table.
    fn expand(atom: &str) -> Option<String> {
        match atom {
            "HOME" => Some("/Users/u".to_string()),
            "TMPDIR" => Some("/tmp/x".to_string()),
            "CWD" => Some("/work/dir".to_string()),
            _ => None,
        }
    }

    #[test]
    fn full_compose_lowers_to_images_and_jail() {
        let spec = parse_compose(
            r#"
images:
  - /img/app.tfs:/app
  - image: /img/data.tfs
    mount: /data
policy: deny
mounts:
  - host: /srv/cache
    access: rw
needs:
  host:
    - path: "$HOME/.config/app"
      access: ro
      why: "reads its config"
"#,
            &expand,
            &|_| true,
        )
        .unwrap();
        assert_eq!(
            spec.images,
            vec!["/img/app.tfs:/app", "/img/data.tfs:/data"]
        );
        let jail = spec.jail.expect("a jail is declared");
        assert_eq!(jail.default, PolicyDefault::Deny);
        assert_eq!(
            jail.mounts,
            vec![
                HostMountSpec {
                    host: "/srv/cache".into(),
                    mount: "/srv/cache".to_string(),
                    access: HostAccess::Rw,
                },
                HostMountSpec {
                    host: "/Users/u/.config/app".into(),
                    mount: "/Users/u/.config/app".to_string(),
                    access: HostAccess::Ro,
                },
            ]
        );
        assert!(jail.arg_files.is_empty());
    }

    #[test]
    fn string_and_map_image_forms() {
        let spec = parse_compose(
            r#"
images:
  - /a.tfs
  - /b.tfs:/b
  - image: /c.tfs
"#,
            &expand,
            &|_| true,
        )
        .unwrap();
        // A bare string stays a bare token (exec defaults the mount); the
        // map form without mount lowers to the bare image token.
        assert_eq!(spec.images, vec!["/a.tfs", "/b.tfs:/b", "/c.tfs"]);
        assert_eq!(spec.jail, None);
    }

    #[test]
    fn unknown_top_level_key_is_a_named_error_naming_the_shim_layer() {
        let e = parse_compose("runtime: ruby-4.0\n", &expand, &|_| true).unwrap_err();
        assert!(e.contains("runtime"), "{e}");
        assert!(e.contains("shim layer"), "{e}");
    }

    #[test]
    fn record_carries_no_grants() {
        let e = parse_compose(
            "policy: record\nneeds:\n  host:\n    - path: /x\n      access: ro\n",
            &expand,
            &|_| true,
        )
        .unwrap_err();
        assert!(e.contains("record carries no grants"), "{e}");
        // …but a bare record policy is fine.
        let spec = parse_compose("policy: record\n", &expand, &|_| true).unwrap();
        assert_eq!(spec.jail.unwrap().default, PolicyDefault::Record);
    }

    #[test]
    fn atoms_expand_in_host_paths_and_images() {
        let spec = parse_compose(
            "images:\n  - $HOME/img/app.tfs:/app\nneeds:\n  host:\n    - path: $TMPDIR/scratch\n      access: rw\n",
            &expand,
            &|_| true,
        )
        .unwrap();
        assert_eq!(spec.images, vec!["/Users/u/img/app.tfs:/app"]);
        assert_eq!(
            spec.jail.unwrap().mounts[0].host,
            std::path::PathBuf::from("/tmp/x/scratch")
        );
    }

    #[test]
    fn unknown_atom_is_a_named_error() {
        let e = parse_compose(
            "needs:\n  host:\n    - path: $QUX/x\n      access: ro\n",
            &expand,
            &|_| true,
        )
        .unwrap_err();
        assert!(e.contains("$QUX"), "{e}");
    }

    #[test]
    fn bad_access_and_policy_are_named_errors() {
        let e = parse_compose(
            "needs:\n  host:\n    - path: /x\n      access: rx\n",
            &expand,
            &|_| true,
        )
        .unwrap_err();
        assert!(e.contains("rx"), "{e}");
        assert!(e.contains("ro|rw"), "{e}");
        let e = parse_compose("policy: maybe\n", &expand, &|_| true).unwrap_err();
        assert!(e.contains("maybe"), "{e}");
        assert!(e.contains("open|deny|record"), "{e}");
    }

    #[test]
    fn needs_without_policy_default_to_deny() {
        // Jailed-safe by default: declaring needs and no policy means the
        // needs are the ONLY grants (spec 23 §2).
        let spec = parse_compose(
            "needs:\n  host:\n    - path: /x\n      access: ro\n",
            &expand,
            &|_| true,
        )
        .unwrap();
        let jail = spec.jail.unwrap();
        assert_eq!(jail.default, PolicyDefault::Deny);
        assert_eq!(jail.mounts.len(), 1);
    }

    #[test]
    fn explicit_open_policy_with_needs_stays_open() {
        let spec = parse_compose(
            "policy: open\nneeds:\n  host:\n    - path: /x\n      access: ro\n",
            &expand,
            &|_| true,
        )
        .unwrap();
        assert_eq!(spec.jail.unwrap().default, PolicyDefault::Open);
    }

    #[test]
    fn empty_compose_is_an_empty_spec() {
        let spec = parse_compose("{}\n", &expand, &|_| true).unwrap();
        assert!(spec.images.is_empty());
        assert_eq!(spec.jail, None);
    }

    #[test]
    fn optional_needs_absent_at_compose_time_are_skipped() {
        // The generator marks probe artifacts `optional: true` — a need
        // that applies only where the path exists (spec 23 §3). Absent at
        // compose time: skipped silently (the floor's courtesy-surface
        // rule); present: granted.
        let spec = parse_compose(
            "needs:\n  host:\n    - path: /gone\n      access: ro\n      optional: true\n    - path: /here\n      access: rw\n      optional: true\n",
            &expand,
            &|p| p == "/here",
        )
        .unwrap();
        let jail = spec.jail.unwrap();
        assert_eq!(jail.mounts.len(), 1);
        assert_eq!(jail.mounts[0].host, std::path::PathBuf::from("/here"));
        assert_eq!(jail.mounts[0].access, HostAccess::Rw);
    }

    #[test]
    fn non_optional_absent_paths_are_named_errors() {
        // Fail-closed for authored grants: a missing path without
        // `optional: true` is a named error that NAMES the path.
        let e = parse_compose(
            "needs:\n  host:\n    - path: /gone\n      access: ro\n",
            &expand,
            &|_| false,
        )
        .unwrap_err();
        assert!(e.contains("/gone"), "{e}");
        assert!(e.contains("optional"), "{e}");
        let e = parse_compose(
            "mounts:\n  - host: /gone\n    access: ro\n",
            &expand,
            &|_| false,
        )
        .unwrap_err();
        assert!(e.contains("/gone"), "{e}");
    }
}

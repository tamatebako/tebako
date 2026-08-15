//! The `TEBAKO_OVERLAYS` / `TEBAKO_DECRYPT` env grammars (spec 24 §4,
//! export step 5′) — the serialized form of a run's bound overlay stores
//! and decrypt recipients, handed to the runtime process beside
//! `TEBAKO_JAIL` and inherited by spawned children (spec 22 class E).
//! One grammar, one parser, one serializer — the same discipline as
//! [`crate::mount_spec`].
//!
//! ```text
//! TEBAKO_OVERLAYS=<mount>=<store>;<mount>=<store>;…
//! TEBAKO_DECRYPT=<mount>=pgp:<keyid>;<mount>=pgp:<keyid>;…
//! ```
//!
//! Entries split on `;` and on the FIRST `=` (the env-var convention: a
//! store path may contain `=`; a drive-qualified windows store keeps its
//! `:`; a mount containing `=` is unrepresentable — the surplus folds
//! into the store side and fails the grammar). Mounts are
//! VFS-absolute (`/…`, or drive-qualified `X:/…` on windows — the mounts
//! the driver establishes); stores are host-absolute. A store containing
//! `;` is unrepresentable: it splits into a second entry that fails the
//! grammar — fail-closed by construction, never a silent misparse. No key
//! MATERIAL ever crosses the channel: the decrypt value is a key
//! REFERENCE (`pgp:` + 16 lowercase hex, the manifest keyid form —
//! [`tpkg::is_valid_keyid`] is the SSOT predicate), resolved against
//! `$TEBAKO_HOME/keys/` at the driver's mount.
//!
//! Malformed forms are named errors quoting the offending entry; the
//! consumer (resolver, then the driver) fails closed with exit 68
//! (`EX_TEBAKO_OVERLAY`, spec 24 §7).
//!
//! Pure safe Rust; no IO.

use std::fmt;

/// One `TEBAKO_OVERLAYS` pair: a mount point and its bound COW store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayBinding {
    /// Absolute virtual mount point (`/…` or drive-qualified `X:/…`).
    pub mount: String,
    /// Absolute host directory backing the overlay.
    pub store: String,
}

/// One `TEBAKO_DECRYPT` pair: a mount point and its bound key reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptBinding {
    /// Absolute virtual mount point (`/…` or drive-qualified `X:/…`).
    pub mount: String,
    /// The recipient reference — `pgp:<keyid>` (16 lowercase hex); never
    /// key material (spec 24 §3).
    pub recipient: String,
}

/// A named, human-readable overlay/decrypt spec parse error (the
/// offending entry is always quoted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySpecError(pub String);

impl fmt::Display for OverlaySpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid overlay binding spec: {}", self.0)
    }
}

impl std::error::Error for OverlaySpecError {}

/// Absolute in the env grammar's sense: POSIX `/…` or drive-qualified
/// `X:/…` (the windows spelling for both VFS mounts and host stores).
fn is_absolute(path: &str) -> bool {
    let b = path.as_bytes();
    !b.is_empty()
        && (b[0] == b'/'
            || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/'))
}

/// Split one `<mount>=<value>` entry at the FIRST `=` and validate the
/// mount side (the env-var convention: the VALUE may carry `=` — host
/// store paths do; a mount containing `=` is unrepresentable, the surplus
/// folds into the value side and fails the grammar — fail-closed).
/// `what` names the value for the error text ("overlay store",
/// "decrypt recipient").
fn split_entry<'a>(
    entry: &'a str,
    var: &str,
    what: &str,
) -> Result<(&'a str, &'a str), OverlaySpecError> {
    let err = |msg: &str| OverlaySpecError(format!("{msg} in {var} entry {entry:?}"));
    let Some((mount, value)) = entry.split_once('=') else {
        return Err(err("needs the <mount>=<value> shape"));
    };
    if mount.is_empty() {
        return Err(err("empty mount"));
    }
    if !is_absolute(mount) {
        return Err(err("mount point is not absolute"));
    }
    if value.is_empty() {
        return Err(err(&format!("empty {what}")));
    }
    Ok((mount, value))
}

/// The entry list of one env value: `;`-separated, no empties, no
/// duplicate mounts (one binding per mount — spec 24 §3's one-store rule).
fn entries<'a>(spec: &'a str, var: &str) -> Result<Vec<&'a str>, OverlaySpecError> {
    if spec.trim().is_empty() {
        return Err(OverlaySpecError(format!("empty {var} spec")));
    }
    let out: Vec<&str> = spec.split(';').collect();
    if out.iter().any(|e| e.is_empty()) {
        return Err(OverlaySpecError(format!(
            "empty entry (stray ';') in {var} {spec:?}"
        )));
    }
    Ok(out)
}

/// One binding per mount (spec 24 §3's one-store rule): a second entry
/// naming an already-bound mount is a named error.
fn reject_duplicate_mount(seen: &[String], mount: &str, var: &str) -> Result<(), OverlaySpecError> {
    if seen.iter().any(|m| m == mount) {
        return Err(OverlaySpecError(format!(
            "duplicate mount {mount:?} in {var} — one binding per mount"
        )));
    }
    Ok(())
}

/// Parse the `TEBAKO_OVERLAYS` env form: `<mount>=<store>` pairs,
/// `;`-separated.
pub fn parse_overlays(spec: &str) -> Result<Vec<OverlayBinding>, OverlaySpecError> {
    let mut out: Vec<OverlayBinding> = Vec::new();
    for entry in entries(spec, "TEBAKO_OVERLAYS")? {
        let (mount, store) = split_entry(entry, "TEBAKO_OVERLAYS", "overlay store")?;
        if !is_absolute(store) {
            return Err(OverlaySpecError(format!(
                "overlay store is not absolute in TEBAKO_OVERLAYS entry {entry:?}"
            )));
        }
        reject_duplicate_mount(
            &out.iter().map(|b| b.mount.clone()).collect::<Vec<_>>(),
            mount,
            "TEBAKO_OVERLAYS",
        )?;
        out.push(OverlayBinding {
            mount: mount.to_string(),
            store: store.to_string(),
        });
    }
    Ok(out)
}

/// Serialize overlay bindings to the `TEBAKO_OVERLAYS` env form (the
/// inverse of [`parse_overlays`]).
pub fn to_env_overlays(bindings: &[OverlayBinding]) -> String {
    bindings
        .iter()
        .map(|b| format!("{}={}", b.mount, b.store))
        .collect::<Vec<_>>()
        .join(";")
}

/// Parse the `TEBAKO_DECRYPT` env form: `<mount>=pgp:<keyid>` pairs,
/// `;`-separated. The recipient is a key REFERENCE — the `pgp:` scheme
/// (spec 04's MECE reference axis; room for future schemes without a
/// grammar break) plus the manifest keyid form ([`tpkg::is_valid_keyid`]).
pub fn parse_decrypt(spec: &str) -> Result<Vec<DecryptBinding>, OverlaySpecError> {
    let mut out: Vec<DecryptBinding> = Vec::new();
    for entry in entries(spec, "TEBAKO_DECRYPT")? {
        let (mount, recipient) = split_entry(entry, "TEBAKO_DECRYPT", "decrypt recipient")?;
        let valid = recipient
            .strip_prefix("pgp:")
            .is_some_and(tpkg::is_valid_keyid);
        if !valid {
            return Err(OverlaySpecError(format!(
                "decrypt recipient {recipient:?} in TEBAKO_DECRYPT entry {entry:?} is not a key reference (want pgp:<keyid>, 16 lowercase hex) — key material never crosses this channel"
            )));
        }
        reject_duplicate_mount(
            &out.iter().map(|b| b.mount.clone()).collect::<Vec<_>>(),
            mount,
            "TEBAKO_DECRYPT",
        )?;
        out.push(DecryptBinding {
            mount: mount.to_string(),
            recipient: recipient.to_string(),
        });
    }
    Ok(out)
}

/// Serialize decrypt bindings to the `TEBAKO_DECRYPT` env form (the
/// inverse of [`parse_decrypt`]).
pub fn to_env_decrypt(bindings: &[DecryptBinding]) -> String {
    bindings
        .iter()
        .map(|b| format!("{}={}", b.mount, b.recipient))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlay(mount: &str, store: &str) -> OverlayBinding {
        OverlayBinding {
            mount: mount.to_string(),
            store: store.to_string(),
        }
    }

    #[test]
    fn overlays_parse_and_round_trip() {
        let spec = "/app=/var/lib/ov/app;/__tfs__=C:/tebako/ov/rt";
        let bindings = parse_overlays(spec).unwrap();
        assert_eq!(
            bindings,
            vec![
                overlay("/app", "/var/lib/ov/app"),
                overlay("/__tfs__", "C:/tebako/ov/rt"),
            ]
        );
        assert_eq!(to_env_overlays(&bindings), spec);
        // A single pair; the root mount is legitimate.
        let one = parse_overlays("/=/tmp/scratch").unwrap();
        assert_eq!(one, vec![overlay("/", "/tmp/scratch")]);
        assert_eq!(to_env_overlays(&one), "/=/tmp/scratch");
    }

    #[test]
    fn overlay_store_may_contain_equals_and_colons() {
        // Split on the FIRST '=' (the env-var convention): the store
        // keeps its own '=' and the drive-qualified windows store keeps
        // its ':'.
        let bindings = parse_overlays("/data=/srv/a=b/store").unwrap();
        assert_eq!(bindings[0].store, "/srv/a=b/store");
        assert_eq!(to_env_overlays(&bindings), "/data=/srv/a=b/store");
        let bindings = parse_overlays("A:/app=D:/overlays/app").unwrap();
        assert_eq!(bindings[0].mount, "A:/app");
        assert_eq!(bindings[0].store, "D:/overlays/app");
    }

    #[test]
    fn overlays_reject_malformed_forms_with_named_errors() {
        for (spec, frag) in [
            ("", "empty TEBAKO_OVERLAYS spec"),
            ("  ", "empty TEBAKO_OVERLAYS spec"),
            ("/app=/store;", "empty entry"),
            (";/app=/store", "empty entry"),
            ("/app", "<mount>=<value>"),
            ("=/store", "empty mount"),
            ("app=/store", "not absolute"),
            ("/app=", "empty overlay store"),
            ("/app=store", "store is not absolute"),
            ("/app=relative/dir", "store is not absolute"),
            ("/a=/s;/a=/t", "duplicate mount"),
        ] {
            let e = parse_overlays(spec).unwrap_err();
            assert!(
                e.0.contains(frag),
                "spec {spec:?}: error {e:?} should mention {frag:?}"
            );
        }
        // A store containing ';' is unrepresentable: it splits into a
        // second entry that fails the grammar — fail-closed, never a
        // silent misparse. A MOUNT containing '=' is unrepresentable the
        // same way: the first-'=' split folds the surplus into the store
        // side, which fails the grammar.
        assert!(parse_overlays("/a=/s;/t").is_err());
        let e = parse_overlays("/x=y=/tmp/s").unwrap_err();
        assert!(e.0.contains("store is not absolute"), "{e:?}");
    }

    #[test]
    fn decrypt_parse_and_round_trip() {
        let spec = "/fonts=pgp:3c8dba971d2b4f01;/data=pgp:0000000000000000";
        let bindings = parse_decrypt(spec).unwrap();
        assert_eq!(bindings[0].mount, "/fonts");
        assert_eq!(bindings[0].recipient, "pgp:3c8dba971d2b4f01");
        assert_eq!(to_env_decrypt(&bindings), spec);
    }

    #[test]
    fn decrypt_rejects_non_reference_recipients() {
        for (recipient, frag) in [
            ("", "empty decrypt recipient"),
            ("3c8dba971d2b4f01", "not a key reference"), // no scheme
            ("pgp:3C8DBA971D2B4F01", "not a key reference"), // uppercase
            ("pgp:3c8dba97", "not a key reference"),     // short
            ("pgp:3c8dba971d2b4f01ff", "not a key reference"), // long
            ("pgp:zz8dba971d2b4f01", "not a key reference"), // non-hex
            (
                "-----BEGIN PGP PRIVATE KEY BLOCK-----",
                "not a key reference",
            ),
        ] {
            let spec = format!("/m={recipient}");
            let e = parse_decrypt(&spec).unwrap_err();
            assert!(e.0.contains(frag), "{spec:?}: {e:?}");
        }
        // The shared malformed forms apply too.
        assert!(parse_decrypt("").is_err());
        assert!(parse_decrypt("/m=pgp:3c8dba971d2b4f01;/m=pgp:3c8dba971d2b4f01").is_err());
        assert!(parse_decrypt("rel=pgp:3c8dba971d2b4f01").is_err());
    }

    /// Round-trip property: every binding list built from the grammar's
    /// representable alphabet parses back to itself. (Mounts containing
    /// '=' are OUTSIDE the representable alphabet — the split takes the
    /// first '='; the malformed-forms test pins the fail-closed answer.)
    #[test]
    fn env_forms_round_trip_over_the_representable_alphabet() {
        let mounts = ["/", "/app", "/a b/c", "A:/t", "/data"];
        let stores = ["/tmp/s", "/srv/a=b", "C:/ov", "/with space/s"];
        for (mi, m) in mounts.iter().enumerate() {
            for (si, s) in stores.iter().enumerate() {
                if si == mi % stores.len() || si == (mi + 1) % stores.len() {
                    let bindings = vec![overlay(m, s)];
                    assert_eq!(
                        parse_overlays(&to_env_overlays(&bindings)).unwrap(),
                        bindings,
                        "{m}={s}"
                    );
                }
            }
        }
        let keyids = ["pgp:3c8dba971d2b4f01", "pgp:0000000000000000"];
        for m in mounts {
            for k in keyids {
                let bindings = vec![DecryptBinding {
                    mount: m.to_string(),
                    recipient: k.to_string(),
                }];
                assert_eq!(
                    parse_decrypt(&to_env_decrypt(&bindings)).unwrap(),
                    bindings,
                    "{m}={k}"
                );
            }
        }
    }
}

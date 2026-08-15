//! The needs generator (spec 23 §8): turn a record-mode journal into a
//! draft `needs:` block for the payload manifest.
//!
//! A `record` run (allow-all + audit, [`crate::policy::PolicyDefault`])
//! journals every host access as `event=jail-allow`; a deny run journals
//! every refused access as `event=jail-deny`. Both events name a path the
//! payload reached for — the generator folds them into one deduplicated
//! list with the strongest observed access (any write ⇒ `rw`), drops the
//! paths no manifest may claim (the platform floor, the tebako home, the
//! exec cache — supplied by the caller as `exclusions`), re-substitutes
//! the symbolic atoms the manifest grammar speaks (`$HOME`, … — supplied
//! as `substitutions`; aggregation keys on the substituted form, so raw
//! variants of one path merge), omits relative and empty paths (cwd- or
//! dirfd-relative — not declarable; counted in the header so the reviewer
//! considers `$CWD` explicitly), collapses ro entries that are strict
//! ancestors of other drafted entries (the bind's traverse set, spec 08
//! §2.1, already covers their reads — an rw ancestor stays), and emits
//! YAML with a `why:` TODO the slice developer replaces while reviewing
//! each `access` bit.
//!
//! The spec 24 §6 extension folds the VFS write gate's journal into the
//! same draft: deny-mode `event=vfs-deny op=write` lines and record-mode
//! `event=vfs-write` lines (writes into HELD image trees — denied EROFS
//! or landed in the run's scratch overlay) aggregate into a draft
//! **`needs.write:`** block — paths mount-relative, persistence
//! `ephemeral` (the observed minimum), the owning mount named in the
//! `why` — and sealed-read denials (`event=vfs-deny op=read class=ekey`,
//! the ENOKEY answer) into a draft **`needs.decrypt:`** block. VFS paths
//! bypass the host pipeline entirely (no exclusions, substitutions, or
//! existence probes — they are in-image surface, never host surface).
//!
//! The draft is a STARTING POINT, never an authoritative grant: the human
//! decides ro/rw, flips persistence, assigns write/decrypt entries to the
//! slice mounted at the named mount, and drops paths the payload only
//! probed. Paths are matched as recorded (raw, pre-canonicalization); the
//! reviewer resolves any symlink ambiguity.
//!
//! Pure safe Rust; no IO — the caller reads the journal.

use std::path::PathBuf;

/// One observed path: the strongest access wins (any write ⇒ `rw`); the
/// per-class counts ride the `why` placeholder.
#[derive(Default)]
struct Observation {
    reads: u64,
    writes: u64,
    /// The path existed at generation time. Probe artifacts (stat'ed but
    /// absent — the JVM's `Info.plist`/`ext`/`endorsed` probes, per-pid
    /// files) get `optional: true`: skipped silently at bind when absent
    /// (the floor's courtesy-surface rule), granted where present.
    present: bool,
}

/// One observed in-image write or sealed read (spec 24 §6): the count
/// rides the `why`; the mounts the path was observed under name the
/// candidate slice for the reviewer.
#[derive(Default)]
struct VfsObservation {
    count: u64,
    mounts: std::collections::BTreeSet<String>,
}

/// Fold a journal's contents into a draft `needs:` YAML block (see the
/// module docs for the contract). `substitutions` are (host prefix, atom)
/// pairs — longest matching prefix wins — applied on path-component
/// boundaries; `exclusions` drop every path under a listed root (same
/// boundary rule). Aggregation keys on the SUBSTITUTED form, so raw
/// variants of one path (`/T/x` vs `/T//x`) merge into one entry.
/// Relative and empty paths are not declarable needs (cwd- or
/// dirfd-relative) — they are counted in the header for the reviewer.
/// `exists` probes the host at generation time (a test seam — the CLI
/// binds it to the filesystem).
pub fn needs_from_journal(
    journal: &str,
    substitutions: &[(PathBuf, &str)],
    exclusions: &[PathBuf],
    exists: &dyn Fn(&str) -> bool,
) -> String {
    let mut observed: std::collections::BTreeMap<String, Observation> = Default::default();
    let mut writes: std::collections::BTreeMap<String, VfsObservation> = Default::default();
    let mut sealed: std::collections::BTreeMap<String, VfsObservation> = Default::default();
    let mut omitted = 0u64;
    for line in journal.lines() {
        if let Some(vfs) = parse_vfs_event_line(line) {
            // In-image surface (spec 24 §6): mount-relative, aggregated,
            // never host-pipelined.
            let (path, mount, into) = match vfs {
                VfsEvent::Write { path, mount } => (path, mount, &mut writes),
                VfsEvent::SealedRead { path, mount } => (path, mount, &mut sealed),
            };
            if let Some(rel) = mount_relative(&path, &mount) {
                let o = into.entry(rel.to_string()).or_default();
                o.count += 1;
                o.mounts.insert(mount);
            }
            continue;
        }
        let Some((path, write)) = parse_event_line(line) else {
            continue;
        };
        if path.is_empty() || !std::path::Path::new(path).is_absolute() {
            omitted += 1;
            continue;
        }
        if exclusions
            .iter()
            .any(|root| std::path::Path::new(path).starts_with(root))
        {
            continue;
        }
        let present = exists(path);
        let o = observed.entry(substitute(path, substitutions)).or_default();
        o.present |= present;
        if write {
            o.writes += 1;
        } else {
            o.reads += 1;
        }
    }
    // The bind derives every grant's strict ancestors as exact-path
    // traverse reads (spec 08 §2.1), so an ro entry that is itself a
    // strict ancestor of another drafted entry is redundant — traverse
    // covers its reads. An rw ancestor stays: traverse never grants
    // write.
    let paths: Vec<String> = observed.keys().cloned().collect();
    for p in &paths {
        let redundant = observed.get(p).is_some_and(|o| o.writes == 0)
            && paths
                .iter()
                .any(|q| q != p && std::path::Path::new(q).starts_with(std::path::Path::new(p)));
        if redundant {
            observed.remove(p);
        }
    }
    emit_yaml(&observed, &writes, &sealed, omitted)
}

/// One parsed spec-24 journal line.
enum VfsEvent {
    /// `vfs-deny op=write` (deny mode) or `vfs-write` (record mode): a
    /// write into a held image tree.
    Write { path: String, mount: String },
    /// `vfs-deny op=read class=ekey`: a read of a sealed path outside
    /// every opened grant (ENOKEY).
    SealedRead { path: String, mount: String },
}

/// Parse one journal line into a VFS event; `None` for anything else.
/// The line shapes are journal.rs's (the vocabulary's owner):
/// `vfs-deny op=<op> path=<p> mount=<mp>[ class=ekey]` and
/// `vfs-write path=<p> mount=<mp>` — the ` path=` / ` mount=` delimiters
/// keep paths containing spaces intact.
fn parse_vfs_event_line(line: &str) -> Option<VfsEvent> {
    let event = line.split(' ').find_map(|f| f.strip_prefix("event="))?;
    let path = line.split_once(" path=")?.1.split_once(" mount=")?.0;
    let rest = line.split_once(" mount=")?.1;
    let mount = rest.split_once(" class=").map(|(m, _)| m).unwrap_or(rest);
    match event {
        "vfs-write" => Some(VfsEvent::Write {
            path: path.to_string(),
            mount: mount.to_string(),
        }),
        "vfs-deny" => {
            let op = line.split_once(" op=")?.1.split_once(" path=")?.0;
            match op {
                "write" => Some(VfsEvent::Write {
                    path: path.to_string(),
                    mount: mount.to_string(),
                }),
                "read" if line.contains(" class=ekey") => Some(VfsEvent::SealedRead {
                    path: path.to_string(),
                    mount: mount.to_string(),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The in-image form of a journaled namespace path: `path` relative to
/// its mount (`/app/var/x` under mount `/app` → `/var/x`; the mount
/// itself → `/`; under the root mount the namespace path already IS the
/// in-image path). `None` when the path is not under the mount — a
/// can't-happen the generator refuses to guess at.
fn mount_relative<'a>(path: &'a str, mount: &str) -> Option<&'a str> {
    let base = mount.trim_end_matches('/');
    if base.is_empty() {
        return path.starts_with('/').then_some(path);
    }
    if path == base {
        return Some("/");
    }
    path.strip_prefix(base).filter(|rest| rest.starts_with('/'))
}

/// Parse one journal line into (path, is-write) for the jail-allow and
/// jail-deny events; `None` for anything else. The path field is delimited
/// by ` op=` so paths containing spaces survive.
fn parse_event_line(line: &str) -> Option<(&str, bool)> {
    let event = line.split(' ').find_map(|f| f.strip_prefix("event="))?;
    if event != "jail-allow" && event != "jail-deny" {
        return None;
    }
    let path = line.split_once(" path=")?.1.split_once(" op=")?.0;
    let write = match line.split_once(" op=")?.1.split(' ').next()? {
        "read" => false,
        "write" => true,
        _ => return None,
    };
    Some((path, write))
}

/// Re-spell a recorded path with the longest matching (prefix, atom)
/// substitution; identity when none matches.
fn substitute(path: &str, substitutions: &[(PathBuf, &str)]) -> String {
    let p = std::path::Path::new(path);
    let mut out = path.to_string();
    let mut best_len = 0;
    for (root, atom) in substitutions {
        if p.starts_with(root) && root.as_os_str().len() > best_len {
            best_len = root.as_os_str().len();
            let rest = p.strip_prefix(root).unwrap();
            out = if rest.as_os_str().is_empty() {
                atom.to_string()
            } else {
                format!("{atom}/{}", rest.display())
            };
        }
    }
    out
}

/// Emit the draft YAML: a review header (with the omitted relative/empty
/// access count when nonzero), then the `host:` section per observed host
/// path, then the spec 24 §6 sections — `write:` (VFS write-gate
/// observations, mount-relative, persistence `ephemeral`) and `decrypt:`
/// (sealed-read observations). Sections emit only when non-empty; an
/// entirely empty observation keeps the historical `host: []` spelling.
/// Sorted by path (deterministic — the input's line order is irrelevant).
fn emit_yaml(
    observed: &std::collections::BTreeMap<String, Observation>,
    writes: &std::collections::BTreeMap<String, VfsObservation>,
    sealed: &std::collections::BTreeMap<String, VfsObservation>,
    omitted: u64,
) -> String {
    let mut out = String::from(
        "# Drafted by `tfs needs --from-journal` (spec 23 §8): every host path the\n\
         # recorded run touched, strongest observed access. Review each `access`\n\
         # (ro|rw) and replace every `why` before merging into the payload manifest.\n\
         # Strict ancestors of granted paths are traversable by construction\n\
         # (spec 08 §2.1) — collapsed out of this draft.\n\
         # `write:`/`decrypt:` entries (spec 24 §6) are in-image paths relative to\n\
         # the mount named in their `why`; assign each to the slice mounted there\n\
         # and review `persistence` (the observed minimum is ephemeral).\n",
    );
    if omitted > 0 {
        out.push_str(&format!(
            "# {omitted} relative/empty-path access(es) omitted (cwd- or dirfd-relative;\n\
             # declare $CWD explicitly if the payload wants it)\n"
        ));
    }
    out.push_str("needs:\n");
    if observed.is_empty() && writes.is_empty() && sealed.is_empty() {
        out.push_str("  host: []\n");
        return out;
    }
    if !observed.is_empty() {
        out.push_str("  host:\n");
        for (path, o) in observed {
            let access = if o.writes > 0 { "rw" } else { "ro" };
            out.push_str(&format!(
                "    - path: \"{}\"\n      access: {}\n",
                yaml_escape(path),
                access
            ));
            if !o.present {
                out.push_str("      optional: true\n");
            }
            out.push_str(&format!(
                "      why: \"TODO — observed: {} read, {} write\"\n",
                o.reads, o.writes
            ));
        }
    }
    if !writes.is_empty() {
        out.push_str("  write:\n");
        for (path, o) in writes {
            out.push_str(&format!(
                "    - path: \"{}\"\n      persistence: ephemeral\n      why: \"TODO — observed: {} write under mount {}\"\n",
                yaml_escape(path),
                o.count,
                o.mounts.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    if !sealed.is_empty() {
        out.push_str("  decrypt:\n");
        for (path, o) in sealed {
            out.push_str(&format!(
                "    - part: \"{}\"\n      why: \"TODO — observed: {} sealed read under mount {}\"\n",
                yaml_escape(path),
                o.count,
                o.mounts.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    out
}

/// Double-quoted YAML scalar escaping.
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strongest_op_wins_and_entries_dedup() {
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/a op=read source=record\n\
             2 event=jail-allow path=/a op=read source=record\n\
             3 event=jail-allow path=/a op=write source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert_eq!(yaml.match_indices("path: \"/a\"").count(), 1, "{yaml}");
        assert!(yaml.contains("access: rw"), "{yaml}");
        assert!(yaml.contains("observed: 2 read, 1 write"), "{yaml}");
    }

    #[test]
    fn deny_events_are_needs_too() {
        // A denial is an UNMET need — it belongs in the draft (the payload
        // reached for the path; the reviewer grants or drops it).
        let yaml = needs_from_journal(
            "1 event=jail-deny path=/secret op=read source=manifest\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/secret\""), "{yaml}");
        assert!(yaml.contains("access: ro"), "{yaml}");
        assert!(yaml.contains("observed: 1 read, 0 write"), "{yaml}");
    }

    #[test]
    fn exclusions_drop_system_and_cache_paths() {
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/usr/lib/x op=read source=record\n\
             2 event=jail-allow path=/Users/u/app/file op=read source=record\n",
            &[],
            &[PathBuf::from("/usr")],
            &|_| true,
        );
        assert!(!yaml.contains("/usr"), "{yaml}");
        assert!(yaml.contains("path: \"/Users/u/app/file\""), "{yaml}");
    }

    #[test]
    fn exclusions_match_on_component_boundaries() {
        // "/usrish" is not under the "/usr" exclusion root — prefix
        // matching is component-wise, never a bare string prefix.
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/usrish/x op=read source=record\n",
            &[],
            &[PathBuf::from("/usr")],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/usrish/x\""), "{yaml}");
    }

    #[test]
    fn home_substitution_respells_the_prefix() {
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/Users/u/.ssh/config op=read source=record\n\
             2 event=jail-allow path=/Users/u op=read source=record\n",
            &[(PathBuf::from("/Users/u"), "$HOME")],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"$HOME/.ssh/config\""), "{yaml}");
        // The bare $HOME entry collapses: an ro strict ancestor of another
        // drafted entry is redundant — the bind's traverse set (spec 08
        // §2.1) already covers its reads.
        assert!(!yaml.contains("path: \"$HOME\""), "{yaml}");
    }

    #[test]
    fn malformed_and_foreign_lines_are_skipped() {
        let yaml = needs_from_journal(
            "garbage\n\
             1 event=composition source=external(/x.yaml) sha256=abc\n\
             2 event=jail-allow op=read source=record\n\
             3 event=jail-allow path=/no-op-field\n\
             4 event=jail-allow path=/bad-op op=execute source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("host: []"), "{yaml}");
    }

    #[test]
    fn output_is_sorted_and_deterministic() {
        let a = needs_from_journal(
            "1 event=jail-allow path=/z op=read source=record\n\
             2 event=jail-allow path=/b op=read source=record\n",
            &[],
            &[],
            &|_| true,
        );
        let b = needs_from_journal(
            "9 event=jail-deny path=/b op=read source=record\n\
             8 event=jail-allow path=/z op=read source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert_eq!(a, b);
        assert!(a.find("/b").unwrap() < a.find("/z").unwrap(), "{a}");
    }

    #[test]
    fn paths_with_spaces_survive() {
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/My Docs/file op=read source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/My Docs/file\""), "{yaml}");
    }

    #[test]
    fn empty_and_relative_paths_are_omitted_and_counted() {
        // The JVM probes stat(""), reads .hotspotrc relative to the cwd,
        // and writes hsperfdata dirfd-relative — none of these is a
        // declarable host need. They leave the entry list and land in the
        // header count so the reviewer knows to consider $CWD explicitly.
        let yaml = needs_from_journal(
            "1 event=jail-allow path= op=read source=record\n\
             2 event=jail-allow path=.hotspotrc op=read source=record\n\
             3 event=jail-allow path=65037 op=write source=record\n\
             4 event=jail-allow path=/real/path op=read source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(!yaml.contains("path: \"\""), "{yaml}");
        assert!(!yaml.contains("hotspotrc"), "{yaml}");
        assert!(!yaml.contains("65037"), "{yaml}");
        assert!(yaml.contains("path: \"/real/path\""), "{yaml}");
        assert!(yaml.contains("3 relative/empty"), "{yaml}");
    }

    #[test]
    fn substitution_merges_path_variants() {
        // `$TMPDIR` ends in '/', the JVM appends "/hsperfdata" — the
        // journal carries both `T/x` and `T//x`. Aggregation keys on the
        // SUBSTITUTED form, so the variants merge into one rw entry.
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/tmp/x/data op=read source=record\n\
             2 event=jail-allow path=/tmp/x//data op=write source=record\n",
            &[(PathBuf::from("/tmp/x/"), "$TMPDIR")],
            &[],
            &|_| true,
        );
        assert_eq!(yaml.match_indices("$TMPDIR/data").count(), 1, "{yaml}");
        assert!(yaml.contains("access: rw"), "{yaml}");
        assert!(yaml.contains("observed: 1 read, 1 write"), "{yaml}");
    }

    #[test]
    fn paths_absent_at_generation_get_the_optional_marker() {
        // Probe artifacts (the JVM's Info.plist / ext / endorsed stats,
        // per-pid files): not present at generation time, so the grant is
        // marked `optional: true` — skipped silently at bind when absent
        // (the floor's courtesy-surface rule), still granted where present.
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/exists op=read source=record\n\
             2 event=jail-allow path=/gone op=read source=record\n",
            &[],
            &[],
            &|p| p == "/exists",
        );
        let after = yaml.split("path: \"/exists\"").nth(1).unwrap();
        let entry: Vec<&str> = after.lines().take(3).collect();
        assert!(!entry.iter().any(|l| l.contains("optional")), "{yaml}");
        let after = yaml.split("path: \"/gone\"").nth(1).unwrap();
        assert!(after.contains("optional: true"), "{yaml}");
    }

    #[test]
    fn empty_journal_drafts_an_empty_block() {
        let yaml = needs_from_journal("", &[], &[], &|_| true);
        assert!(yaml.contains("needs:"), "{yaml}");
        assert!(yaml.contains("host: []"), "{yaml}");
    }

    // ---------------------------------------------------------------
    // The spec 24 §6 fold: the VFS write gate's journal → needs.write /
    // needs.decrypt drafts
    // ---------------------------------------------------------------

    #[test]
    fn vfs_write_and_deny_fold_into_a_write_draft() {
        // Deny-mode vfs-deny write lines and record-mode vfs-write lines
        // aggregate into one draft write area — paths mount-relative,
        // persistence ephemeral (the observed minimum), the mount named.
        let yaml = needs_from_journal(
            "1 event=vfs-deny op=write path=/app/var/cache/a mount=/app\n\
             2 event=vfs-write path=/app/var/cache/b mount=/app\n\
             3 event=vfs-write path=/app/var/cache/a mount=/app\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("  write:\n"), "{yaml}");
        assert_eq!(
            yaml.match_indices("path: \"/var/cache/a\"").count(),
            1,
            "{yaml}"
        );
        assert!(yaml.contains("persistence: ephemeral"), "{yaml}");
        assert!(
            yaml.contains("observed: 2 write under mount /app"),
            "{yaml}"
        );
        assert!(yaml.contains("path: \"/var/cache/b\""), "{yaml}");
        // No host observations → no host section.
        assert!(!yaml.contains("  host:"), "{yaml}");
    }

    #[test]
    fn vfs_events_from_several_mounts_keep_their_attribution() {
        let yaml = needs_from_journal(
            "1 event=vfs-deny op=write path=/app/x mount=/app\n\
             2 event=vfs-deny op=write path=/__tfs__/lib/gems/x mount=/__tfs__\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/x\""), "{yaml}");
        assert!(yaml.contains("under mount /app"), "{yaml}");
        assert!(yaml.contains("path: \"/lib/gems/x\""), "{yaml}");
        assert!(yaml.contains("under mount /__tfs__"), "{yaml}");
    }

    #[test]
    fn vfs_paths_at_the_mount_and_below_the_root_mount() {
        // The mount itself maps to `/`; under the root mount the
        // namespace path already IS the in-image path.
        let yaml = needs_from_journal(
            "1 event=vfs-write path=/app mount=/app\n\
             2 event=vfs-write path=/etc/hosts mount=/\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/\""), "{yaml}");
        assert!(yaml.contains("path: \"/etc/hosts\""), "{yaml}");
    }

    #[test]
    fn sealed_read_denials_fold_into_a_decrypt_draft() {
        let yaml = needs_from_journal(
            "1 event=vfs-deny op=read path=/data/fonts/licensed/f.ttf mount=/data class=ekey\n\
             2 event=vfs-deny op=read path=/data/fonts/licensed/g.ttf mount=/data class=ekey\n\
             3 event=vfs-deny op=read path=/data/fonts/licensed/f.ttf mount=/data class=ekey\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("  decrypt:\n"), "{yaml}");
        assert!(yaml.contains("part: \"/fonts/licensed/f.ttf\""), "{yaml}");
        assert!(
            yaml.contains("observed: 2 sealed read under mount /data"),
            "{yaml}"
        );
        assert!(yaml.contains("part: \"/fonts/licensed/g.ttf\""), "{yaml}");
        // A plain vfs-deny READ without the ekey class is not a sealed
        // read — no decrypt SECTION (the header prose names the sections;
        // the section line is the indented form).
        let yaml = needs_from_journal(
            "1 event=vfs-deny op=read path=/data/x mount=/data\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(!yaml.contains("  decrypt:\n"), "{yaml}");
        assert!(yaml.contains("host: []"), "{yaml}");
    }

    #[test]
    fn vfs_and_host_events_compose_in_one_draft() {
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/home/u/.cfg op=read source=record\n\
             2 event=vfs-deny op=write path=/app/var mount=/app\n\
             3 event=overlay mount=/app store=/tmp/ov source=ephemeral\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/home/u/.cfg\""), "{yaml}");
        assert!(yaml.contains("path: \"/var\""), "{yaml}");
        // The boot-time binding records are audit lines, not needs.
        assert!(!yaml.contains("/tmp/ov"), "{yaml}");
    }

    #[test]
    fn vfs_paths_with_spaces_and_malformed_lines() {
        let yaml = needs_from_journal(
            "1 event=vfs-write path=/app/My Docs/f mount=/app\n\
             2 event=vfs-write path=no-mount-field\n\
             3 event=vfs-deny op=execute path=/app/x mount=/app\n\
             4 event=vfs-deny op=write path=/other/x mount=/app\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(yaml.contains("path: \"/My Docs/f\""), "{yaml}");
        // A vfs line whose path is not under its named mount is a
        // can't-happen the generator refuses to guess at.
        assert!(!yaml.contains("/other"), "{yaml}");
        assert!(!yaml.contains("no-mount"), "{yaml}");
    }

    #[test]
    fn ro_strict_ancestors_of_other_entries_collapse() {
        // The bind derives every grant's strict ancestors as exact-path
        // traverse reads (spec 08 §2.1), so a drafted entry that is itself
        // a strict ancestor of another drafted entry is redundant when ro
        // — traverse covers its reads. An ancestor with observed WRITES
        // stays: traverse never grants write.
        let yaml = needs_from_journal(
            "1 event=jail-allow path=/a op=read source=record\n\
             2 event=jail-allow path=/a/b op=read source=record\n\
             3 event=jail-allow path=/a/b/c op=read source=record\n\
             4 event=jail-allow path=/x op=write source=record\n\
             5 event=jail-allow path=/x/y op=read source=record\n",
            &[],
            &[],
            &|_| true,
        );
        assert!(!yaml.contains("path: \"/a\""), "{yaml}");
        assert!(!yaml.contains("path: \"/a/b\""), "{yaml}");
        assert!(yaml.contains("path: \"/a/b/c\""), "{yaml}");
        assert!(yaml.contains("path: \"/x\""), "{yaml}");
        assert!(yaml.contains("path: \"/x/y\""), "{yaml}");
    }
}

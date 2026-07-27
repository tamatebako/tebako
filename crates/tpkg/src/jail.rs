//! The host-access jail model (spec 08 — jails): the single owner of the
//! authored policy shape shared by every surface.
//!
//! Two wire forms, one model:
//!
//! - **YAML** (manifests — the payload manifest's `capabilities.host`,
//!   spec 08 §4, and the package manifest's `jail:` block, spec 03 §6):
//!
//!   ```yaml
//!   default: open | deny          # open = today's behavior (cwd + writes pass through)
//!   mounts:
//!     - host: $HOME/sources
//!       mount: /work
//!       access: ro | rw
//!   argument_files: auto-allowed  # the input file you hand the command is allowed even under deny
//!   ```
//!
//! - **env** (`TEBAKO_JAIL`, spec 08 §1 — the form the runtime driver and
//!   the preload shim consume): `open|deny` + `;host:mount:ro|rw` grants +
//!   `;@file` argument files, e.g. `deny;/home/u/src:/work:rw;@/home/u/in.csv`.
//!
//! This module is the AUTHORED model: it parses, validates, serializes and
//! composes policies; it never touches the filesystem for enforcement.
//! Binding (realpath canonicalization) and per-path gating live in the
//! engine (`tfs::policy`), which consumes the env form this module emits.
//!
//! **Precedence (locked, spec 08 §2/§4):** the package's manifest REQUESTS
//! access (`capabilities.host` / the package `jail:` block); the user can
//! always TIGHTEN it (`tebako run --jail pkg`, `--mount src:/work:rw`,
//! `--no-host`) — user policy always wins, and it intersects, never
//! loosens: [`effective`] composes request ∩ tightening so the tighter
//! answer for every path survives (deny defaults win; each side's mount
//! grants are capped by the other side's allowance at the same prefix;
//! argument files union, `auto-allowed` honored when either side asks).
//!
//! Pure safe Rust; the only IO is reading a `--jail <file.yaml>` and
//! probing argv entries for the `auto-allowed` resolution.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::{Path, PathBuf};

/// A mount's grant bit (`access: ro | rw`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JailAccess {
    /// Read-only.
    Ro,
    /// Read-write.
    Rw,
}

impl JailAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            JailAccess::Ro => "ro",
            JailAccess::Rw => "rw",
        }
    }

    /// The tighter of two grants (ro caps rw).
    fn tighter(self, other: JailAccess) -> JailAccess {
        match (self, other) {
            (JailAccess::Ro, _) | (_, JailAccess::Ro) => JailAccess::Ro,
            _ => JailAccess::Rw,
        }
    }
}

impl Serialize for JailAccess {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for JailAccess {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<JailAccess, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "ro" => Ok(JailAccess::Ro),
            "rw" => Ok(JailAccess::Rw),
            other => Err(serde::de::Error::custom(format_args!(
                "jail mount access must be ro|rw, got {other:?}"
            ))),
        }
    }
}

/// One host-mount grant (`{host, mount, access}`, docker `-v` semantics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JailMount {
    /// Host directory the grant covers (prefix-matched on path-component
    /// boundaries; realpath-canonicalized at bind time by the engine).
    pub host: String,
    /// Virtual mount point the host dir is exposed at (must be absolute).
    pub mount: String,
    pub access: JailAccess,
}

/// The `argument_files:` block: EITHER the scalar `auto-allowed` (dispatch
/// surfaces resolve the argv entries that name existing files into
/// read-only grants) OR an explicit list of granted paths. The model keeps
/// both flags separately because the composed policy (request ∩
/// tightening, [`intersect`]) may carry an `auto` request AND explicit
/// files; the YAML authored form is always one or the other.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArgumentFiles {
    /// `auto-allowed` — resolve argv files into read grants at dispatch.
    pub auto: bool,
    /// Explicit read-only grants (honored even under deny).
    pub files: Vec<String>,
}

impl ArgumentFiles {
    fn is_empty(&self) -> bool {
        !self.auto && self.files.is_empty()
    }
}

impl Serialize for ArgumentFiles {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.auto && self.files.is_empty() {
            s.serialize_str("auto-allowed")
        } else {
            // The authored list form. (The composed auto+files state is not
            // representable in the spec's YAML shape; composed policies
            // travel via the env form, which carries @files explicitly.)
            self.files.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for ArgumentFiles {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ArgumentFiles, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Auto(String),
            List(Vec<String>),
        }
        match Repr::deserialize(d)? {
            Repr::Auto(s) if s == "auto-allowed" => Ok(ArgumentFiles {
                auto: true,
                files: Vec::new(),
            }),
            Repr::Auto(s) => Err(serde::de::Error::custom(format_args!(
                "argument_files must be \"auto-allowed\" or a path list, got {s:?}"
            ))),
            Repr::List(files) => Ok(ArgumentFiles { auto: false, files }),
        }
    }
}

/// The host-access jail of a payload or package (spec 08 §1/§4).
///
/// The default is `open` with no mounts and no argument files — today's
/// behavior, so a manifest without the block and a package pressed without
/// `--jail` behave byte-identically to before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostJail {
    /// `default: open | deny` (serde renders the bool as the locked words).
    #[serde(
        rename = "default",
        with = "default_mode",
        default = "default_open_true"
    )]
    pub default_open: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<JailMount>,
    #[serde(default, skip_serializing_if = "ArgumentFiles::is_empty")]
    pub argument_files: ArgumentFiles,
}

fn default_open_true() -> bool {
    true
}

/// `default:` serde: `open` ↔ true, `deny` ↔ false.
mod default_mode {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(open: &bool, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(if *open { "open" } else { "deny" })
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "open" => Ok(true),
            "deny" => Ok(false),
            other => Err(serde::de::Error::custom(format_args!(
                "jail default must be open|deny, got {other:?}"
            ))),
        }
    }
}

/// Error of the jail surfaces.
///
/// Deliberately separate from [`crate::ManifestError`] and
/// [`crate::PackageManifestError`]: the jail model serves the bootstrap
/// and the dispatch surfaces too, which never see a manifest.
#[derive(Debug)]
pub enum JailError {
    /// YAML parse/serialize failure of the block form.
    Yaml(serde_yml::Error),
    /// Env/cli spec parse failure (the message quotes the offending spec).
    Spec(String),
    /// Semantic validation failure (`validate()`), or an unreadable
    /// `--jail <file.yaml>`.
    Invalid(String),
}

impl fmt::Display for JailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JailError::Yaml(e) => write!(f, "jail yaml error: {e}"),
            JailError::Spec(m) => write!(f, "invalid jail spec: {m}"),
            JailError::Invalid(m) => write!(f, "invalid jail: {m}"),
        }
    }
}

impl std::error::Error for JailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JailError::Yaml(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_yml::Error> for JailError {
    fn from(e: serde_yml::Error) -> JailError {
        JailError::Yaml(e)
    }
}

impl Default for HostJail {
    fn default() -> Self {
        Self::open()
    }
}

impl HostJail {
    /// The open jail (today's behavior; the model's zero value).
    pub const fn open() -> HostJail {
        HostJail {
            default_open: true,
            mounts: Vec::new(),
            argument_files: ArgumentFiles {
                auto: false,
                files: Vec::new(),
            },
        }
    }

    /// `deny`: every host path outside the grants fails EPERM.
    pub const fn deny() -> HostJail {
        HostJail {
            default_open: false,
            mounts: Vec::new(),
            argument_files: ArgumentFiles {
                auto: false,
                files: Vec::new(),
            },
        }
    }

    /// `deny:arg`: the file-scoped tight jail (spec 08 §1 profile 3) —
    /// deny everything, then let the dispatch surface allow the input
    /// files the command was handed.
    pub const fn deny_with_arg_files() -> HostJail {
        HostJail {
            default_open: false,
            mounts: Vec::new(),
            argument_files: ArgumentFiles {
                auto: true,
                files: Vec::new(),
            },
        }
    }

    /// True when the policy is indistinguishable from no policy at all
    /// (open default, no grants — `auto-allowed` under an open default
    /// grants nothing the default does not already allow). Surfaces skip
    /// exporting such a policy, keeping the no-policy path byte-identical.
    pub fn is_trivially_open(&self) -> bool {
        self.default_open && self.mounts.is_empty() && self.argument_files.files.is_empty()
    }

    /// Parse and validate the YAML block form.
    pub fn from_yaml(text: &str) -> Result<HostJail, JailError> {
        let jail: HostJail = serde_yml::from_str(text)?;
        jail.validate()?;
        Ok(jail.normalized())
    }

    /// Validate and serialize to the YAML block form.
    pub fn to_yaml(&self) -> Result<String, JailError> {
        self.validate()?;
        Ok(serde_yml::to_string(self)?)
    }

    /// Semantic checks beyond the serde structure: non-empty hosts,
    /// absolute mount points, non-empty argument-file entries. Unknown
    /// keys are tolerated (the manifest convention).
    pub fn validate(&self) -> Result<(), JailError> {
        for m in &self.mounts {
            if m.host.is_empty() {
                return Err(JailError::Invalid(
                    "mounts[].host must not be empty".to_string(),
                ));
            }
            if !m.mount.starts_with('/') {
                return Err(JailError::Invalid(format!(
                    "mounts[].mount {:?} is not absolute",
                    m.mount
                )));
            }
        }
        if self.argument_files.files.iter().any(|f| f.is_empty()) {
            return Err(JailError::Invalid(
                "argument_files entries must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Trailing-slash normalization of mount hosts ("/a/" ≡ "/a"; the root
    /// keeps its slash) so composition dedup/coalescing compares like with
    /// like. Enforcement canonicalizes at bind regardless — this only
    /// keeps the composed grant set tidy.
    fn normalized(mut self) -> HostJail {
        for m in &mut self.mounts {
            while m.host.len() > 1 && m.host.ends_with('/') {
                m.host.pop();
            }
        }
        self
    }

    /// Parse a `--jail` spec (press/run/shim surfaces): the profiles
    /// `open` | `deny` | `deny:arg`, a YAML file carrying the block form,
    /// or the `TEBAKO_JAIL` env grammar itself (spec 08 §1).
    pub fn from_cli_spec(spec: &str) -> Result<HostJail, JailError> {
        match spec {
            "open" => return Ok(HostJail::open()),
            "deny" => return Ok(HostJail::deny()),
            "deny:arg" => return Ok(HostJail::deny_with_arg_files()),
            _ => {}
        }
        let path = Path::new(spec);
        if path.is_file() {
            let text = std::fs::read_to_string(path).map_err(|e| {
                JailError::Invalid(format!("cannot read the jail file {spec:?}: {e}"))
            })?;
            return HostJail::from_yaml(&text);
        }
        Self::parse_env_spec(spec)
    }

    /// The dispatch flags → the user's tightening, shared by `tebako run`
    /// and `tebako-shim` (spec 08 §2): `--jail` supplies the base policy
    /// (open when absent), `--no-host` tightens the default to deny, each
    /// `--mount` adds a grant. `Ok(None)` when no flag was given at all
    /// (the surface then dispatches the manifest's request alone — or
    /// nothing).
    pub fn from_dispatch_flags(
        jail: Option<&str>,
        mounts: &[String],
        no_host: bool,
    ) -> Result<Option<HostJail>, JailError> {
        let mut user: Option<HostJail> = jail.map(HostJail::from_cli_spec).transpose()?;
        if no_host {
            let base = user.take().unwrap_or_else(HostJail::open);
            user = Some(intersect(&base, &HostJail::deny()));
        }
        for m in mounts {
            let mount = parse_mount_directive(m)?;
            user.get_or_insert_with(HostJail::open).mounts.push(mount);
        }
        Ok(user)
    }

    /// Parse the `TEBAKO_JAIL` env form (spec 08 §1):
    ///
    /// ```text
    /// jail      = directive *( ";" directive )
    /// directive = "open" | "deny"          # namespace default (default: open)
    ///           | host ":" mount ":" mode  # docker -v grant; mount absolute
    ///           | "@" path                 # argument file (read-only grant)
    /// mode      = "ro" | "rw"
    /// ```
    ///
    /// Errors on: empty spec, conflicting or duplicated `open`/`deny`,
    /// unknown access modes, non-absolute mount points, empty
    /// hosts/mounts/argument files, and unrecognised directive shapes. The
    /// messages mirror the engine's `tfs::policy::JailSpec` parser — one
    /// grammar, one message set.
    pub fn parse_env_spec(spec: &str) -> Result<HostJail, JailError> {
        let err = |msg: String| JailError::Spec(format!("{msg} in {spec:?}"));
        if spec.trim().is_empty() {
            return Err(JailError::Spec("empty spec".to_string()));
        }
        let mut default_open: Option<bool> = None;
        let mut mounts = Vec::new();
        let mut files = Vec::new();
        for token in spec.split(';') {
            if token.is_empty() {
                return Err(err("empty directive (stray ';')".to_string()));
            }
            match token {
                "open" | "deny" => {
                    let value = token == "open";
                    if default_open.is_some() {
                        return Err(err(format!(
                            "duplicate/conflicting default directive {token:?}"
                        )));
                    }
                    default_open = Some(value);
                }
                _ if token.starts_with('@') => {
                    let path = &token[1..];
                    if path.is_empty() {
                        return Err(err("empty argument file".to_string()));
                    }
                    files.push(path.to_string());
                }
                _ => {
                    mounts.push(parse_mount_directive(token).map_err(|m| err(m.to_string()))?);
                }
            }
        }
        Ok(HostJail {
            default_open: default_open.unwrap_or(true),
            mounts,
            argument_files: ArgumentFiles { auto: false, files },
        }
        .normalized())
    }

    /// Serialize to the `TEBAKO_JAIL` env form. `resolved_arg_files` are
    /// the dispatch surface's resolution of `auto-allowed` (argv entries
    /// naming existing files — see [`resolve_argument_files`]); they are
    /// unioned with the explicit list, deduplicated, in first-seen order.
    pub fn to_env_spec(&self, resolved_arg_files: &[PathBuf]) -> String {
        let mut out = String::from(if self.default_open { "open" } else { "deny" });
        for m in &self.mounts {
            out.push(';');
            out.push_str(&m.host);
            out.push(':');
            out.push_str(&m.mount);
            out.push(':');
            out.push_str(m.access.as_str());
        }
        let mut emitted = std::collections::BTreeSet::new();
        for f in &self.argument_files.files {
            if emitted.insert(f.clone()) {
                out.push(';');
                out.push('@');
                out.push_str(f);
            }
        }
        for f in resolved_arg_files {
            let s = f.to_string_lossy().into_owned();
            if emitted.insert(s.clone()) {
                out.push(';');
                out.push('@');
                out.push_str(&s);
            }
        }
        out
    }
}

/// Parse one `host:mount:ro|rw` grant (a single env-form directive; the
/// `--mount` flag of the dispatch surfaces takes exactly this shape).
/// Splits from the RIGHT so host paths containing ':' survive.
pub fn parse_mount_directive(token: &str) -> Result<JailMount, JailError> {
    let Some((head, access)) = token.rsplit_once(':') else {
        return Err(JailError::Spec(format!(
            "directive {token:?} is not open|deny, @file, or host:mount:ro|rw"
        )));
    };
    let access = match access {
        "ro" => JailAccess::Ro,
        "rw" => JailAccess::Rw,
        other => {
            return Err(JailError::Spec(format!(
                "unknown access mode {other:?} (want ro|rw)"
            )))
        }
    };
    let Some((host, mount)) = head.rsplit_once(':') else {
        return Err(JailError::Spec(format!(
            "grant {token:?} needs the host:mount:ro|rw shape"
        )));
    };
    if host.is_empty() {
        return Err(JailError::Spec(format!("empty host in grant {token:?}")));
    }
    if !mount.starts_with('/') {
        return Err(JailError::Spec(format!(
            "mount point {mount:?} is not absolute"
        )));
    }
    Ok(JailMount {
        host: host.to_string(),
        mount: mount.to_string(),
        access,
    })
}

/// Policy Q's allowance at `host`: the longest of Q's grants covering the
/// path (component-boundary prefix), else Q's default (`None` = denied).
fn allowance_at(q: &HostJail, host: &str) -> Option<JailAccess> {
    let host_path = Path::new(host);
    let best = q
        .mounts
        .iter()
        .filter(|m| host_path.starts_with(Path::new(&m.host)))
        .max_by_key(|m| m.host.len());
    match best {
        Some(m) => Some(m.access),
        None if q.default_open => Some(JailAccess::Rw),
        None => None,
    }
}

/// Tighten one grant against the other policy: dropped when that policy
/// denies the grant's prefix outright; clamped to ro when it allows reads
/// but not writes there.
fn tighten_mount(m: &JailMount, q: &HostJail) -> Option<JailMount> {
    let allowance = allowance_at(q, &m.host)?;
    Some(JailMount {
        host: m.host.clone(),
        mount: m.mount.clone(),
        access: m.access.tighter(allowance),
    })
}

/// Request ∩ tightening (spec 08 §2/§4 — the user TIGHTENS the package's
/// request, never loosens). The composition is exact for the mount model:
/// the default is deny when either side denies; each side's grants survive
/// only capped by the other side's allowance at the same prefix (so a
/// `--no-host` drops every request grant the user did not re-allow, and an
/// ro bind stays ro under every combination); identical host prefixes
/// coalesce to the tighter access; argument files union and `auto-allowed`
/// is honored when either side asks. Associative and idempotent, so a
/// surface and the bootstrap may compose in sequence with no drift.
pub fn intersect(request: &HostJail, tightening: &HostJail) -> HostJail {
    let mut mounts: Vec<JailMount> = Vec::new();
    for m in &tightening.mounts {
        if let Some(m) = tighten_mount(m, request) {
            mounts.push(m);
        }
    }
    for m in &request.mounts {
        if let Some(m) = tighten_mount(m, tightening) {
            mounts.push(m);
        }
    }
    let mut coalesced: Vec<JailMount> = Vec::new();
    for m in mounts {
        match coalesced.iter_mut().find(|c| c.host == m.host) {
            Some(c) => c.access = c.access.tighter(m.access),
            None => coalesced.push(m),
        }
    }
    let mut files = tightening.argument_files.files.clone();
    for f in &request.argument_files.files {
        if !files.contains(f) {
            files.push(f.clone());
        }
    }
    HostJail {
        default_open: request.default_open && tightening.default_open,
        mounts: coalesced,
        argument_files: ArgumentFiles {
            auto: request.argument_files.auto || tightening.argument_files.auto,
            files,
        },
    }
}

/// The locked composition of the two policy sources (spec 08 §2): the
/// package's manifest REQUEST and the user's tightening. Returns the
/// effective jail and the audit source label (`manifest` / `user` /
/// `manifest+user`) recorded as `TEBAKO_JAIL_SOURCE` and in the journal.
pub fn effective(
    request: Option<&HostJail>,
    tightening: Option<&HostJail>,
) -> Option<(HostJail, &'static str)> {
    match (request, tightening) {
        (None, None) => None,
        (Some(r), None) => Some((r.clone(), "manifest")),
        (None, Some(t)) => Some((t.clone(), "user")),
        (Some(r), Some(t)) => Some((intersect(r, t), "manifest+user")),
    }
}

/// Resolve `auto-allowed` argument files against an argv: entries that
/// name an existing file or directory become absolute read-only grants
/// (relative names resolve against the process cwd; entries starting with
/// `-` are flags, never files). Non-existent entries are skipped. The
/// engine canonicalizes at bind, so lexical absolutes are fine here.
pub fn resolve_argument_files(args: &[String]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for a in args {
        if a.starts_with('-') {
            continue;
        }
        let p = PathBuf::from(a);
        let abs = if p.is_absolute() {
            p
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(p),
                Err(_) => continue,
            }
        };
        if abs.exists() && !out.contains(&abs) {
            out.push(abs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mount(host: &str, mount: &str, access: JailAccess) -> JailMount {
        JailMount {
            host: host.to_string(),
            mount: mount.to_string(),
            access,
        }
    }

    // ---------------------------------------------------------------
    // YAML block form
    // ---------------------------------------------------------------

    #[test]
    fn yaml_block_round_trip() {
        let text = "default: deny\n\
                    mounts:\n\
                    \x20 - {host: /home/u/sources, mount: /work, access: rw}\n\
                    \x20 - {host: /data, mount: /data, access: ro}\n\
                    argument_files: auto-allowed\n";
        let jail = HostJail::from_yaml(text).unwrap();
        assert!(!jail.default_open);
        assert_eq!(jail.mounts.len(), 2);
        assert_eq!(jail.mounts[0].access, JailAccess::Rw);
        assert!(jail.argument_files.auto);
        let rendered = jail.to_yaml().unwrap();
        let back = HostJail::from_yaml(&rendered).unwrap();
        assert_eq!(back, jail);
    }

    #[test]
    fn yaml_argument_files_list_form() {
        let jail =
            HostJail::from_yaml("default: deny\nargument_files: [/in/a.csv, /in/b.csv]\n").unwrap();
        assert!(!jail.argument_files.auto);
        assert_eq!(
            jail.argument_files.files,
            vec!["/in/a.csv".to_string(), "/in/b.csv".to_string()]
        );
        let back = HostJail::from_yaml(&jail.to_yaml().unwrap()).unwrap();
        assert_eq!(back, jail);
    }

    #[test]
    fn yaml_minimal_block_defaults_to_open() {
        let jail = HostJail::from_yaml("{}\n").unwrap();
        assert_eq!(jail, HostJail::open());
        // …and an omitted default serializes explicitly (self-describing).
        assert!(jail.to_yaml().unwrap().contains("default: open"));
    }

    #[test]
    fn yaml_rejections() {
        for (text, frag) in [
            ("default: strict\n", "open|deny"),
            (
                "mounts: [{host: /h, mount: rel, access: ro}]\n",
                "not absolute",
            ),
            ("mounts: [{host: /h, mount: /m, access: xo}]\n", "ro|rw"),
            ("argument_files: sometimes\n", "auto-allowed"),
        ] {
            let e = HostJail::from_yaml(text).unwrap_err();
            assert!(
                e.to_string().contains(frag),
                "{text:?}: error {e} should mention {frag:?}"
            );
        }
        // Unknown keys tolerated (the manifest convention).
        let jail = HostJail::from_yaml("default: deny\nfuture: yes\n").unwrap();
        assert!(!jail.default_open);
    }

    // ---------------------------------------------------------------
    // env form (spec 08 §1 — one grammar with tfs::policy::JailSpec)
    // ---------------------------------------------------------------

    #[test]
    fn env_spec_parses_all_directive_kinds() {
        let j =
            HostJail::parse_env_spec("deny;/home/u/src:/work:rw;@/home/u/in.csv;/data:/data:ro")
                .unwrap();
        assert!(!j.default_open);
        assert_eq!(
            j.mounts,
            vec![
                mount("/home/u/src", "/work", JailAccess::Rw),
                mount("/data", "/data", JailAccess::Ro),
            ]
        );
        assert_eq!(j.argument_files.files, vec!["/home/u/in.csv".to_string()]);
    }

    #[test]
    fn env_spec_defaults_to_open_and_tolerates_colons_in_hosts() {
        let j = HostJail::parse_env_spec("/Volumes/a:b/work:/w:rw").unwrap();
        assert!(j.default_open);
        assert_eq!(j.mounts[0].host, "/Volumes/a:b/work");
    }

    #[test]
    fn env_spec_rejections_mirror_the_engine() {
        for (spec, frag) in [
            ("", "empty spec"),
            ("   ", "empty spec"),
            ("open;deny", "duplicate/conflicting"),
            ("deny;deny", "duplicate/conflicting"),
            ("deny;;/h:/w:ro", "empty directive"),
            ("/h:/w:xx", "unknown access mode"),
            ("/h:w:ro", "not absolute"),
            ("/h:/w", "unknown access mode"),
            ("frob", "host:mount:ro|rw"),
            (":/w:ro", "empty host"),
            ("@", "empty argument file"),
        ] {
            let e = HostJail::parse_env_spec(spec).unwrap_err();
            assert!(
                e.to_string().contains(frag),
                "spec {spec:?}: error {e} should mention {frag:?}"
            );
        }
        assert!(HostJail::parse_env_spec("frob")
            .unwrap_err()
            .to_string()
            .contains("\"frob\""));
    }

    #[test]
    fn env_spec_round_trip() {
        let j = HostJail::parse_env_spec("deny;/a:/work:rw;@/in.csv").unwrap();
        let env = j.to_env_spec(&[]);
        assert_eq!(env, "deny;/a:/work:rw;@/in.csv");
        let back = HostJail::parse_env_spec(&env).unwrap();
        assert_eq!(back, j);
        assert_eq!(HostJail::open().to_env_spec(&[]), "open");
    }

    #[test]
    fn to_env_spec_unions_resolved_argument_files() {
        let mut j = HostJail::deny_with_arg_files();
        j.argument_files.files = vec!["/explicit".to_string()];
        let env = j.to_env_spec(&[PathBuf::from("/resolved"), PathBuf::from("/explicit")]);
        assert_eq!(env, "deny;@/explicit;@/resolved");
    }

    // ---------------------------------------------------------------
    // --jail cli spec forms
    // ---------------------------------------------------------------

    #[test]
    fn cli_spec_profiles() {
        assert_eq!(HostJail::from_cli_spec("open").unwrap(), HostJail::open());
        assert_eq!(HostJail::from_cli_spec("deny").unwrap(), HostJail::deny());
        let j = HostJail::from_cli_spec("deny:arg").unwrap();
        assert!(!j.default_open);
        assert!(j.argument_files.auto);
    }

    #[test]
    fn cli_spec_yaml_file() {
        let dir = std::env::temp_dir().join(format!("tpkg-jail-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("jail.yaml");
        std::fs::write(
            &file,
            "default: deny\nmounts: [{host: /h, mount: /m, access: ro}]\n",
        )
        .unwrap();
        let j = HostJail::from_cli_spec(file.to_str().unwrap()).unwrap();
        assert!(!j.default_open);
        assert_eq!(j.mounts, vec![mount("/h", "/m", JailAccess::Ro)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cli_spec_env_grammar_passthrough() {
        let j = HostJail::from_cli_spec("deny;/data:/data:ro").unwrap();
        assert_eq!(j.mounts, vec![mount("/data", "/data", JailAccess::Ro)]);
        assert!(HostJail::from_cli_spec("frob").is_err());
    }

    // ---------------------------------------------------------------
    // intersect / effective — the locked precedence (spec 08 §2/§4)
    // ---------------------------------------------------------------

    #[test]
    fn intersect_deny_wins_either_side() {
        let open = HostJail::open();
        let deny = HostJail::deny();
        assert!(!intersect(&open, &deny).default_open);
        assert!(!intersect(&deny, &open).default_open);
        assert!(intersect(&open, &open).default_open);
    }

    #[test]
    fn intersect_user_no_host_drops_request_grants() {
        // The manifest requests /a ro; the user says deny with no grants:
        // the effective policy allows NOTHING (the user's tightening caps
        // the request — never the reverse).
        let mut request = HostJail::deny();
        request.mounts = vec![mount("/a", "/a", JailAccess::Ro)];
        let eff = intersect(&request, &HostJail::deny());
        assert!(!eff.default_open);
        assert!(eff.mounts.is_empty(), "{eff:?}");
    }

    #[test]
    fn intersect_user_grant_capped_by_request_deny() {
        // The user adds /x rw under an open user default; the manifest
        // denies everything outside /a: /x must NOT survive (the manifest
        // request is a ceiling — wider latitude comes by trust policy,
        // never by a flag).
        let mut request = HostJail::deny();
        request.mounts = vec![mount("/a", "/a", JailAccess::Rw)];
        let mut user = HostJail::open();
        user.mounts = vec![mount("/x", "/x", JailAccess::Rw)];
        let eff = intersect(&request, &user);
        assert!(!eff.default_open);
        assert_eq!(eff.mounts, vec![mount("/a", "/a", JailAccess::Rw)]);
    }

    #[test]
    fn intersect_ro_is_sticky_across_combinations() {
        // ro bind in the request, rw at the same prefix in the tightening:
        // the tighter access wins (docker-style, never loosened).
        let mut request = HostJail::deny();
        request.mounts = vec![mount("/a", "/a", JailAccess::Ro)];
        let mut user = HostJail::deny();
        user.mounts = vec![mount("/a", "/a", JailAccess::Rw)];
        let eff = intersect(&request, &user);
        assert_eq!(eff.mounts, vec![mount("/a", "/a", JailAccess::Ro)]);
    }

    #[test]
    fn intersect_nested_prefixes_clamp_to_the_covering_allowance() {
        // request grants /a/b rw; the tightening grants /a ro: /a/b clamps
        // to ro, and the request's broader context is capped to /a/b.
        let mut request = HostJail::deny();
        request.mounts = vec![mount("/a/b", "/b", JailAccess::Rw)];
        let mut user = HostJail::deny();
        user.mounts = vec![mount("/a", "/a", JailAccess::Ro)];
        let eff = intersect(&request, &user);
        assert_eq!(eff.mounts, vec![mount("/a/b", "/b", JailAccess::Ro)]);
    }

    #[test]
    fn intersect_open_request_keeps_user_grants_verbatim() {
        // No manifest request (open): the user's policy is applied as-is.
        let mut user = HostJail::deny();
        user.mounts = vec![mount("/work", "/work", JailAccess::Rw)];
        let eff = intersect(&HostJail::open(), &user);
        assert_eq!(eff, user);
    }

    #[test]
    fn intersect_argument_files_union_and_auto_either_side() {
        let mut request = HostJail::deny_with_arg_files();
        request.argument_files.files = vec!["/a".to_string()];
        let mut user = HostJail::deny();
        user.argument_files.files = vec!["/b".to_string(), "/a".to_string()];
        let eff = intersect(&request, &user);
        assert!(eff.argument_files.auto);
        assert_eq!(
            eff.argument_files.files,
            vec!["/b".to_string(), "/a".to_string()]
        );
    }

    #[test]
    fn intersect_is_idempotent_and_recomposition_stable() {
        // A surface composes, then the bootstrap composes the result with
        // the manifest again: no drift (the double-compose property the
        // `tebako run` → bootstrap chain relies on).
        let mut request = HostJail::deny();
        request.mounts = vec![mount("/a", "/a", JailAccess::Rw)];
        request.argument_files.auto = true;
        let mut user = HostJail::deny();
        user.mounts = vec![
            mount("/a", "/a", JailAccess::Ro),
            mount("/b", "/b", JailAccess::Rw),
        ];
        let once = intersect(&request, &user);
        let twice = intersect(&request, &once);
        assert_eq!(once, twice);
        assert_eq!(intersect(&once, &once), once);
    }

    #[test]
    fn effective_labels_the_source() {
        let deny = HostJail::deny();
        assert!(effective(None, None).is_none());
        assert_eq!(effective(Some(&deny), None).unwrap().1, "manifest");
        assert_eq!(effective(None, Some(&deny)).unwrap().1, "user");
        assert_eq!(
            effective(Some(&deny), Some(&deny)).unwrap().1,
            "manifest+user"
        );
    }

    #[test]
    fn trivially_open_detection() {
        assert!(HostJail::open().is_trivially_open());
        assert!(!HostJail::deny_with_arg_files().is_trivially_open());
        let mut j = HostJail::open();
        j.mounts = vec![mount("/a", "/a", JailAccess::Ro)];
        assert!(!j.is_trivially_open());
        // auto-allowed under an open default grants nothing extra.
        j.mounts.clear();
        j.argument_files.auto = true;
        assert!(j.is_trivially_open());
    }

    #[test]
    fn resolve_argument_files_picks_existing_entries() {
        let dir = std::env::temp_dir().join(format!("tpkg-jail-args-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("input.csv");
        std::fs::write(&file, b"x").unwrap();
        let args = vec![
            file.to_string_lossy().into_owned(),
            "--verbose".to_string(),
            dir.join("missing").to_string_lossy().into_owned(),
            file.to_string_lossy().into_owned(), // duplicate
        ];
        let resolved = resolve_argument_files(&args);
        assert_eq!(resolved, vec![file]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trailing_slashes_normalize() {
        let j = HostJail::parse_env_spec("deny;/a/:/w:ro").unwrap();
        assert_eq!(j.mounts[0].host, "/a");
        let j = HostJail::parse_env_spec("deny;/:/w:rw").unwrap();
        assert_eq!(j.mounts[0].host, "/");
    }
}

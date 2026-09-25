//! Authored and managed configuration surfaces (all YAML — spec 00
//! invariant 6, spec 07 §4):
//!
//! - `~/.tebako/config.yaml` — USER-authored: `defaults:` (per-tool
//!   version, written by `tebako use <tool>@<version>`), `registries:`
//!   (spec 04 refs), `runtimes:` (per-engine runtime preferences, written
//!   by `tebako use --runtime <engine>@<version>`). The dispatcher only
//!   READS this file; the one write path is `tebako add-registry`
//!   ([`add_registry`]) — a structural edit that preserves keys, not
//!   comments.
//! - `~/.tebako/shims/.disabled.yaml` — SHIM-managed state (enable /
//!   disable). Kept out of the authored config so `tebako-shim disable`
//!   never rewrites a hand-maintained file.
//! - `tpkg-registry.yaml` — the developer-hosted registry (spec 04 §2).
//!   The registry-default chain link resolves every registry form through
//!   tebako-resolve behind the dispatch-time cache ([`crate::regcache`]:
//!   24 h TTL, `tebako update-registries`, `TEBAKO_OFFLINE` = cache-or-
//!   named-error); the registry model is tebako-resolve's (one model,
//!   parse + validate).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use tebako_resolve::credentials::CredentialEntry;
use tebako_resolve::credentials::{CredentialBook, Tier1Entry};

use crate::{fail, Ctx, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST};

#[derive(Debug, Default, Deserialize)]
pub struct UserConfig {
    /// Tool/command name → default pin (`tebako use <tool>@<version>`).
    /// The value is either a bare version string or the widened map form
    /// `{version: "…", slices: [name@ver, …]}` (spec 07 §4 — extension
    /// slices); [`DefaultPin`] covers both spellings.
    #[serde(default)]
    pub defaults: BTreeMap<String, DefaultPin>,
    /// Spec 37 §2's registry book. Each entry is either a bare spec 04
    /// ref string (the pre-book spelling) or a map carrying `ref:` plus
    /// the optional `name:` / `default:` / `require_signed:` policy keys;
    /// [`RegistryBookEntry`] covers both spellings. The validated,
    /// alias-resolved view is [`UserConfig::registry_book`].
    #[serde(default)]
    pub registries: Vec<RegistryBookEntry>,
    /// Spec 37 §5's credential book: env var NAMES (never secrets)
    /// keyed by registry alias (tier 1) or host (tier 2). The
    /// validated, book-resolved view is [`UserConfig::credential_book`].
    #[serde(default)]
    pub credentials: Vec<CredentialEntry>,
    /// Engine → runtime preference (the download fallback of spec 05 §5:
    /// "download the newest compatible" needs an exact ref; the
    /// preference names it until the runtime registry ships).
    #[serde(default)]
    pub runtimes: BTreeMap<String, RuntimePref>,
    /// Document-level auto-slice gate (spec 07 §4): `false` kills the
    /// spec 07 §2 step-3 store scan; explicit slice pins still attach.
    /// Absent = auto discovery on. A project file's `auto_slices:` wins
    /// over this user-config value (same precedence as versions).
    #[serde(default)]
    pub auto_slices: Option<bool>,
    /// Fetch-pipeline worker count (spec 05 §6): `TEBAKO_FETCH_JOBS` wins
    /// over this value; absent = the default 3.
    #[serde(default)]
    pub fetch_jobs: Option<u32>,
    /// Enterprise networking (TODO.v2-1/33, spec 04 amendment): proxy +
    /// trust anchors. Env wins per key; see [`install_network_config`].
    #[serde(default)]
    pub network: NetworkSection,
}

/// One `defaults:` entry (spec 07 §4; `registry:` added by spec 37 §3).
/// The pre-slices shape is a bare version string; the widened shape is
/// a map carrying an optional `version:` plus a `slices:` list of
/// `name@version` pins, plus spec 37's optional `registry:` — the alias
/// of the registry the pin resolves through. Both spellings
/// deserialize here; accessors keep every consumer off the shape
/// detail.
///
/// The hand-rolled Deserialize goes through `serde_yaml::Value` because
/// serde_yaml hands a bare-`String` target the RAW scalar text — the
/// pre-slices `defaults: {tool: version}` grammar has always accepted
/// unquoted scalars (`app: 1.0` → "1.0"), and untagged-enum buffering
/// would silently lose that leniency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultPin {
    /// `tool: "1.16.2"` — the pre-slices spelling.
    Version(String),
    /// `tool: {version: "1.16.2", slices: [metanorma-bsi@1.2.0],
    /// registry: nist}` — any key may be absent (a slices-only entry
    /// does not pin the base version; a registry-only entry only
    /// scopes where resolution looks).
    Full {
        version: Option<String>,
        slices: Vec<String>,
        registry: Option<String>,
    },
}

impl<'de> Deserialize<'de> for DefaultPin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        match serde_yaml::Value::deserialize(deserializer)? {
            serde_yaml::Value::String(s) => Ok(DefaultPin::Version(s)),
            serde_yaml::Value::Number(n) => Ok(DefaultPin::Version(n.to_string())),
            serde_yaml::Value::Bool(b) => Ok(DefaultPin::Version(b.to_string())),
            serde_yaml::Value::Mapping(m) => {
                let mut version = None;
                let mut slices = Vec::new();
                let mut registry = None;
                for (k, v) in m {
                    let key = k.as_str().ok_or_else(|| {
                        D::Error::custom("a `defaults` map entry's keys must be strings")
                    })?;
                    match key {
                        "version" => {
                            version =
                                match v {
                                    serde_yaml::Value::Null => None,
                                    serde_yaml::Value::String(s) => Some(s),
                                    serde_yaml::Value::Number(n) => Some(n.to_string()),
                                    _ => return Err(D::Error::custom(
                                        "a `defaults` map entry's `version:` is a version string",
                                    )),
                                };
                        }
                        "slices" => {
                            slices = serde_yaml::from_value(v).map_err(D::Error::custom)?;
                        }
                        "registry" => {
                            registry = match v {
                                serde_yaml::Value::Null => None,
                                serde_yaml::Value::String(s) => Some(s),
                                _ => {
                                    return Err(D::Error::custom(
                                        "a `defaults` map entry's `registry:` is a registry alias string",
                                    ))
                                }
                            };
                        }
                        _ => {}
                    }
                }
                Ok(DefaultPin::Full {
                    version,
                    slices,
                    registry,
                })
            }
            _ => Err(D::Error::custom(
                "a `defaults` entry is a version string or a {version, slices} map",
            )),
        }
    }
}

impl DefaultPin {
    /// The pinned base version, if this entry pins one.
    pub fn version(&self) -> Option<&str> {
        match self {
            DefaultPin::Version(v) => Some(v.as_str()),
            DefaultPin::Full { version, .. } => version.as_deref(),
        }
    }

    /// The pinned slices (`name@version` strings, the grammar of
    /// `tpkg::toolpin::ToolPin` with a required payload part).
    pub fn slices(&self) -> &[String] {
        match self {
            DefaultPin::Version(_) => &[],
            DefaultPin::Full { slices, .. } => slices,
        }
    }

    /// The registry alias the pin resolves through (spec 37 §3), if
    /// authored.
    pub fn registry(&self) -> Option<&str> {
        match self {
            DefaultPin::Version(_) => None,
            DefaultPin::Full { registry, .. } => registry.as_deref(),
        }
    }
}

/// One `registries:` entry (spec 37 §2 — the registry book). The
/// pre-book shape is a bare spec 04 reference string; the book shape is
/// a map carrying `ref:` plus optional `name:` (the LOCAL alias — never
/// published, never embedded, never on any wire), `default:` (§2.1's
/// publish/UX anchor, never a resolution tiebreaker), and
/// `require_signed:` (§2.2's fail-closed trust policy). Both spellings
/// deserialize here; a bare entry's flags are absent/false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryBookEntry {
    /// The spec 04 registry reference (the canonical string form —
    /// `add-registry` registers the canonical spelling).
    pub reference: String,
    /// The authored local alias (`name:`), if any. Absent = the alias
    /// derives from the ref's `owner/repo` at book resolution.
    pub name: Option<String>,
    /// §2.1's publish/UX anchor. At most one entry in the book.
    pub default: bool,
    /// §2.2's fail-closed signature policy for this registry's rows.
    pub require_signed: bool,
}

impl RegistryBookEntry {
    /// A bare entry (no alias, no policy) — the pre-book spelling.
    pub fn bare(reference: String) -> Self {
        RegistryBookEntry {
            reference,
            name: None,
            default: false,
            require_signed: false,
        }
    }

    /// The registry reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// True when no book keys are set — the entry serializes as the
    /// bare ref string (round-trip cleanliness for pre-book configs).
    pub fn is_bare(&self) -> bool {
        self.name.is_none() && !self.default && !self.require_signed
    }

    /// The YAML form of this entry for authored-config writes: a bare
    /// string when [`RegistryBookEntry::is_bare`], else the map shape.
    pub fn to_yaml_value(&self) -> serde_yaml::Value {
        if self.is_bare() {
            return serde_yaml::Value::String(self.reference.clone());
        }
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("ref".to_string()),
            serde_yaml::Value::String(self.reference.clone()),
        );
        if let Some(name) = &self.name {
            m.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(name.clone()),
            );
        }
        if self.default {
            m.insert(
                serde_yaml::Value::String("default".to_string()),
                serde_yaml::Value::Bool(true),
            );
        }
        if self.require_signed {
            m.insert(
                serde_yaml::Value::String("require_signed".to_string()),
                serde_yaml::Value::Bool(true),
            );
        }
        serde_yaml::Value::Mapping(m)
    }
}

impl<'de> Deserialize<'de> for RegistryBookEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        match serde_yaml::Value::deserialize(deserializer)? {
            serde_yaml::Value::String(s) => Ok(RegistryBookEntry::bare(s)),
            serde_yaml::Value::Mapping(m) => {
                let mut entry = RegistryBookEntry::bare(String::new());
                for (k, v) in m {
                    let key = k.as_str().ok_or_else(|| {
                        D::Error::custom("a `registries` map entry's keys must be strings")
                    })?;
                    match key {
                        "ref" => {
                            entry.reference = match v {
                                serde_yaml::Value::String(s) => s,
                                _ => {
                                    return Err(D::Error::custom(
                                        "a `registries` map entry's `ref:` is a reference string",
                                    ))
                                }
                            };
                        }
                        "name" => {
                            entry.name = match v {
                                serde_yaml::Value::Null => None,
                                serde_yaml::Value::String(s) => Some(s),
                                _ => {
                                    return Err(D::Error::custom(
                                        "a `registries` map entry's `name:` is a string",
                                    ))
                                }
                            };
                        }
                        "default" => {
                            entry.default = match v {
                                serde_yaml::Value::Bool(b) => b,
                                _ => {
                                    return Err(D::Error::custom(
                                        "a `registries` map entry's `default:` is a boolean",
                                    ))
                                }
                            };
                        }
                        "require_signed" => {
                            entry.require_signed = match v {
                                serde_yaml::Value::Bool(b) => b,
                                _ => {
                                    return Err(D::Error::custom(
                                        "a `registries` map entry's `require_signed:` is a boolean",
                                    ))
                                }
                            };
                        }
                        // Unknown keys are ignored (forward compat — the
                        // same leniency as `defaults:` map entries).
                        _ => {}
                    }
                }
                if entry.reference.is_empty() {
                    return Err(D::Error::custom(
                        "a `registries` map entry needs a `ref:` key",
                    ));
                }
                Ok(entry)
            }
            _ => Err(D::Error::custom(
                "a `registries` entry is a reference string or a {ref, name?, default?, require_signed?} map",
            )),
        }
    }
}

/// The registry-alias grammar (spec 37 §2): `[a-z][a-z0-9-]*`. THE OWNER
/// is tpkg (the edge `registry:` pin validates there — the flow direction
/// cannot reverse, tpkg is the lower crate); the book's alias checks flow
/// it from there.
pub fn valid_registry_alias(alias: &str) -> bool {
    tpkg::valid_registry_alias(alias)
}

/// One resolved book row: the entry plus its computed alias (the
/// authored `name:` or the derivation from the ref's `owner/repo` —
/// spec 37 §2). `alias` is None for refs with no `owner/repo` shape
/// (`file:`, `tfs+https:`, plain paths) and no authored name; such a
/// row is unaddressable by the qualified `alias/name` form but still
/// participates in bare-name resolution.
#[derive(Debug, Clone)]
pub struct BookRow<'a> {
    pub entry: &'a RegistryBookEntry,
    pub alias: Option<String>,
}

/// Derive the alias from a registry reference: the `repo` segment of a
/// service reference (`tfs:github:owner/repo[…]` → `repo`), None for
/// every other form. Parsed through tebako-resolve's ONE registry-ref
/// grammar — never string-sliced here.
fn derive_alias(reference: &str) -> Option<String> {
    use tebako_resolve::registry::RegistryRef;
    match RegistryRef::parse(reference) {
        Ok(RegistryRef::DefaultBranch { repo, .. }) => Some(repo),
        Ok(RegistryRef::ReleaseArtifact(tebako_resolve::Reference::Service { repo, .. })) => {
            Some(repo)
        }
        _ => None,
    }
}

impl UserConfig {
    /// The validated registry book (spec 37 §2): every entry paired
    /// with its computed alias, fail-closed at config load —
    /// `DuplicateRegistryAlias` on a collision (both entries named,
    /// never a silent rename), `DuplicateDefaultRegistry` on two
    /// `default: true` entries, and a malformed authored alias is a
    /// plain config error naming the grammar.
    pub fn registry_book(&self) -> Result<Vec<BookRow<'_>>, ShimError> {
        let mut rows = Vec::with_capacity(self.registries.len());
        let mut seen: Vec<(String, &RegistryBookEntry)> = Vec::new();
        let mut default_entry: Option<&RegistryBookEntry> = None;
        for entry in &self.registries {
            if let Some(name) = &entry.name {
                if !valid_registry_alias(name) {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "registry entry `{}` names the alias '{name}' — the alias grammar is [a-z][a-z0-9-]* (spec 37 §2)",
                            entry.reference
                        ),
                    );
                }
            }
            let alias = entry
                .name
                .clone()
                .or_else(|| derive_alias(&entry.reference));
            if let Some(alias) = &alias {
                if let Some((_, prior)) = seen.iter().find(|(a, _)| a == alias) {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "registries `{}` and `{}` resolve to the same alias '{alias}' (DuplicateRegistryAlias) — name one of them explicitly",
                            prior.reference, entry.reference
                        ),
                    );
                }
                seen.push((alias.clone(), entry));
            }
            if entry.default {
                if let Some(prior) = default_entry {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "registries `{}` and `{}` both carry `default: true` (DuplicateDefaultRegistry) — exactly one entry may",
                            prior.reference, entry.reference
                        ),
                    );
                }
                default_entry = Some(entry);
            }
            rows.push(BookRow { entry, alias });
        }
        Ok(rows)
    }

    /// The registry refs a resolution runs against (spec 37 §3):
    /// `None` → every registered entry (the bare-name rule); `Some(alias)`
    /// → the ONE entry whose computed alias matches. An unknown alias is
    /// the named `UnknownRegistryAlias` listing the book's aliases —
    /// fail-closed, never a silent fall back to the whole book.
    pub fn registry_refs_scoped(&self, scope: Option<&str>) -> Result<Vec<&str>, ShimError> {
        Ok(self
            .registry_book_scoped(scope)?
            .iter()
            .map(|row| row.entry.reference())
            .collect())
    }

    /// The book ROWS a resolution runs against — the row-carrying form
    /// of [`UserConfig::registry_refs_scoped`] (one scoping code path):
    /// consumers that need an entry's policy flags (§2.2's
    /// `require_signed`) or its computed alias take the rows. The book
    /// validates at [`load_config`], so the `None` arm's revalidation
    /// here never produces an error the load did not already name.
    pub fn registry_book_scoped(&self, scope: Option<&str>) -> Result<Vec<BookRow<'_>>, ShimError> {
        match scope {
            None => self.registry_book(),
            Some(alias) => {
                let book = self.registry_book()?;
                if let Some(row) = book.iter().find(|row| row.alias.as_deref() == Some(alias)) {
                    return Ok(vec![BookRow {
                        entry: row.entry,
                        alias: row.alias.clone(),
                    }]);
                }
                let aliases: Vec<&str> = book.iter().filter_map(|r| r.alias.as_deref()).collect();
                let listing = if aliases.is_empty() {
                    "(none)".to_string()
                } else {
                    aliases.join(", ")
                };
                fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "no registered registry carries the alias '{alias}' (UnknownRegistryAlias)\n  registered aliases: {listing}"
                    ),
                )
            }
        }
    }
}

// ---------------------------------------------------------------------
// spec 37 §5 — the credential book
// ---------------------------------------------------------------------

/// The `token_env` grammar (config holds env var NAMES, never secrets):
/// `[A-Za-z_][A-Za-z0-9_]*`.
fn valid_token_env(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The hosts a tier-1 credential for the registry at `reference` may be
/// presented to (spec 37 §5's confinement): the ref's own host plus the
/// service's API host — derived from the ADAPTERS' base-url
/// construction ([`tebako_resolve::adapters::service_hosts`], the SSOT),
/// never a hand-written mapping here. A GitBlob ref contributes the git
/// host, an Https ref the URL host, a File ref nothing.
fn registry_ref_hosts(reference: &str) -> BTreeSet<String> {
    use tebako_resolve::{Reference, RegistryRef};
    match RegistryRef::parse(reference) {
        Ok(RegistryRef::DefaultBranch { service, host, .. })
        | Ok(RegistryRef::ReleaseArtifact(Reference::Service { service, host, .. })) => {
            tebako_resolve::adapters::service_hosts(service, host.as_deref())
        }
        Ok(RegistryRef::GitBlob(Reference::Git { url, .. })) => url
            .split('/')
            .next()
            .into_iter()
            .map(str::to_string)
            .collect(),
        Ok(RegistryRef::Https(Reference::Https { url, .. })) => {
            tebako_resolve::credentials::url_host(&url)
                .into_iter()
                .map(str::to_string)
                .collect()
        }
        _ => BTreeSet::new(),
    }
}

/// The canonical form of a registry ref (the ref_index's lookup key —
/// `resolve_registry` reverse-looks its parsed ref's canonical string).
/// An unparseable ref indexes under its authored spelling (resolution
/// names it later, at its own boundary).
fn canonical_ref(reference: &str) -> String {
    tebako_resolve::RegistryRef::parse(reference)
        .map(|r| r.as_canonical_string())
        .unwrap_or_else(|_| reference.to_string())
}

impl UserConfig {
    /// The validated credential book (spec 37 §5), fail-closed at
    /// config load like the registry book: exactly one selector per
    /// entry (`registry:` XOR `host:` — both or neither is a named
    /// config error), a well-formed `token_env`, and no duplicate
    /// tier-1 alias or tier-2 host (`DuplicateCredentialSelector`,
    /// both entries named). A tier-1 alias matching NO registry book
    /// entry is ACCEPTED (absent-behavior — an entry for a later-added
    /// registry must not break loads); its allowed-host set stays
    /// empty until the registry exists.
    pub fn credential_book(&self) -> Result<CredentialBook, ShimError> {
        let mut tier1: Vec<Tier1Entry> = Vec::new();
        let mut tier2: Vec<(String, String)> = Vec::new();
        for entry in &self.credentials {
            match (&entry.registry, &entry.host) {
                (Some(_), Some(_)) => {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        "a credentials entry names both `registry:` and `host:` (InvalidCredentialEntry) — exactly one selector per entry (spec 37 §5)",
                    )
                }
                (None, None) => {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        "a credentials entry names neither `registry:` nor `host:` (InvalidCredentialEntry) — exactly one selector per entry (spec 37 §5)",
                    )
                }
                _ => {}
            }
            if !valid_token_env(&entry.token_env) {
                return fail(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "a credentials entry's token_env '{}' is malformed (InvalidCredentialEntry) — the grammar is [A-Za-z_][A-Za-z0-9_]* (config holds env var NAMES, never secrets)",
                        entry.token_env
                    ),
                );
            }
            if let Some(alias) = &entry.registry {
                if let Some(prior) = tier1.iter().find(|e| &e.alias == alias) {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "credentials entries for registry '{alias}' name both {} and {} (DuplicateCredentialSelector) — one entry per selector",
                            prior.token_env, entry.token_env
                        ),
                    );
                }
                tier1.push(Tier1Entry {
                    alias: alias.clone(),
                    token_env: entry.token_env.clone(),
                    allowed_hosts: BTreeSet::new(),
                });
            }
            if let Some(host) = &entry.host {
                if let Some((_, prior_env)) = tier2.iter().find(|(h, _)| h == host) {
                    return fail(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "credentials entries for host '{host}' name both {prior_env} and {} (DuplicateCredentialSelector) — one entry per selector",
                            entry.token_env
                        ),
                    );
                }
                tier2.push((host.clone(), entry.token_env.clone()));
            }
        }
        // The tier-1 allowed-host sets flow from the registry book: the
        // alias's ref, parsed through the ONE registry-ref grammar.
        let rows = self.registry_book()?;
        for entry in &mut tier1 {
            if let Some(row) = rows
                .iter()
                .find(|r| r.alias.as_deref() == Some(entry.alias.as_str()))
            {
                entry.allowed_hosts = registry_ref_hosts(row.entry.reference());
            }
        }
        let ref_index = rows
            .iter()
            .filter_map(|row| {
                row.alias
                    .clone()
                    .map(|alias| (canonical_ref(row.entry.reference()), alias))
            })
            .collect();
        Ok(CredentialBook {
            tier1,
            tier2,
            ref_index,
        })
    }
}

/// The `network:` section of `~/.tebako/config.yaml` — all keys optional.
/// These are the config MIRRORS of the env spellings; the environment
/// always wins per key (merge in `tebako_http::netconfig`).
#[derive(Debug, Default, Deserialize)]
pub struct NetworkSection {
    /// `network.proxy: http://[user[:pass]@]host[:port]` — the explicit
    /// (CONNECT) proxy; NO_PROXY env still applies on top.
    pub proxy: Option<String>,
    /// `network.tls_roots: platform` — the OS store (GPO/MDM-pushed
    /// enterprise roots). Absent/`webpki` = the bundled Mozilla roots.
    pub tls_roots: Option<String>,
    /// `network.extra_ca: [/path/ca.pem, …]` — PEMs added TO the bundled
    /// store (never instead of it; combine with `platform` = named error).
    #[serde(default)]
    pub extra_ca: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
pub struct RuntimePref {
    /// Language version, e.g. `4.0.6`.
    pub version: String,
    /// The tebako (launcher) abi version the runtime was built with,
    /// e.g. `0.16.0` — the `<ver>` of the cache layout
    /// `runtimes/<lang>-<lv>-<ver>-<triplet>/`. Absent/empty when written
    /// by `tebako-shim use --runtime <engine>@<langver>` without the
    /// `:<tebako>` part — readers treat it as the product default line
    /// (tebako-resolve::DEFAULT_TEBAKO_VERSION).
    #[serde(default)]
    pub tebako: String,
    /// The per-engine download base pin (spec 05 §2's channel 1, #567):
    /// a release download base URL this engine's runtimes resolve from,
    /// ahead of `TEBAKO_RUNTIME_MIRROR` (a differing mirror value is
    /// shadowed — loud + journaled). Absent = the rest of the chain
    /// (mirror env → registry-derived → the ruby default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub fn config_path(home: &Path) -> PathBuf {
    home.join("config.yaml")
}

pub fn load_config(home: &Path) -> Result<UserConfig, ShimError> {
    let path = config_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => Some(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let cfg: UserConfig = match text {
        None => UserConfig::default(),
        Some(text) => serde_yaml::from_str(&text).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "cannot parse {} ({e}) — fix or remove it; run `tebako-shim doctor`",
                    path.display()
                ),
            )
        })?,
    };
    // The registry book validates at load (spec 37 §2 — fail-closed):
    // a malformed alias, a collision, or two defaults is the named
    // error here, never a surprise mid-resolution.
    cfg.registry_book()?;
    // The credential book validates at load too (spec 37 §5 — selector
    // shape, token_env grammar, duplicate selectors), then installs
    // process-wide HERE — the ONE install point, so every consumer
    // (the cli, shim dispatch, pkg) decides with no call-site churn.
    // Book-empty configs install the empty book: decisions
    // short-circuit to the ambient/anonymous behavior and the fetch
    // journal stays silent.
    let book = cfg.credential_book()?;
    tebako_resolve::credentials::install_book(book);
    tebako_resolve::credentials::set_journal_home(Some(home.to_path_buf()));
    Ok(cfg)
}

// ---------------------------------------------------------------------
// enterprise networking (TODO.v2-1/33): install the effective config
// ---------------------------------------------------------------------

/// Resolve the effective network config — env over the config file's
/// `network:` section — WITHOUT installing it (spec 35's doctor reads
/// this; the fetching binaries install via [`install_network_config`]).
/// A malformed section is the same named error either way.
pub fn effective_network_config(
    home: &Path,
) -> Result<tebako_http::netconfig::NetworkConfig, ShimError> {
    let cfg = load_config(home)?;
    let roots = match cfg.network.tls_roots.as_deref() {
        None => None,
        Some("platform") => Some(tebako_http::netconfig::TlsRoots::Platform),
        Some("webpki") => Some(tebako_http::netconfig::TlsRoots::WebPki),
        Some(other) => {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "config.yaml network.tls_roots: `{other}` — expected `platform` or `webpki`"
                ),
            )
        }
    };
    let effective = tebako_http::netconfig::NetworkConfig::from_env().merge_file(
        cfg.network.proxy,
        roots,
        cfg.network.extra_ca,
        &config_path(home),
    );
    effective
        .validate()
        .map_err(|e| ShimError::new(EX_TEBAKO_MANIFEST, e.to_string()))?;
    Ok(effective)
}

/// Resolve the `network:` section under the environment and install it
/// as tebako-http's process-global config. Every binary that fetches
/// calls this at startup, before the first request (agent construction
/// caches the transport). Audit lines land in the journal — the loud
/// record the trust story requires; best-effort, like every journal
/// write. A malformed section is a named error at startup, never a
/// silent fallback.
pub fn install_network_config(home: &Path) -> Result<(), ShimError> {
    let effective = effective_network_config(home)?;
    if !effective.audit.is_empty() {
        // Local append (the shim carries no tfs dep); same best-effort
        // discipline as tfs::journal — the answer never depends on the
        // record.
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(home.join("journal.log"))
        {
            use std::io::Write as _;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            for line in &effective.audit {
                let _ = f.write_all(format!("{now} event=network-config {line}\n").as_bytes());
            }
        }
    }
    tebako_http::set_network_config(effective);
    Ok(())
}

// ---------------------------------------------------------------------
// registry registration (the `tebako add-registry` write side)
// ---------------------------------------------------------------------

/// The outcome of [`add_registry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddRegistryOutcome {
    Added,
    AlreadyPresent,
    /// The ref was already registered; its book keys (`name:` /
    /// `default:` / `require_signed:`) were rewritten to the requested
    /// ones (spec 37 §2 — the CLI path to (re)policy an entry).
    Updated,
}

/// The book keys of an `add-registry` call (spec 37 §2). All absent =
/// the pre-book bare-ref behavior exactly.
#[derive(Debug, Default, Clone)]
pub struct AddRegistryOptions {
    /// `--name <alias>` — the local alias; validated against the
    /// `[a-z][a-z0-9-]*` grammar and against collision with every
    /// existing entry's computed alias (`DuplicateRegistryAlias`).
    pub name: Option<String>,
    /// `--require-signed` — §2.2's fail-closed trust policy.
    pub require_signed: bool,
    /// `--default` — §2.1's publish/UX anchor; a second default in the
    /// book is `DuplicateDefaultRegistry`.
    pub default: bool,
}

/// Append `reg_ref` to `registries:` in `~/.tebako/config.yaml`,
/// preserving every other key. This is the ONE authored-config write the
/// toolchain performs (spec 04 §2: `tebako add-registry <ref>` registers
/// one; the dispatcher itself still never writes this file). The edit is
/// structural (serde_yaml Value surgery), so user comments/formatting are
/// not preserved — keys and values are. The write is tmp + rename, the
/// same discipline as the disabled-state file.
///
/// Book behavior (spec 37 §2): a bare `opts` appends the bare ref
/// string (the pre-book spelling); any set key appends the map form.
/// Re-adding an already-registered ref reports `AlreadyPresent` when
/// the stored entry's book keys already match, else rewrites that entry
/// in place and reports `Updated`. The book invariants are checked
/// BEFORE the write — the file never holds an invalid book.
pub fn add_registry(
    home: &Path,
    reg_ref: &str,
    opts: &AddRegistryOptions,
) -> Result<AddRegistryOutcome, ShimError> {
    if let Some(name) = &opts.name {
        if !valid_registry_alias(name) {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "the registry alias '{name}' is malformed — the alias grammar is [a-z][a-z0-9-]* (spec 37 §2)"
                ),
            );
        }
    }
    let mut outcome = AddRegistryOutcome::Added;
    edit_config(home, |mapping| {
        let key = serde_yaml::Value::String("registries".to_string());
        let entry = mapping
            .entry(key)
            .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
        let seq = entry.as_sequence_mut().ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{}: `registries` must be a list",
                    config_path(home).display()
                ),
            )
        })?;
        // Parse the existing entries through the ONE book model so a
        // bare string and a map form of the same ref compare equal.
        let mut entries: Vec<RegistryBookEntry> = Vec::with_capacity(seq.len());
        for v in seq.iter() {
            let e: RegistryBookEntry = serde_yaml::from_value(v.clone()).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!(
                        "{}: a `registries` entry is malformed ({e})",
                        config_path(home).display()
                    ),
                )
            })?;
            entries.push(e);
        }
        let requested = RegistryBookEntry {
            reference: reg_ref.to_string(),
            name: opts.name.clone(),
            default: opts.default,
            require_signed: opts.require_signed,
        };
        // A bare re-add never strips an existing entry's book keys —
        // only an explicit policy flag rewrites (`tebako add-registry
        // <ref>` on a `require_signed:` entry is AlreadyPresent, not a
        // silent downgrade).
        let has_opts = opts.name.is_some() || opts.default || opts.require_signed;
        if let Some(pos) = entries.iter().position(|e| e.reference == reg_ref) {
            if !has_opts || entries[pos] == requested {
                outcome = AddRegistryOutcome::AlreadyPresent;
                return Ok(());
            }
            entries[pos] = requested;
            outcome = AddRegistryOutcome::Updated;
        } else {
            entries.push(requested);
        }
        // The book invariants against the WOULD-BE book — never write
        // an invalid one (spec 37 §2's named errors, both entries named).
        let book = UserConfig {
            registries: entries.clone(),
            ..UserConfig::default()
        };
        book.registry_book()?;
        *seq = entries.into_iter().map(|e| e.to_yaml_value()).collect();
        Ok(())
    })?;
    Ok(outcome)
}

/// Merge engine → runtime preferences into `~/.tebako/config.yaml`,
/// preserving every other key (the same structural-surgery discipline as
/// [`add_registry`]). A preference for an already-present engine is
/// REPLACED — the caller is the authority (publish's built-in verify
/// re-anchors the proof home to the publisher's picks, spec 16 §5).
/// This is the second authored-config write the toolchain performs;
/// the dispatcher itself still never writes this file.
pub fn set_runtime_prefs(
    home: &Path,
    prefs: &BTreeMap<String, RuntimePref>,
) -> Result<(), ShimError> {
    let path = config_path(home);
    let mut root: serde_yaml::Value = match std::fs::read_to_string(&path) {
        Ok(t) => serde_yaml::from_str(&t).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "cannot parse {} ({e}) — fix or remove it; run `tebako-shim doctor`",
                    path.display()
                ),
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let mapping = root.as_mapping_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{} must be a YAML mapping", path.display()),
        )
    })?;
    let key = serde_yaml::Value::String("runtimes".to_string());
    let entry = mapping
        .entry(key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let rt_map = entry.as_mapping_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{}: `runtimes` must be a mapping", path.display()),
        )
    })?;
    for (engine, pref) in prefs {
        let value = serde_yaml::to_value(pref).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot serialize the runtime preference for {engine}: {e}"),
            )
        })?;
        rt_map.insert(serde_yaml::Value::String(engine.clone()), value);
    }
    let text = serde_yaml::to_string(&root).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize {}: {e}", path.display()),
        )
    })?;
    std::fs::create_dir_all(home).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", home.display()),
        )
    })?;
    let tmp = home.join(format!("config.yaml.{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })?;
    Ok(())
}

// ---------------------------------------------------------------------
// the `tebako-shim use` write path (spec 07 §0/§3 — the authored-config
// write verb beside add_registry; spec 07 §2 step 0.5 amendment)
// ---------------------------------------------------------------------

/// Load `~/.tebako/config.yaml` (missing → an empty mapping), apply
/// `edit` to its root mapping, and write it back tmp + rename — the
/// shared scaffolding of every authored-config write (structural
/// surgery: keys preserved, comments not).
fn edit_config(
    home: &Path,
    edit: impl FnOnce(&mut serde_yaml::Mapping) -> Result<(), ShimError>,
) -> Result<(), ShimError> {
    let path = config_path(home);
    let mut root: serde_yaml::Value = match std::fs::read_to_string(&path) {
        Ok(t) => serde_yaml::from_str(&t).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "cannot parse {} ({e}) — fix or remove it; run `tebako-shim doctor`",
                    path.display()
                ),
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        }
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let mapping = root.as_mapping_mut().ok_or_else(|| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("{} must be a YAML mapping", path.display()),
        )
    })?;
    edit(mapping)?;
    let text = serde_yaml::to_string(&root).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize {}: {e}", path.display()),
        )
    })?;
    std::fs::create_dir_all(home).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", home.display()),
        )
    })?;
    let tmp = home.join(format!("config.yaml.{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })?;
    Ok(())
}

/// Set or clear (`None`) the user-default pin for `tool` — the
/// `use <tool> <pin>` / `use --clear <tool>` write. The caller validates
/// the pin against `tpkg::toolpin::ToolPin` BEFORE calling. A widened
/// map entry (spec 07 §4) keeps its `slices:` across both verbs: `use`
/// rewrites only `version:`, `--clear` drops only `version:` (a
/// slices-only entry survives; a left-empty entry goes). Returns
/// whether the file changed.
pub fn set_default(home: &Path, tool: &str, pin: Option<&str>) -> Result<bool, ShimError> {
    let mut changed = false;
    edit_config(home, |mapping| {
        let key = serde_yaml::Value::String("defaults".to_string());
        let version_key = serde_yaml::Value::String("version".to_string());
        match pin {
            Some(pin) => {
                let entry = mapping
                    .entry(key)
                    .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
                let defaults = entry.as_mapping_mut().ok_or_else(|| {
                    ShimError::new(
                        EX_TEBAKO_MANIFEST,
                        format!(
                            "{}: `defaults` must be a mapping",
                            config_path(home).display()
                        ),
                    )
                })?;
                let tool_key = serde_yaml::Value::String(tool.to_string());
                match defaults.get_mut(&tool_key) {
                    Some(serde_yaml::Value::Mapping(tool_map)) => {
                        // Widened entry: rewrite `version:`, keep `slices:`.
                        let new = serde_yaml::Value::String(pin.to_string());
                        if tool_map.get(&version_key) != Some(&new) {
                            tool_map.insert(version_key, new);
                            changed = true;
                        }
                    }
                    Some(serde_yaml::Value::String(_)) => {
                        let new = serde_yaml::Value::String(pin.to_string());
                        if defaults.get(&tool_key) != Some(&new) {
                            defaults.insert(tool_key, new);
                            changed = true;
                        }
                    }
                    Some(_) => {
                        return fail(
                            EX_TEBAKO_MANIFEST,
                            format!(
                                "{}: `defaults.{tool}` must be a version string or a mapping",
                                config_path(home).display()
                            ),
                        )
                    }
                    None => {
                        defaults.insert(tool_key, serde_yaml::Value::String(pin.to_string()));
                        changed = true;
                    }
                }
            }
            None => {
                if let Some(entry) = mapping.get_mut(&key) {
                    let defaults = entry.as_mapping_mut().ok_or_else(|| {
                        ShimError::new(
                            EX_TEBAKO_MANIFEST,
                            format!(
                                "{}: `defaults` must be a mapping",
                                config_path(home).display()
                            ),
                        )
                    })?;
                    let tool_key = serde_yaml::Value::String(tool.to_string());
                    match defaults.get_mut(&tool_key) {
                        Some(serde_yaml::Value::Mapping(tool_map)) => {
                            changed = tool_map.remove(&version_key).is_some();
                            if tool_map.is_empty() {
                                defaults.remove(&tool_key);
                            }
                        }
                        Some(_) => {
                            changed = defaults.remove(&tool_key).is_some();
                        }
                        None => {}
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(changed)
}

/// The `use --runtime <engine>@<langver>[:<tebako>]` write: one engine's
/// runtime preference. Without the `:<tebako>` part only `version` is
/// written — the tebako line then follows the product default at read
/// time (see [`RuntimePref::tebako`]).
pub fn set_runtime_pref(
    home: &Path,
    engine: &str,
    version: &str,
    tebako: Option<&str>,
) -> Result<(), ShimError> {
    edit_config(home, |mapping| {
        let key = serde_yaml::Value::String("runtimes".to_string());
        let entry = mapping
            .entry(key)
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        let rt_map = entry.as_mapping_mut().ok_or_else(|| {
            ShimError::new(
                EX_TEBAKO_MANIFEST,
                format!(
                    "{}: `runtimes` must be a mapping",
                    config_path(home).display()
                ),
            )
        })?;
        let mut pref = serde_yaml::Mapping::new();
        pref.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::String(version.to_string()),
        );
        if let Some(tebako) = tebako {
            pref.insert(
                serde_yaml::Value::String("tebako".to_string()),
                serde_yaml::Value::String(tebako.to_string()),
            );
        }
        rt_map.insert(
            serde_yaml::Value::String(engine.to_string()),
            serde_yaml::Value::Mapping(pref),
        );
        Ok(())
    })
}

// ---------------------------------------------------------------------
// the registry-default chain link (spec 07 §2.1, last resort)
// ---------------------------------------------------------------------

/// The registry default version for `payload_name`. Unscoped (`None`)
/// scans the user's registered registries in order (first match wins);
/// scoped (`Some(alias)`, spec 37 §3) consults the ONE registry the
/// alias names — an unknown alias is `UnknownRegistryAlias` at
/// pin-resolution time, fail-closed. Every registry form of spec 04 §2
/// resolves through the dispatch-time cache ([`crate::regcache`]); the
/// registry model is tebako-resolve's.
pub fn registry_default(
    home: &Path,
    config: &UserConfig,
    payload_name: &str,
    scope: Option<&str>,
    confine: &[String],
    ctx: &Ctx,
) -> Result<Option<(String, String)>, ShimError> {
    for reg_ref in config.registry_refs_scoped(scope)? {
        // Spec 37 §7's origin binding: a confined walk (the payload's
        // installed versions are bound and no authored scope overrides)
        // consults the origin registry set only.
        if !confine.is_empty() && !confine.iter().any(|b| b == reg_ref) {
            continue;
        }
        let registry = crate::regcache::registry_for(home, reg_ref, ctx)?;
        if let Some(p) = registry.payload(payload_name) {
            if let Some(default) = &p.default {
                return Ok(Some((default.clone(), reg_ref.to_string())));
            }
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------
// disabled state (shim-managed; never interleaved with authored config)
// ---------------------------------------------------------------------

/// Tool name → list of disabled selectors (strings on disk; parsed
/// through `tpkg::toolpin::DisableSelector`, the ONE grammar — spec 00
/// invariant 10): `all`, a bare version, `payload@all`, or
/// `payload@version` (spec 07 §0, the 2026-09-05 routing amendment).
pub type Disabled = BTreeMap<String, Vec<String>>;

pub fn disabled_path(home: &Path) -> PathBuf {
    home.join("shims").join(".disabled.yaml")
}

pub fn load_disabled(home: &Path) -> Result<Disabled, ShimError> {
    let path = disabled_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Disabled::default()),
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", path.display())),
    };
    let disabled: Disabled = serde_yaml::from_str(&text).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_MANIFEST,
            format!("cannot parse {} ({e})", path.display()),
        )
    })?;
    // Every selector validates at LOAD (invariant 9 — an unknown string
    // is a named error naming the file and entry, never silently
    // ignored).
    for (tool, selectors) in &disabled {
        for selector in selectors {
            tpkg::toolpin::DisableSelector::parse(selector).map_err(|e| {
                ShimError::new(
                    EX_TEBAKO_MANIFEST,
                    format!("{}: {tool}: {e}", path.display()),
                )
            })?;
        }
    }
    Ok(disabled)
}

pub fn save_disabled(home: &Path, disabled: &Disabled) -> Result<(), ShimError> {
    let dir = home.join("shims");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot create {}: {e}", dir.display()),
        )
    })?;
    let path = disabled_path(home);
    let tmp = dir.join(format!(".disabled.yaml.{}.tmp", std::process::id()));
    let text = serde_yaml::to_string(disabled).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot serialize disabled state: {e}"),
        )
    })?;
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })
}

/// Is the claim `payload` makes for `tool` at `version` gated?
/// Selectors parse through `tpkg::toolpin::DisableSelector` (validated
/// at load — a stored selector always parses).
pub fn is_disabled(disabled: &Disabled, tool: &str, payload: &str, version: &str) -> bool {
    disabled.get(tool).is_some_and(|selectors| {
        selectors.iter().any(|s| {
            tpkg::toolpin::DisableSelector::parse(s).is_ok_and(|sel| sel.matches(payload, version))
        })
    })
}

/// The provider scan's skip test: is the payload's WHOLE claim for
/// `tool` gated? Only `all` and `<payload>@all` gate a whole claim —
/// a version selector leaves the claim routable at its other versions.
pub fn claim_disabled(disabled: &Disabled, tool: &str, payload: &str) -> bool {
    disabled.get(tool).is_some_and(|selectors| {
        selectors.iter().any(|s| {
            matches!(
                tpkg::toolpin::DisableSelector::parse(s),
                Ok(tpkg::toolpin::DisableSelector::All)
            ) || matches!(
                tpkg::toolpin::DisableSelector::parse(s),
                Ok(tpkg::toolpin::DisableSelector::PayloadAll(ref p)) if p == payload
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tebako-shim-config-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pref(version: &str, tebako: &str) -> RuntimePref {
        RuntimePref {
            version: version.to_string(),
            tebako: tebako.to_string(),
            source: None,
        }
    }

    #[test]
    fn set_runtime_prefs_writes_a_fresh_config() {
        let home = fresh_home("fresh");
        let mut prefs = BTreeMap::new();
        prefs.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        set_runtime_prefs(&home, &prefs).unwrap();
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.runtimes.get("java"), Some(&pref("21.0.12", "2.1.0")));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn set_runtime_prefs_preserves_other_keys_and_replaces_the_engine() {
        let home = fresh_home("merge");
        add_registry(&home, "tfs:github:acme/app", &AddRegistryOptions::default()).unwrap();
        let mut first = BTreeMap::new();
        first.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        set_runtime_prefs(&home, &first).unwrap();
        let mut second = BTreeMap::new();
        second.insert("java".to_string(), pref("21.0.13", "2.1.0"));
        second.insert("ruby".to_string(), pref("3.3.12", "0.16.18"));
        set_runtime_prefs(&home, &second).unwrap();
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.runtimes.get("java"), Some(&pref("21.0.13", "2.1.0")));
        assert_eq!(cfg.runtimes.get("ruby"), Some(&pref("3.3.12", "0.16.18")));
        assert_eq!(
            cfg.registries,
            vec![RegistryBookEntry::bare("tfs:github:acme/app".to_string())]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn set_runtime_prefs_rejects_a_non_mapping_runtimes_key() {
        let home = fresh_home("badshape");
        std::fs::write(config_path(&home), "runtimes: [nope]\n").unwrap();
        let mut prefs = BTreeMap::new();
        prefs.insert("java".to_string(), pref("21.0.12", "2.1.0"));
        let err = set_runtime_prefs(&home, &prefs).unwrap_err();
        assert!(
            err.message.contains("`runtimes` must be a mapping"),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn network_section_parses_all_keys() {
        let home = fresh_home("netparse");
        std::fs::write(
            config_path(&home),
            "network:\n  proxy: http://proxy.corp:3128\n  tls_roots: platform\n  extra_ca: [/etc/pki/corp.pem]\n",
        )
        .unwrap();
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.network.proxy.as_deref(), Some("http://proxy.corp:3128"));
        assert_eq!(cfg.network.tls_roots.as_deref(), Some("platform"));
        assert_eq!(
            cfg.network.extra_ca,
            vec![PathBuf::from("/etc/pki/corp.pem")]
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_network_config_rejects_an_unknown_tls_roots() {
        let home = fresh_home("netbadroots");
        std::fs::write(config_path(&home), "network:\n  tls_roots: corporate\n").unwrap();
        let err = install_network_config(&home).unwrap_err();
        assert!(err.message.contains("corporate"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_network_config_rejects_extra_ca_with_platform_roots() {
        let home = fresh_home("netconflict");
        std::fs::write(
            config_path(&home),
            "network:\n  tls_roots: platform\n  extra_ca: [/etc/pki/corp.pem]\n",
        )
        .unwrap();
        let err = install_network_config(&home).unwrap_err();
        assert!(
            err.message.contains("cannot combine with extra_ca"),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_network_config_rejects_a_malformed_proxy() {
        let home = fresh_home("netbadproxy");
        std::fs::write(config_path(&home), "network:\n  proxy: \"not a url\"\n").unwrap();
        let err = install_network_config(&home).unwrap_err();
        assert!(err.message.contains("invalid proxy URL"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn install_network_config_audits_a_redacted_proxy() {
        let home = fresh_home("netaudit");
        std::fs::write(
            config_path(&home),
            "network:\n  proxy: http://user:secret@proxy.corp:3128\n",
        )
        .unwrap();
        install_network_config(&home).unwrap();
        let journal = std::fs::read_to_string(home.join("journal.log")).unwrap();
        let line = journal
            .lines()
            .find(|l| l.contains("event=network-config") && l.contains("proxy="))
            .unwrap_or_else(|| panic!("no proxy audit line in {journal}"));
        assert!(line.contains("proxy=http://***@proxy.corp:3128"), "{line}");
        assert!(!line.contains("secret"), "{line}");
        // The process-global config carries the real spelling.
        let g = tebako_http::network_config();
        assert_eq!(
            g.proxy_url.as_deref(),
            Some("http://user:secret@proxy.corp:3128")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    // -------------------------------------------------------------
    // spec 37 §2 — the registry book
    // -------------------------------------------------------------

    #[test]
    fn book_bare_ref_derives_the_repo_alias() {
        let cfg: UserConfig =
            serde_yaml::from_str("registries:\n  - tfs:github:tebako-packages/registry\n").unwrap();
        let book = cfg.registry_book().unwrap();
        assert_eq!(book.len(), 1);
        assert_eq!(book[0].alias.as_deref(), Some("registry"));
        assert_eq!(
            book[0].entry.reference(),
            "tfs:github:tebako-packages/registry"
        );
        assert!(book[0].entry.is_bare());
    }

    #[test]
    fn book_map_form_parses_every_key_and_the_name_wins() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - ref: tfs:github:metanorma/metanorma-flavor-nist\n    name: nist\n    default: true\n    require_signed: true\n",
        )
        .unwrap();
        let book = cfg.registry_book().unwrap();
        assert_eq!(book[0].alias.as_deref(), Some("nist"));
        assert!(book[0].entry.default);
        assert!(book[0].entry.require_signed);
        assert!(!book[0].entry.is_bare());
    }

    #[test]
    fn book_map_form_ignores_unknown_keys() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - ref: tfs:github:acme/app\n    future_key: whatever\n",
        )
        .unwrap();
        let book = cfg.registry_book().unwrap();
        assert_eq!(book[0].entry.reference(), "tfs:github:acme/app");
    }

    #[test]
    fn book_map_form_without_ref_is_a_config_error() {
        let err =
            serde_yaml::from_str::<UserConfig>("registries:\n  - name: orphan\n").unwrap_err();
        assert!(err.to_string().contains("`ref:`"), "{err}");
    }

    #[test]
    fn book_file_ref_has_no_derived_alias() {
        let cfg: UserConfig =
            serde_yaml::from_str("registries:\n  - file:///opt/tpkg-registry.yaml\n").unwrap();
        let book = cfg.registry_book().unwrap();
        assert_eq!(book[0].alias, None);
    }

    #[test]
    fn book_rejects_a_malformed_authored_alias() {
        let home = fresh_home("badalias");
        std::fs::write(
            config_path(&home),
            "registries:\n  - ref: tfs:github:acme/app\n    name: 9bad\n",
        )
        .unwrap();
        let err = load_config(&home).unwrap_err();
        assert!(err.message.contains("[a-z][a-z0-9-]*"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn book_duplicate_alias_names_both_entries_at_load() {
        let home = fresh_home("dupalias");
        // The explicit `nist` collides with acme/nist's DERIVED alias.
        std::fs::write(
            config_path(&home),
            "registries:\n  - ref: tfs:github:metanorma/flavor\n    name: nist\n  - tfs:github:acme/nist\n",
        )
        .unwrap();
        let err = load_config(&home).unwrap_err();
        assert!(err.message.contains("DuplicateRegistryAlias"), "{err:?}");
        assert!(
            err.message.contains("tfs:github:metanorma/flavor"),
            "{err:?}"
        );
        assert!(err.message.contains("tfs:github:acme/nist"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn book_two_defaults_are_a_named_error_at_load() {
        let home = fresh_home("dupdefault");
        std::fs::write(
            config_path(&home),
            "registries:\n  - ref: tfs:github:acme/one\n    default: true\n  - ref: tfs:github:acme/two\n    default: true\n",
        )
        .unwrap();
        let err = load_config(&home).unwrap_err();
        assert!(err.message.contains("DuplicateDefaultRegistry"), "{err:?}");
        assert!(err.message.contains("tfs:github:acme/one"), "{err:?}");
        assert!(err.message.contains("tfs:github:acme/two"), "{err:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_bare_keeps_the_bare_spelling() {
        let home = fresh_home("addbare");
        add_registry(&home, "tfs:github:acme/app", &AddRegistryOptions::default()).unwrap();
        let text = std::fs::read_to_string(config_path(&home)).unwrap();
        assert!(text.contains("- tfs:github:acme/app"), "{text}");
        assert!(!text.contains("ref:"), "{text}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_with_flags_writes_the_map_form() {
        let home = fresh_home("addflags");
        let opts = AddRegistryOptions {
            name: Some("nist".to_string()),
            require_signed: true,
            default: true,
        };
        add_registry(&home, "tfs:github:acme/flavor-nist", &opts).unwrap();
        let cfg = load_config(&home).unwrap();
        let book = cfg.registry_book().unwrap();
        assert_eq!(book[0].alias.as_deref(), Some("nist"));
        assert!(book[0].entry.default);
        assert!(book[0].entry.require_signed);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_bare_readd_never_strips_the_book_keys() {
        let home = fresh_home("nostrip");
        let opts = AddRegistryOptions {
            name: None,
            require_signed: true,
            default: false,
        };
        add_registry(&home, "tfs:github:acme/app", &opts).unwrap();
        let outcome =
            add_registry(&home, "tfs:github:acme/app", &AddRegistryOptions::default()).unwrap();
        assert_eq!(outcome, AddRegistryOutcome::AlreadyPresent);
        let cfg = load_config(&home).unwrap();
        assert!(cfg.registries[0].require_signed);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_with_new_flags_updates_in_place() {
        let home = fresh_home("update");
        add_registry(&home, "tfs:github:acme/app", &AddRegistryOptions::default()).unwrap();
        let opts = AddRegistryOptions {
            name: Some("acme".to_string()),
            require_signed: false,
            default: false,
        };
        let outcome = add_registry(&home, "tfs:github:acme/app", &opts).unwrap();
        assert_eq!(outcome, AddRegistryOutcome::Updated);
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.registries.len(), 1);
        assert_eq!(cfg.registries[0].name.as_deref(), Some("acme"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_refuses_a_second_default_before_writing() {
        let home = fresh_home("seconddefault");
        let opts = AddRegistryOptions {
            name: None,
            require_signed: false,
            default: true,
        };
        add_registry(&home, "tfs:github:acme/one", &opts).unwrap();
        let err = add_registry(&home, "tfs:github:acme/two", &opts).unwrap_err();
        assert!(err.message.contains("DuplicateDefaultRegistry"), "{err:?}");
        // The refused write never landed: the file still holds one entry.
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.registries.len(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_refuses_an_alias_collision_before_writing() {
        let home = fresh_home("aliascollision");
        let opts = AddRegistryOptions {
            name: Some("nist".to_string()),
            require_signed: false,
            default: false,
        };
        add_registry(&home, "tfs:github:metanorma/flavor", &opts).unwrap();
        // acme/nist's DERIVED alias collides with the explicit one.
        let err = add_registry(
            &home,
            "tfs:github:acme/nist",
            &AddRegistryOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("DuplicateRegistryAlias"), "{err:?}");
        let cfg = load_config(&home).unwrap();
        assert_eq!(cfg.registries.len(), 1);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn add_registry_validates_the_alias_grammar() {
        let home = fresh_home("badname");
        let opts = AddRegistryOptions {
            name: Some("BAD".to_string()),
            require_signed: false,
            default: false,
        };
        let err = add_registry(&home, "tfs:github:acme/app", &opts).unwrap_err();
        assert!(err.message.contains("[a-z][a-z0-9-]*"), "{err:?}");
        assert!(!config_path(&home).exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn scoped_refs_unscoped_returns_every_entry() {
        let cfg: UserConfig =
            serde_yaml::from_str("registries:\n  - tfs:github:acme/one\n  - tfs:github:acme/two\n")
                .unwrap();
        let refs = cfg.registry_refs_scoped(None).unwrap();
        assert_eq!(refs, vec!["tfs:github:acme/one", "tfs:github:acme/two"]);
    }

    #[test]
    fn scoped_refs_by_derived_and_authored_alias() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - tfs:github:acme/derived\n  - ref: file:///opt/reg.yaml\n    name: local\n",
        )
        .unwrap();
        assert_eq!(
            cfg.registry_refs_scoped(Some("derived")).unwrap(),
            vec!["tfs:github:acme/derived"]
        );
        assert_eq!(
            cfg.registry_refs_scoped(Some("local")).unwrap(),
            vec!["file:///opt/reg.yaml"]
        );
    }

    #[test]
    fn scoped_refs_unknown_alias_lists_the_book() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - tfs:github:acme/derived\n  - ref: file:///opt/reg.yaml\n    name: local\n",
        )
        .unwrap();
        let err = cfg.registry_refs_scoped(Some("nosuch")).unwrap_err();
        assert!(err.message.contains("UnknownRegistryAlias"), "{err:?}");
        assert!(err.message.contains("derived"), "{err:?}");
        assert!(err.message.contains("local"), "{err:?}");
    }

    #[test]
    fn scoped_book_rows_carry_the_policy_flags() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - tfs:github:acme/plain\n  - ref: tfs:github:acme/signed\n    name: priv\n    require_signed: true\n",
        )
        .unwrap();
        let rows = cfg.registry_book_scoped(None).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(!rows[0].entry.require_signed);
        assert_eq!(rows[0].alias.as_deref(), Some("plain"));
        assert!(rows[1].entry.require_signed);
        assert_eq!(rows[1].alias.as_deref(), Some("priv"));

        let rows = cfg.registry_book_scoped(Some("priv")).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].entry.require_signed);
        assert_eq!(rows[0].entry.reference(), "tfs:github:acme/signed");
    }

    // spec 37 §5 — the credential book

    #[test]
    fn credentials_parse_the_map_form() {
        let cfg: UserConfig = serde_yaml::from_str(
            "credentials:\n  - registry: nist\n    token_env: NIST_GH_TOKEN\n  - host: ghe.corp.internal\n    token_env: GHE_TOKEN\n",
        )
        .unwrap();
        assert_eq!(cfg.credentials.len(), 2);
        assert_eq!(cfg.credentials[0].registry.as_deref(), Some("nist"));
        assert_eq!(cfg.credentials[0].host, None);
        assert_eq!(cfg.credentials[0].token_env, "NIST_GH_TOKEN");
        assert_eq!(cfg.credentials[1].registry, None);
        assert_eq!(
            cfg.credentials[1].host.as_deref(),
            Some("ghe.corp.internal")
        );
        assert_eq!(cfg.credentials[1].token_env, "GHE_TOKEN");
    }

    #[test]
    fn credentials_reject_an_unknown_key_at_parse() {
        let err = serde_yaml::from_str::<UserConfig>(
            "credentials:\n  - registry: nist\n    token_env: NIST_GH_TOKEN\n    token: hunter2\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("token"), "{err}");
    }

    #[test]
    fn credential_book_rejects_both_selectors() {
        let cfg: UserConfig = serde_yaml::from_str(
            "credentials:\n  - registry: nist\n    host: api.github.com\n    token_env: NIST_GH_TOKEN\n",
        )
        .unwrap();
        let err = cfg.credential_book().unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(err.message.contains("(InvalidCredentialEntry)"), "{err:?}");
    }

    #[test]
    fn credential_book_rejects_neither_selector() {
        let cfg: UserConfig =
            serde_yaml::from_str("credentials:\n  - token_env: NIST_GH_TOKEN\n").unwrap();
        let err = cfg.credential_book().unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(err.message.contains("(InvalidCredentialEntry)"), "{err:?}");
    }

    #[test]
    fn credential_book_rejects_a_malformed_token_env() {
        for bad in ["9LIVES", "MY-TOKEN", "MY TOKEN", ""] {
            let cfg: UserConfig = serde_yaml::from_str(&format!(
                "credentials:\n  - registry: nist\n    token_env: \"{bad}\"\n"
            ))
            .unwrap();
            let err = cfg.credential_book().unwrap_err();
            assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{bad}: {err:?}");
            assert!(
                err.message.contains("(InvalidCredentialEntry)"),
                "{bad}: {err:?}"
            );
        }
    }

    #[test]
    fn credential_book_rejects_a_duplicate_alias_naming_both_envs() {
        let cfg: UserConfig = serde_yaml::from_str(
            "credentials:\n  - registry: nist\n    token_env: NIST_GH_TOKEN\n  - registry: nist\n    token_env: OTHER_TOKEN\n",
        )
        .unwrap();
        let err = cfg.credential_book().unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(
            err.message.contains("(DuplicateCredentialSelector)"),
            "{err:?}"
        );
        assert!(err.message.contains("NIST_GH_TOKEN"), "{err:?}");
        assert!(err.message.contains("OTHER_TOKEN"), "{err:?}");
        assert!(err.message.contains("nist"), "{err:?}");
    }

    #[test]
    fn credential_book_rejects_a_duplicate_host_naming_both_envs() {
        let cfg: UserConfig = serde_yaml::from_str(
            "credentials:\n  - host: ghe.corp.internal\n    token_env: GHE_TOKEN\n  - host: ghe.corp.internal\n    token_env: GHE_TOKEN_2\n",
        )
        .unwrap();
        let err = cfg.credential_book().unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(
            err.message.contains("(DuplicateCredentialSelector)"),
            "{err:?}"
        );
        assert!(err.message.contains("GHE_TOKEN"), "{err:?}");
        assert!(err.message.contains("GHE_TOKEN_2"), "{err:?}");
        assert!(err.message.contains("ghe.corp.internal"), "{err:?}");
    }

    #[test]
    fn credential_book_derives_tier1_confinement_from_the_registry_book() {
        let cfg: UserConfig = serde_yaml::from_str(
            "registries:\n  - ref: tfs:github:acme/priv\n    name: nist\n  - ref: file:///opt/reg.yaml\n    name: local\ncredentials:\n  - registry: nist\n    token_env: NIST_GH_TOKEN\n  - registry: local\n    token_env: LOCAL_TOKEN\n  - registry: notyet\n    token_env: LATER_TOKEN\n  - host: ghe.corp.internal\n    token_env: GHE_TOKEN\n",
        )
        .unwrap();
        let book = cfg.credential_book().unwrap();
        let tier1 = |alias: &str| {
            book.tier1
                .iter()
                .find(|e| e.alias == alias)
                .unwrap_or_else(|| panic!("no tier-1 entry for {alias}"))
        };
        // The github ref confines to the service hosts (the adapters'
        // SSOT), the file ref confines to nothing, and an alias the
        // book does not carry yet is accepted with an empty set.
        assert_eq!(
            tier1("nist").allowed_hosts,
            BTreeSet::from(["api.github.com".to_string(), "github.com".to_string()])
        );
        assert!(tier1("local").allowed_hosts.is_empty());
        assert!(tier1("notyet").allowed_hosts.is_empty());
        assert_eq!(
            book.tier2,
            vec![("ghe.corp.internal".to_string(), "GHE_TOKEN".to_string())]
        );
        assert_eq!(
            book.alias_of("tfs:github:acme/priv"),
            Some("nist".to_string())
        );
        assert_eq!(
            book.alias_of("file:///opt/reg.yaml"),
            Some("local".to_string())
        );
    }

    #[test]
    fn load_config_validates_the_credential_book_fail_closed() {
        let home = fresh_home("credloadbad");
        std::fs::write(
            config_path(&home),
            "credentials:\n  - registry: nist\n    token_env: NIST_GH_TOKEN\n  - registry: nist\n    token_env: OTHER_TOKEN\n",
        )
        .unwrap();
        let err = load_config(&home).unwrap_err();
        assert_eq!(err.code, EX_TEBAKO_MANIFEST, "{err:?}");
        assert!(
            err.message.contains("(DuplicateCredentialSelector)"),
            "{err:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

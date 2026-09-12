//! `tebako doctor` — the spec 35 diagnostics verb: five read-only
//! sections (store / dispatch / network / trust / registries), every
//! check a named verdict, exit 0 healthy / 1 problems / 64 usage.
//! Compose-never-duplicate: the dispatch section IS tebako-shim's
//! structured report; the CLI owns the store/network/trust planes.

use std::path::{Path, PathBuf};

use crate::error::TebakoError;

/// One finding line. `Note` renders as `note:` and never moves the exit
/// code; `Problem` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Note,
    Problem,
}

#[derive(Debug)]
pub struct Finding {
    pub severity: Severity,
    pub text: String,
}

#[derive(Debug)]
pub struct Section {
    pub name: &'static str,
    pub findings: Vec<Finding>,
}

impl Section {
    fn named(name: &'static str) -> Self {
        Section {
            name,
            findings: Vec::new(),
        }
    }
    fn ok(&mut self, text: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Ok,
            text: text.into(),
        });
    }
    fn note(&mut self, text: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Note,
            text: text.into(),
        });
    }
    fn problem(&mut self, text: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Problem,
            text: text.into(),
        });
    }
}

fn sha256_hex(path: &Path) -> std::io::Result<String> {
    use sha2::Digest as _;
    let bytes = std::fs::read(path)?;
    let digest = sha2::Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// The exe marker is a bare hex line; the image/payload sidecar is the
/// `<hex>  <name>` form. Both parse to the hex digest.
fn sidecar_hex(text: &str) -> Option<&str> {
    let first = text.lines().next()?.trim();
    let hex = first.split_whitespace().next()?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

/// Verify one artifact against its sidecar; the finding names the
/// artifact on mismatch, the sidecar on its own absence.
fn check_artifact(section: &mut Section, artifact: &Path, sidecar: &Path, label: &str) {
    let expected = match std::fs::read_to_string(sidecar) {
        Ok(text) => match sidecar_hex(&text) {
            Some(h) => h.to_string(),
            None => {
                section.problem(format!(
                    "{label}: {} is not a sha256 sidecar — reinstall the artifact",
                    sidecar.display()
                ));
                return;
            }
        },
        Err(_) => {
            section.problem(format!(
                "{label}: missing trust anchor {}",
                sidecar.display()
            ));
            return;
        }
    };
    match sha256_hex(artifact) {
        Ok(actual) if actual == expected => {}
        Ok(_) => section.problem(format!(
            "{label}: sha256 MISMATCH — {} does not match its anchor; remove the entry and reinstall",
            artifact.display()
        )),
        Err(e) => section.problem(format!("{label}: {} unreadable: {e}", artifact.display())),
    }
}

/// Free bytes on the filesystem holding `path` (libc statfs on unix,
/// GetDiskFreeSpaceExW on Windows). None where the probe itself fails.
#[cfg(unix)]
fn disk_free(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt as _;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statfs(c.as_ptr(), &mut stat) } == 0)
        .then(|| stat.f_bavail * stat.f_bsize as u64)
}

#[cfg(windows)]
fn disk_free(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free: u64 = 0;
    (unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0)
        .then_some(free)
}

/// The 120 s install flock (spec 05 §4): a lock older than that with no
/// holder is stale; the finding carries the removal hint.
fn check_locks(section: &mut Section, home: &Path) {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let locks_dir = home.join("locks");
    if let Ok(rd) = std::fs::read_dir(&locks_dir) {
        candidates.extend(rd.flatten().map(|e| e.path()).filter(|p| p.is_file()));
    }
    for sub in ["runtimes", "payloads"] {
        if let Ok(rd) = std::fs::read_dir(home.join(sub)) {
            for entry in rd.flatten() {
                let lock = entry.path().join(".install.lock");
                if lock.is_file() {
                    candidates.push(lock);
                }
            }
        }
    }
    let now = std::time::SystemTime::now();
    for lock in candidates {
        let age = lock
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if age > 120 {
            section.problem(format!(
                "stale lock {} ({}s old — the install flock is 120 s): remove it if no tebako process holds it",
                lock.display(),
                age
            ));
        }
    }
}

fn store_section(home: &Path) -> Section {
    let mut s = Section::named("store");
    s.ok(format!("store root: {}", home.display()));

    // Layout stamp (spec 18 C13): read-only — doctor never stamps.
    let stamp = home.join(tebako_resolve::store::LAYOUT_VERSION_FILE);
    match std::fs::read_to_string(&stamp) {
        Ok(text) => match text.trim().parse::<u32>() {
            Ok(v) if v == tebako_resolve::store::STORE_LAYOUT_VERSION => {
                s.ok(format!("store layout: version {v} (current)"))
            }
            Ok(v) if v > tebako_resolve::store::STORE_LAYOUT_VERSION => s.problem(format!(
                "store layout: version {v} is NEWER than this CLI's {} — use a newer tebako",
                tebako_resolve::store::STORE_LAYOUT_VERSION
            )),
            Ok(v) => s.note(format!(
                "store layout: version {v} — the next store-touching verb migrates it"
            )),
            Err(_) => s.problem(format!(
                "store layout: {} is not a version stamp — remove it and let the next verb restamp",
                stamp.display()
            )),
        },
        Err(_) => s.note("store layout: unstamped (predates layout versioning — the next store-touching verb stamps it)"),
    }

    // Writability: create+remove a probe under an EXISTING directory
    // (nothing is created that was not there).
    let probe_dir = if home.join("tmp").is_dir() {
        home.join("tmp")
    } else {
        home.to_path_buf()
    };
    if home.is_dir() {
        let probe = probe_dir.join(format!(".doctor-probe-{}", std::process::id()));
        match std::fs::File::create(&probe) {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                s.ok("store writable");
            }
            Err(e) => s.problem(format!("store not writable: {e}")),
        }
    } else {
        s.note("store does not exist yet — the first install creates it");
    }

    match disk_free(home) {
        Some(free) if free < (1 << 30) => s.problem(format!(
            "disk headroom: {} MiB free — runtimes are tens of MiB each",
            free >> 20
        )),
        Some(free) if free < (5 << 30) => s.note(format!("disk headroom: {} GiB free", free >> 30)),
        Some(free) => s.ok(format!("disk headroom: {} GiB free", free >> 30)),
        None => s.note("disk headroom: probe unavailable on this platform"),
    }

    check_locks(&mut s, home);

    // Artifact digests: existence of the anchor is the shim's check;
    // agreement is this one (spec 35 §2).
    let mut verified = 0u64;
    let runtimes = home.join("runtimes");
    if let Ok(rd) = std::fs::read_dir(&runtimes) {
        for entry in rd.flatten().filter(|e| e.path().is_dir()) {
            let dir = entry.path();
            let mut exes = Vec::new();
            let mut images = Vec::new();
            if let Ok(files) = std::fs::read_dir(&dir) {
                for f in files.flatten() {
                    let name = f.file_name().to_string_lossy().into_owned();
                    // The exe is `tebako-runtime-<ver>-<lang>-<triplet>[.exe]`
                    // (the version carries dots — no naive `contains('.')`);
                    // images and sidecars ride known suffixes.
                    let is_image_or_marker = name.ends_with(".tfs")
                        || name.ends_with(".dwarfs")
                        || name.ends_with(".sha256")
                        || name.ends_with(".origin");
                    if name.starts_with("tebako-runtime-") && !is_image_or_marker {
                        exes.push(f.path());
                    } else if name.ends_with(".tfs") || name.ends_with(".dwarfs") {
                        images.push(f.path());
                    }
                }
            }
            for exe in exes {
                check_artifact(&mut s, &exe, &dir.join("sha256"), "runtime exe");
                verified += 1;
            }
            for image in images {
                let sidecar = image.with_file_name(format!(
                    "{}.sha256",
                    image.file_name().unwrap().to_string_lossy()
                ));
                check_artifact(&mut s, &image, &sidecar, "runtime image");
                verified += 1;
            }
        }
    }
    let payloads = home.join("payloads");
    if let Ok(rd) = std::fs::read_dir(&payloads) {
        for entry in rd.flatten().filter(|e| e.path().is_dir()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Ok(files) = std::fs::read_dir(entry.path()) {
                for f in files.flatten() {
                    let fname = f.file_name().to_string_lossy().into_owned();
                    if fname.ends_with(".tfs") {
                        let sidecar = f.path().with_file_name(format!("{fname}.sha256"));
                        check_artifact(&mut s, &f.path(), &sidecar, &format!("payload {name}"));
                        verified += 1;
                    }
                }
            }
        }
    }
    if verified > 0 {
        s.ok(format!(
            "{verified} artifact(s) verified against their trust anchors"
        ));
    }
    s
}

fn dispatch_section(ctx: &tebako_shim::Ctx) -> Section {
    let mut s = Section::named("dispatch");
    let report = tebako_shim::manage::doctor_report(ctx);
    for note in &report.notes {
        s.ok(note.clone());
    }
    for problem in &report.problems {
        s.problem(problem.clone());
    }
    if report.notes.is_empty() && report.problems.is_empty() {
        s.ok("no shims or payloads installed");
    }
    s
}

/// The hosts the doctor probes (spec 35 §3): the release-resolution API
/// host, the asset host, and every configured remote registry's host.
fn probe_hosts(registries: &[String]) -> Vec<String> {
    let mut hosts = vec!["api.github.com".to_string(), "github.com".to_string()];
    for reg in registries {
        if let Some(rest) = reg.strip_prefix("tfs:github:") {
            let _ = rest; // the contents API — api.github.com, already listed
        } else if let Some(rest) = reg
            .strip_prefix("tfs+https://")
            .or_else(|| reg.strip_prefix("https://"))
        {
            let host = rest.split('/').next().unwrap_or_default();
            if !host.is_empty() && !hosts.iter().any(|h| h == host) {
                hosts.push(host.to_string());
            }
        }
        // tfs+git:// and file:// carry no TLS surface here.
    }
    hosts
}

fn network_section(home: &Path, offline: bool) -> Section {
    let mut s = Section::named("network");
    let effective = match tebako_shim::config::effective_network_config(home) {
        Ok(c) => c,
        Err(e) => {
            s.problem(format!("network config: {}", e.message));
            return s;
        }
    };
    if effective.audit.is_empty() {
        s.ok("netconfig: defaults (bundled webpki roots, no explicit proxy)");
    } else {
        for line in &effective.audit {
            s.ok(format!("netconfig: {line}"));
        }
    }
    if offline {
        s.note("offline — probes skipped (TEBAKO_OFFLINE=1)");
        return s;
    }
    let registries = tebako_shim::config::load_config(home)
        .map(|c| c.registries)
        .unwrap_or_default();
    for host in probe_hosts(&registries) {
        match tebako_http::probe_tls(&host, &effective, std::time::Duration::from_secs(10)) {
            tebako_http::TlsProbe::EffectiveOk => s.ok(format!("tls {host}: chain verifies")),
            tebako_http::TlsProbe::PlatformOnly => s.problem(format!(
                "tls {host}: the served chain is rejected by the effective roots but accepted by the platform store — a TLS-intercepting proxy is in the path\n  \
                 remediation: set `network: tls_roots: platform` in {} (or TEBAKO_EXTRA_CA=/path/to/corp-ca.pem) — spec 04's enterprise-networking amendment",
                tebako_shim::config::config_path(home).display()
            )),
            tebako_http::TlsProbe::NeitherTrusted => s.problem(format!(
                "tls {host}: the served chain verifies under neither the effective nor the platform roots — do not bypass; investigate before trusting"
            )),
            tebako_http::TlsProbe::Unreachable(why) => {
                s.note(format!("tls {host}: unreachable ({why})"))
            }
        }
    }
    s
}

fn trust_section(home: &Path, env: &std::collections::BTreeMap<String, String>) -> Section {
    let mut s = Section::named("trust");
    if env.contains_key("TEBAKO_REQUIRE_SIGNED") {
        s.ok("TEBAKO_REQUIRE_SIGNED=1 — unsigned artifacts fail closed");
    } else {
        s.note("TEBAKO_REQUIRE_SIGNED unset — unsigned artifacts install with a loud warning");
    }
    s.ok(format!(
        "embedded trust root: {}",
        tebako_signer::short_fingerprint(tebako_signer::ROOT_FINGERPRINT)
    ));
    if env.contains_key("TEBAKO_TRUSTED_ROOT") {
        s.note("TEBAKO_TRUSTED_ROOT override active (the dev/test seam — never in production)");
    }
    let keyring = home.join("keys");
    if keyring.is_dir() {
        s.ok(format!("keyring: {}", keyring.display()));
    } else {
        s.note("no local keyring (~/.tebako/keys) — the embedded root alone anchors trust");
    }
    // Unsigned payloads in the store, listed loudly (spec 35 §2: "is
    // everything here signed?" is one command). The manifest mirror's
    // identity.signing.state is the record; unsigned is first-class, so
    // these are notes — TEBAKO_REQUIRE_SIGNED=1 is the install-time
    // refusal, doctor only reports.
    let mut signed = 0u64;
    let mut unsigned: Vec<String> = Vec::new();
    let payloads = home.join("payloads");
    if let Ok(rd) = std::fs::read_dir(&payloads) {
        for entry in rd.flatten().filter(|e| e.path().is_dir()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Ok(files) = std::fs::read_dir(entry.path()) {
                for f in files.flatten() {
                    let fname = f.file_name().to_string_lossy().into_owned();
                    let Some(version) = fname.strip_suffix(".manifest.yaml") else {
                        continue;
                    };
                    match tebako_shim::manifest::Manifest::load(&f.path()) {
                        Ok(m) => match m.payload_manifest().identity.signing.state {
                            tpkg::SigningState::Signed => signed += 1,
                            tpkg::SigningState::Unsigned => {
                                unsigned.push(format!("{name} {version}"))
                            }
                        },
                        Err(_) => {} // a corrupt mirror is the dispatch section's finding
                    }
                }
            }
        }
    }
    if signed > 0 {
        s.ok(format!("{signed} signed payload(s) in the store"));
    }
    if !unsigned.is_empty() {
        s.note(format!(
            "unsigned payload(s) in the store: {} (unsigned is first-class; TEBAKO_REQUIRE_SIGNED=1 refuses them at install)",
            unsigned.join(", ")
        ));
    }
    // The audit journal records every accepted-unsigned install (spec 09
    // §3's loud-warning discipline): surface its count + last entry.
    let journal = home.join("journal.log");
    if let Ok(text) = std::fs::read_to_string(&journal) {
        let unsigned: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("event=legacy-unsigned-accepted"))
            .collect();
        if !unsigned.is_empty() {
            s.note(format!(
                "{} unsigned install(s) on record — last: {}",
                unsigned.len(),
                unsigned.last().unwrap()
            ));
        }
    }
    s
}

/// Registries: the shim's report covers cache freshness and file
/// existence; this section adds reachability per remote ref (the probe
/// result from the network section is the transport verdict — here the
/// mapping ref → probed host).
fn registries_section(home: &Path, offline: bool) -> Section {
    let mut s = Section::named("registries");
    match tebako_shim::config::load_config(home) {
        Err(e) => s.problem(format!("config.yaml: {}", e.message)),
        Ok(cfg) if cfg.registries.is_empty() => {
            s.note("no registries configured — tebako add-registry <ref> registers one")
        }
        Ok(cfg) => {
            for reg in &cfg.registries {
                if reg.starts_with("file://") || Path::new(reg).is_absolute() {
                    s.ok(format!("{reg}: local"));
                } else if offline {
                    s.note(format!("{reg}: remote (reachability skipped — offline)"));
                } else {
                    s.ok(format!(
                        "{reg}: remote (transport probed in the network section)"
                    ));
                }
            }
        }
    }
    s
}

/// The full report, in spec order. `offline` mirrors TEBAKO_OFFLINE;
/// `env` is the environment the checks inspect (main.rs passes the
/// process's; tests pass a controlled one — PATH drives the shim's
/// on-PATH check, TEBAKO_REQUIRE_SIGNED the trust section).
pub fn report(
    home: &Path,
    offline: bool,
    env: &std::collections::BTreeMap<String, String>,
) -> Vec<Section> {
    let ctx = tebako_shim::Ctx {
        home: home.to_path_buf(),
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        env: env.clone(),
    };
    vec![
        store_section(home),
        dispatch_section(&ctx),
        network_section(home, offline),
        trust_section(home, env),
        registries_section(home, offline),
    ]
}

fn problem_count(sections: &[Section]) -> usize {
    sections
        .iter()
        .flat_map(|s| &s.findings)
        .filter(|f| f.severity == Severity::Problem)
        .count()
}

/// Render the report; returns (text, exit code) — spec 35 §4. The caller
/// prints and exits (the info/inspect convention; tests assert the text).
pub fn run(
    home: &Path,
    json: bool,
    offline: bool,
    env: &std::collections::BTreeMap<String, String>,
) -> Result<(String, i32), TebakoError> {
    let sections = report(home, offline, env);
    let problems = problem_count(&sections);
    let mut out = String::new();
    if json {
        use tebako_json::Value as J;
        let doc = J::Object(vec![
            ("doctor_schema".to_string(), J::Number("1".to_string())),
            (
                "sections".to_string(),
                J::Array(
                    sections
                        .iter()
                        .map(|s| {
                            J::Object(vec![
                                ("name".to_string(), J::String(s.name.to_string())),
                                (
                                    "findings".to_string(),
                                    J::Array(
                                        s.findings
                                            .iter()
                                            .map(|f| {
                                                J::Object(vec![
                                                    (
                                                        "severity".to_string(),
                                                        J::String(
                                                            match f.severity {
                                                                Severity::Ok => "ok",
                                                                Severity::Note => "note",
                                                                Severity::Problem => "problem",
                                                            }
                                                            .to_string(),
                                                        ),
                                                    ),
                                                    ("text".to_string(), J::String(f.text.clone())),
                                                ])
                                            })
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("problems".to_string(), J::Number(problems.to_string())),
        ]);
        out.push_str(&tebako_json::to_string(&doc));
        out.push('\n');
    } else {
        use std::fmt::Write as _;
        for s in &sections {
            let _ = writeln!(out, "{}:", s.name);
            for f in &s.findings {
                let tag = match f.severity {
                    Severity::Ok => "ok",
                    Severity::Note => "note",
                    Severity::Problem => "problem",
                };
                let _ = writeln!(out, "  {tag}: {}", f.text);
            }
        }
        if problems == 0 {
            let _ = writeln!(out, "tebako doctor: no problems found");
        } else {
            let _ = writeln!(out, "tebako doctor: {problems} problem(s)");
        }
    }
    Ok((out, if problems == 0 { 0 } else { 1 }))
}

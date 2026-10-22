//! The conformance half of `tebako registry doctor` (arc 03): the document
//! gate proves a registry FILE is well formed; this proves a registry HOST
//! answers the vocabulary the file declares — the transport class the ref
//! names is the parsed one, every row resolves for the host platform, and
//! (with `--full`) the served bytes match the row's pin and the declared
//! signatures verify.
//!
//! The row resolution is `install::resolve_row_reference` — the same
//! function `tebako install` runs — so the doctor can never bless a row an
//! install would refuse, or refuse one an install would take.

use tebako_resolve::registry::{PlatformSelection, Registry, RegistryPayload, RegistryVersion};
use tebako_resolve::{release_download_locator, Fetcher, Reference};

use crate::error::TebakoError;

use super::{err, read_registry_bytes, EX_TEBAKO_MANIFEST};

/// How loudly one finding speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A row that conformance-checked.
    Ok,
    /// Something the registry does not claim to serve (a platform it does
    /// not publish, a sidecar the doctor did not fetch) — not a defect.
    Note,
    /// A claim the host does not honour.
    Problem,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::Ok => "ok",
            Severity::Note => "note",
            Severity::Problem => "problem",
        }
    }
}

/// One line of the conformance report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub payload: Option<String>,
    pub version: Option<String>,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    fn new(
        severity: Severity,
        payload: Option<&str>,
        version: Option<&str>,
        message: String,
    ) -> Self {
        Finding {
            payload: payload.map(str::to_string),
            version: version.map(str::to_string),
            severity,
            message,
        }
    }
}

/// The conformance report: the document verdict, the transport that served
/// it, the rows checked, and every finding in encounter order.
pub struct Report {
    pub registry: String,
    pub transport: String,
    pub host: String,
    pub full: bool,
    pub payloads: usize,
    pub versions: usize,
    pub rows_checked: usize,
    pub artifacts_fetched: usize,
    pub document_ok: bool,
    pub findings: Vec<Finding>,
}

impl Report {
    fn problems(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Problem)
            .count()
    }

    /// The human report. One header block, one line per row verdict, then
    /// the notes and problems verbatim (a reader should not have to
    /// re-derive which row failed from a count).
    pub fn text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("registry: {}\n", self.registry));
        out.push_str(&format!("transport: {}\n", self.transport));
        out.push_str(&format!("host: {}\n", self.host));
        if self.document_ok {
            out.push_str(&format!(
                "document: ok ({} payloads, {} version rows)\n",
                self.payloads, self.versions
            ));
        } else {
            out.push_str("document: refused\n");
        }
        out.push_str(&format!(
            "rows checked: {} ({} artifact(s) downloaded{})\n",
            self.rows_checked,
            self.artifacts_fetched,
            if self.full { ", --full" } else { "" }
        ));
        for finding in &self.findings {
            let mut subject = String::new();
            if let Some(payload) = &finding.payload {
                subject.push_str(payload);
                if let Some(version) = &finding.version {
                    subject.push(' ');
                    subject.push_str(version);
                }
                subject.push_str(": ");
            }
            out.push_str(&format!(
                "  {:<8}{}{}\n",
                finding.severity.label(),
                subject,
                finding.message
            ));
        }
        let problems = self.problems();
        if problems == 0 {
            out.push_str("tebako registry doctor: conformant\n");
        } else {
            out.push_str(&format!(
                "tebako registry doctor: {} problem(s)\n",
                problems
            ));
        }
        out
    }

    /// The machine report (`registry_doctor_schema: 1`).
    pub fn json(&self) -> String {
        use tebako_pkg::{json_to_string, JsonValue as J};
        let s = |v: &str| J::String(v.to_string());
        let findings: Vec<J> = self
            .findings
            .iter()
            .map(|f| {
                let mut obj = vec![
                    ("severity".to_string(), s(f.severity.label())),
                    ("message".to_string(), s(&f.message)),
                ];
                if let Some(p) = &f.payload {
                    obj.insert(0, ("payload".to_string(), s(p)));
                }
                if let Some(v) = &f.version {
                    obj.insert(1, ("version".to_string(), s(v)));
                }
                J::Object(obj)
            })
            .collect();
        let n = |v: u64| J::Number(v.to_string());
        let doc = J::Object(vec![
            ("registry_doctor_schema".to_string(), n(1)),
            ("registry".to_string(), s(&self.registry)),
            ("transport".to_string(), s(&self.transport)),
            ("host".to_string(), s(&self.host)),
            ("full".to_string(), J::Bool(self.full)),
            (
                "document".to_string(),
                J::Object(vec![
                    ("ok".to_string(), J::Bool(self.document_ok)),
                    ("payloads".to_string(), n(self.payloads as u64)),
                    ("versions".to_string(), n(self.versions as u64)),
                ]),
            ),
            (
                "rows".to_string(),
                J::Object(vec![
                    ("checked".to_string(), n(self.rows_checked as u64)),
                    (
                        "artifacts_fetched".to_string(),
                        n(self.artifacts_fetched as u64),
                    ),
                ]),
            ),
            ("findings".to_string(), J::Array(findings)),
            ("problems".to_string(), n(self.problems() as u64)),
        ]);
        format!("{}\n", json_to_string(&doc))
    }
}

/// `tebako registry doctor <ref|path> [--full] [--sample <n>] [--json]`.
/// Returns the report text and the verdict: 0 conformant, 1 problems.
/// The INPUT being unreadable is an error (the resolve/IO mapping), never
/// a verdict — a doctor that cannot reach the host has not judged it.
pub fn doctor(
    input: &str,
    full: bool,
    sample: Option<usize>,
    json: bool,
) -> Result<(String, i32), TebakoError> {
    let path = std::path::PathBuf::from(input);
    let transport = if path.is_file() {
        "file".to_string()
    } else {
        let reference = tebako_resolve::RegistryRef::parse(input).map_err(|e| {
            err(
                EX_TEBAKO_MANIFEST,
                format!(
                    "'{input}' is not an existing file and does not parse as a registry reference: {e}"
                ),
            )
        })?;
        registry_class(&reference).to_string()
    };
    // The fetch IS the transport proof: whatever class the ref names has to
    // answer for the document before any row can be judged.
    let bytes = read_registry_bytes(input)?;
    let text = String::from_utf8(bytes).map_err(|e| {
        err(
            EX_TEBAKO_MANIFEST,
            format!("{input}: {e} decoding the registry file"),
        )
    })?;
    let registry = match Registry::from_yaml(&text) {
        Ok(registry) => registry,
        Err(e) => {
            // The document gate is `validate`'s (one authority): its exact
            // refusal is the whole report, and the verdict is a problem.
            let report = Report {
                registry: input.to_string(),
                transport,
                host: String::new(),
                full,
                payloads: 0,
                versions: 0,
                rows_checked: 0,
                artifacts_fetched: 0,
                document_ok: false,
                findings: vec![Finding::new(Severity::Problem, None, None, e.to_string())],
            };
            return Ok((if json { report.json() } else { report.text() }, 1));
        }
    };
    Ok(doctor_registry(
        input, &registry, transport, full, sample, json,
    ))
}

/// The row walk, split from the input handling so fixtures can drive it
/// with a parsed registry and a chosen transport label.
fn doctor_registry(
    input: &str,
    registry: &Registry,
    transport: String,
    full: bool,
    sample: Option<usize>,
    json: bool,
) -> (String, i32) {
    let host = match crate::install::host_platform() {
        Ok(host) => host,
        Err(e) => {
            let report = Report {
                registry: input.to_string(),
                transport,
                host: String::new(),
                full,
                payloads: registry.payloads.len(),
                versions: registry.payloads.iter().map(|p| p.versions.len()).sum(),
                rows_checked: 0,
                artifacts_fetched: 0,
                document_ok: true,
                findings: vec![Finding::new(Severity::Problem, None, None, e.message)],
            };
            return (if json { report.json() } else { report.text() }, 1);
        }
    };
    let mut report = Report {
        registry: input.to_string(),
        transport,
        host: host.as_triplet().to_string(),
        full,
        payloads: registry.payloads.len(),
        versions: registry.payloads.iter().map(|p| p.versions.len()).sum(),
        rows_checked: 0,
        artifacts_fetched: 0,
        document_ok: true,
        findings: Vec::new(),
    };
    // --full downloads bytes, so it samples one version per payload unless
    // the caller asked for more; the metadata pass walks every row.
    let limit = sample.or(if full { Some(1) } else { None });
    let head_key = registry
        .signing
        .as_ref()
        .map(|signing| signing.key.as_bytes().to_vec());
    let fetcher = Fetcher::new();

    for payload in &registry.payloads {
        let versions: Vec<&RegistryVersion> = match limit {
            Some(n) => payload.versions.iter().take(n).collect(),
            None => payload.versions.iter().collect(),
        };
        for entry in versions {
            report.rows_checked += 1;
            check_row(
                &mut report,
                &RowContext {
                    fetcher: &fetcher,
                    input,
                    host,
                    full,
                    head_key: head_key.as_deref(),
                },
                payload,
                entry,
            );
        }
    }
    let code = if report.problems() == 0 { 0 } else { 1 };
    (if json { report.json() } else { report.text() }, code)
}

/// The per-run facts a row walk needs (grouped so the row itself stays a
/// three-argument call).
struct RowContext<'a> {
    fetcher: &'a Fetcher<tebako_resolve::HttpTransport>,
    input: &'a str,
    host: tpkg::Platform,
    full: bool,
    head_key: Option<&'a [u8]>,
}

/// One row against the host: resolve (the install's own resolver), then —
/// with `full` — download the bytes, compare the pin, and verify the
/// declared signature against the registry's head key.
fn check_row(
    report: &mut Report,
    ctx: &RowContext<'_>,
    payload: &RegistryPayload,
    entry: &RegistryVersion,
) {
    let RowContext {
        fetcher,
        input,
        host,
        full,
        head_key,
    } = *ctx;
    let name = payload.name.as_str();
    let version = entry.version.as_str();
    let view = match entry.resolve_variant(payload.default_variant.as_deref()) {
        Ok(view) => view,
        Err(e) => {
            report.findings.push(Finding::new(
                Severity::Problem,
                Some(name),
                Some(version),
                e.to_string(),
            ));
            return;
        }
    };
    let (reference, pin) =
        match crate::install::resolve_row_reference(input, payload, entry, &view, host, false) {
            Ok(pair) => pair,
            Err(e) => {
                // A registry that does not publish this host is normal
                // (the platform matrix is per payload); a malformed row
                // is not. The resolver's own codes carry that difference.
                let severity = if e.code == EX_TEBAKO_MANIFEST {
                    Severity::Problem
                } else {
                    Severity::Note
                };
                report
                    .findings
                    .push(Finding::new(severity, Some(name), Some(version), e.message));
                return;
            }
        };
    // The OCI mirror, when the row declares one: the locator has to parse
    // here even without a download (a malformed mirror is the row's
    // defect), and --full resolves the manifest digest.
    let mirror = match view.select(host) {
        Some(PlatformSelection::Selected { oci, .. }) => oci.map(str::to_string),
        _ => None,
    };
    if let Some(locator) = &mirror {
        if let Err(e) = Reference::parse(locator) {
            report.findings.push(Finding::new(
                Severity::Problem,
                Some(name),
                Some(version),
                format!("the row's oci: mirror does not parse: {e}"),
            ));
            return;
        }
    }
    if !full {
        let described = describe_reference(&reference);
        report.findings.push(Finding::new(
            Severity::Ok,
            Some(name),
            Some(version),
            format!("resolves to {described}"),
        ));
        return;
    }
    // The ref's own `?sha256=` is the same pin the comparison below
    // applies (the universal arm has no row pin — the ref carries it);
    // fetching WITHOUT it keeps a drift a reported mismatch ("the row pins
    // X, the host serves Y") instead of a generic download failure that
    // hides which side owns the defect.
    let pin = pin.or_else(|| reference.sha256().map(str::to_string));
    let unpinned = without_pin(&reference);
    let fetched = match fetcher.fetch(&unpinned) {
        Ok(fetched) => fetched,
        Err(e) => {
            report.findings.push(Finding::new(
                Severity::Problem,
                Some(name),
                Some(version),
                format!(
                    "the host does not serve {}: {}",
                    describe_reference(&reference),
                    e
                ),
            ));
            return;
        }
    };
    report.artifacts_fetched += 1;
    if let Some(pin) = &pin {
        if !fetched.sha256.eq_ignore_ascii_case(pin) {
            report.findings.push(Finding::new(
                Severity::Problem,
                Some(name),
                Some(version),
                format!(
                    "served bytes digest {} but the row pins {pin}",
                    fetched.sha256
                ),
            ));
            return;
        }
    }
    if let Some(sig) = &entry.signature {
        let asc = match crate::install::signature_reference(sig, &reference, &fetched.sha256) {
            Ok(asc) => asc,
            Err(e) => {
                report.findings.push(Finding::new(
                    Severity::Problem,
                    Some(name),
                    Some(version),
                    e.message,
                ));
                return;
            }
        };
        let signature = match fetcher.fetch(&asc) {
            Ok(bytes) => bytes.bytes,
            Err(e) => {
                report.findings.push(Finding::new(
                    Severity::Problem,
                    Some(name),
                    Some(version),
                    format!("the row declares a signature but its .asc does not serve: {e}"),
                ));
                return;
            }
        };
        match head_key {
            Some(key) => match tebako_signer::verify_detached_full(key, &fetched.bytes, &signature)
            {
                Ok(_) => report.findings.push(Finding::new(
                    Severity::Ok,
                    Some(name),
                    Some(version),
                    format!(
                        "{} bytes verify the pin and the signature (head key)",
                        fetched.bytes.len()
                    ),
                )),
                Err(e) => report.findings.push(Finding::new(
                    Severity::Problem,
                    Some(name),
                    Some(version),
                    format!("the signature does not verify against the registry's head key: {e}"),
                )),
            },
            None => report.findings.push(Finding::new(
                Severity::Note,
                Some(name),
                Some(version),
                format!(
                    "{} bytes verify the pin; the row is signed but the registry declares no head key to verify against",
                    fetched.bytes.len()
                ),
            )),
        }
        return;
    }
    report.findings.push(Finding::new(
        Severity::Ok,
        Some(name),
        Some(version),
        format!("{} bytes verify the pin", fetched.bytes.len()),
    ));
}

/// The reference with its `?sha256=` pin removed (the doctor compares the
/// pin itself, so the fetch must not answer the question first).
fn without_pin(reference: &Reference) -> Reference {
    let mut reference = reference.clone();
    match &mut reference {
        Reference::Service { sha256, .. }
        | Reference::Git { sha256, .. }
        | Reference::Oci { sha256, .. }
        | Reference::Https { sha256, .. }
        | Reference::File { sha256, .. } => *sha256 = None,
    }
    reference
}

/// The transport class a registry ref names, for the report header.
fn registry_class(reference: &tebako_resolve::RegistryRef) -> &'static str {
    use tebako_resolve::RegistryRef;
    match reference {
        RegistryRef::DefaultBranch { service, .. } => service.scheme(),
        RegistryRef::ReleaseArtifact(Reference::Service { service, .. }) => service.scheme(),
        RegistryRef::ReleaseArtifact(_) => "release",
        RegistryRef::GitBlob(_) => "git",
        RegistryRef::Https(_) => "https",
        RegistryRef::Oci(_) => "oci",
        RegistryRef::File(_) => "file",
    }
}

/// A reference in one line: the origin-shaped coordinates, not the Rust
/// debug form.
fn describe_reference(reference: &Reference) -> String {
    match release_download_locator(reference).and_then(|locator| locator.dir_url()) {
        Some(dir) => match reference {
            Reference::Service {
                artifact: Some(name),
                ..
            } => format!("{dir}/{name}"),
            _ => dir,
        },
        None => reference.to_string(),
    }
}

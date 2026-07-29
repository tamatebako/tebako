//! The MECE payload reference syntax (spec 04 §1): one scheme family, the
//! adapter explicit in the scheme, **no default service** anywhere. Every
//! string has exactly one home; anything else is a named error listing the
//! classes — superseded forms (`tfs://github.com/…`) and host-inferred
//! shorthand are rejected, never guessed.
//!
//! ```text
//! tfs:github:owner/repo:version[#artifact]   service adapter (GitHub releases)
//! tfs:gitlab:owner/repo:version[#artifact]   service adapter (GitLab releases)
//! tfs:bb:owner/repo:version[#artifact]       service adapter (Bitbucket downloads)
//! tfs+git://host/owner/repo.git[@ref][#path]   git protocol adapter
//! tfs+https://cdn.example.com/tool.tfs   verbatim HTTPS fetch
//! https://cdn.example.com/tool.tfs       (same class, bare form)
//! file:///opt/images/tool.tfs            local file
//! …?sha256=<64 hex>                      digest pin, query form, any class
//! ```
//!
//! `#artifact` selects one asset within a multi-artifact release (spec 04
//! §1, locked): with it the fetcher takes exactly that asset; without it
//! the candidate class is `.tfs` images — exactly one is used, zero is
//! `AssetNotFound`, more than one is `AmbiguousAssets`. The adapter NEVER
//! auto-picks by host triplet; platform selection is the registry's
//! declarative job (spec 04 §2).

use std::fmt;

use crate::error::ReferenceError;

/// The explicit service adapters (spec 04 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Github,
    Gitlab,
    Bitbucket,
}

impl Service {
    /// The scheme keyword as written in a reference (`tfs:<scheme>:…`).
    pub fn scheme(self) -> &'static str {
        match self {
            Service::Github => "github",
            Service::Gitlab => "gitlab",
            Service::Bitbucket => "bb",
        }
    }

    /// Human name for diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Service::Github => "github",
            Service::Gitlab => "gitlab",
            Service::Bitbucket => "bitbucket",
        }
    }
}

/// A parsed payload reference. `sha256` is the optional digest pin
/// (`?sha256=<hex>`, normalized to lowercase) on any class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// `tfs:github:owner/repo:version[#artifact]` (gitlab, bb). `owner`
    /// keeps any nested groups (`group/sub`) for hosts that support them.
    /// `artifact` is the `#name` asset selector of a multi-artifact
    /// release (spec 04 §1); None means the single-`.tfs` rule.
    Service {
        service: Service,
        owner: String,
        repo: String,
        version: String,
        artifact: Option<String>,
        sha256: Option<String>,
    },
    /// `tfs+git://host/owner/repo.git[@ref][#path]` — `url` is `host/path`
    /// exactly as written (no scheme; the git adapter maps it to the
    /// transport). `git_ref` None means the remote's default branch;
    /// `path` selects the image when the repo holds many (spec 04 §1).
    Git {
        url: String,
        git_ref: Option<String>,
        path: Option<String>,
        sha256: Option<String>,
    },
    /// `tfs+https://…` or bare `https://…` — `url` is the canonical
    /// `https://…` form (the pin is removed from the query string; other
    /// query parameters, e.g. CDN signatures, are preserved).
    Https { url: String, sha256: Option<String> },
    /// `file:///absolute/path` (tests, air-gapped mirrors).
    File {
        path: String,
        sha256: Option<String>,
    },
}

impl Reference {
    /// Parse a reference string (the spec 04 §1 dispatch rule).
    pub fn parse(input: &str) -> Result<Reference, ReferenceError> {
        let input = input.trim();
        for (prefix, service) in [
            ("tfs:github:", Service::Github),
            ("tfs:gitlab:", Service::Gitlab),
            ("tfs:bb:", Service::Bitbucket),
        ] {
            if let Some(rest) = input.strip_prefix(prefix) {
                return parse_service(input, rest, service);
            }
        }
        if let Some(rest) = input.strip_prefix("tfs+git://") {
            return parse_git(input, rest);
        }
        if let Some(rest) = input.strip_prefix("tfs+https://") {
            return parse_https(input, rest);
        }
        if let Some(rest) = input.strip_prefix("https://") {
            return parse_https(input, rest);
        }
        if let Some(rest) = input.strip_prefix("file://") {
            return parse_file(input, rest);
        }
        // Recognized-but-malformed families get a targeted reason; anything
        // else is the named error listing the classes (never a guess).
        for family in ["tfs:", "tfs+git:", "tfs+https:", "http://"] {
            if input.starts_with(family) {
                return Err(ReferenceError::Invalid {
                    input: input.to_string(),
                    reason: format!(
                        "malformed {family} reference; expected one of {}",
                        crate::error::REFERENCE_CLASSES
                    ),
                });
            }
        }
        Err(ReferenceError::UnknownScheme {
            input: input.to_string(),
        })
    }

    /// The digest pin (`?sha256=<hex>`), lowercase, if present.
    pub fn sha256(&self) -> Option<&str> {
        match self {
            Reference::Service { sha256, .. }
            | Reference::Git { sha256, .. }
            | Reference::Https { sha256, .. }
            | Reference::File { sha256, .. } => sha256.as_deref(),
        }
    }
}

impl fmt::Display for Reference {
    /// The canonical reference string; `parse(display(r)) == r`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pin = |f: &mut fmt::Formatter<'_>, sha256: &Option<String>, prefix: &str| {
            if let Some(sha) = sha256 {
                write!(f, "{prefix}sha256={sha}")?;
            }
            Ok(())
        };
        match self {
            Reference::Service {
                service,
                owner,
                repo,
                version,
                artifact,
                sha256,
            } => {
                write!(f, "tfs:{}:{owner}/{repo}:{version}", service.scheme())?;
                pin(f, sha256, "?")?;
                if let Some(a) = artifact {
                    write!(f, "#{a}")?;
                }
                Ok(())
            }
            Reference::Git {
                url,
                git_ref,
                path,
                sha256,
            } => {
                write!(f, "tfs+git://{url}")?;
                if let Some(r) = git_ref {
                    write!(f, "@{r}")?;
                }
                pin(f, sha256, "?")?;
                if let Some(p) = path {
                    write!(f, "#{p}")?;
                }
                Ok(())
            }
            Reference::Https { url, sha256 } => {
                let bare = url.strip_prefix("https://").unwrap_or(url);
                write!(f, "tfs+https://{bare}")?;
                // The pin merges back into any preserved query parameters.
                pin(f, sha256, if bare.contains('?') { "&" } else { "?" })
            }
            Reference::File { path, sha256 } => {
                write!(f, "file://{path}")?;
                pin(f, sha256, "?")
            }
        }
    }
}

impl std::str::FromStr for Reference {
    type Err = ReferenceError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Reference::parse(s)
    }
}

// ---- parsing helpers -------------------------------------------------------

pub(crate) fn invalid(input: &str, reason: impl Into<String>) -> ReferenceError {
    ReferenceError::Invalid {
        input: input.to_string(),
        reason: reason.into(),
    }
}

/// `?sha256=<64 hex>` (lowercase-normalized). `query` is the text after a
/// `?`; classes without other legal parameters must match exactly.
pub(crate) fn parse_exact_pin(
    input: &str,
    query: Option<&str>,
) -> Result<Option<String>, ReferenceError> {
    match query {
        None => Ok(None),
        Some(q) => {
            let Some(hex) = q.strip_prefix("sha256=") else {
                return Err(invalid(
                    input,
                    "only the ?sha256=<64 hex> digest pin is supported here",
                ));
            };
            validate_hex(input, hex).map(Some)
        }
    }
}

fn validate_hex(input: &str, hex: &str) -> Result<String, ReferenceError> {
    let ok = hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit());
    if ok {
        Ok(hex.to_ascii_lowercase())
    } else {
        Err(invalid(
            input,
            format!("sha256 pin must be exactly 64 hex characters, got '{hex}'"),
        ))
    }
}

/// Reject components that are empty or carry control/whitespace/forbidden
/// characters; everything else (dots, dashes, unicode) is fair game.
pub(crate) fn check_component(
    input: &str,
    what: &str,
    value: &str,
    forbidden: &[char],
) -> Result<(), ReferenceError> {
    if value.is_empty() {
        return Err(invalid(input, format!("empty {what}")));
    }
    let bad: Option<char> = value
        .chars()
        .find(|c| c.is_control() || c.is_whitespace() || forbidden.contains(c));
    if let Some(c) = bad {
        return Err(invalid(
            input,
            format!("{what} '{value}' contains forbidden character '{c}'"),
        ));
    }
    Ok(())
}

/// `tfs:<svc>:<owner>/<repo>:<version>[?sha256=…][#artifact]` — split at
/// the LAST ':' so owner paths keep nested groups (gitlab `group/sub/repo`).
/// The `#artifact` fragment (spec 04 §1 multi-artifact rule) selects one
/// release asset by exact name; the pin stays in query form and never
/// clashes with the fragment.
fn parse_service(input: &str, rest: &str, service: Service) -> Result<Reference, ReferenceError> {
    let (before_frag, artifact) = match rest.split_once('#') {
        Some((b, f)) => {
            if f.is_empty() {
                return Err(invalid(input, "empty #artifact fragment"));
            }
            (b, Some(f))
        }
        None => (rest, None),
    };
    let (body, query) = match before_frag.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (before_frag, None),
    };
    let sha256 = parse_exact_pin(input, query)?;
    let Some((path, version)) = body.rsplit_once(':') else {
        return Err(invalid(
            input,
            "missing ':version' — the form is tfs:<service>:owner/repo:version",
        ));
    };
    let Some((owner, repo)) = path.rsplit_once('/') else {
        return Err(invalid(input, "missing 'owner/repo' path"));
    };
    check_component(input, "owner", owner, &['?', '#', ':', '@'])?;
    check_component(input, "repo", repo, &['?', '#', ':', '@'])?;
    check_component(input, "version", version, &['?', '#', ':'])?;
    if let Some(a) = artifact {
        check_component(input, "artifact", a, &['?', '#', '/'])?;
    }
    Ok(Reference::Service {
        service,
        owner: owner.to_string(),
        repo: repo.to_string(),
        version: version.to_string(),
        artifact: artifact.map(str::to_string),
        sha256,
    })
}

/// `tfs+git://<host>/<path>[.@ref][?sha256=…][#path-in-repo]`. The ref is
/// everything after the FIRST '@' (refs may themselves contain '@'); the
/// fragment is everything after the first '#'.
fn parse_git(input: &str, rest: &str) -> Result<Reference, ReferenceError> {
    let (before_frag, frag) = match rest.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (rest, None),
    };
    if let Some(f) = frag {
        if f.is_empty() {
            return Err(invalid(input, "empty #path fragment"));
        }
    }
    let (before_query, query) = match before_frag.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (before_frag, None),
    };
    let sha256 = parse_exact_pin(input, query)?;
    let (host_path, git_ref) = match before_query.split_once('@') {
        Some((h, r)) => {
            check_component(input, "git ref", r, &['?', '#'])?;
            (h, Some(r.to_string()))
        }
        None => (before_query, None),
    };
    let Some((host, path)) = host_path.split_once('/') else {
        return Err(invalid(
            input,
            "missing repository path — the form is tfs+git://host/owner/repo.git[@ref][#path]",
        ));
    };
    check_component(input, "host", host, &['?', '#'])?;
    if path.is_empty() {
        return Err(invalid(input, "empty repository path"));
    }
    if path.bytes().any(|b| b < 0x20) || path.contains('?') || path.contains('#') {
        return Err(invalid(
            input,
            "repository path contains forbidden characters",
        ));
    }
    Ok(Reference::Git {
        url: host_path.to_string(),
        git_ref,
        path: frag.map(str::to_string),
        sha256,
    })
}

/// `(tfs+)https://…` — the pin is extracted from the query string; other
/// query parameters are preserved in the stored canonical URL.
fn parse_https(input: &str, rest: &str) -> Result<Reference, ReferenceError> {
    if rest.contains('#') {
        return Err(invalid(
            input,
            "URL fragments are not meaningful for payload fetch; use ?sha256=<hex> to pin a digest",
        ));
    }
    let (base, query) = match rest.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (rest, None),
    };
    let host = base.split('/').next().unwrap_or_default();
    check_component(input, "host", host, &['?', '#'])?;
    let mut sha256 = None;
    let mut kept: Vec<&str> = Vec::new();
    if let Some(q) = query {
        for param in q.split('&') {
            if let Some(hex) = param.strip_prefix("sha256=") {
                if sha256.is_some() {
                    return Err(invalid(input, "duplicate sha256 pin"));
                }
                sha256 = Some(validate_hex(input, hex)?);
            } else if !param.is_empty() {
                kept.push(param);
            }
        }
    }
    let mut url = format!("https://{base}");
    if !kept.is_empty() {
        url.push('?');
        url.push_str(&kept.join("&"));
    }
    Ok(Reference::Https { url, sha256 })
}

/// `file://<absolute-path>[?sha256=…]` — a `?` is only legal as the digest
/// pin delimiter (named error otherwise, never a guess).
fn parse_file(input: &str, rest: &str) -> Result<Reference, ReferenceError> {
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let sha256 = parse_exact_pin(input, query)?;
    if !path.starts_with('/') {
        return Err(invalid(
            input,
            "file references need an absolute path (file:///…)",
        ));
    }
    if path.bytes().any(|b| b == 0) {
        return Err(invalid(input, "path contains a NUL byte"));
    }
    Ok(Reference::File {
        path: normalize_file_path(path).to_string(),
        sha256,
    })
}

/// RFC 8089 path recovery: `file:///C:/x` names the Windows path `C:/x` —
/// the third slash separates the (empty) authority from the drive path,
/// while on unix the path begins AT that slash. Windows must drop the
/// leading slash before a drive letter or the result (`/C:/x`) is not a
/// valid filesystem path (os error 123).
fn normalize_file_path(path: &str) -> &str {
    #[cfg(windows)]
    {
        let b = path.as_bytes();
        if b.len() > 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' && b[3] == b'/'
        {
            return &path[1..];
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_references() {
        let r = Reference::parse("tfs:github:metanorma/metanorma:1.2.3").unwrap();
        assert_eq!(
            r,
            Reference::Service {
                service: Service::Github,
                owner: "metanorma".into(),
                repo: "metanorma".into(),
                version: "1.2.3".into(),
                artifact: None,
                sha256: None,
            }
        );
        assert_eq!(r.to_string(), "tfs:github:metanorma/metanorma:1.2.3");

        // gitlab keeps nested groups in the owner.
        let r = Reference::parse("tfs:gitlab:group/sub/tool:v2").unwrap();
        assert_eq!(
            r,
            Reference::Service {
                service: Service::Gitlab,
                owner: "group/sub".into(),
                repo: "tool".into(),
                version: "v2".into(),
                artifact: None,
                sha256: None,
            }
        );

        let sha = "A".repeat(64);
        let r = Reference::parse(&format!("tfs:bb:o/r:1.0?sha256={sha}")).unwrap();
        assert_eq!(r.sha256(), Some(sha.to_ascii_lowercase().as_str()));
        assert_eq!(
            r.to_string(),
            format!("tfs:bb:o/r:1.0?sha256={}", sha.to_ascii_lowercase())
        );
    }

    #[test]
    fn service_references_with_artifact() {
        let r = Reference::parse(
            "tfs:github:metanorma/metanorma:1.2.3#metanorma-1.2.3-macos-arm64.tfs",
        )
        .unwrap();
        assert_eq!(
            r,
            Reference::Service {
                service: Service::Github,
                owner: "metanorma".into(),
                repo: "metanorma".into(),
                version: "1.2.3".into(),
                artifact: Some("metanorma-1.2.3-macos-arm64.tfs".into()),
                sha256: None,
            }
        );
        assert_eq!(
            r.to_string(),
            "tfs:github:metanorma/metanorma:1.2.3#metanorma-1.2.3-macos-arm64.tfs"
        );
        assert_eq!(Reference::parse(&r.to_string()).unwrap(), r);

        // pin (query form) and artifact (fragment) never clash — pin first.
        let sha = "e".repeat(64);
        let r = Reference::parse(&format!("tfs:gitlab:o/r:1.0?sha256={sha}#tool.tfs")).unwrap();
        assert!(
            matches!(&r, Reference::Service { artifact: Some(a), sha256: Some(s), .. } if a == "tool.tfs" && s == &sha)
        );
        assert_eq!(
            r.to_string(),
            format!("tfs:gitlab:o/r:1.0?sha256={sha}#tool.tfs")
        );

        // the registry's pinned-immutable form (spec 04 §2) parses too
        let r = Reference::parse("tfs:github:o/r:v1#tpkg-registry.yaml").unwrap();
        assert!(
            matches!(&r, Reference::Service { artifact: Some(a), .. } if a == "tpkg-registry.yaml")
        );

        for bad in [
            "tfs:github:o/r:1.0#",        // empty fragment
            "tfs:github:o/r:1.0#a/b.tfs", // artifacts are file names, no '/'
            "tfs:github:o/r:1.0#a.tfs#b", // one fragment only
            "tfs:github:o/r:1.0#a t.tfs", // no whitespace
        ] {
            assert!(
                matches!(Reference::parse(bad), Err(ReferenceError::Invalid { .. })),
                "{bad} must be a named error"
            );
        }
    }

    #[test]
    fn git_references() {
        let r = Reference::parse("tfs+git://git.example.com/team/repo.git@v1.0#images/tool.tfs")
            .unwrap();
        assert_eq!(
            r,
            Reference::Git {
                url: "git.example.com/team/repo.git".into(),
                git_ref: Some("v1.0".into()),
                path: Some("images/tool.tfs".into()),
                sha256: None,
            }
        );
        assert_eq!(
            r.to_string(),
            "tfs+git://git.example.com/team/repo.git@v1.0#images/tool.tfs"
        );

        // branch-style refs with slashes; ref with '@' inside
        let r = Reference::parse("tfs+git://h/r.git@refs/heads/feature/x").unwrap();
        assert_eq!(r.sha256(), None);
        assert!(
            matches!(&r, Reference::Git { git_ref: Some(g), .. } if g == "refs/heads/feature/x")
        );
        let r = Reference::parse("tfs+git://h/r.git@user@domain").unwrap();
        assert!(matches!(&r, Reference::Git { git_ref: Some(g), .. } if g == "user@domain"));

        // pin before fragment (query form never clashes with #path)
        let sha = "b".repeat(64);
        let r = Reference::parse(&format!("tfs+git://h/r.git@main?sha256={sha}#p.tfs")).unwrap();
        assert!(
            matches!(&r, Reference::Git { sha256: Some(s), path: Some(p), .. } if s == &sha && p == "p.tfs")
        );

        // no ref, no path: the repo IS the registry (parse is fine; fetch
        // is the named GitPathRequired error)
        let r = Reference::parse("tfs+git://h/registry.git").unwrap();
        assert!(matches!(
            &r,
            Reference::Git {
                git_ref: None,
                path: None,
                ..
            }
        ));
    }

    #[test]
    fn https_references() {
        let r = Reference::parse("tfs+https://cdn.example.com/tool.tfs").unwrap();
        assert_eq!(
            r,
            Reference::Https {
                url: "https://cdn.example.com/tool.tfs".into(),
                sha256: None,
            }
        );
        assert_eq!(r.to_string(), "tfs+https://cdn.example.com/tool.tfs");

        // bare https is the same class
        let r = Reference::parse("https://cdn.example.com/tool.tfs").unwrap();
        assert!(matches!(&r, Reference::Https { .. }));
        assert_eq!(r.to_string(), "tfs+https://cdn.example.com/tool.tfs");

        // pin extracted, CDN signature preserved
        let sha = "c".repeat(64);
        let r = Reference::parse(&format!(
            "tfs+https://cdn.example.com/t.tfs?sig=abc&sha256={sha}"
        ))
        .unwrap();
        assert_eq!(
            r,
            Reference::Https {
                url: "https://cdn.example.com/t.tfs?sig=abc".into(),
                sha256: Some(sha.clone()),
            }
        );
        assert_eq!(
            r.to_string(),
            format!("tfs+https://cdn.example.com/t.tfs?sig=abc&sha256={sha}")
        );
        assert_eq!(Reference::parse(&r.to_string()).unwrap(), r);
    }

    #[test]
    fn file_references() {
        let r = Reference::parse("file:///opt/images/tool.tfs").unwrap();
        assert_eq!(
            r,
            Reference::File {
                path: "/opt/images/tool.tfs".into(),
                sha256: None,
            }
        );
        assert_eq!(r.to_string(), "file:///opt/images/tool.tfs");

        let sha = "d".repeat(64);
        let r = Reference::parse(&format!("file:///opt/t.tfs?sha256={sha}")).unwrap();
        assert!(matches!(&r, Reference::File { sha256: Some(s), .. } if s == &sha));
    }

    #[test]
    fn named_error_lists_the_classes() {
        for bad in [
            "metanorma",
            "tfs://github.com/o/r:1.0",
            "github:o/r:1.0",
            "tfs:gitea:o/r:1.0",
            "",
        ] {
            let err = Reference::parse(bad).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("tfs:github:")
                    && msg.contains("tfs+git://")
                    && msg.contains("file://"),
                "{bad}: message must list the classes, got: {msg}"
            );
        }
    }

    #[test]
    fn malformed_references_are_named_errors() {
        for bad in [
            "tfs:github:o/r",               // no version
            "tfs:github:o:1.0",             // no repo path
            "tfs:github:/r:1.0",            // empty owner
            "tfs:github:o/:1.0",            // empty repo
            "tfs:github:o/r:",              // empty version
            "tfs:github:o/r:1.0?x=1",       // only sha256 pin is legal
            "tfs:github:o/r:1.0?sha256=zz", // bad hex
            "tfs+git://h",                  // no repo path
            "tfs+git://h/",                 // empty repo path
            "tfs+git://h/r.git@",           // empty ref
            "tfs+git://h/r.git#",           // empty fragment
            "tfs+https://",                 // empty host
            "https://cdn/x#frag",           // fragments rejected
            "file://relative/path",         // not absolute
            "file:///p?x=1",                // only sha256 pin is legal
        ] {
            let err = Reference::parse(bad).unwrap_err();
            assert!(
                matches!(
                    err,
                    ReferenceError::Invalid { .. } | ReferenceError::UnknownScheme { .. }
                ),
                "{bad} must be a named error"
            );
        }
    }
}

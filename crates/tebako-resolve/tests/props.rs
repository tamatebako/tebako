//! Round-trip property tests (spec 14 §1.4): any valid reference must
//! survive display → parse unchanged, and the parser must never panic on
//! arbitrary input — it answers with a named error or a reference.

use proptest::prelude::*;
use tebako_resolve::{Reference, Service};

/// A conservative component alphabet: no syntax delimiters (`: ? # @ /`
/// are controlled per position by the grammar), no whitespace/controls.
fn arb_segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9][a-zA-Z0-9._-]{0,15}"
}

fn arb_owner() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_segment(), 1..=3).prop_map(|v| v.join("/"))
}

fn arb_sha256() -> impl Strategy<Value = Option<String>> {
    prop::option::of("[0-9a-fA-F]{64}").prop_map(|o| o.map(|s| s.to_ascii_lowercase()))
}

fn arb_reference() -> impl Strategy<Value = Reference> {
    let service = (
        prop::sample::select(vec![Service::Github, Service::Gitlab, Service::Bitbucket]),
        arb_owner(),
        arb_segment(),
        "[a-zA-Z0-9][a-zA-Z0-9._-]{0,10}",
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9._-]{0,15}\\.tfs"),
        arb_sha256(),
    )
        .prop_map(
            |(service, owner, repo, version, artifact, sha256)| Reference::Service {
                service,
                owner,
                repo,
                version,
                artifact,
                sha256,
            },
        );
    let git = (
        arb_segment(),
        prop::collection::vec(arb_segment(), 1..=3),
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9._/-]{0,20}"),
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9._/-]{0,20}"),
        arb_sha256(),
    )
        .prop_map(|(host, path_segs, git_ref, path, sha256)| Reference::Git {
            url: format!("{host}/{}.git", path_segs.join("/")),
            git_ref,
            path,
            sha256,
        });
    let https = (
        arb_segment(),
        prop::collection::vec(arb_segment(), 0..=3),
        prop::option::of("[a-z]{1,6}=[a-z0-9]{1,8}"),
        arb_sha256(),
    )
        .prop_map(|(host, path_segs, extra_query, sha256)| {
            let mut url = format!("https://{host}");
            if !path_segs.is_empty() {
                url.push('/');
                url.push_str(&path_segs.join("/"));
            }
            if let Some(q) = extra_query {
                url.push('?');
                url.push_str(&q);
            }
            Reference::Https { url, sha256 }
        });
    let file =
        (prop::collection::vec(arb_segment(), 1..=3), arb_sha256()).prop_map(|(segs, sha256)| {
            Reference::File {
                path: format!("/{}", segs.join("/")),
                sha256,
            }
        });
    prop_oneof![service, git, https, file]
}

proptest! {
    /// parse ∘ display is identity for every valid reference.
    #[test]
    fn display_parse_round_trip(r in arb_reference()) {
        let s = r.to_string();
        let back = Reference::parse(&s).unwrap_or_else(|e| panic!("{s:?} did not re-parse: {e}"));
        prop_assert_eq!(&back, &r);
        // and display is stable (parse → display → parse → same string)
        prop_assert_eq!(back.to_string(), s);
    }

    /// The parser never panics on arbitrary input; rejections are named
    /// errors whose message lists the reference classes or a reason.
    #[test]
    fn parse_never_panics(s in ".*") {
        let _ = Reference::parse(&s);
    }

    /// Arbitrary strings that DO start with a known prefix still never
    /// panic (the deep grammar paths).
    #[test]
    fn prefixed_junk_never_panics(
        prefix in prop::sample::select(vec![
            "tfs:github:", "tfs:gitlab:", "tfs:bb:", "tfs+git://",
            "tfs+https://", "https://", "file://", "tfs:",
        ]),
        tail in "\\PC*",
    ) {
        let _ = Reference::parse(&format!("{prefix}{tail}"));
    }

    /// Unknown schemes are the named class-listing error, never a guess.
    #[test]
    fn unknown_scheme_is_named(s in "[a-z]{1,8}:[a-z]{1,8}") {
        if !["tfs", "https", "file"].contains(&s.split(':').next().unwrap_or_default()) {
            let err = Reference::parse(&s).unwrap_err();
            prop_assert!(err.to_string().contains("tfs:github:"));
        }
    }
}

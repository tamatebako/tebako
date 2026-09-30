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
                host: None,
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
    // spec 38 §2: lowercase host (+ optional port), lowercase repo
    // components, and a tag XOR a manifest digest (both is the named
    // Invalid) — the byte pin composes with either.
    let oci = (
        "[a-z0-9][a-z0-9.-]{0,15}",
        prop::option::of(1u16..=65535),
        prop::collection::vec("[a-z0-9]+(-[a-z0-9]+){0,2}", 1..=3),
        prop::option::of("[a-zA-Z0-9][a-zA-Z0-9._-]{0,20}"),
        prop::option::of("[0-9a-f]{64}"),
        arb_sha256(),
    )
        .prop_map(|(host, port, repo_segs, tag, digest, sha256)| {
            let host = match port {
                Some(p) => format!("{host}:{p}"),
                None => host,
            };
            // The grammar refuses tag AND digest on one reference.
            let (tag, digest) = match (tag, digest) {
                (Some(t), Some(_)) => (Some(t), None),
                other => other,
            };
            Reference::Oci {
                host,
                repo: repo_segs.join("/"),
                tag,
                digest,
                sha256,
            }
        });
    prop_oneof![service, git, https, file, oci]
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
            "tfs+oci://", "tfs:oci:",
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

    /// spec 38 §2: a reference carrying BOTH a tag and a manifest digest
    /// is the named Invalid, never a silent precedence.
    #[test]
    fn oci_tag_and_digest_is_named_invalid(
        tag in "[a-z0-9][a-z0-9-]{0,10}",
        digest in "[0-9a-f]{64}",
    ) {
        let s = format!("tfs+oci://reg.example/ns/tool:{tag}@sha256:{digest}");
        let err = Reference::parse(&s).unwrap_err();
        prop_assert!(err.to_string().contains("tag") || err.to_string().contains("digest"));
    }

    /// spec 38 §3: the payload tag derives <version>[-<triplet>] and the
    /// derivation is injective within each domain (universal versions
    /// distinct ⇒ tags distinct; same-triplet versions distinct ⇒ tags
    /// distinct; distinct triplets on one version ⇒ tags distinct).
    #[test]
    fn oci_tag_derivation_injective(
        v1 in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        v2 in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        t1 in prop::sample::select(vec!["linux-gnu-x86_64", "macos-arm64", "windows-msvc-x86_64"]),
        t2 in prop::sample::select(vec!["linux-gnu-x86_64", "macos-arm64", "windows-msvc-x86_64"]),
    ) {
        if v1 != v2 {
            prop_assert_ne!(
                tebako_resolve::payload_tag(&v1, None),
                tebako_resolve::payload_tag(&v2, None)
            );
            prop_assert_ne!(
                tebako_resolve::payload_tag(&v1, Some(t1)),
                tebako_resolve::payload_tag(&v2, Some(t1))
            );
        }
        if t1 != t2 {
            prop_assert_ne!(
                tebako_resolve::payload_tag(&v1, Some(t1)),
                tebako_resolve::payload_tag(&v1, Some(t2))
            );
        }
        // and every derived tag stays inside the OCI tag grammar
        let tag = tebako_resolve::payload_tag(&v1, Some(t1));
        prop_assert!(tag.len() <= 128);
    }

    /// spec 38 §3: the signature tag is keyed by the signed blob's
    /// digest — distinct digests derive distinct tags.
    #[test]
    fn oci_signature_tag_keyed(d1 in "[0-9a-f]{64}", d2 in "[0-9a-f]{64}") {
        let tag = tebako_resolve::signature_tag(&d1);
        prop_assert_eq!(&tag, &format!("sha256-{d1}.asc"));
        if d1 != d2 {
            prop_assert_ne!(tag, tebako_resolve::signature_tag(&d2));
        }
    }
}

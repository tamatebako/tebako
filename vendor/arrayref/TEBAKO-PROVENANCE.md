# Vendored arrayref 0.3.9 — TEMPORARY (2026-08-20)

Why this is here: on 2026-08-20 at ~07:15Z the arrayref author yanked
every version from 0.3.5 through 0.3.9 on crates.io and deleted the
upstream GitHub repository (droundy/arrayref → 404). blake3 1.5.0
requires `arrayref = "^0.3.5"`, so every fresh `cargo` resolution of
this workspace (no committed Cargo.lock by design — see the root
AGENTS.md §13) failed with "failed to select a version for the
requirement `arrayref = ^0.3.5`" — all CI legs that compile the
workspace (incl. the runtime factory's link-unit staging) went down.

Provenance: these bytes are the published crates.io artifact, fetched
from https://static.crates.io/crates/arrayref/arrayref-0.3.9.crate
(yanked versions remain downloadable), sha256
`76a2e8124351fda1ef8aaaa3bbd7ebbcb486bbcd4225aca0aa0d84bb2db8fecb` —
byte-identical with the sparse-index `cksum` for 0.3.9. License:
BSD-2-Clause (see LICENSE, shipped in the artifact). Author: David
Roundy.

Removal condition: revert the `[patch.crates-io]` entry in the root
Cargo.toml and delete this directory once crates.io upstream settles —
an unyank, or a maintained successor release that blake3 (or our
limnifs-core dependency chain) moves to. Tracked in the ecosystem
PROGRESS/05 notes.

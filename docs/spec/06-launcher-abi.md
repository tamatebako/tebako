# Spec 06 — Launcher ABI (bootstrap → runtime)

Normative specification of the handoff contract. Version: **1**. Status:
SHIPPED (macOS/Linux; Windows exec/lock port PARTIAL — roadmap 02).

## 1. The handoff

```
<runtime> --tebako-image <self>:<slot>:<mount> ...
          --tebako-entry <argv0> <user args...>
```

- One `--tebako-image` triple per payload slot to mount:
  `<self>` = the package's own path, `<slot>` = slot index, `<mount>` =
  the slot's mount point.
- Slots whose role is runtime are **never** handed over as mounts.
- `--tebako-entry` separates loader-consumed args from user args;
  `<argv0>` is the entrypoint name inside the mounted tree.
- `--tebako-extract` is a runtime-side option riding the user-arg
  passthrough (the loader never interprets it).

## 2. Image-era addition (additive, ABI stays 1)

When the runtime_ref carries `;image` (spec 05 §1), the loader exports:

```
TEBAKO_RUNTIME_IMAGE=<absolute path of the cached .tfs>
```

The runtime driver prefers the env image over any embedded image
(one-file driver patch, `docs/tebako-main.cpp.30b.patch`); v1 runtimes
ignore the env and use their embedded image — graceful degradation, no
republish of v1-era runtimes needed.

## 3. Loader behavior contract (tebako-bootstrap)

1. Read own trailer (spec 02; absent → classic-bundle error path).
2. Require `launcher_abi == 1` (else exit 66).
2a. **`--tebako-install` verb (TODO.v2-1/12):** after the chain + ABI
   gates — the package's `TPKG_FLAG_NO_INSTALL` refuses (exit 76); any
   other package answers with the named guidance to
   `tebako install <path>` (exit 76 — the manifest read needs the TFS
   engine the size-capped bootstrap deliberately does not carry). A run
   is a run: the verb is the ONLY way slices reach the store.
3. Trust handling, by build and trailer flag:
   - **`TPKG_FLAG_SIGNED_V2`, verification ENABLED** (`openpgp-verify`
     feature): verify the OpenPGP signature against the trusted keyring
     and each slot's SHA-256 — always strict (spec 09).
   - **`TPKG_FLAG_SIGNED_V2`, verification DISABLED** (unverified-first,
     the shipped default until roadmap 72's crypto toolkit): loud
     UNVERIFIED warning + audit journal, then enforce each slot's
     SHA-256 as integrity-vs-corruption (the anchor is unverified —
     documented, spec 09 §7). `TEBAKO_REQUIRE_SIGNED=1` here fails
     CLOSED with exit 71 naming the missing capability ("built without
     OpenPGP verification") — a strict-mode request is never silently
     downgraded to unverified.
   - **Unsigned (v1)**: loud warning + audit journal (or exit 71 under
     `TEBAKO_REQUIRE_SIGNED=1`).
4. Resolve the runtime per spec 05 §5 — negotiating the contract version
   (§6) fail-closed before any checksum acceptance.
5. Image-era: ensure `<asset>.tfs` + trust markers in the cache entry
   (fetch + verify on miss), install read-only.
6. Exec the handoff. Never returns on success.

## 4. Exit codes (named, stable)

| code | name | meaning |
|-----:|------|---------|
| 65 | `EX_TEBAKO_MANIFEST` | trailer missing/corrupt/invalid |
| 66 | `EX_TEBAKO_ABI` | launcher_abi mismatch |
| 67 | `EX_TEBAKO_RUNTIME_REF` | unparseable/unsupported runtime_ref |
| 69 | `EX_TEBAKO_UNAVAILABLE` | runtime unresolvable (offline miss, download failure) |
| 70 | `EX_TEBAKO_SHA` | sha256 mismatch (runtime or image) |
| 71 | `EX_TEBAKO_SIGNATURE` | invalid signature; or unsigned under `TEBAKO_REQUIRE_SIGNED=1` |
| 72 | `EX_TEBAKO_TRUST` | signer key not in the trusted keyring |
| 73 | `EX_TEBAKO_JAIL` | jail policy could not be applied (malformed `TEBAKO_JAIL`; fail-closed — spec 08) |
| 74 | `EX_TEBAKO_IO` | filesystem/lock/install failure |
| 75 | `EX_TEBAKO_CONTRACT` | runtime declares an unsupported `contract_version` (§6) |
| 76 | `EX_TEBAKO_INSTALL` | `--tebako-install` refused (`TPKG_FLAG_NO_INSTALL`) or needs the CLI |

stderr body: `tebako-bootstrap: <message>\n` — message bodies match the
C++ reference bootstrap 1:1 (golden parity).

## 5. Progress UX (locked 2026-07-26)

When the loader fetches a runtime or image, the user SEES the work and
the benefit. Rules:

- **TTY-only:** full progress rendering iff stderr is a TTY and
  `TERM != dumb`; otherwise exactly two single lines (start + done), CI/
  log-safe. Opt-outs: `NO_COLOR`, `TEBAKO_NO_PROGRESS=1`.
- **Phases, one line each:** `resolving <runtime_ref>` →
  `downloading <asset> (<size>)` with the live bar →
  `verifying sha256` → `installing (locked)` → done.
- **The bar** (hand-rolled ANSI, no deps — the size gate forbids
  indicatif-class crates): `\r[=====>    ] 62%  14.2/23.0 MB  3.1 MB/s`
  throttled to ≤ 10 redraws/s; unknown content-length → spinner frames +
  byte count.
- **The benefit is stated:** on completion —
  `installed ruby-3.4.2-0.15.9-linux-gnu-x86_64 (23.0 MB) — cached at
  ~/.tebako/runtimes/… and shared by every tebako app on this machine`.
  A cache HIT prints one quiet line: `runtime ruby-3.4.2 (cached)`.
- Progress output goes to **stderr**, never stdout (stdout belongs to
  the payload).
- Implementation: a tiny `tebako-term` micro-crate (TTY detect, bar,
  spinner, phase lines) consumed by tebako-bootstrap; tebako-shim and
  tebako-cli reuse it (no third copy). tebako-http gains an
  `on_progress(bytes_so_far, content_length)` callback hooking the
  stream — the bar is transport-accurate, not estimated.

## 6. Bootstrap↔runtime contract negotiation (roadmap 45)

The launcher ABI (§1–§2) is the wire format; the **contract version** is
its semantics version, declared by the runtime and enforced by the
bootstrap. It exists so a future handoff change (env names, argv shape,
image-handoff semantics) can be introduced without old bootstraps
silently mis-executing new runtimes.

| contract | semantics |
|---------:|-----------|
| 1 | this document, §1–§5: `--tebako-image`/`--tebako-entry` argv, `TEBAKO_RUNTIME_IMAGE` env handoff, trailer/ABI gating as in §3 |

**Bump rule.** Any change to env/argv/handoff semantics — a renamed or
repurposed variable, a new loader-consumed flag, a change in image
precedence — increments the contract version by exactly 1. Additive
payload-side changes that v1 runtimes may ignore (like §2's env image
for v1 runtimes) do NOT bump the contract.

**Declaration.** The runtime release manifest's per-package entry carries
`"contract_version": <uint>` (spec 05 §5's manifest.json; additive —
older consumers ignore it). The runtime driver has the same value
compiled in (`TEBAKO_CONTRACT_VERSION`); the runtime repo's CI fails any
release where the manifest and the driver disagree.

**Negotiation rule (bootstrap, fail-closed).** During runtime/image
resolution, before any checksum acceptance:

1. Manifest entry declares `contract_version == SUPPORTED_CONTRACT`
   (currently 1) → proceed.
2. Field absent or unparseable → a pre-contract release, which is
   contract 1 by definition → proceed while 1 is the supported contract.
3. Any other declared value → refuse: nothing is installed, the cache is
   untouched, exit 75 (`EX_TEBAKO_CONTRACT`) naming both generations and
   the remedy (upgrade tebako, or pin an older runtime).

A negotiation failure is never a crash and never a silent accept: it is
a clean, explained refusal. When contract 2 exists, `SUPPORTED_CONTRACT`
becomes a range and rule 1 accepts any value in it.

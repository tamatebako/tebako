# Spec 24 — Declarative overlays: write areas and key bindings

**Status: PARTIAL.** This change ships the TFS-layer mechanism: the
gated COW composite (`CowBackend::with_write_areas` — the §5 write
gate), the mount plumbing carrying declared write areas
(`crates/tfs/src/mount.rs`'s `Overlay`), the `TEBAKO_OVERLAYS` /
`TEBAKO_DECRYPT` env grammars (`crates/tfs/src/overlay_spec.rs`), the
journal vocabulary (`vfs-deny` / `vfs-write` / `overlay` / `decrypt` —
`crates/tfs/src/journal.rs`) with `vfs-deny` journaling live at the
engine's write denials, the `tfs needs` generator's fold into draft
`needs.write:` / `needs.decrypt:` blocks (§6), and the exit-68
taxonomy row (§7 — `tpkg::EX_TEBAKO_OVERLAY`, owner-signed-off
2026-08-15). Still PLANNED (the implementation chain, in dependency
order): the D1 `needs.write` / `needs.decrypt` manifest model
(`crates/tpkg`; open point 3), the D2 `overlays:` / `decrypt:`
composition keys with resolver steps 3a–3c (§4), the driver's and
preload shim's consumption of the env forms (the boot-time `overlay` /
`decrypt` audit producers, the gated restack, and the sealed-read
`class=ekey` producer), and record mode's ephemeral scratch stacking
(§6's `vfs-write` producer).

The transform machinery was already SHIPPED — `CowBackend` over
`HostDirBackend` with the `.tfs-whiteouts` journal (spec 11 §4),
`EncBackend` + `KeySource` with the `/__tpkg__/envelopes.yaml` grant
manifest (spec 10 §7), the `TEBAKO_MOUNT_COW` mode and the
`*_with_mode` mount family (spec 11 §7). What this spec adds is the
DECLARATIVE surface: a manifest key letting a slice say "I write here"
or "I am sealed — bind a key", and a composition key letting an
operator bind the overlay store or the recipient. This spec is the
spec-23 amendment that closes that gap. One law follows, extending
spec 23's declaration law:

**Nothing is transformed that was not declared.** A write area, an
overlay store, a key binding — each is WRITTEN DOWN in exactly one
document per concern (the slice's D1 `needs:`, the composition's D2,
the press-baked D3), resolved BEFORE EXEC, journaled at run time. The
transforms law (spec 00, invariant 5) is unchanged and sharpened:
transforms exist ONLY in the Rust TFS, and now enter a run ONLY through
declarations.

## 1. The pinned stack order (normative)

Per mount point, top (syscall side) to bottom (bytes):

```
COW  →  ENC  →  format backend
CowBackend( EncBackend( DwarfsBackend | SquashfsBackend | … ) )
```

**COW is always outermost; ENC wraps exactly one image's backend and
sits under COW.** A union mount (spec 03 §6 `mode: union`) composes the
per-image stacks beneath COW — `EncBackend` is a `Backend`, so sealed
union members wrap individually and merge as plaintext views. Rationale,
in order of decisiveness:

1. `EncBackend` is a decrypting READ view: it implements `Backend`
   only — no `WritableBackend` (`crates/tfs/src/backends_enc.rs`;
   `CowBackend` is the writable one,
   `crates/tfs/src/backends_cow.rs`). The write-intercepting transform
   must be the first thing the write gate meets, or writes die EROFS on
   the ENC view of a writable composite. `Enc(COW(base))` can never
   accept a write; only `COW(Enc(base))` yields a writable sealed-base
   mount.
2. Copy-up reads THROUGH the ENC layer: modifying a sealed base file
   copies its plaintext into the overlay. The overlay is operator-bound
   host store — outside the image's confidentiality envelope, stated
   honestly (a host that must protect scratch at rest uses host disk
   encryption; that is the host's layer, never a tebako declaration).
3. One order, one semantics: journals, errors, and the §4 resolution
   are deterministic because the grammar pins exactly one stack.
   `Enc(COW(base))` remains expressible programmatically — the mount
   API composes any `Box<dyn Backend>` — it is simply never the
   declarative spelling.
4. Whiteouts name plaintext view paths — consistent with the ENC
   format keeping directory structure, names, and symlink targets
   plaintext; only regular-file CONTENT is ciphertext
   (`crates/tfs/src/backends_enc.rs`).

## 2. D1 — the `needs:` extension (spec 23 §2 amendment)

Two new blocks under the ONE `needs:` key (MECE: spec 23 owns the
spelling; this spec extends the grammar). Schema mechanics per the
evolution law (spec 18 §3): an additive MINOR bump of the payload
manifest — `schema_minor` +1; old readers ignore the blocks, new
readers enforce. The grammar text lands in the schema-registry doc
(`docs/spec/schemas/payload-manifest.yaml`) and the generated
`schema/tpkg-manifest-v1.schema.json` in the implementation chain (open
point 3, §11).

### 2.1 `needs.write` — writable areas (COW)

```yaml
needs:
  write:
    - path: /app/var/cache     # absolute in-image path, inside THIS slice's tree
      persistence: ephemeral   # ephemeral (DEFAULT) | retained
      when: [macos]            # OPTIONAL platform filter (spec 23 §2 form)
      why: "mnconvert writes its font cache at boot"   # MANDATORY
```

Rules (MECE, fail-closed):

- A write area is a PATH the slice will write, not a mechanism: COW is
  the only mechanism (the transforms law), so resolution always
  satisfies a write need with a `CowBackend` stack. There is no second
  spelling.
- The same path declared twice by one slice: the persistences must
  agree, else a named manifest error (spec 23 §2's conflict rule,
  extended).
- Ancestor/descendant write areas: the ancestor's persistence must be
  at least the descendant's (an ephemeral ancestor may not hide a
  retained need).
- A path appearing in BOTH `needs.write` and `needs.host` of one slice
  is a named manifest error — a path is VFS surface or host surface,
  never both.
- `persistence` is the slice's CONTRACT about its own data:
  `ephemeral` = any per-run scratch satisfies me; `retained` = my
  writes must survive the run, so an operator-bound durable store is
  mandatory (§3, §4). The operator may always bind an ephemeral need
  to a durable store (durability widens upward); a retained need may
  never be demoted to scratch — declarations request, the operator
  tightens (spec 08 §2's precedence).
- A data slice declares no `needs.write` (data is read-only by kind,
  spec 03 §2.2) — a data slice with one is a named manifest error,
  alongside spec 23 §6 step 1's rule.

### 2.2 `needs.decrypt` — key-binding requirements (ENC)

The FACT of encryption is already owned by IDENTITY
(`identity.encryption.state/parts` — paths, algorithm, `envelope_refs`;
NEVER keys; spec 03 §2.1, shipped in `crates/tpkg/src/manifest.rs`).
`needs.decrypt` carries the one bit the identity block cannot: WHICH
sealed parts the workload requires opened in order to run.

```yaml
needs:
  decrypt:
    - part: /fonts/licensed    # a path from identity.encryption.parts[].paths
      why: "the converter reads the licensed font tree"   # MANDATORY
```

Rules:

- `part` MUST name a path listed in (or a strict ancestor of paths
  listed in) `identity.encryption.parts[].paths` — cross-validated at
  manifest parse; a `decrypt` entry against an unsealed path is a
  named manifest error. SSOT: the identity block owns what is sealed;
  the need only references it.
- Default when `state: encrypted` and NO `needs.decrypt` block: ALL
  parts are required — fail-closed, exactly today's `EncBackend::new`
  contract (mount REQUIRES an opening grant;
  `crates/tfs/src/backends_enc.rs`).
- With entries: the listed parts are required; unlisted parts MAY stay
  sealed — the mount opens what the bound key opens
  (`KeySource::Recipient` against the in-image
  `/__tpkg__/envelopes.yaml`, `tpkg::EnvelopeManifest`), and a read of
  a sealed path answers `ENOKEY` (§5). This is spec 10 §2's
  per-audience model given a declarative spelling: one artifact, N
  audiences, each run opening exactly its declared subtree.
- A data slice is the ONE exception to spec 23 §6 step 1's
  no-needs rule: a sealed data slice may carry `needs.decrypt`
  (it has no host surface; its seal is its only declarable need). Any
  other needs block on a data slice stays a named manifest error.

## 3. D2 — the composition bindings

The composition document (spec 23 §3) gains two top-level keys; D5
maps 1:1 (`--overlay`, `--decrypt`, repeatable; flags win, spec 23
§3's precedence):

```yaml
# tebako.yaml (D2)
overlays:                     # COW backing-store bindings
  - slice: metanorma          # a slices[] name
    store: ./overlays/metanorma   # host dir; symbolic atoms expand (spec 23 §2)
decrypt:                      # ENC key bindings — REFERENCES, never material
  - slice: fonts-secure
    recipient: pgp:3c8dba971d2b4f01   # a key in $TEBAKO_HOME/keys/ (spec 09)
```

Rules:

- **Bindings satisfy needs; they never create capability.** An
  `overlays:`/`decrypt:` entry naming a slice with no matching
  declared need — D1, or a composition-level `needs.write` /
  `needs.decrypt` addition, D2 carrying the D1 grammar per spec 23
  §3 — is a named ORPHAN-BINDING error at resolution, never a silent
  grant. Giving an undeclared slice a write area takes both lines:
  the composition declares the need AND binds the store.
- One store per MOUNT: a slice's declared write areas share one
  `CowBackend` overlay; the write gate (§5) consults the declared
  path set. A second `overlays:` entry for one slice is a named
  error.
- The store directory is created when missing (the mount layer's
  discipline — `crates/tfs/src/mount.rs`'s mode plumbing) and gains a
  DERIVED rw identity grant in the effective policy — host=store,
  mount=store, rw — the same lowering `tfs exec --compose` applies to
  needs entries (`crates/tfs-cli/src/compose.rs`). Overlay stores are
  NOT the system self-surface (spec 23 §5's `$TEBAKO_HOME` is ro at
  bind): a store under the store root — including §4's ephemeral
  scratch — derives its grant like any other. Declared write access
  to host surface stays visible by construction.
- `recipient:` names a key REFERENCE — `pgp:<keyid>`, 16 lowercase
  hex (the manifest's keyid form, spec 03 §2.1) — resolved against
  `$TEBAKO_HOME/keys/`, the only secret-key home (spec 09). Key
  MATERIAL in any authored document (D1–D5) is a named validation
  error, not a grammar. The `pgp:` scheme rides spec 04's MECE
  reference axis (amendment in the implementation chain — room for
  future schemes, e.g. `pkcs11:`, without a grammar break).
- Trust posture (spec 23 §9, unchanged): overlay stores and key
  bindings are operator domain. A package signature covers slices and
  trailer, NOT the bindings; an overlay store's content is host
  state, outside every image's trust anchor — stated honestly.

## 4. Resolution (spec 23 §6 amendment)

The resolver (shim / bootstrap / `tebako run` / `press`) gains, IN
ORDER, between spec 23's steps 3 and 4:

- **3a. Overlay needs union**: every slice's `needs.write` /
  `needs.decrypt`, platform-filtered, conflict rules per §2. The
  runtime's release manifest unions identically (spec 23 §6 step 2) —
  the env image is a mount like any other; a runtime hosting a
  writable gem tree declares it here.
- **3b. Bind**: match D2 bindings to needs.
  - A write need WITH an `overlays:` binding: the store is checked
    NOW (creatable, writable) — a store that cannot be opened is a
    named resolution failure, never a mid-run EIO.
  - A write need with NO binding: `ephemeral` binds a per-run scratch
    under `$TEBAKO_HOME/tmp/overlays/<run-id>/` (the store's own tmp
    discipline — tmp + rename, cleaned at unmount), journaled at
    boot (`event=overlay mount=<mp> store=<dir> source=ephemeral`);
    `retained` is a named RESOLUTION FAILURE:
    `slice <name> needs a retained writable <path> (why: <why>) — no
    overlay store bound`.
  - A decrypt need WITH a `decrypt:` binding: the recipient key must
    EXIST in `$TEBAKO_HOME/keys/` and open an envelope covering the
    part — verified against the in-image `/__tpkg__/envelopes.yaml`
    at bind, never at first read. No binding, or a key that opens
    nothing required, is a named RESOLUTION FAILURE:
    `slice <name> needs key material opening <part> — no recipient
    bound` / `recipient pgp:<keyid> opens no required part of slice
    <name>`.
  - Orphan bindings: named error (§3).
- **3c.** spec 23 step 4's needs-check now covers overlay needs: a
  declared write or decrypt need the effective composition does not
  satisfy fails BEFORE EXEC with the named forms above. A need never
  surfaces as a mid-run EROFS/ENOKEY — the needs-check law, extended.
- **5′. Export** (extends spec 23 step 5): the bound overlay set
  serializes to `TEBAKO_OVERLAYS` (`<mount>=<store>` pairs,
  `;`-separated, split on the FIRST `=` — a store keeps its own
  `=`; drive-qualified windows stores keep their `:`) and
  `TEBAKO_DECRYPT` (`<mount>=pgp:<keyid>`). No key material
  crosses the channel — references resolve at the driver's mount. Spawned children inherit
  both alongside `TEBAKO_JAIL` and re-bind identically (spec 22
  class E; spec 08 §2.1's re-derivation discipline). Malformed env
  forms fail closed: exit 68 (§7). The parser is the one owner of
  both grammars (`crates/tfs/src/overlay_spec.rs`); the malformed
  forms, each a named error quoting the offending entry (SHIPPED):
  an empty spec; an empty entry (a stray `;`); a missing `=`; an
  empty or non-absolute mount (absolute = `/…` or drive-qualified
  `X:/…`); an empty or non-absolute store; a duplicate mount (one
  binding per mount, §3); a decrypt recipient that is not `pgp:` +
  16 lowercase hex. A store containing `;` is unrepresentable by
  construction — it splits into a second entry that fails the
  grammar; fail-closed, never a silent misparse. A mount containing
  `=` is unrepresentable the same way: the first-`=` split folds
  the surplus into the store side, which fails the grammar.

## 5. Run time: the write gate and the sealed read

- A mount carrying ≥1 write area stacks `CowBackend` per §1; the
  driver passes the store through the `_with_mode` mount family
  (`TEBAKO_MOUNT_COW`, spec 11 §7) with the declared area set —
  `mount::Overlay::gated(store, areas)` on the Rust mount API. The
  jail installs AFTER the mounts (spec 17) with the derived store
  grants in force. (The C ABI's `*_with_mode` family keeps the
  UNGATED programmatic form — a store and no areas, spec 11 §4
  unchanged; the gated form is the declarative surface and never
  crosses the C ABI.)
- The write gate (spec 11's `path_is_held` discipline) gains the
  declared-area set: writes under a declared write area land in the
  overlay; writes elsewhere in a held tree stay **EROFS** — the
  jail-safe default, unchanged, now JOURNALED:
  `event=vfs-deny op=write path=<p> mount=<mp>` (best-effort, the
  jail journal's discipline, spec 08 §4). An undeclared write is not
  a needs-check case (nothing was declared) and never silently
  succeeds. The predicate (locked, SHIPPED in
  `crates/tfs/src/backends_cow.rs`): areas are absolute in-image
  paths normalized at mount time (no leading or trailing `/`; the
  root area `/` covers the whole mount); a write to an area itself
  or any path BELOW it — component boundary, `/a/b` never covers
  `/a/bc` — is permitted; all four write verbs (`pwrite`,
  `truncate`, `mkdir`, `remove`) are gated identically; reads are
  never gated; the whiteout journal file keeps its `EPERM` under
  every area set. A malformed area (relative, an empty component,
  `.`/`..`) fails the mount with EINVAL — fail-closed, never a
  silent widening. The journaled denial covers the RO mount's EROFS
  on a held path and the gated COW mount's out-of-area EROFS alike
  (the write-open EROFS included — the fd write family stays the
  spec 11 §7 later milestone).
- A read of a sealed path outside every opened grant answers
  **ENOKEY** (126 — the named EKEY class owned by
  `crates/tfs/src/backends_enc.rs`), never garbage, journaled
  `event=vfs-deny op=read path=<p> mount=<mp> class=ekey`.
- A store-side IO failure during a write (ENOSPC, EACCES on the host
  dir) surfaces as the write syscall's own errno — the run degrades
  exactly like a full or read-only disk; no tebako-specific
  translation, stated plainly.

## 6. Record mode (spec 23 §8 amendment)

Discovery extends to the VFS write gate:

- Under `policy: record` EVERY image mount stacks an ephemeral
  scratch COW (store per §4's ephemeral rule) — spec 23 §8's
  "nothing is denied" promise now covers writes into held trees: the
  write lands in scratch and is journaled
  `event=vfs-write path=<p> mount=<mp>`. Scratch is discarded at
  exit; the payload observed a writable world it never owned.
- Deny-mode `vfs-deny` write lines and record-mode `vfs-write` lines
  both fold into `tfs needs --from-journal`
  (`crates/tfs/src/needs.rs`'s `needs_from_journal` — the engine's
  contract holds: aggregate, drop automatic surface, draft-only), now
  emitting a draft **`needs.write:`** block beside `needs.host:` —
  paths mount-relative, persistence `ephemeral` (the observed
  minimum), each with the `why:` TODO. The human gate is unchanged:
  the author flips persistence, prunes noise, fills `why`, merges
  into D1 or D2. The generator never edits a manifest.
- Sealed-read denials (`class=ekey`) fold into a draft
  `needs.decrypt:` entry naming the part prefix.

## 7. Errors and exit codes

Run-time errnos are existing and unchanged: EROFS (undeclared write),
ENOKEY (sealed read). Resolution-time failures are named errors; the
bootstrap/shim exit table (spec 06 §4) carries ONE added row —
**owner-signed-off 2026-08-15**; the code constant is
`tpkg::EX_TEBAKO_OVERLAY`:

| 68 | `EX_TEBAKO_OVERLAY` | overlay/decrypt binding failure: unbound retained store, missing or non-opening key material, unwritable store, orphan binding, malformed `TEBAKO_OVERLAYS` / `TEBAKO_DECRYPT` |

## 8. Post-bake swap (spec 23 §9, unchanged in shape)

Overlay and key bindings live in the composition layer — swappable by
construction:

- **Managed mode**: edit `tebako.yaml` / pass flags. Nothing to add.
- **Standalone packages**: the override document (argv `--compose` >
  `TEBAKO_COMPOSE` > sidecar) carries `overlays:`/`decrypt:` with the
  same grammar; D3 (spec 23 §4) bakes the resolved needs union and
  the press-time bindings as the DEFAULT composition. The reserved L2
  `mounts[].mode: cow` spelling (spec 03 §6) becomes live for exactly
  this; `enc` STAYS reserved — a sealed slice's fact is its own
  manifest's (§2.2), never the L2 row's.
- **`tfs exec --compose`** speaks the image-layer form: `image:
  <path>` where D2 says `slice: <name>` — slice names resolve at the
  shim layer, the MECE split `crates/tfs-cli/src/compose.rs` already
  enforces. Atoms expand at compose time; `--compose` combined with
  `--image`/`--jail` stays a named error (one composition source per
  run).
- **Audit**: the boot journal names what ran —
  `event=overlay mount=<mp> store=<dir> source=declared|ephemeral`
  and `event=decrypt mount=<mp> recipient=pgp:<keyid>
  grants=<opened-ids>` (the `EncBackend` grant report,
  `opened_grant_id`) — beside spec 23 §9's `event=composition`.
  Key MATERIAL never touches a journal (spec 11 §11's log
  discipline).

## 9. SSOT register (every cross-layer value, one owner)

| Value | Owner |
|---|---|
| `needs:` spelling and conflict rules | spec 23 §2 (extended here — one grammar, one semantics) |
| Write-gate semantics; whiteout journal (`.tfs-whiteouts`, v1) | `crates/tfs/src/backends_cow.rs` (+ spec 11 §10) |
| ENC construction (`tfsenc01`, HKDF labels); `ENOKEY` = 126 | `crates/tfs/src/backends_enc.rs` (+ spec 10) |
| Envelope manifest path and grammar | `ENVELOPES_BACKEND_PATH` + `tpkg::EnvelopeManifest` (spec 10 §7) |
| Mount-mode flags (`TEBAKO_MOUNT_*`), the `Overlay` plumbing (store + declared write areas) | `crates/tfs/src/mount.rs` (spec 11 §7) |
| The `TEBAKO_OVERLAYS` / `TEBAKO_DECRYPT` env grammars | `crates/tfs/src/overlay_spec.rs` (§4 5′) |
| Encryption FACTS (state/parts/envelope_refs) | `identity.encryption` (spec 03 §2.1) — `needs.decrypt` only references |
| Key home and trust layout (`keys/`, `trust/`, `tmp/`) | spec 09 + the store layout (§8 of the ecosystem charter) |
| The `pgp:<keyid>` reference spelling | spec 04's MECE reference axis (amendment in chain) |
| Exit codes | spec 06 §4 (the 68 row; code constant `tpkg::EX_TEBAKO_OVERLAY`) |
| Journal event vocabulary (`jail-*`, `vfs-*`, `overlay`, `decrypt`) | `crates/tfs/src/journal.rs` |
| Payload-manifest schema | `docs/spec/schemas/payload-manifest.yaml` → `schema/tpkg-manifest-v1.schema.json` (MINOR bump, spec 18 §3) |
| Composition-document schema | **`schema/tebako-compose-v1.schema.json`** — the D2 document's versioned JSON Schema, NAMED here; created by the schema pipeline (spec 18 §3.9), not part of this change |

## 10. Explicitly OUT

- **Backends never learn to write** (invariant 5, the transforms
  law): `TEBAKO_MOUNT_RW` stays `ENOTSUP` in-tree; no format backend
  gains a write path; dwarfs-t stays read-only + creation-time
  Writer.
- No auto-overlay outside record mode's scratch; no undeclared write
  ever succeeds; no namespace-global write layer — overlays are
  per-mount, declared, and resolvable at composition time.
- No key material in any authored document or env channel (D1–D5,
  `TEBAKO_*`) — references only.
- `Enc(COW(base))` and deeper programmatic stacks: expressible at the
  mount API, never a declarative spelling (§1).
- Overlay lifecycle verbs — checkpoint, squash-into-RO-layer, export
  (spec 11 §4's write-side lifecycle): tooling over a bound store;
  they need no declaration grammar and get none here.
- Metadata hiding stays spec 10 §2's deferred option.
- External `envelope_refs` resolution: the identity block permits
  externally-held grant envelopes, but no shipped spec pins where
  they live — this grammar binds recipients against the IN-IMAGE
  `/__tpkg__/envelopes.yaml` only (open point 2, §11).

## 11. Open points for the owner (pinned defaults in force until ruled)

1. **Exit code 68** — RESOLVED: signed off by the owner 2026-08-15.
   The spec 06 §4 table carries the row and `tpkg::EX_TEBAKO_OVERLAY`
   is the code constant. (Reusing 73 `EX_TEBAKO_JAIL` or 74
   `EX_TEBAKO_IO` was rejected — the jail and IO classes would blur a
   distinct failure family.)
2. **External envelope storage** (spec 03 §2.1's `envelope_refs`):
   undeclared by any shipped spec; v1 binds against in-image
   envelopes only.
3. **Grammar-doc ownership**: the `needs.write` / `needs.decrypt`
   schema text lands in `docs/spec/schemas/payload-manifest.yaml` in
   the implementation chain — to be coordinated with the in-flight
   spec-03/schemas work. This spec is normative for the semantics
   either way.

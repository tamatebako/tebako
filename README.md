# tebako-rs

The Rust workspace for the tebako stack: the `tebako_fs_*` C ABI
(libtfs-rs), the `tpkg` package-trailer format, and — in later milestones —
the package tooling, CLI and bootstrap. Repo strategy and naming are
**locked** in the tamatebako restructure plan (`TODO.restructure/21` —
repo map + archival gates — with the design in items 19 (libtfs-rs) and
22 (bootstrap)).

## Workspace map (locked)

```
tebako-rs/
  crates/
    tpkg/              # the manifest format — single source of truth
    tfs/               # libtfs-rs: the tebako_fs_* ABI (cdylib libtfs.so/
                       # .dylib/.dll + staticlib libtfs.a — a drop-in for
                       # C++ libtfs). Mount table, fd/dir tables, path
                       # dispatch, errno channel, backends
    tebako-pkg/        # PLANNED — tpkg trailer surgery only (bundle/
                       # unbundle/reassemble/insert-image/remove-image/
                       # set-runtime/info; item 25)
    tfs-cli/           # PLANNED — the tfs binary (generic VFS ops; item 25)
    libtfs-preload/    # the preload interposition shim (spec 07 §8 tier 1;
                       # roadmap 30) — cdylib injected via
                       # DYLD_INSERT_LIBRARIES / LD_PRELOAD; plus the safe
                       # TEBAKO_TFS_MOUNTS spec parser shared with tfs exec
    tebako-cli/        # the tebako packager CLI: lean/fat press, cache
                       # (item 17)
    tebako-http/       # in-process HTTPS downloads (ureq+rustls,
                       # webpki-roots bundled) shared by the CLI and the
                       # bootstrap
    tebako-term/       # terminal progress (spec 06 §5): TTY detect,
                       # hand-rolled ANSI bar/spinner — zero deps
    tebako-bootstrap/  # the Rust bootstrap runner (item 22,
                       # < 3 MB static, size-gated)
  tests/contract/      # the parity oracle: the C++ libtfs c_api suite
                       # running through the Rust C ABI
```

Naming is locked: crates are `tfs`, `tebako-pkg`, `tebako-cli`,
`tebako-bootstrap`, `tpkg` ("-rs" belongs on repos, not crates). The DwarFS
binding is NOT in this workspace: it lives in its own repo
[`tamatebako/dwarfs-t-rs`](https://github.com/tamatebako/dwarfs-t-rs) and is
consumed as an ordinary external dependency (git/path dep early, crates.io
when published). dwarfs-t stays C++ forever; `libdwarfs_c` is the only
Rust-consumable surface for it.

## Status

### SHIPPED (roadmap 30)

- **libtfs-preload + `tfs exec`: the preload interposition shim (spec 07
  §8 tier 1)** — the mainline native-exec mechanism for DYNAMIC binaries:
  `crates/libtfs-preload` (a cdylib, `libtfs_preload.{dylib,so}`)
  interposes the libc file-IO family (open/openat/stat/lstat/fstat/
  access/faccessat/opendir/readdir(+readdir64)/closedir/pread/read/lseek/
  close/mkdir/unlink/rename + dlopen) via `DYLD_INSERT_LIBRARIES` (macOS
  `__DATA,__interpose`) and `LD_PRELOAD` (linux-gnu symbol interposition).
  On init the shim mounts `TEBAKO_TFS_MOUNTS` (`image:mount,…`) through
  the tfs crate in-process and installs `TEBAKO_JAIL` (the spec 08 §1 env
  form: `open|deny`, docker-style `host:mount:ro|rw` grants, `@` argument
  files — parsed in `tfs::policy::JailSpec`, shared with the launcher).
  Memfs paths are served by the engine; host paths pass through gated by
  the SAME `host_policy` (deny → EPERM, ro-grant writes → EROFS); memfs
  libraries dlopen via the `dlmap2file` host cache; unsafe is confined to
  the interpose/dlsym module. The `tfs exec <image>[:mount] [--image
  <image:mount>]... [--jail <spec>] -- <cmd> [args...]` launcher (tfs-cli)
  sets the preload env on the CHILD ONLY, materializes an in-image
  entrypoint through `dlmap2file` (execve needs a host path), and execs
  with stdio inherited — children inherit the env, so the process tree
  stays in the VFS (SIP platform binaries strip `DYLD_*` and leave it).
  The shim is located next to the `tfs` binary (`TEBAKO_TFS_PRELOAD`
  overrides).
  Misformatted `TEBAKO_TFS_MOUNTS`/`TEBAKO_JAIL` is a named error (clear
  stderr + exit 78). macOS and linux-gnu first-class; windows is phase 2
  (DLL injection). v1 limits (stated in the crate docs): execve of memfs
  paths not virtualized (children re-spawn via `argv[0]`), no
  fstatat/statx/getdents64/openat2/__xstat interposition, non-UTF-8 paths
  always pass through, a mount at `/` is refused (jail bypass). E2E
  proofs (macOS native + linux): in-image tool reads data with no
  extraction, deny jail fails /etc reads EPERM while memfs stays
  readable, a grandchild stays in the VFS, dlopen of a memfs
  .dylib/.so works; the proof tools skip cleanly without a `cc`.

### SHIPPED (milestone 10)

- **tpkg: typed extension blocks + the L2 package manifest (spec 02 §5b /
  spec 03 §6)** — the container's OCI-style L2 home: `[u32be type][u32be
  length][payload]` blocks walked forward from the end of the slot table,
  the v2 signing extension keeping its historical tail position (type 1 is
  RESERVED, never reframed — it predates the block mechanism and delimits
  from the tail). `crates/tpkg` owns the codec (`parse_ext_blocks` /
  `encode_ext_blocks` free walkers, `Manifest::ext_block` /
  `insert_ext_block` / `remove_ext_block`, `validate_strict`): readers
  skip unknown types and carry them verbatim (rewrites preserve what they
  do not understand), strict validation rejects them with a named error,
  and v1/v2 trailers without blocks stay byte-identical (the C golden
  vectors are untouched and green). Type 2 is the `PackageManifest`
  (authored YAML, `schema/tpkg-package-manifest-v1.schema.json`,
  schema-cross-checked): package identity, per-command entries with
  per-entry `runtime_ref` (no 128-byte cap — suites, multi-runtime),
  package-level jail + env. Blocks sit INSIDE the canonical signed
  region — the v2 signature covers them. `crates/tebako-pkg`:
  `bundle --package-manifest <file.yaml>` embeds the block, `info --full`
  prints the package section when present (default `info` is parity-
  untouched), and insert-image / remove-image / set-runtime carry the
  block through the atomic rewrite path (signing state preserved as
  before). `crates/tebako-bootstrap`: when the type-2 block is present,
  `entries[0].runtime_ref` drives runtime resolution (block-less packages
  fall back to the v1 trailer field byte-identically); the handoff argv
  is unchanged — launcher ABI stays 1. Remaining (suite press, out of
  scope here): unbundle/reassemble representation of extension blocks
  (unbundle warns it drops them), `--slot N` / `--json` / `--verify`
  info modes (spec 15), dispatcher-side per-entry suite dispatch
  (spec 07).

- **The info surface (spec 15)** — payload and package introspection in
  the two MECE front-ends, sharing the new **`crates/tebako-info`**
  engine (no third tool; default outputs keep C++ byte-parity, every
  richer view is an explicit flag):
  - `tfs info` (tfs-cli): `--manifest` (the parsed model re-serialized as
    YAML), `--provides` / `--requires` / `--platforms` (kind-specialized
    sections; edges as `kind:name:constraint → mount`; triplet ↔
    release-asset-name mapping via tpkg's `Platform`), `--json` (one
    document, `"info_schema": 1`), `--backend-json` (the pre-spec-15
    `--json`: backend metadata), `--verify` / `--require-signed` (spec 03
    checks with exit codes 0/65/70/71/72). Derived block: shims from
    entrypoints, runtime compatibility vs `~/.tebako/runtimes`
    (satisfied-by / requires-download / incompatible), dependency names
    (1 level). A manifest-less image is a named note, not an error.
  - `tebako-pkg info`: `--full` (the §3 container report — package line,
    size/trailer/bootstrap bytes, runtime_ref, trust section (stored
    state labeled `unverified` until `--verify` runs) and the per-slot
    table with in-place mount-from-region payload reads), `--slot N`,
    `--json`, `--verify`, `--depth 0|1|2`, and **`tebako-pkg validate`**
    (the strict standalone form: tpkg structural → per-slot sha256 (v2) →
    signature → per-slot manifest schema → digest agreement). Formats are
    auto-detected per slot (dwarfs-t FlatBuffers vs upstream Thrift read
    off the schema-section marker in the backend's own metadata JSON;
    squashfs, zip, tar) — `format_id` stays a hint, and 4 is reported as
    `runtime (legacy role)`.
  - `tebako cache list --json`: cached runtimes and payloads with their
    trust anchors, origins and sizes (additive; the version banner moves
    to stderr so stdout is the document alone).
  The tar backend's offset-0 region mounts got their index-pass seek
  fixed along the way (`crates/tfs`, regression test included).

### SHIPPED (roadmap 25)

- **Native-extension deploy in `crates/tebako-cli`** (the last M7
  deviation closed): a port of the reference gem's RuntimeSdk +
  SDK-enabled deploy driver (tebako-chainwt `lib/tebako/runtime_sdk.rb`,
  `runtime_deployer.rb`). Whenever deploy ops run on POSIX — the gem has
  no flag and none was added — the press provisions the runtime SDK into
  the packaging environment (`<prefix>/deps/sdk/...`, never the runtime
  cache): the pre-patched ruby src release the runtime was built from
  (`tamatebako/ruby`, sha256-verified against the release's SHA256SUMS,
  `TEBAKO_SDK_SRC_RELEASE`/`TEBAKO_SDK_SRC_MIRROR` knobs, `file://`
  mirrors for offline flows) is extracted **in-process** (flate2+tar, no
  tar binary), the runtime's own configure arguments are replayed from
  its rbconfig — read from the **mounted runtime image** through the tfs
  C ABI for image-era runtimes (no `--tebako-extract`, no `layout/` in
  the cache; v1-era keeps the extracted-layout flow) with build-machine
  paths filtered out — to generate the matching header tree, and a
  symbol-stub archive re-declares every ruby-ABI symbol the runtime
  executable exports (mkmf link probes get true yes/no resolution; the
  shipped extension never links it). The deploy driver's RbConfig
  overrides point rubyhdrdir/rubyarchhdrdir/LIBRUBYARG at the SDK and
  re-resolve the recorded toolchain on the press host (the gem's
  cc_override); the driver's script mode now emulates the ruby command
  line (-r/-I/-e/--) for the mkmf spawns. Built extensions land at their
  gem-correct paths inside the app image; the image carries the runtime's
  own memfs-relative rbconfig, so they resolve at runtime. Named errors
  throughout: 135 (missing headers / configure / nm / stub / platform
  mismatch), 122 (src download), 125 (SDK lock), and a failed build
  surfaces the driver's full output. e2e: `tests/fixtures/native-ext-app`
  (a path gem with a one-function C extension) presses against an
  image-era runtime pair and the packaged binary prints
  `toyext.answer = 42` from the memfs.
  **Documented deviations**: the SDK cache key's host tag is the tebako
  platform id (`macos-arm64`; the gem derives it from the press host's
  ruby RbConfig — `darwin24-arm64` — which a hostless CLI cannot
  reproduce); the src tarball is extracted in-process (the gem shells
  out to `tar`); nm/cc/ar and the downloaded `./configure` run as
  processes exactly like the gem (a native build is impossible without a
  C toolchain — the same reliance the deploy toolchain fallback table
  already carries); the gem/gemspec scenarios stay unported (130).

### SHIPPED (milestone 9)

- **tfs: tar/tar.gz/tar.zst backend (pure Rust, read-only)** — roadmap 13's
  tar adapter (`backends_tar.rs`): a mount-time offset index built in ONE
  streaming pass (ustar/GNU-longname/pax via the `tar` crate), directories
  synthesized from entry paths, hard links resolved at lookup, GNU sparse
  files stat-only (pread → ENOTSUP), duplicate names last-wins. Detection
  follows spec 11 §3: strong magic first, the tar header-checksum heuristic
  LAST (gzip/zstd envelopes claim tar since it is the only gzip/zstd
  payload). Random access: plain tar does positioned reads (1 GiB archive,
  512 scattered preads: **+1.6 MiB peak RSS vs the 64 MiB budget**);
  tar.gz resumes from cloned miniz_oxide `InflateState` checkpoints every
  16 MiB of uncompressed stream (~0.2 % memory, the zran pattern, no C
  zlib); tar.zst uses a forward-only ruzstd cursor (cold backward seeks
  re-decode from the start — documented cost model, no state snapshots
  exist in pure Rust). Memory profile documented in the module docs.

- **tfs: COW overlay + mount modes (spec 11 §3/§4, the transforms law)** —
  `CowBackend { base, overlay }` (`backends_cow.rs`) stacks any image
  backend over `HostDirBackend` (`backends_hostdir.rs`, a host directory
  exposed as a TFS backend — independently useful, disposable by
  deletion). Reads fall through to the base unless shadowed; writes and
  deletes land in the overlay only (base files copy up on first write);
  deletes record whiteouts in `.tfs-whiteouts` inside the overlay (strict
  v1 text format, atomic rewrites — the complete delete-side audit
  delta). Whiteouts mask base entries only: an overlay entry of the same
  name always wins (upper-replaces-whiteout, overlayfs semantics). The
  base image is byte-identical after unmount (proven in tests). Mount
  modes wire through every mount entry point: `TEBAKO_MOUNT_RO` (0,
  default — writes EROFS, behavior unchanged), `TEBAKO_MOUNT_COW` (1,
  overlay dir created when missing), `TEBAKO_MOUNT_RW` (2, honestly
  ENOTSUP — no in-tree backend writes in place). Additive ABI:
  `tebako_fs_mount_from_{file,file_at,memory}_with_mode`; the legacy
  entry points delegate as RO. Path-level writes route through the
  context (`pwrite_path`/`truncate_path`/`mkdir_path`/`remove_path`);
  the fd-based write family (spec 11 §7) stays PLANNED. Write support
  lives ONLY in the composite layer — format backends never learn to
  write (spec 00, invariant 5).

### PARTIAL (roadmap 07)

- **`crates/tebako-resolve` — references, fetch, payload cache,
  registries** (spec 04/05): the MECE reference parser (`tfs:github:` /
  `tfs:gitlab:` / `tfs:bb:` / `tfs+git://` / `tfs+https://` / bare
  `https://` / `file://`, `?sha256=<hex>` pin on any class, `#artifact`
  on service releases — the locked multi-artifact rule: with `#` exactly
  that asset, without it one/zero/many `.tfs` candidates → used /
  `AssetNotFound` / `AmbiguousAssets`, never host-triplet guessing; no
  default service, unknown forms are a named error listing the classes;
  proptest round-trip + never-panic), a fetcher over an injected
  `Transport` (tebako-http in production, mocks in tests; `tfs+git:` via
  gitoxide — never the git CLI — with GitHub/GitLab/Bitbucket release-API
  adapters behind one trait), the shared
  `~/.tebako/payloads/<name>/<version>.tfs` cache (per-entry flock 120 s,
  tmp+rename, 0444 + `.sha256` trust anchor + `.origin` markers,
  `TEBAKO_OFFLINE` hard error, digest mismatch caches nothing), and the
  **`tpkg-registry.yaml` model + resolution** (spec 04 §2: default-branch
  contents API, pinned release-artifact, git-blob and `file://` forms —
  exactly one location per form; serde model with parse-then-validate;
  declarative `universal` / `platforms[triplet]` host selection via
  tpkg's `Platform`). The workspace JSON parser moved to
  **`crates/tebako-json`** (re-exported by tebako-pkg — API unchanged) so
  the release adapters stay free of native deps.
- **`tebako add-registry | list-registries | install | uninstall`**
  (spec 04 §2, spec 16 §3.3, roadmap 28 item 1): `add-registry` validates
  + fetches the registry once and records it in `~/.tebako/config.yaml`
  `registries:` (other keys preserved); `install` takes a direct ref
  (resolve → fetch → verify → cache → register shims) or a nickname
  searched across the REGISTERED registries only (zero hits → a named
  error listing them plus the hint; several → `AmbiguousRegistries`;
  version omitted → registry default; `@ver` → exactly that) with
  declarative host-triplet selection from the registry entry. Registry
  sha256 pins are enforced at install; entries carrying
  `signature: {keyid, asc}` are OpenPGP-verified before anything enters
  the cache (strict; untrusted → exit 72, invalid → 71), unsigned
  entries take the spec 09 v1-legacy warn + audit-journal line
  (`TEBAKO_REQUIRE_SIGNED=1` hard-fails). The manifest mirror comes from
  the image's embedded manifest when present (tier 1, cross-checked
  against the registry) and is synthesized from the registry's tier-3
  fields otherwise (loud note, `/<command>` path convention); shims link
  through tebako-shim's library API (no spawn). `uninstall` removes
  shims + cache entry and journals the trust anchors.
- **The install-UX stack (spec 04 §2, 03 §6, 16 §5; roadmap 33/40/34/41)**:
  the shim's registry-default chain link resolves **every registry form**
  at dispatch behind the per-ref cache
  `~/.tebako/registries/<sha256-of-ref>.yaml` (24 h TTL, `tebako
  update-registries`, `TEBAKO_OFFLINE` = cache-or-named-error; `tebako
  add-registry` primes it, `tebako-shim doctor` reports freshness). The
  dispatch mirror IS the unified `tpkg::PayloadManifest` (40) — per-entry
  runtime requirements included, which makes **suites** (34) work end to
  end: `tebako press --suite <suite.yaml>` (per-entry imaging + slots +
  the type-2 package manifest with per-entry runtime_refs), the
  bootstrap's argv0 entry selection (exact match, entries[0] fallback),
  N shims per installed suite each on its own runtime (two commands of
  one package run different runtime versions simultaneously), and
  `tebako install` registers every suite shim. **`tebako publish`** (41):
  accept per-triplet payloads → optional `--sign[=<keyid>]` (per-artifact
  `<artifact>.asc`) → upload to the referenced GitHub release in-process
  (no gh CLI; `--upload-mirror` for file:// rehearsal) →
  `tpkg-registry.yaml` upsert (idempotent re-publish) → `--tap` formula
  render from the vendored template → built-in clean-cache `tebako
  install` proof. Remaining: the GitLab/Bitbucket write legs, the brew
  tap repo + install.sh channels.

### SHIPPED (milestone 8)

- **Runtime-as-image, consumer side (item 30b)**: the runtime splits into
  interpreter + `.tfs` image, and the launcher ABI learns the image era
  ([docs/runtime-as-image.md](docs/runtime-as-image.md) — the exact
  trailer/manifest/runtime_ref semantics). **`crates/tebako-bootstrap`**:
  a runtime_ref carrying the bare **`;image` flag**
  (`ruby@<rv>;tebako=<ver>[;image][;sha256=<hex>]`) resolves the
  `<asset>.tfs` alongside the executable — download via tebako-http
  (manifest `image` key primary, SHA256SUMS line fallback), sha256
  verified against the release index, installed **read-only with
  `<image>.sha256`/`<image>.origin` trusted markers** into the same cache
  entry, never extracted — and exports `TEBAKO_RUNTIME_IMAGE=<path>` to
  the runtime (the v1 handoff is byte-identical; refs without `;image`
  change nothing). **`crates/tebako-cli`**: the index parsers learn the
  additive image entries; press resolves the image into the cache
  (bootstrap interop), seeds the packaging environment by extracting the
  image **in-process through the tfs C ABI** (no `layout/` tree in the
  cache, no `--tebako-extract` for image-era releases), and emits the
  `;image` runtime_ref when the release carries an image entry (v1-era
  refs stay byte-identical, golden parity intact). Proven end-to-end:
  agent-123's published images (mirror mode) and a build-matched pair
  manufactured from the official 0.15.9 runtime both press and cold-run
  (the cache holds interpreter + image + markers only), and the current
  driver mounts the standalone image and executes its stub with **zero
  driver change**; the image-era driver patch
  ([docs/tebako-main.cpp.30b.patch](docs/tebako-main.cpp.30b.patch) —
  prefer `TEBAKO_RUNTIME_IMAGE` over the incbin image) is upstream's
  only change, needed only for runtimes that stop embedding.
- **Fixture note**: the locally-built 0.15.9 runtime in
  `tebako-runtime-ruby/runtime-packages/` is not the released binary
  (sha differs; its boot fails default-gem loading even from its own
  image — `Gem.home` undefined, bundler's ProcessLock lands on the
  read-only memfs). The gemfile scenario of the image-era e2e therefore
  runs against the build-matched official pair; the simple scenario and
  all mount/resolution proofs run against agent-123's pair.

### SHIPPED (milestone 7)

- **`crates/tebako-cli` — the `tebako` packager CLI** (item 17's
  SELF-HOSTING design): a port of the reference gem's lean/fat press
  (tebako-chainwt `lib/tebako`). `tebako press -r <root> -e <entry>
  [-o pkg] [-p prefix] [-c cwd] [-R ruby] [-m lean|fat] [--image
  path:mount]...` resolves the prebuilt runtime into the shared cache
  (gem-identical `~/.tebako` layout, flock'd installs, manifest.json /
  SHA256SUMS index, TEBAKO_OFFLINE / mirror env vars, error codes
  120–125), seeds the packaging environment from the runtime's extracted
  layout, deploys the application **under the runtime itself** (the
  stub-driver image stitched onto an empty base and exec'd via
  `--tebako-image`, with the RUBYOPT/RUBYLIB/BUNDLE_* env scrub), images
  the app and stitches a three-part package (LEAN flag, launcher ABI 1,
  `ruby@<rv>;tebako=<v>` runtime_ref, fat mode adds the FORMAT_RUNTIME
  payload slot + `;sha256=`). Scenarios: simple script and Gemfile
  (lockfile-pinned bundler per `BUNDLED WITH`; Gemfile `ruby` directive
  honored). `tebako cache list` / `cache prune [--all] [--older-than Nd]`
  manage the runtime cache. **Everything in-process (owner rule)**: all
  downloads via `crates/tebako-http` (ureq + rustls, webpki-roots
  bundled, HTTPS-only, `file://` mirrors, OS trust opt-in via
  `TEBAKO_TLS_PLATFORM_ROOTS` — no curl anywhere) and all images via the
  dwarfs-t `Writer` binding (dwarfs-t-rs) — no mkdwarfs binary, no PATH
  lookup; produced images carry dwarfs-t-native FlatBuffers metadata and
  are named `.tfs` (`fs.tfs`, `deploy-driver.tfs`; `.dwarfs` stays for
  upstream-compatible images). tfs-cli's `mkimage` moved to the same
  Writer. **Golden parity vs the gem**: same fixture, same prefix — the
  press stdout and the packaged binaries' output are byte-identical (CI
  leg checks out `tamatebako/tebako` feat/ci-slim-model and diffs
  side-by-side; the RuntimeSdk provisioning lines and the two sides'
  image-build lines are filtered — see the crate README).
  **Documented deviations**: the bootstrap portion defaults to the
  in-workspace Rust tebako-bootstrap (`--bootstrap`/`TEBAKO_BOOTSTRAP`
  override; otherwise the C++ release is resolved like the gem does); the
  gem/gemspec scenarios, classic mode, `.tebako.yml`, and the gem's
  unconditional 5 s press pause are not ported (the pause is kept only
  when a warning is actually printed); images stitch densely (tpkg slots
  carry absolute offsets; the gem's 8-byte padding is cosmetic). (The
  RuntimeSdk/src-release subsystem this milestone deferred landed in
  roadmap 25, above.)

### SHIPPED (milestone 6)

- **`crates/tebako-bootstrap` — the Rust bootstrap runner** (item 22): a
  port of the C99 `tebako-bootstrap.c` v0.2.0 contract. Trailer parse
  (crates/tpkg), launcher-ABI check (v1), runtime_ref parse
  (`type@version;tebako=<abi>[;sha256=<hex>]`), shared-cache hit, **fat**
  payload extraction from its own executable (SHA256-verified against the
  `;sha256=` parameter), **lean** download from the release mirror
  (manifest.json primary, SHA256SUMS.txt fallback; `TEBAKO_OFFLINE` cache
  mode; `TEBAKO_RUNTIME_MIRROR`), per-entry install lock (flock, 120 s
  timeout with the stale-lock hint), atomic install (tmp + rename,
  sha256/origin metadata), and the launcher-ABI-v1 exec handoff
  (`--tebako-image <self>:<slot>:<mount>` … `--tebako-entry <argv0> <user
  args…>`; runtime payload slots never handed over; `--tebako-extract` is
  a runtime-side option that rides the passthrough like the C++). Named
  exit codes 65/66/67/69/70/74 with the C++ message bodies. The ten
  `self-test.sh` scenarios ported as integration tests, plus a direct
  parity run of the same fixtures against the C++ oracle binary
  (`TEBAKO_CPP_BOOTSTRAP`).
- **SIZE — the 3 MB gate (item 22, hard gate; budget raised 2 MB → 3 MB
  by the owner when the download path went in-process)**: release profile
  `opt-level="z", lto="fat", codegen-units=1, panic="abort",
  strip="symbols"`; no async runtime, no clap. Measured:

  | platform | Rust bootstrap | C++ v0.2.0 | budget |
  |---|---|---|---|
  | macOS arm64 | **1,842,144 B (1.76 MB)** | 53,536 B | < 3 MB ✔ (1.6× under) |

  (CI publishes the per-platform artifact sizes on every run and fails
  at ≥ 3 MB.)
- **Progress UX (spec 06 §5, locked)**: a fetch shows the work and the
  benefit, on stderr always (stdout stays the payload's). Phases:
  `resolving <ref>` → `downloading <asset> (<size>)` with the live bar →
  `verifying sha256` → `installing (locked)` → done — and the done line
  states the benefit: `installed <name> (<size>) — cached at <path> and
  shared by every tebako app on this machine`. A cache hit prints one
  quiet `runtime <ref> (cached)` line. Full rendering only when stderr is
  a TTY and `TERM != dumb` (opt-outs `NO_COLOR`, `TEBAKO_NO_PROGRESS=1`);
  otherwise exactly the start + done single lines, CI/log-safe. The bar
  is hand-rolled ANSI throttled to ≤ 10 redraws/s (unknown content-length
  → spinner + byte count) in the zero-dependency `crates/tebako-term`
  micro-crate — the size gate forbids indicatif-class crates — fed
  transport-accurate byte counts via tebako-http's
  `on_progress(so_far, content_length)` hook. Size cost in the artifact:
  **+16.6 KB** (1,825,520 B → 1,842,144 B on macOS arm64).

  ```text
  downloading tebako-runtime-0.15.9-3.4.2-linux-gnu-x86_64 (23.0 MB)
  [=====>    ] 62%  14.2/23.0 MB  3.1 MB/s
  verifying sha256
  installing (locked)
  installed ruby-3.4.2-0.15.9-linux-gnu-x86_64 (23.0 MB) — cached at ~/.tebako/runtimes/ruby-3.4.2-0.15.9-linux-gnu-x86_64 and shared by every tebako app on this machine
  ```
- **HTTP/TLS choice (owner rule: no curl anywhere)**: all downloads are
  in-process via `crates/tebako-http` — ureq + rustls (ring) with
  webpki-roots **bundled** (the OS trust store is opt-in via
  `TEBAKO_TLS_PLATFORM_ROOTS`), HTTPS-only, redirects ≤ 5, `file://`
  mirrors. Size cost in the artifact: **≈ +867 KB** vs the previous
  curl-CLI path (371,776 B → 1,238,384 B on macOS arm64) — the owner
  extended the budget 2 MB → 3 MB to pay for it. The C++ bootstrap
  shells out to curl; the Rust side no longer does anywhere.
  Windows exec/lock ports land with the Windows CI leg (v1 ships
  macOS/Linux like the rest of the matrix).

### SHIPPED (milestone 5)

- **`crates/tfs-cli` — the `tfs` binary, the generic VFS image tool** (item
  25's tfs : libtfs :: sqlite3 : libsqlite3; the image-operation half of
  the C++ tebakofs — trailer surgery stays in tebako-pkg). Exact C++
  output/exit-code/stream-split parity, verified against the tebakofs
  oracle: `info` (plain-archive summary; the spec-15 surface of milestone
  10 added `--manifest`/`--provides`/`--requires`/`--platforms`/`--json`/
  `--backend-json`/`--verify`), `ls`/`ls -r`/`ls -l`,
  `tree`, `cat` (pread-chunked, no full materialization), `stat`,
  `extract` (whole-archive via `tebako_fs_extract_all` with mtime
  preservation + selected paths with the stderr warning split), `find`
  (fnmatch), `mkimage` (shells out to mkdwarfs via `TEBAKO_MKDWARFS`/PATH
  — **the dwarfs WRITER is deliberately not bound; binding it is a
  separate future decision**, like the tebako-pkg golden leg CI downloads
  the released mkdwarfs/tebakofs binaries per platform).
  Contract discovery folded in: the zip backend is **explicit-entry-only**
  through the C ABI (implicit parents of deeper entries list nothing and
  ENOENT — verified 1:1 against the oracle), and ZIP DOS timestamps are
  interpreted in the local timezone (libzip semantics).

### SHIPPED (milestone 4)

- **`crates/tebako-pkg` — the package (tpkg) trailer surgery CLI** (item 25's
  scoping: TPKG TRAILER operations ONLY — generic image ops belong to the
  future tfs-cli). Subcommands with the exact C++ tebakofs semantics
  (flags, output formats, exit codes, stderr bodies):
  - `info <archive>` — trailer dump (slots/offsets/mounts/lean/abi/
    runtime_ref) or plain-archive summary via the tfs C ABI
  - `bundle --bootstrap <exe> --image <img[:mountpoint]>... -o <file>
    [--runtime-ref <ref>] [--lean] [--launcher-abi <n>]`
  - `unbundle <binary> -o <dir>` (bootstrap.bin + image-N.bin +
    manifest.json with per-part crc32)
  - `reassemble <dir> -o <file>`
  - `insert-image <binary> <img[:mountpoint]>` /
    `remove-image <binary> <slot>` /
    `set-runtime <binary> <runtime-file>` (atomic in-place rewrites)
  Built on `crates/tpkg` (trailer byte-parity; a streaming `tpkg::Crc32`
  was added for the part checksums) and `crates/tfs` (the info fallback).
  **Golden parity vs the C++ tool (proven byte-for-byte)**: bundle output,
  info output, manifest.json, unbundled parts, reassemble round-trip,
  insert/remove/set-runtime rewrites — all byte-identical to tebakofs
  (libtfs main). Golden tests auto-detect the oracle (`TEBAKOFS_CPP`, the
  libtfs build tree, or PATH) and skip without it; the round-trip and CLI
  suites are oracle-free.

### SHIPPED (milestone 3)

- **tfs: SquashFS backend via `crates/sqfs-sys`** — hand-written FFI to
  libsquashfs (squashfs-tools-ng), same discipline as dwarfs-t-sys: ~15
  externs pinned by `abi_check.c` (`_Static_assert`s over struct
  sizes/offsets/constants), a tiny `shim.c` keeping the variable-layout
  `sqfs_compressor_config_t` C-side, and a Rust-side memory-backed
  `sqfs_file_t` (mirrors the C++ one). Installed via vcpkg with the SAME
  overlay port libtfs uses (squashfs-tools-ng 1.3.2, gzip/lz4/xz/zstd
  decompressors) — version parity on both sides of the gate. Feature flag
  `vendored-squashfs` (default on). squashfs-tools-ng is POSIX/autotools
  only (no Windows — same restriction as the C++ side).
- **Contract parity audit** ([docs/parity.md](docs/parity.md)): every
  C-ABI-visible C++ test group ported (CApiTest 79/79, CApiMultiMountTest
  18/18, CApiOffsetTest 11/11, abi version 1/1, squashfs 20 through the
  ABI, extraction 11), the remainder consciously not ported with reasons
  (C++-internal types, path utils, tebako-pkg binary functionality,
  perf/large fixtures).
- **Extraction parity**: `tebako_fs_extract_all` now preserves mtime as
  well as permissions (best effort); ported the C++ `ExtractionTest`
  mappable cases incl. sqfs permissions/empty-dir and spaced paths.
- **Contract suite: 164 tests** (64 zip + 18 multi-mount + 22 io-surface
  + 12 dwarfs + 20 squashfs + 9 extraction + 1 C harness + 17 tpkg
  + 1 unit).

### SHIPPED (milestone 2)

- **tfs: dwarfs backend via the external
  [`dwarfs-rs`](https://github.com/tamatebako/dwarfs-t-rs) crate** (crates
  `dwarfs-t`/`dwarfs-t-sys`; currently a *path* dependency — becomes a git
  dependency when dwarfs-rs has a release, crates.io later). Mount from
  file / memory / region (offset+length — the C++ semantics: dwarfs
  regions open in place, zip regions read into memory), stat, pread,
  directory iteration. The milestone-1 ENOTSUP cases are now real: dwarfs
  images mount with backend name "DwarFS" and pass the ported
  `CApiOffsetTest` suite; squashfs stays ENOTSUP. Feature flag:
  `vendored-dwarfs` (default on; `--no-default-features` keeps the build
  pure-cargo with dwarfs mounts failing ENOTSUP).
- **tfs: multi-mount** — `tebako_mount_t` handles (monotonic, never
  reused), `tebako_fs_mount_from_file` / `_from_file_at` / `_from_memory`,
  `tebako_fs_unmount_handle` (force-closes only the mount's own fds/dirs →
  EBADF, releases the mount point), longest-prefix dispatch (nested mounts
  shadow outer ones), legacy `init*` single-mount semantics on top (EEXIST
  when anything is mounted, `tebako_fs_unmount()` tears down everything),
  extract rules (1 mount → dest root; N mounts → per-mount
  mount-point-basename subtrees).
- **tfs: remaining io surface** — `tebako_fs_dir_is_embedded`,
  `rewinddir`, `telldir`/`seekdir` (index-based cookies),
  `tebako_fs_extract_all`, `tebako_fs_dlmap2file` (per-process tmpdir,
  memfs-path→host cache keyed by full path, caller-`free()`d returned
  string, teardown removal at exit), and `tebako_fs_abi_version()`
  (== `TEBAKO_FS_ABI_VERSION` = 1, byte-for-byte with libtfs main
  0f3e444).
- **Contract suite: 116 ported cases** — 64 zip (M1) + 18 multi-mount +
  22 io-surface + 12 dwarfs offset/backend, plus the plain-C harness.

### SHIPPED (milestone 1)

- **`crates/tpkg` — complete.** Parse / serialize / validate / crc32 for
  the tpkg v1 trailer, byte-exact with the reference C implementation
  (`include/tebako/tpkg.h` in libtfs): golden vectors generated by that C
  implementation (field offsets asserted), absent-vs-corrupt
  discrimination (`TEBA` prefix → `NoTrailer`, broken magic → `Magic`,
  bad crc → `Crc`), bounds/version/slot-count/validation errors, and
  proptest round-trips (encode→parse identity, parser never panics on
  garbage). Error codes and strings are 1:1 with `TPKG_ERR_*` /
  `tpkg_strerror()`.
  *cbindgen note:* the generated C `tpkg.h` target lands with
  `crates/tebako-bootstrap` (item 22), the first real C-side consumer.
- **`crates/tfs` — the `tebako_fs_*` C ABI core.** Thread-local errno
  channel, `FsContext` mount table + fd/dir tables, longest-prefix
  dispatch, `Backend` trait, ZIP backend (pure-Rust [`zip`] crate — the
  ABI contract is behavioral, not library-level; no native libzip).
  Exported symbols: exactly the `tebako_*` functions (nm-verified; nothing
  else leaks).

### PLANNED (next milestones, in order)

1. **Chain-of-trust operations (item 29, remaining)**: the release key
   ceremony (create the production root, fill
   `EMBEDDED_ROOT_FINGERPRINT`), fingerprint publication on tebako.org,
   CI secrets wiring for the release signing step, the revocation drill
   (documented procedure + rehearsed successor statement), and the Ruby
   `scripts/upload_release.rb` integration invoking `tebako-pkg sign`.
2. crates.io publication of `tpkg`/`tfs` (after the API settles
   post-parity); cbindgen `tpkg.h` with it.
3. tebako-bootstrap Windows exec/lock port + windows CI leg (v1 shipped
   macOS/Linux); tfs-cli interactive shell + serve/exec modes (later).
4. tebako-cli: classic press mode, gem/gemspec scenarios, the
   RuntimeSdk/src-release subsystem (native-extension deploy), .tebako.yml.

### v2 notes (recorded decisions)

- **Root of trust (item 29 phase 2, SHIPPED mechanics)**: one OpenPGP
  keypair signs the tamatebako releases. **Creation/storage**: the root
  key is generated offline, passphrase-protected, held on hardware
  (YubiKey/HSM recommended); an armored, passphrase-protected export lives
  in CI secrets for the release job's signing step, and only the public
  half is distributed. **Publication**: the root fingerprint is published
  on tebako.org and embedded in the bootstrap at release time
  (`EMBEDDED_ROOT_FINGERPRINT`, empty until the ceremony);
  `TEBAKO_TRUSTED_ROOT` (fingerprint, or path to the root public key)
  overrides it in development. **Rotation**: a signed *successor-key
  statement* (`TEBAKO-ROOT-SUCCESSOR-V1` — four canonical lines binding
  predecessor → successor fingerprint, detached OpenPGP signature from
  the predecessor) lets any machine forward trust with no out-of-band
  step: the bootstrap applies the statement chain from a trusted root and
  TOFU-registers the successor's public key. **Release tooling**:
  `tebako-pkg sign` (detached `.asc` per artifact + signed `SHA256SUMS`;
  the Ruby `scripts/upload_release.rb` in the tebako repo will invoke it
  later) and `tebako-pkg verify` for consumers (Trusted/Untrusted/Invalid
  per artifact).
- **Signing and encryption are OPTIONAL, per package** (owner directive).
  `tebako-pkg bundle` produces an unsigned (v1, byte-identical to
  pre-signing) package unless `--sign[=keyid]` is given: `--sign` uses the
  press-local key (generated and cached under `$TEBAKO_HOME/keys` on
  first explicit use, auto-registered in the local trusted keyring),
  `--sign=<keyid>` selects a secret key from `$TEBAKO_HOME/keys`. Rewrite
  operations (insert-image/remove-image/set-runtime/reassemble) preserve
  the input's signing state. **Only the presence of a signature is
  optional — verification of signed packages is always strict** (v2-signed
  → full OpenPGP verify against the trusted keyring + per-slot SHA-256,
  named exit codes; unsigned v1 → legacy acceptance with a loud warning
  + journal; `TEBAKO_REQUIRE_SIGNED=1` hard-fails unsigned). Encryption
  (item 28, when it lands) is likewise per-image opt-in by design.
- **Byte paths**: v1 validates path arguments as UTF-8 at the FFI boundary
  (non-UTF-8 → `EINVAL`). Ruby integration will need byte paths (`OsStr`
  on POSIX — ruby paths are raw bytes, not guaranteed UTF-8). The v2 C API
  revision is where this lands; the errno surface does not change.
- **ZIP mtime**: interpreted as UTC (ZIP DOS timestamps carry no zone);
  libzip (C++ libtfs) uses process-local time. The contract never compares
  mtimes; documented in `backends_zip.rs`.

## Parity-oracle approach

`tebako_fs_*` and `tpkg` are **byte-level contracts** (locked). Tests come
from the C++ side and run unchanged against the Rust implementation:

- C ABI: the C++ `tests/test_c_api.cpp` cases are ported against the same
  fixtures with the same expectations (errno values included); the full
  493-test suite follows the same pattern (item 19 acceptance gate).
- tpkg: golden vectors are bytes produced by the reference C
  implementation, asserted offset-by-offset; a mismatch means the contract
  moved and both sides must move together.
- The dwarfs backend binds `dwarfs_c_*` via `dwarfs-rs`; never a
  workspace-local dwarfs-sys.

## Building

```console
$ export DWARFS_RS_VCPKG_ROOT=/path/to/vcpkg   # dwarfs + squashfs backends
$ cargo test --workspace    # tpkg + tfs + contract (incl. the C harness)

$ cargo test -p tfs --no-default-features   # pure-cargo, no vcpkg
```

The native backends come from vcpkg: dwarfs-t via dwarfs-rs's CMake
submodule build (env `DWARFS_RS_VCPKG_ROOT`) and squashfs-tools-ng via
sqfs-sys's manifest+overlay port (env `SQFS_SYS_VCPKG_ROOT`, falling back
to `DWARFS_RS_VCPKG_ROOT` / `VCPKG_ROOT`). First cold builds compile the
native dep chains; the vcpkg archive cache makes later builds fast.
Without default features the workspace is pure-cargo. CI: ubuntu-24.04 +
macos-14 with the dwarfs-t vcpkg baseline pinned plus an archive cache
keyed on both manifests, and a separate `--no-default-features` leg.

## Releasing (item 31)

Tags `v*` run [.github/workflows/release.yml](.github/workflows/release.yml)
(the parsanol-rs shape: prepare → native runners per platform → upload):
each builds the four binaries with the release profile (vendored
dwarfs-t/sqfs via vcpkg — no vcpkg in the consumer path), strips them,
and publishes the size table; `tebako-bootstrap` is hard-gated at
**< 3 MB** (item 22's extended budget).

- **Artifacts** per platform, raw binaries named
  `<tool>-<version>-<platform>`: `tebako-bootstrap` (the lean/fat
  launcher), `tfs` (the VFS image tool), `tebako-pkg` (trailer surgery),
  `tebako` (the packager CLI).
- **Platforms**: `macos-arm64`, `macos-x86_64`, `linux-gnu-x86_64`,
  `linux-gnu-arm64` (native runners, including `ubuntu-24.04-arm`),
  `linux-musl-x86_64`, `linux-musl-arm64` (Alpine containers via
  `docker run` from the host — node actions cannot run on musl; vcpkg
  bootstrapped from source with the musl overlay triplets, the proven
  native-musl path — see `ci/musl-build.sh`). Windows (ucrt64) is a
  disabled leg with the blockers documented in the workflow
  (dwarfs-t-sys's windows state, sqfs POSIX-only, the bootstrap's
  exec/lock ports).
- **Integrity**: `SHA256SUMS` over every asset from day one; detached
  OpenPGP signatures slot into the finalize job with item 29 phase 2's
  tooling (no restructuring — unsigned is a supported state).
- **manifest.json** in exactly the shape the gem's BootstrapManager and
  tebako-cli's resolution consume today (`assets[]` per platform for the
  bootstrap, plus an additive `tools` map for the other binaries).
- The finalize job carries the item-11 completeness gate: the release
  fails unless every expected asset (4 × platforms + sums + manifest)
  actually landed.

**Consuming the Rust bootstrap today** (before the repo switch flips):
```console
$ export TEBAKO_BOOTSTRAP_MIRROR=https://github.com/tamatebako/tebako-rs/releases/download
$ export TEBAKO_BOOTSTRAP_VERSION=<tag-minus-v>
$ tebako press ...   # the gem (or tebako-cli) resolves our tebako-bootstrap
```
tebako-cli itself needs no installer: download `tebako-<version>-<platform>`
onto PATH (a pure-Ruby installer shim for the `gem install tebako` habit
is the gem-retirement path, item 31's note).

## License

BSD-2-Clause (same as the tebako C++ project). Note the DwarFS backend
links dwarfs-t / DwarFS, which is GPL-3.0 — see the dwarfs-rs README for
the plain statement of what that means for binaries.

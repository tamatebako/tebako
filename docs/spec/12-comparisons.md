# Spec 12 — Tebako vs other technologies

Positioning analysis: what tebako is for the **user** who runs deployed
code and the **developer** who ships it; why its security, integrity, and
reproducibility model is categorically different; and where other
technologies genuinely win. Honest throughout — but complete.

## 1. The claim: package once, run anywhere — FOR REAL

"Run anywhere" has been promised before. It always came with asterisks:

- **Java**: run anywhere *a compatible JVM is already installed, with the
  right major version, and the host's native libs behave*. The environment
  was never part of the artifact.
- **Electron**: the runtime IS shipped — per app, per update, hundreds of
  megabytes duplicated on every machine, with no sharing and no integrity
  model beyond the store/channel.
- **AppImage**: one file, but Linux only, no shared runtimes, no jail, no
  signatures, no version management.
- **Docker/OCI**: reproducible *inside a Linux ABI*, behind an engine,
  built for servers — never a desktop/CLI citizen, and "docker desktop"
  on macOS/Windows is a hidden VM.
- **snap/flatpak**: needs a system daemon and (practically) a store;
  confinement varies with the host kernel; per-app bundled runtimes.
- **Homebrew / apt / dnf**: the environment is whatever the repo serves
  *today* — installs are not reproducible across time or across machines,
  and everything mutates the system.

Tebako removes the asterisks by making three properties true at once —
no other system has all three:

1. **Environment fidelity.** The runtime the code runs on is a
   digest-pinned, content-addressed artifact — the *same bytes* the
   developer tested against, not "whatever ruby happens to be installed".
   The interpreter, its stdlib, its gems, and the app's files are one
   sealed tree, identical on every machine.
2. **Zero install, zero mutation.** One file. Run it. Nothing in `/usr`,
   no daemon, no store account, no admin rights, no PATH pollution beyond
   one optional shims dir. Uninstall = delete files.
3. **Real cross-platform.** Per-triplet static executables for the
   loader, and — for pure-language apps — ONE universal payload that runs
   on all of them. Linux gnu/musl, macOS, Windows: same command, same
   behavior, no engine, no VM, no compatibility layer.

## 2. The user's world

A person who wants to *run* metanorma (or any tebako app):

- **Downloads one file and runs it.** No "first install ruby 3.3 via
  rbenv, then bundle install, then fix the native extension that won't
  compile on your macOS version". That entire genre of documentation
  ceases to exist.
- **First run fetches the runtime once per machine** — verified by
  digest — and every other tebako app reuses it. The second app downloads
  *nothing*. This is the Electron/npm duplication problem solved without
  re-creating the Homebrew/apt global-version-lock problem: runtimes are
  shared AND multiple versions coexist, because they are content
  addressed, not system installed.
- **The artifact is self-authenticating.** A signed tebako package
  verifies offline against a published root fingerprint — over any
  transport: a release page, a mirror, a USB stick, an email attachment.
  Trust lives in the *object*, not in the channel or the store
  (spec 09). Nobody else in application distribution does this: every
  store model authenticates the *pipeline*, and a file that leaves the
  pipeline is just a file.
- **Running strangers' software becomes safe by default.** The user can
  run any payload in a tight jail — one input file visible, nothing else
  (spec 08) — with no daemon, no root, no kernel module. No other app
  distribution format gives the *end user* declarative per-run
  confinement as a first-class feature.
- **Encrypted apps are runnable secrets.** A publisher can ship a payload
  only *your* key opens — per-subtree, with in-memory-only plaintext and
  crypto-shredding revocation (spec 10). Licensed/proprietary tooling
  distribution without a license server.
- **Versions follow the user, not the machine.** Per-project
  `.tebako-tools.yaml` pins, per-user defaults, env overrides (spec 07).
  Entering a project directory means running the right version — the
  "works on my machine" class of failure ends for users too.
- **Fully offline after first use** (`TEBAKO_OFFLINE`); air-gapped sites
  use `file://` mirrors. The artifact never phones home because there is
  no home to phone — registries are plain git-host releases.

## 3. The developer's world

A person *shipping* an application:

- **Package once.** The payload image is built one time per variant: a
  pure-language app is ONE universal payload for every OS; a
  platform-triplet-bound payload builds in a matrix — but each leg is ONE
  mechanical press step (never a porting effort), and per-platform
  handling (platform-specific content, flags, entrypoints) is a
  first-class feature of the manifest, not a hack.
- **Runtimes are optional, not assumed.** A payload whose entrypoint is
  native needs no interpreter at all — tebako serves runtime-free slices
  (the inkscape case) exactly as it serves ruby apps. Runtime payloads
  (ruby today; python, julia, others tomorrow) plug into the same
  provides/requires graph when they ARE needed.
- **No per-distro packaging, ever again.** No deb, rpm, AUR, formula,
  cask, snapcraft.yaml, flatpak manifest, MSI, dmg, pkg, store review
  queue, or per-store policy surprise. One artifact type, one flow:
  press → sign → upload to *your own* releases.
- **Distribution sovereignty.** The registry is a YAML file in the
  developer's own git host (spec 04). No central store: no gatekeepers,
  no fees, no takedown risk, no telemetry you didn't choose.
- **Dependencies stop existing for users.** An app that needs `inkscape`
  declares it; the dispatcher resolves a payload that PROVIDES it and
  mounts it into the app's namespace (spec 03). The user's system is
  never touched, and version conflicts with system packages are
  impossible by construction — the app sees only its own composed
  filesystem.
- **Small updates stay small.** The app payload ships alone; the runtime
  is referenced, not bundled. An Electron-class "update the app, re-ship
  a browser" event is a few-megabyte image here.
- **One package, many commands.** Suites: multiple entrypoints in one
  package, each with its own runtime pin (spec 07) — no other packaging
  system can express that.
- **Native-extension honesty.** ABI-line constraints are declared in the
  manifest; a wrong-line runtime produces a named error, never a
  segfault in production.
- **Reproducible support.** Payload digest + runtime digest fully
  determines the user's environment. "Send me the two hashes" replaces
  "what OS/ruby/gem versions do you have".
- **Opt-in trust without ceremony.** One flag signs; the chain of trust
  and rotation machinery (spec 09) does the rest. Encryption is there
  when the business model needs it — per-customer subtrees in a single
  artifact.

## 4. Security, integrity, reproducibility — the structural difference

Most distribution systems secure a *channel* (TLS to a store, signed repo
metadata on a server you must trust today and forever). Tebako secures
the *object*:

- **Three independent layers.** Transport integrity (SHA-256 vs the
  release index), object authenticity (OpenPGP signature over the
  canonical trailer bytes, verified offline against an embedded root),
  and runtime enforcement (per-slot digests verified before mount;
  fail-closed named errors, spec 06/09). Compromising any single layer
  does not compromise the system.
- **Trust that survives redistribution.** Because the signature is on the
  artifact, mirrors are untrusted infrastructure. CDNs, caches, USB
  drives — all safe to use, because verification never depends on them.
- **Confidentiality with granularity.** Encryption is per-subtree with
  hierarchical keys (spec 10): one artifact, many audiences; revocation
  by crypto-shredding; plaintext exists only in locked memory. No store,
  repo, or container registry offers object-level confidentiality at
  subtree granularity.
- **Reproducibility as a closed function of bytes.** The deployment is
  fully determined by verified, content-addressed artifacts: payload
  digest + runtime digest + manifest (integrity-bound inside the image).
  Nothing depends on host state, repo state, or time. apt/dnf/Homebrew
  cannot say this (installs float with repo state); gems cannot (native
  builds differ per host); Docker approximates it (image digests) but
  only inside one ABI behind an engine; snaps approximate it per confine-
  ment profile of the host kernel. Tebako says it across four OS families
  with no infrastructure.
- **Honesty about what it is not.** Jails are VFS-level policy, not a
  hardware security boundary — hostile *native* code belongs in a VM.
  Tebako is not a multi-process orchestrator (Docker's home) and not an
  OS package manager (apt/dnf's home). The claims above are about
  *application distribution and execution*, and there they are unique.

## 5. Gatekeeper and Authenticode: the OS trust gates vs tebako

Gatekeeper (macOS) and Authenticode (Windows) are the two OS-level trust
systems users actually rely on. They answer one question — *"may this
binary run?"* — and tebako answers a different one — *"is every part of
what runs what it claims to be?"* The systems are complementary layers,
not competitors; this section shows where tebako defers to them, and where
it strictly exceeds them.

| axis | Gatekeeper (macOS) | Authenticode (Windows) | tebako |
|------|--------------------|------------------------|--------|
| object verified | the app bundle / binary | the binary / driver | every slice + every manifest + every runtime + every payload + registry items + the sums manifest — and partial re-stacking stays verifiable (per-slot digests) |
| when verified | at **every execution**, by the OS | at install/UAC + SmartScreen heuristics | at fetch, at install, **at mount** (per-slot digest before attach), at publish (author side) |
| anchor of trust | Apple-issued Developer ID cert (vendor CA) | CA-issued cert (EV for reputation) | embedded tamatebako root key (self-sovereign) + user trust store + successor-chain rotation |
| unsigned path | block or warn (policy) | warn via SmartScreen reputation | unverified-first with loud warning + audit journal; strict mode `TEBAKO_REQUIRE_SIGNED=1` fails closed |
| revocation | CRL/OCSP (online, coarse) | CRL/OCSP (online, coarse) | signed successor statements (offline, forward-verifiable) + per-key registration/removal + policy |
| offline / air-gap | verifies offline; notarizing needs online at build time | mostly offline; timestamping needs online at sign time | **full offline first-class**: embedded root + pinned keys + signed sums + air-gap bundles |
| scope beyond executables | none (binaries only) | none (binaries only) | runtimes, data slices, toolkits, suites, registries, manifests, indexes — the whole object graph |
| cross-platform | macOS only | Windows only | one model, four OS families + musl |
| provenance | developer identity only | publisher identity only | identity + builder + source ref + payload tree hash + `contract_version` + reproducibility path (roadmap 56) |
| key rotation | cert renewal via the CA | cert renewal via the CA | self-describing: old root signs its successor; clients forward-verify **offline** |
| enforcement level | OS syscall gate (the app cannot bypass it) | OS gate + kernel driver signing | loader-level — honestly: a patched loader is beyond us, which is exactly where the OS gates protect *us* (below) |
| ubiquity | every Mac, nothing to install | every Windows box, nothing to install | anywhere our single binary lands — zero host prerequisites |

### Where tebako defers (and integrates)

- **Continuous execution-time enforcement.** Gatekeeper's syscall-level
  gate and SmartScreen's reputation network cannot be replicated by an
  application-level loader, and a patched loader is outside any loader's
  reach. The tebako answer is the D08 tooling: sign tebako-produced
  binaries so the OS gates protect *our* root anchor continuously — their
  enforcement guards our bootstrap, our verification guards everything
  below it.
- **Ubiquity and notarization review.** Apple/Microsoft are already on
  the machine, and Apple's notarization includes actual scanning. We do
  not imitate this; we slot into it.

### Where tebako strictly exceeds them

- **Granularity.** They see a binary; tebako sees every slice inside it.
  Swap one slice and only its slot needs re-verifying — a whole-app
  signature cannot express that.
- **Scope.** Runtimes, data slices, toolkits, suites, registries and
  their indexes all ride the same signed-image algebra. The OS gates
  have no concept of any of these.
- **Sovereignty.** No corporate CA in the loop: tamatebako's root is
  embedded, third parties register in the user's trust store, and root
  rotation is a self-describing signed chain verifiable offline.
- **Offline/air-gap.** Verification never touches the network — not at
  fetch, not at mount, not at rotation.
- **Provenance.** Builder, source ref, payload tree hash, contract
  version, and a reproducibility path — compared to "Developer ID:
  Company X".

## 6. The matrix

| axis | tebako | rubygems | rbenv/rvm | Homebrew | apt/dnf | snap/flatpak | AppImage | Docker/OCI | VMs |
|------|--------|----------|-----------|----------|---------|--------------|----------|------------|-----|
| unit of distribution | payload image (.tfs) or stitched binary | gem | interpreter version | formula/bottle | package | snap/app | one file | image + registry | machine image |
| install footprint | one binary, or binary + shared cache | per-interpreter tree | per-version tree | system-wide | system-wide | daemon + mounts | one file per app | daemon + store | hypervisor + GBs |
| runtime sharing across apps | YES — machine cache, one download for all apps | per interpreter | n/a | global lock | global lock | partial (content snaps) | none (duplicated) | layer sharing | none |
| startup cost | process exec (~ms, in-process mount) | process exec | shim exec | process exec | process exec | snapd setup (slow cold) | process exec (FUSE mount) | container create | seconds–minutes |
| isolation | declarative VFS jails, ro/rw, in-process | none | none | none | none | AppArmor sandbox | none (opt. firejail) | namespaces+cgroups (daemon) | strongest (hardware) |
| object authenticity | opt-in OpenPGP on the artifact + per-slot digests + signed index | gem signing (rare) | none | same-channel hashes | repo metadata | store assertions | none standard | digests + cosign | n/a |
| object confidentiality | encrypted volumes, per-subtree keys, in-memory-only plaintext | none | none | none | none | none | none | none | coarse disk encryption |
| reproducibility | closed function of verified bytes, across OSes | floats (native builds) | floats | floats with repo state | floats with repo state | varies by host kernel | good (static) | good within Linux ABI | good |
| offline | full after cache warm (`TEBAKO_OFFLINE`) | after install | after install | after install | after install | after install | always | after pull | after create |
| cross-platform | static binary per triplet; universal payloads | any ruby host | per-platform build | per-OS | per-distro | snapd-centric | Linux only | Linux ABI (VM elsewhere) | any (heavy) |
| transparency to user | total — the app need not know tebako exists | needs ruby+gems | needs version mgr | needs brew | needs root/distro | needs snapd | good | needs engine | needs hypervisor |
| version management | per-project pins, per-entrypoint runtime pins | per-gem | yes | no | no | channels | none | tags | snapshots |
| system mutation | ~/.tebako + one PATH entry | gem home | ~/.rbenv | /usr/local | full system | snapd + mounts | none | daemon + groups | hypervisor |
| distribution sovereignty | developer's own git releases = the registry | rubygems.org (central) | n/a | tap repos (central-ish) | distro-controlled | store-controlled | self-host yes | registry (self-hostable) | n/a |

## 7. Where each alternative genuinely wins

- **rubygems**: dev-library sharing. Tebako packages *applications*; gems
  remain how developers share *code*.
- **rbenv/rvm**: hacking on ruby itself from source.
- **Homebrew / apt / dnf**: OS-integrated components and shared system
  libraries with distro security-update reach. Tebako deliberately never
  manages the OS.
- **snap/flatpak**: store discoverability and daemon-managed auto-updates
  (paid for with the daemon, the store model, and platform coverage).
- **Docker/OCI**: server-side multi-process orchestration and its
  ecosystem. Tebako is single-executable distribution, not a container
  platform.
- **VMs**: hard security boundaries for hostile native code.

## 8. Capabilities that exist nowhere else

1. A single file that is simultaneously the app, its own verifier, and
   its own runtime resolver — authenticating itself offline over any
   transport.
2. Universal payloads + digest-pinned shared runtimes: Electron's
   fidelity with a shared runtime's footprint, and neither's lock-in.
3. Per-entrypoint runtime pins inside one package (suites).
4. Recursive payload composition across independently published images,
   with consumer-declared mount points and a provides/requires capability
   graph (an app mounting `inkscape` without the user's system ever
   knowing).
5. Declarative per-run jails for the end user — no daemon, no root, no
   kernel module.
6. Object-level encryption with per-subtree audiences, hierarchical
   keys, crypto-shredding, and in-memory-only plaintext — in an
   application distribution format.
7. A packaging system whose every artifact (sources, runtimes, payloads,
   packages) is produced by its own factory repos and consumed through
   one coherent signed-image algebra — the same `.tfs` at every level.

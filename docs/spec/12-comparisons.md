# Spec 12 — Tebako vs other technologies

Positioning analysis: what tebako replaces, complements, and deliberately
concedes. Honest — where another technology genuinely wins, it says so.

## The comparison matrix

| axis | tebako | rubygems | rbenv/rvm | Homebrew | apt/dnf | snap/flatpak | AppImage | Docker/OCI | VMs |
|------|--------|----------|-----------|----------|---------|--------------|----------|------------|-----|
| unit of distribution | payload image (.tfs) or stitched binary | gem | interpreter version | formula/bottle | package | snap/app | one file | image + registry | machine image |
| install footprint | one binary, or binary + shared cache | per-interpreter gem tree | per-version tree | system-wide | system-wide | system daemon + squashfs mounts | one file per app | daemon + image store | hypervisor + GBs |
| runtime sharing across apps | YES — machine-wide cache, one download serves all apps | per interpreter (duplicated across versions) | n/a | shared system libs (version-locked globally) | shared (version-locked globally) | bundled per snap (content snap sharing partial) | bundled per app (duplicated) | layer sharing (registry-local) | none |
| startup cost | process exec (~ms; memfs mount in-process) | process exec | shim exec | process exec | process exec | snapd confinement setup (slow cold start) | process exec (squashfs mount via FUSE) | container create (100ms–s) | seconds–minutes |
| isolation | declarative VFS jails (spec 08), ro/rw binds, in-process | none | none | none | none | confinement (AppArmor), sandbox per snap | none (optionally firejail) | namespaces + cgroups (strong, daemon-mediated) | strongest (hardware) |
| integrity/authenticity | opt-in OpenPGP chain of trust + per-slot SHA-256 + signed index (spec 09); opt-in encryption (spec 10) | gem signing (rarely used, no chain) | none | bottle hashes (same-channel) | repo signatures (archive-level) | snap assertions (store-mediated) | none standard | content digests + signatures (cosign) | n/a |
| confidentiality | encrypted volumes, selective per-subtree disclosure, in-memory-only plaintext (spec 10) | none | none | none | none | none | none | none (registry private repos) | disk encryption (coarse) |
| offline behavior | fully offline after cache warm (TEBAKO_OFFLINE) | after install | after install | after install | after install | after install (refresh needs net) | always | after pull | after create |
| cross-platform | one static binary per triplet; universal payloads run everywhere | any ruby host | per-platform compile | per-OS (linuxbrew partial) | per-distro | per-distro snapd (ubuntu-centric) | linux only | linux containers (mac/win via VM) | any (heavy) |
| transparency to end user | total — a tebako app need not know tebako exists | requires ruby + gem env | requires version manager | requires brew | requires root/distro | requires snapd + store account concepts | good (chmod +x) | requires docker engine | requires hypervisor |
| version management | built-in dispatch: per-project pins, per-entrypoint runtime pins (spec 07) | per-gem versions | yes (its whole job) | no (one system version) | no | channels | none | tags | snapshots |
| system mutation | none beyond ~/.tebako + one PATH entry | gem home | ~/.rbenv + shims | /usr/local or /opt | full system | snapd + mounts | none | daemon + groups | hypervisor |

## Where each alternative genuinely wins

- **rubygems**: library distribution for development. Tebako packages
  applications, not dev libraries — gems remain how developers SHARE CODE;
  tebako is how users RUN it.
- **rbenv/rvm**: developer version switching against system ruby builds.
  tebako-shim supersedes the *user-facing* case (pinned, isolated,
  integrity-checked runtimes) but developers hacking on ruby itself still
  want source builds.
- **Homebrew/apt/dnf**: deep OS integration and shared system libraries
  with security-update reach. Tebako deliberately does not manage the OS.
- **snap/flatpak**: store discoverability + daemon-mediated auto-updates
  (at the cost of the daemon, the store model, and platform coverage).
- **Docker/OCI**: server-side multi-process orchestration and the
  ecosystem around it. Tebako targets single-executable desktop/CLI
  distribution — it is not a container orchestrator.
- **VMs**: strongest isolation. Tebako's jails are VFS-level policy, not
  a security boundary against hostile native code — documented honestly.

## The one-paragraph positioning

Tebako is the only system where an application ships as ONE file (or a
file + content-addressed shared cache), runs identically across
linux-gnu/linux-musl/macOS/Windows, mounts its own recursive payload
filesystem in-process with zero privileges, declares exactly what host
access it needs (and runs jailed to it), and carries opt-in
authentication, signatures, and encryption in the artifact itself — with
runtimes downloaded once per machine and shared by every app, and
version/runtime selection managed per command, per project, or per user.

# Tebako Architecture

The normative architecture of the tebako ecosystem lives in the
**specification set**: [docs/spec/](spec/00-INDEX.md) — indexed, layered,
and per-topic. Start there.

- What tebako is (packaging + loading ecosystem, any runtime / platform /
  payload / composition): [spec 01](spec/01-overview.md)
- The tpkg container (byte-exact, authenticated/signed):
  [spec 02](spec/02-tpkg-wire-format.md)
- Payload manifests (IDENTITY / PROVIDES / DEPENDS):
  [spec 03](spec/03-payload-manifest.md)
- References, registries, resolution, cache:
  [spec 04](spec/04-references-and-registry.md),
  [spec 05](spec/05-resolution-and-cache.md)
- Launcher ABI and exit codes: [spec 06](spec/06-launcher-abi.md)
- Shims and version dispatch: [spec 07](spec/07-shims-and-dispatch.md)
- Jails: [spec 08](spec/08-jails.md)
- Trust/signing and encryption: [spec 09](spec/09-trust-and-signing.md),
  [spec 10](spec/10-encryption.md)
- The TFS virtual filesystem: [spec 11](spec/11-tfs-vfs-model.md)
- Comparisons with rubygems/rbenv/Homebrew/apt-dnf/snap/flatpak/AppImage/
  Docker-OCI/VMs: [spec 12](spec/12-comparisons.md)
- Factories and release pipelines: [spec 13](spec/13-factories-and-releases.md)
- Engineering process: [spec 14](spec/14-process.md)

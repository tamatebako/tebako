# The executable slice

An executable slice is an application packaged as a single mountable
image: the code, its dependencies, and a manifest describing what the
image offers and what it needs. Metanorma is the flagship example;
fontist is a smaller one; a non-interpreted tool such as Inkscape can be
packaged the same way, with no language runtime at all.

## The manifest

Every slice carries a manifest file inside the image
(`/__tpkg__/manifest.yaml`). It records:

- **Identity** — the application's name and version, who produced it,
  and a content hash of the whole tree, so the image can be verified
  and addressed precisely.
- **Provides** — the commands the application offers. One slice may
  offer several; each can even request its own runtime version, so two
  commands from one package may run on different runtimes.
- **Requires** — what the application needs: a language runtime with a
  version range, and any other slices it wants mounted (for example a
  data slice at a specific path).
- **Platform** — whether the image is universal (pure language) or tied
  to specific platforms (anything containing compiled extensions).

The manifest is the authoritative description. Registries and local
stores only mirror it; they never override it.

## Platform coverage

A pure-language application produces one universal image that runs
everywhere a compatible runtime exists. An application with compiled
extensions produces one image per platform, and the install machinery
selects the right variant for the user's machine declaratively — from
the registry's platform table, not by probing.

## Lifecycle

1. **Press** — the application's source tree is compressed into an
   image and the manifest is stamped in.
2. **Publish** — the image goes to a registry, or is stacked directly
   into a fat package.
3. **Install** — on the user's machine the image is downloaded,
   verified (its signature if it has one, its checksum always), and
   stored once, shared by anything that needs it.
4. **Run** — the image is mounted read-only and its entrypoint is
   started under the resolved runtime. Inside a jail, the application
   sees only the paths it was granted.
5. **Upgrade** — a new version is a new image stored next to the old
   one. Nothing is edited in place; rollback is keeping the old
   version around.

## Relationship to runtimes

The executable slice never hard-codes a runtime version. It declares a
range, the resolver picks the newest compatible runtime on the machine
(or downloads it), and the application runs against that. Changing
runtimes — for performance, or to test a newer Ruby — requires no
rebuild of the application.

## Implementation

Pressing and publishing: `crates/tebako-cli`. The manifest format:
`crates/tpkg`. Resolution and storage: `crates/tebako-resolve` and
`crates/tebako-shim`. Mounting at run time: the `crates/tfs` engine
inside the runtime.

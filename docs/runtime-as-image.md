# Runtime as image — the launcher-ABI extension (item 30b)

Status: implemented in `crates/tebako-bootstrap` (resolution + handoff)
and `crates/tebako-cli` (press-side emission + layout). The upstream
driver patch is in tebako-chainwt `src/tebako-main.cpp` (see "Driver
change" below).

## The split

The prebuilt runtime becomes TWO artifacts (30a produces both):

- the **interpreter** — `tebako-runtime-<ver>-<ruby>-<platform>[.exe]`,
  the existing runtime executable (unchanged resolution, unchanged
  handoff);
- the **runtime image** — `tebako-runtime-<ver>-<ruby>-<platform>.tfs`,
  the runtime's files (lib/ruby, gems, /local/stub.rb; /bin empty) as one
  dwarfs-t-native (FlatBuffers) image. Immutable, sha256-verified,
  mounted — never extracted into the cache.

## runtime_ref: the `;image` flag

```
ruby@<ruby-version>;tebako=<tebako-version>[;image][;sha256=<64 hex>]
```

- `;image` (bare flag): the runtime is image-era — resolve the `.tfs`
  alongside the executable. The image's expected sha256 comes from the
  release index (manifest.json `image` key, else the SHA256SUMS line) —
  exactly the trust source the executable's own checksum already uses.
- `;sha256=` keeps its existing meaning (fat payload checksum) and is
  unaffected.
- A bare flag (not `;image=<sha>`) keeps image-era **fat** refs inside
  the 127-byte runtime_ref budget: `ruby@3.3.7;tebako=0.15.9;image;
  sha256=<64>` fits; a second 64-hex parameter would not.
- launcher_abi stays **1**: the handoff options are unchanged, the env
  is additive, and refs without `;image` behave **byte-identically** to
  v1 (no image lookup, no download, no env — today's flow untouched).

tebako-cli's press emits `;image` when the resolved release index
carries an image entry for the runtime; otherwise the ref is the v1 form
(golden parity with the gem preserved).

## Cache layout (shared with the executable entry)

```
runtimes/ruby-<rv>-<ver>-<platform>/
  tebako-runtime-<ver>-<rv>-<platform>[.exe]   # interpreter (0755)
  sha256 / origin                              # executable metadata
  tebako-runtime-<ver>-<rv>-<platform>.tfs     # runtime image (0444, immutable)
  tebako-runtime-<...>.tfs.sha256              # trusted marker: "<sha>  <file>\n"
  tebako-runtime-<...>.tfs.origin              # the URL it was fetched from
```

The `.tfs.sha256` marker IS the trust anchor: presence means the image
was sha256-verified at install (the image is re-verified only when
re-fetched, never per run). No `layout/` extraction tree is created for
the image — the cache holds the immutable artifact only.

## Resolution rules (bootstrap)

1. Executable: today's flow (cache hit / payload slot / download+verify).
2. When the ref carries `;image` and `<entry>/<asset>.tfs` or its
   `.sha256` marker is missing: lock the entry, fetch
   `<base>/v<ver>/<asset>.tfs` (same mirror/offline rules as the
   executable), take the expected sha256 from manifest.json's `image`
   key (fallback: the SHA256SUMS line), verify, install 0444 + markers.
   Mismatch → the sha error shape, download deleted, cache untouched.
3. Exec: the v1 handoff **unchanged** (`--tebako-image <self>:<slot>:
   <mount> … --tebako-entry <argv0> <args>`), plus
   `TEBAKO_RUNTIME_IMAGE=<abs path to the cached .tfs>` in the
   environment when the ref carried `;image`.

## Driver change (upstream, tebako-chainwt src/tebako-main.cpp)

Minimal, additive: in the classic/incbin startup path, before the
embedded-image memory mount, prefer the environment:

```c
const char* rt_image = getenv("TEBAKO_RUNTIME_IMAGE");
if (rt_image != nullptr && rt_image[0] != '\0') {
    mounted = tebako_fs_init_from_file(rt_image, mount_point.c_str()) == 0;
} else {
    mounted = tebako_fs_init(&gfsData[0], gfsSize, mount_point.c_str()) == 0;
}
```

- v1 runtimes ignore the env and use the embedded image (the published
  0.15.9 executables — graceful degradation, no republish needed).
- Image-era runtime builds ship without the incbin image; the env (or a
  cache-resolved default) becomes how `--tebako-extract`, standalone
  runtime mode and the SDK find the runtime's files.
- The lean handoff (`--tebako-image`) is untouched: app images are
  self-contained and mount as today.

## Press against image-era runtimes (tebako-cli)

- The release index parsers learn the additive entries: manifest.json's
  `image: {filename, sha256, size_bytes}` key and the SHA256SUMS
  `<asset>.tfs` line.
- Press resolves the image into the same cache entry (the bootstrap
  finds it there at first run — one install serves both).
- Layout seeding: instead of `runtime --tebako-extract <cache>/layout`
  (which mounts the embedded image and writes a tree into the cache),
  the CLI extracts the cached `.tfs` **in-process** (the tfs C ABI,
  `tebako_fs_extract_all`) straight into the packaging environment
  (`<prefix>/o/s`). No extracted tree in the cache; the layout is
  rebuilt per press. v1-era releases keep the extract-via-runtime flow
  (byte-identical golden vs the gem).
- Arch alignment is a no-op for image-era seeds by construction (the
  image IS the runtime's own layout), so the alignment step is skipped.
- Deploy under the runtime executable (stub driver, bundle ops) is
  unchanged — the interpreter is the same artifact as today. The
  RuntimeSdk (native-extension deploy, roadmap 25) is ported: it reads
  the runtime's rbconfig (the configure-args provenance) from the
  mounted image in-process rather than an extracted cache tree — the
  headers themselves still come from the `tamatebako/ruby` src release,
  exactly like the gem (the image carries none: the runtime stripper
  removes `include/` and `*.a`), and provisions into the packaging
  environment (`<prefix>/deps/sdk/...`), never the cache.

## Backward compat

- v1 packages (no `;image`): byte-identical resolve+exec; no image is
  fetched even when the release publishes one.
- Image-era packages against v1-era releases (no image in the index):
  press cannot emit `;image` (no entry), so this combination cannot
  arise from tebako-cli; a hand-stitched `;image` ref against such a
  release fails resolution with the named "no checksum" error.
- Fat payloads: unchanged (the payload slot carries the interpreter;
  `;image` additionally resolves the image from the mirror at first
  run — image-era fat payloads that carry the `.tfs` in the package are
  30c's CAS work).

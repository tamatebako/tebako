# tebako-cli — the `tebako` packager CLI

The packager front end of the tebako-rs workspace (item 17's SELF-HOSTING
design): a port of the reference Ruby gem's lean/fat press
(`tamatebako/tebako`, three-part model branch).

```console
$ tebako press -r <root> -e <entry> [-o <output>] [-p <prefix>]
               [-c <cwd>] [-R <ruby>] [-m lean|fat] [-l error|warn|debug|trace]
               [--image <path>:<mount>]... [--bootstrap <path>]
               [--tebako-version <v>]
$ tebako cache list
$ tebako cache prune [--all] [--older-than Nd]
```

## Lean press flow

1. **resolve** the prebuilt runtime into the shared cache (`$TEBAKO_HOME`
   or `~/.tebako`; gem-identical layout, flock'd installs, manifest.json /
   SHA256SUMS release index, `TEBAKO_OFFLINE`, `TEBAKO_RUNTIME_MIRROR`,
   error codes 120–125) — downloads are in-process (crates/tebako-http);
2. **seed** the packaging environment (`<prefix>/o/s`) from the runtime's
   extracted filesystem layout;
3. **deploy** the application under the runtime itself: the deploy ops
   (bundle config/install for the Gemfile scenario) are serialized into a
   stub driver placed at `/local/stub.rb` of a throwaway image, which is
   imaged in-process, stitched onto an empty base and exec'd as
   `runtime --tebako-image <driver>:0:/__tebako_memfs__` with a scrubbed
   environment (`RUBYOPT`/`RUBYLIB`/`BUNDLE_*`/`BUNDLER_*` unset,
   `GEM_HOME`/`GEM_PATH`/`GEM_SPEC_CACHE`/`SSL_CERT_*`/
   `TEBAKO_PASS_THROUGH` set);
4. **strip** build artefacts, align the arch layout to the runtime, write
   the entry dispatcher at `/local/stub.rb`;
5. **image** the app (in-process dwarfs-t Writer → `fs.tfs`) and
   **stitch** the three-part package:
   bootstrap + image slot(s) + tpkg trailer (LEAN flag, launcher ABI 1,
   runtime_ref `ruby@<rv>;tebako=<v>`; `fat` mode adds the runtime as a
   FORMAT_RUNTIME payload slot and appends `;sha256=<hex>`).

At first run the packaged binary's bootstrap resolves the runtime into
the shared cache (fat: installs the payload instead) and hands over via
the launcher ABI.

## Scenarios

- **simple script** (`-e app.rb` or a root without Gemfile/*.gem) — the
  script tree is packaged as-is, no deploy step;
- **Gemfile** (`<root>/Gemfile`) — `bundle config set --local` for
  ffi/nokogiri/force_ruby_platform (+ openssl when a libtfs-deps vcpkg
  tree is provisioned) and `bundle install --jobs=N --prefer-local`,
  executed inside the runtime's own ruby. `Gemfile.lock` pins the bundler
  version (`BUNDLED WITH`, minimum 2.4.22); the Gemfile `ruby` directive
  selects the runtime's ruby version. Gems with **native extensions**
  build inside the deploy driver against the runtime SDK (below) — the
  built artifacts land at their gem-correct paths inside the app image.

## RuntimeSdk (native-extension deploy)

Prebuilt runtime images are stripped for size (no bin/ruby, no ruby
headers), so mkmf-driven extension builds cannot run against them
directly. Like the reference gem, whenever deploy ops run on POSIX (the
gem has no flag for this and neither does the CLI — the gem/gemspec
scenarios stay unported), the press provisions the runtime SDK into the
packaging environment (`<prefix>/deps/sdk/<ruby>-<src>-<platform>/`,
flock'd, `.sdk-complete` marker — never the runtime cache):

1. the pre-patched ruby src release the runtime was built from
   (`tfs-ruby-<ver>-src.tar.gz` from `tamatebako/ruby` releases,
   `TEBAKO_SDK_SRC_RELEASE` default `v0.2.1`,
   `TEBAKO_SDK_SRC_MIRROR` for mirrors — `file://` works offline) is
   downloaded in-process and sha256-verified against the release's
   SHA256SUMS;
2. the tarball is extracted **in-process** (flate2 + tar — no tar
   binary) and `./configure` runs with the runtime's own configure
   arguments, replayed from its rbconfig (read from the **mounted
   runtime image** via the tfs C ABI for image-era runtimes — no
   `--tebako-extract`, no cache `layout/`; v1-era runtimes keep the
   extracted-layout flow) with the build machine's paths and compiler
   assignments filtered out;
3. `include/` + the generated `archhdr/ruby/config.h` are installed,
   and `nm` on the runtime executable yields a `libruby-stub.a`
   re-declaring every ruby-ABI symbol it exports (mkmf's link probes get
   true yes/no resolution; shipped extensions never link the stub).

The deploy driver's RbConfig overrides then point
`rubyhdrdir`/`rubyarchhdrdir`/`LIBRUBYARG` at the SDK (and `bindir` at
the ruby shim, whose script mode emulates the ruby command line:
`-r`/`-I`/`-e`/`--`), while the cc_override re-resolves the runtime's
recorded toolchain against the press host. Failures are named errors:
135 (missing headers/configure/nm/stub/platform mismatch), 122 (src
download), 125 (SDK lock timeout).

## Where the pieces come from

- **runtime**: `~/.tebako/runtimes/...` (downloaded from the
  tebako-runtime-ruby release, `v<tebako-version>`; default 0.15.9).
  Image-era releases additionally carry `tebako-runtime-<...>.tfs` (the
  runtime's files as a dwarfs-t-native image, item 30): the CLI resolves
  it into the same cache entry (read-only + trusted markers), seeds the
  packaging environment by extracting it **in-process** through the tfs
  C ABI (no `layout/` tree in the cache), and emits the `;image` flag in
  the package's runtime_ref so the first run resolves the image too
  ([../../docs/runtime-as-image.md](../../docs/runtime-as-image.md));
- **bootstrap**: `--bootstrap` > `$TEBAKO_BOOTSTRAP` > the Rust
  `tebako-bootstrap` binary next to the `tebako` executable (dogfooding
  milestone 6) > the C++ tebako-bootstrap release, resolved with the gem's
  BootstrapManager machinery (`TEBAKO_BOOTSTRAP_VERSION`,
  `TEBAKO_BOOTSTRAP_MIRROR`);
- **downloads**: all in-process via `crates/tebako-http` (ureq + rustls,
  webpki-roots bundled; HTTPS-only, redirects ≤ 5, `file://` mirrors,
  `TEBAKO_OFFLINE`; the OS trust store is opt-in via
  `TEBAKO_TLS_PLATFORM_ROOTS`). No curl anywhere;
- **images**: built in-process via the dwarfs-t `Writer` (dwarfs-t-rs) —
  no mkdwarfs binary, no PATH lookup, no provisioning. Images carry
  dwarfs-t-native (FlatBuffers) metadata and are named `.tfs`
  (`fs.tfs`, `deploy-driver.tfs`); `.dwarfs` stays for
  upstream-compatible images.

## Deviations from the reference gem (documented, deliberate)

- the bootstrap portion **defaults to the in-workspace Rust
  tebako-bootstrap**; the gem always uses the C++ release (reachable via
  the lookup chain above);
- **no mkdwarfs anywhere** (owner rule): the gem shells out to a
  provisioned mkdwarfs binary; the CLI builds images in-process and the
  golden test filters the two sides' image-build lines when diffing
  press stdout;
- the **RuntimeSdk host tag** in the SDK cache key is the tebako
  platform id (`macos-arm64`); the gem derives it from the press host's
  ruby RbConfig (`darwin24-arm64`), which a hostless CLI cannot
  reproduce — both are stable per-platform cache keys. The src tarball
  is extracted **in-process** (flate2 + tar; the gem shells out to
  `tar`); `nm`/`cc`/`ar` and the downloaded `./configure` run as
  processes exactly like the gem (a native build needs a C toolchain —
  the same reliance the deploy toolchain fallback table carries);
- the **gem/gemspec scenarios** (which ride the `bundle_exec` op), the
  **classic** press mode, `tebako setup/clean/hash`, and `.tebako.yml`
  are later milestones (the `runtime` mode is rejected with exit 133
  like the gem);
- the bundler version for a lockfile-less Gemfile that pins bundler is
  taken from rubygems' `latest.json` (the gem's SpecFetcher picks the
  latest released bundler satisfying the requirement — identical unless
  the requirement excludes the latest release);
- images are stitched **densely** (tpkg slots carry absolute offsets; the
  gem's 8-byte alignment padding is cosmetic);
- the gem's unconditional 5-second press pause runs only when a
  package/prefix-inside-root warning was actually printed.

Everything else is byte-level parity: the press stdout, the produced
package's trailer fields, the packaged binary's output, the cache layout
(shared with the gem), and the exit codes (106–135 + the packaging error
table).

## Tests

```console
$ cargo test -p tebako-cli                     # unit + fast CLI tests
$ TEBAKO_REFERENCE_GEM=/path/to/tebako-gem \   # optional: golden diff
  TEBAKO_MKDWARFS=/path/to/mkdwarfs \          # (for the gem's own press)
  cargo test -p tebako-cli --test cli_e2e      # press + run + golden
```

The e2e tests download the prebuilt runtime into the cache (network) and
skip cleanly when `TEBAKO_CLI_SKIP_E2E` is set. The golden test
additionally needs a host ruby with the thor gem, a checkout of the
reference gem at the matching version, and an mkdwarfs binary **for the
reference gem's own press** (the CLI itself needs none). The
native-extension e2e (`native_ext_press_builds_and_packages`) builds the
`native-ext-gem` fixture into `toyext-0.1.0.gem` **with the resolved
runtime itself** (no host ruby), vendors it into the `native-ext-app`
fixture's `vendor/cache` (the dependency resolves offline from the
lockfile), mirrors the ruby src release over `file://`, presses, and
asserts the built `toyext.{so,bundle}` sits in the app image at its
gem-correct path before a cold run proves the packaged app loads it from
the memfs.

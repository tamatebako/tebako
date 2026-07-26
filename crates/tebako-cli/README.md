# tebako-cli — the `tebako` packager CLI

The packager front end of the tebako-rs workspace (item 17's SELF-HOSTING
design): a port of the reference Ruby gem's lean/fat press
(`tamatebako/tebako`, three-part model branch).

```console
$ tebako press -r <root> -e <entry> [-o <output>] [-p <prefix>]
               [-c <cwd>] [-R <ruby>] [-m lean|fat] [-l error|warn|debug|trace]
               [--image <path>:<mount>]... [--bootstrap <path>]
               [--mkdwarfs <path>] [--tebako-version <v>]
$ tebako cache list
$ tebako cache prune [--all] [--older-than Nd]
```

## Lean press flow

1. **resolve** the prebuilt runtime into the shared cache (`$TEBAKO_HOME`
   or `~/.tebako`; gem-identical layout, flock'd installs, manifest.json /
   SHA256SUMS release index, `TEBAKO_OFFLINE`, `TEBAKO_RUNTIME_MIRROR`,
   error codes 120–125);
2. **seed** the packaging environment (`<prefix>/o/s`) from the runtime's
   extracted filesystem layout;
3. **deploy** the application under the runtime itself: the deploy ops
   (bundle config/install for the Gemfile scenario) are serialized into a
   stub driver placed at `/local/stub.rb` of a throwaway image, which is
   mkdwarfs'd, stitched onto an empty base and exec'd as
   `runtime --tebako-image <driver>:0:/__tebako_memfs__` with a scrubbed
   environment (`RUBYOPT`/`RUBYLIB`/`BUNDLE_*`/`BUNDLER_*` unset,
   `GEM_HOME`/`GEM_PATH`/`GEM_SPEC_CACHE`/`SSL_CERT_*`/
   `TEBAKO_PASS_THROUGH` set);
4. **strip** build artefacts, align the arch layout to the runtime, write
   the entry dispatcher at `/local/stub.rb`;
5. **mkdwarfs** the app image and **stitch** the three-part package:
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
  selects the runtime's ruby version.

## Where the pieces come from

- **runtime**: `~/.tebako/runtimes/...` (downloaded from the
  tebako-runtime-ruby release, `v<tebako-version>`; default 0.15.9);
- **bootstrap**: `--bootstrap` > `$TEBAKO_BOOTSTRAP` > the Rust
  `tebako-bootstrap` binary next to the `tebako` executable (dogfooding
  milestone 6) > the C++ tebako-bootstrap release, resolved with the gem's
  BootstrapManager machinery (`TEBAKO_BOOTSTRAP_VERSION`,
  `TEBAKO_BOOTSTRAP_MIRROR`);
- **mkdwarfs**: `--mkdwarfs` > `$TEBAKO_MKDWARFS` > PATH >
  `<prefix>/deps/bin/mkdwarfs*` (error 128 otherwise).

## Deviations from the reference gem (documented, deliberate)

- the bootstrap portion **defaults to the in-workspace Rust
  tebako-bootstrap**; the gem always uses the C++ release (reachable via
  the lookup chain above);
- the mkdwarfs lookup chain adds the flag/env/PATH sources (the gem only
  consults `<prefix>/deps/bin`);
- the **RuntimeSdk / src-release subsystem is not ported** — the gem
  downloads the ruby source release so native-extension builds (mkmf,
  cmake) can compile inside the deploy driver. Pure-ruby bundler flows
  never need it; native-extension deploy is a later milestone. The
  driver's `build_overrides` therefore carry only the bindir override,
  which is exactly what the gem emits when no SDK was resolved;
- the **gem/gemspec scenarios** (which ride the `bundle_exec` op and the
  SDK), the **classic** press mode, `tebako setup/clean/hash`, and
  `.tebako.yml` are later milestones (the `runtime` mode is rejected with
  exit 133 like the gem);
- the bundler version for a lockfile-less Gemfile that pins bundler is
  taken from rubygems' `latest.json` (the gem's SpecFetcher picks the
  latest released bundler satisfying the requirement — identical unless
  the requirement excludes the latest release);
- downloads shell out to the `curl` CLI (item 17's design note mentions
  in-process ureq+rustls; the curl choice is milestone 6's audited TLS
  decision — zero binary-weight, platform-native TLS, `file://` mirrors
  for free, curl present on all supported hosts). `tebako setup`'s
  checksummed mkdwarfs provisioning is a later milestone (mkdwarfs comes
  from the lookup chain above until then);
- images are stitched **densely** (tpkg slots carry absolute offsets; the
  gem's 8-byte alignment padding is cosmetic);
- the gem's unconditional 5-second press pause runs only when a
  package/prefix-inside-root warning was actually printed;
- the cache version file is written but never checked-and-cleaned (the
  gem's cmake-cache-era guard).

Everything else is byte-level parity: the press stdout, the produced
package's trailer fields, the packaged binary's output, the cache layout
(shared with the gem), and the exit codes (106–134 + the packaging error
table).

## Tests

```console
$ cargo test -p tebako-cli                     # unit + fast CLI tests
$ TEBAKO_MKDWARFS=/path/to/mkdwarfs \
  TEBAKO_REFERENCE_GEM=/path/to/tebako-gem \   # optional: golden diff
  cargo test -p tebako-cli --test cli_e2e      # press + run + golden
```

The e2e tests download the prebuilt runtime into the cache (network) and
skip cleanly when mkdwarfs is unavailable or `TEBAKO_CLI_SKIP_E2E` is set.
The golden test additionally needs a host ruby with the thor gem and a
checkout of the reference gem at the matching version.

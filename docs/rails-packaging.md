# Packaging a Rails application

How to package a Ruby on Rails application as a tebako payload — the
walkthrough is validated end to end on Windows (ucrt64) by CI: a freshly
generated Rails application is pressed, every native extension it needs
is compiled from source against the packaged runtime's SDK, and the
packaged executable boots the full Rails stack. The same steps apply on
macOS and Linux. The native-extension build chain on Windows — the
loader/DLL split, the SDK's configure replay, the import library — is
documented separately in `windows-native-extensions.md`.

## What a Rails payload is

A Rails application is an `app`-kind payload whose entrypoint is the Rails
console command surface you want exposed (`bin/rails`, `bin/rake`, or the
application's own binstubs). The runtime is the prebuilt ruby package for
your platform — the payload carries the application, its bundled gems, and
the assets; the runtime carries the interpreter.

## The Tebakofile

```yaml
name: myapp
kind: app

entry-point: bin/rails       # the binstub the package boots
Ruby: "3.4"                  # the prebuilt runtime line to resolve
```

## Writable surfaces

Rails wants to WRITE at run time — `tmp/`, `log/`, and `tmp/cache/`. The
payload image is read-only by design; the packaged run's writable surfaces
come from the jail policy's write grants (spec 08): declare the write
paths in the payload manifest's jail block and tebako routes them to the
host:

```yaml
jail:
  writes:
    - ./tmp
    - ./log
```

Asset precompilation belongs at PRESS time — run `rails
assets:precompile` in the staged tree before the press, so the payload
carries `public/assets` ready-made and the packaged run never compiles.

## Native extensions

Gems that ship a precompiled variant for the platform install as
binaries. Gems that do not — on Windows today that is bigdecimal, date,
json, prism, erb, io-console and nio4r — compile from source during the
press, against a header tree the SDK regenerates from the runtime's own
build provenance. The press needs a C toolchain for this step on the
host it runs on; the packaged result needs nothing — the compiled
extensions ride the payload, and the target machine runs them against
the runtime that the loader resolves.

The Windows chain has the sharpest edges, all documented with their
evidence in `windows-native-extensions.md`: the runtime is a 99 KB
loader plus the interpreter DLL, the SDK's configure replay pins its
shell and toolchain explicitly, and extension links resolve against an
import library built from the DLL's true export table.

## The Windows notes

- The prebuilt runtime for `x86_64-windows-ucrt` (and arm64) carries the
  MSYS2 toolchain's runtime DLLs; native extensions resolve through the
  runtime's library aliases once built.
- Executables from `tebako press --mode=self-contained` boot the runtime
  from their own slots; the packaged app needs nothing installed on the
  target machine.

## The validation

The repository's CI includes the end-to-end leg
(`.github/workflows/rails-windows.yml`, nightly and on dispatch): a
freshly generated Rails application is pressed on `windows-latest` with
the released toolchain, and the packaged executable boots the full
Rails stack — `myapp.exe runner` loads the autoloaders, initializers,
and railties inside the package, then proves a jailed write round-trips
through `tmp/`. The leg closed the Rails-on-Windows gap (issue #753,
green at v2.8.44) and now serves as the regression guard: if a Rails,
ruby or runtime update regresses a Rails payload, it fails there first.
The leg also accepts an iteration input (`tools_run`, a release run id)
that presses a staged, not-yet-published tool build — the
fix-to-verdict loop for the next chain.

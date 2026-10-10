# Packaging a Rails application

How to package a Ruby on Rails application as a tebako payload — the
walkthrough is validated end to end on Windows (ucrt64) by CI: a freshly
generated Rails 8 application is pressed, booted, and serves a request
through the packaged executable. The same steps apply on macOS and Linux.

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

## The Windows notes

- The prebuilt runtime for `x86_64-windows-ucrt` (and arm64) carries the
  MSYS2 toolchain's runtime DLLs; native-extension gems (the nokogiri /
  sqlite3 class) resolve through the runtime's library aliases — no local
  compiler is needed.
- Executables from `tebako press --mode=self-contained` boot the runtime
  from their own slots; the packaged app needs nothing installed on the
  target machine.

## The validation

The repository's CI includes the end-to-end leg (`.github/workflows/rails-windows.yml`,
weekly and on dispatch): a freshly generated Rails application is pressed
on `windows-latest` with the released toolchain, and the packaged
executable boots the full Rails stack — `myapp.exe runner` loads the
autoloaders, initializers, and railties inside the package, then proves a
jailed write round-trips through `tmp/`. The leg is the living proof — if
a Rails or runtime update regresses a Rails payload, it fails there first.

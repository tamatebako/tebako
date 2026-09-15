#!/bin/bash
# windows-msi-build.sh — build (or re-hash) the system MSI installer from
# the release's own staged windows exes (roadmap 83, spec 16 §7).
#
# Subcommands (the sign-then-hash law, spec 34 §1.3, splits the two —
# signing is a workflow ACTION between them):
#   build   verify the staged exes against the windows leg's OWN
#           fragments (frag-<platform>/<tool>-<platform>.sha256 — the
#           sign-then-hash anchors, spec 34 §1.3; THIS release's
#           SHA256SUMS does not exist yet — finalize makes it AFTER us),
#           then wix build the template into out/<asset>.msi (unsigned —
#           the Azure Artifact Signing step signs it in place when armed).
#   frag    hash the FINAL bytes (signed when the gate armed) into
#           fragments/frag-installers-<platform>/<asset>.{sha256,size}
#           for finalize.
#
# Required env: VERSION (tag minus v), SRC_DIR (the staged windows exes),
# FRAG_DIR (the downloaded frag-<platform> artifact), RUNNER_TEMP.
# Optional: PLATFORM (default windows-ucrt64), PRODUCT_NAME (default
# tebako), MANUFACTURER (default tamatebako), MSI_UPGRADE_CODE (REQUIRED
# for build — the pipeline passes the locked per-product GUID; clients
# pass their own). Web-bootstrapper (spec 16 §7): BOOTSTRAP_REGISTRY (the
# client's registry ref) + BOOTSTRAP_PAYLOADS (space-separated names) —
# stages <root>\bootstrap-seed.cmd (self-locating, user-re-runnable) and
# binds the template's Bootstrap block. Optional third knob
# BOOTSTRAP_WARM (a subset of BOOTSTRAP_PAYLOADS): the seed also
# dispatches each named shim once, pulling its RUNTIME into the shared
# home at install time (as SYSTEM) — after a warm, every user's dispatch
# is read-only against the machine home (the runtime download is the only
# dispatch-time write; the registry refresh degrades to loud stale-serve,
# the journal is best-effort). Warm only bounded, print-and-exit
# entrypoints: the seed runs no timeout.
#
# The staged tool set is CURATED (the 4 PATH tools: tebako, tebako-shim,
# tfs, tebako-pkg) — the bootstrap and runtime-launcher ship on the
# release for press/embedding, not for PATH. A missing staged exe fails
# HERE, named, never in a user's Add/Remove Programs.
set -euo pipefail
[ "$#" -eq 1 ] || { echo "usage: $0 build|frag" >&2; exit 64; }
MODE="$1"

: "${VERSION:?VERSION is required (the release version, tag minus v)}"
: "${SRC_DIR:?SRC_DIR is required (staged exes + SHA256SUMS)}"
PLATFORM="${PLATFORM:-windows-ucrt64}"
PRODUCT_NAME="${PRODUCT_NAME:-tebako}"
MANUFACTURER="${MANUFACTURER:-tamatebako}"
TOOLS="tebako tebako-shim tfs tebako-pkg"
ASSET="${PRODUCT_NAME}-setup-${VERSION}-${PLATFORM}.msi"

sha256_of() {
  # stdin, not a path argument: GNU coreutils *sum escape-flag a path
  # containing backslashes by prefixing the OUTPUT LINE with '\' (the
  # v2.8.0 MSI leg: SRC_DIR's D:\a\… spelling made got="\<hash>" ≠ want).
  if command -v sha256sum >/dev/null 2>&1; then sha256sum < "$1" | cut -d' ' -f1
  else shasum -a 256 < "$1" | cut -d' ' -f1; fi
}

case "$MODE" in
  build)
    : "${MSI_UPGRADE_CODE:?MSI_UPGRADE_CODE is required (the locked per-product UpgradeCode GUID)}"
    : "${FRAG_DIR:?FRAG_DIR is required (the downloaded frag-<platform> artifact)}"
    # Verify the staged exes against the windows leg's OWN fragments —
    # the MSI wraps EXACTLY the published signed bytes (the leg's frags
    # were re-anchored post-signing, spec 34 §1.3; a corrupted download
    # inside a signed installer is the failure this precludes).
    mkdir -p "msi-input"
    for tool in $TOOLS; do
      exe="${tool}-${VERSION}-${PLATFORM}.exe"
      [ -f "$SRC_DIR/$exe" ] || { echo "::error::$exe not staged in $SRC_DIR"; exit 1; }
      [ -f "$FRAG_DIR/${tool}-${PLATFORM}.sha256" ] || { echo "::error::no fragment ${tool}-${PLATFORM}.sha256 in $FRAG_DIR — the windows leg did not complete"; exit 1; }
      want=$(cat "$FRAG_DIR/${tool}-${PLATFORM}.sha256")
      got=$(sha256_of "$SRC_DIR/$exe")
      [ "$got" = "$want" ] || { echo "::error::sha256 mismatch on $exe (download corruption) — got $got want $want"; exit 1; }
      cp "$SRC_DIR/$exe" "msi-input/$tool.exe"
    done
    echo "staged + verified against the leg's signed-byte fragments: $TOOLS"
    # Optional web-bootstrapper seed (spec 16 §7): the client's registry +
    # payload names compose a SELF-LOCATING seed script (%~dp0 is the
    # install root) the template installs beside bin/ and runs once
    # (deferred, SYSTEM, failure-ignored). Re-runnable by hand — tebako
    # install is idempotent.
    BOOTSTRAP_BIND=()
    if [ -n "${BOOTSTRAP_REGISTRY:-}" ]; then
      : "${BOOTSTRAP_PAYLOADS:?BOOTSTRAP_PAYLOADS is required when BOOTSTRAP_REGISTRY is set (space-separated payload names)}"
      # WixQuietExec64 lives in the Util extension — pinned to the WiX
      # tool's line (the workflow installs wix 5.0.2; bump in lockstep).
      wix extension add --global WixToolset.Util.wixext/5.0.2 >/dev/null
      {
        printf '@echo off\r\n'
        printf 'rem %s bootstrap seed (spec 16 §7) — composed at build time; safe to re-run.\r\n' "$PRODUCT_NAME"
        printf 'set "TEBAKO_HOME=%%~dp0home"\r\n'
        printf 'if not exist "%%TEBAKO_HOME%%\\shims" mkdir "%%TEBAKO_HOME%%\\shims"\r\n'
        printf '"%%~dp0bin\\tebako.exe" add-registry %s\r\n' "$BOOTSTRAP_REGISTRY"
        for p in $BOOTSTRAP_PAYLOADS; do
          printf '"%%~dp0bin\\tebako.exe" install %s\r\n' "$p"
        done
        # Optional warm (spec 16 §7): dispatch each named shim once so its
        # RUNTIME lands in the shared home at install time — after that a
        # user dispatch is read-only against the machine home.
        for w in ${BOOTSTRAP_WARM:-}; do
          case " $BOOTSTRAP_PAYLOADS " in
            *" $w "*) ;;
            *) echo "::error::BOOTSTRAP_WARM entry $w is not in BOOTSTRAP_PAYLOADS — warm is a subset"; exit 1 ;;
          esac
          printf '"%%TEBAKO_HOME%%\\shims\\%s.exe" >NUL 2>&1\r\n' "$w"
        done
      } > msi-input/bootstrap-seed.cmd
      BOOTSTRAP_BIND=(-d Bootstrap=1)
      echo "bootstrap seed staged: registry $BOOTSTRAP_REGISTRY — payloads: $BOOTSTRAP_PAYLOADS"
    fi
    mkdir -p out
    command -v wix >/dev/null 2>&1 || { echo "::error::wix (WiX v5 dotnet tool) not on PATH — the workflow installs it"; exit 1; }
    # MSI ProductVersion is a strict numeric triplet — a roadmap-89
    # rehearsal's VERSION is a PR ref ("590-merge"), not a product
    # version. The rehearsal MSI installs/uninstalls the same either way.
    MSI_VERSION="$VERSION"
    [[ "$MSI_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || MSI_VERSION="0.0.0"
    # BinDir must be ABSOLUTE and WINDOWS-spelled: WiX resolves relative
    # authoring paths against the .wxs's directory, and a Git-Bash $PWD
    # ("/d/a/…") is not a path a native .NET tool can open.
    BIN_BINDIR="$PWD/msi-input"
    command -v cygpath >/dev/null 2>&1 && BIN_BINDIR=$(cygpath -w "$BIN_BINDIR")
    wix build -arch x64 \
      templates/installers/windows/tebako.wxs \
      -d "ProductName=$PRODUCT_NAME" \
      -d "ProductVersion=$MSI_VERSION" \
      -d "Manufacturer=$MANUFACTURER" \
      -d "UpgradeCode=$MSI_UPGRADE_CODE" \
      ${BOOTSTRAP_BIND[@]+"${BOOTSTRAP_BIND[@]}"} \
      -d "BinDir=$BIN_BINDIR" \
      -o "out/$ASSET"
    echo "built: out/$ASSET (unsigned — the Azure step signs in place when armed)"
    ;;
  frag)
    [ -f "out/$ASSET" ] || { echo "::error::out/$ASSET missing — build step did not run?"; exit 1; }
    mkdir -p "fragments/frag-installers-$PLATFORM"
    sha256_of "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.sha256"
    stat -c %s "out/$ASSET" 2>/dev/null > "fragments/frag-installers-$PLATFORM/$ASSET.size" \
      || stat -f %z "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.size"
    echo "frag: $(cat fragments/frag-installers-$PLATFORM/$ASSET.sha256)  $ASSET (over the FINAL bytes — spec 34 §1.3)"
    ;;
  *) echo "usage: $0 build|frag" >&2; exit 64 ;;
esac

#!/usr/bin/env bash
# vcpkg-nuget-cache.sh — point vcpkg's binary cache at the tamatebako org's
# GitHub Packages NuGet feed.
#
# Mirrored per repo (tebako, dwarfs-t-rs, tebako-runtime-ruby): CI glue is
# a per-repo copy by convention here (same keep-in-step rule as the
# VCPKG_COMMIT pins) so no repo's CI depends on another repo's main.
#
# Why this exists: lukka/run-vcpkg@v11 injects
# VCPKG_BINARY_SOURCES=clear;x-gha,readwrite into every later step's
# environment. vcpkg removed the x-gha provider in the 2025.06.13 release
# (the GitHub Actions cache service API it rode on was retired), so the
# injected spec degrades to a warning plus NO binary cache at all — and the
# old actions/cache step over the vcpkg archives dir was dead twice over
# (the `clear` drops the default files provider, and
# VCPKG_DEFAULT_BINARY_CACHE is redirected to run-vcpkg's scratch dir).
# Every baseline miss became a full from-source port build, and when a
# github.com tarball fetch 403'd mid-build the leg went red (tebako run
# 30982378190: boost-mp11:x64-mingw-static).
#
# The surviving native GitHub integration is vcpkg's NuGet binary-cache
# provider backed by the org's GitHub Packages feed. This script fetches a
# pinned nuget.exe, registers the feed + the job token in the user-level
# nuget.config (shared with whatever nuget.exe vcpkg drives), shadows PATH
# with it, then overrides VCPKG_BINARY_SOURCES for the remaining steps via
# GITHUB_ENV.
#
# Why a self-fetched nuget.exe instead of `vcpkg fetch nuget`: the ARM64
# runner images set VCPKG_FORCE_SYSTEM_BINARIES=1, which forbids vcpkg's
# own tool downloads — its nuget lookup then falls back to PATH and finds
# Mono.framework's `nuget` WRAPPER SCRIPT (not an assembly: "File does not
# contain a valid CIL image" under mono). A real nuget.exe earlier on PATH
# satisfies that lookup; where FORCE_SYSTEM_BINARIES is unset vcpkg simply
# downloads its own copy and only the shared nuget.config matters.
#
# Usage: ci/vcpkg-nuget-cache.sh   (GH_TOKEN must be set)
#
# Contract:
# - Run AFTER lukka/run-vcpkg (it is the env writer this overrides) and
#   BEFORE anything that invokes vcpkg (the sqfs pre-install, cargo builds
#   — dwarfs-t-sys's CMake toolchain invocation inherits the step env).
# - Non-Windows runners need mono on PATH (vcpkg drives nuget.exe through
#   mono; macos runners ship it, the ubuntu legs apt-install mono-complete).
# - The job needs packages:write for the push side; fork PRs get a
#   read-only token, where pushes degrade to a non-fatal vcpkg note while
#   restores keep working — and a cold feed just builds from source,
#   exactly the pre-fix behavior. Any failure here is a warning, never a
#   red leg: cache setup must not take the build down with it.
# - GitHub Packages quirk (community discussion #159893): the FIRST push
#   of a new package id cannot be done by GITHUB_TOKEN (403 Forbidden on
#   creation — proven on tebako run 30989981488's ubuntu leg). Set the
#   org secret TEBAKO_PACKAGES_PAT (classic PAT, write:packages) and the
#   workflow's GH_TOKEN prefers it; GITHUB_TOKEN remains the read path
#   and the fallback while the secret is absent.
set -euo pipefail

: "${GH_TOKEN:?set GH_TOKEN to secrets.GITHUB_TOKEN}"
FEED="https://nuget.pkg.github.com/tamatebako/index.json"
# nuget.exe 7.0.1, the release vcpkg 2025.06+ fetches (same pin + sha256 as
# dwarfs-t's _build.yml).
NUGET_URL="https://dist.nuget.org/win-x86-commandline/v7.0.1/nuget.exe"
NUGET_SHA256="8ddc8cc04298fa08277efdca35373eb158f0c95f5bb1b15efcab2b62952028f6"

NUGET_DIR="${RUNNER_TEMP:-/tmp}/tebako-nuget-bin"
mkdir -p "$NUGET_DIR"
NUGET="$NUGET_DIR/nuget.exe"
if ! curl -fsSL -o "$NUGET" "$NUGET_URL" ||
   ! { echo "$NUGET_SHA256  $NUGET" | { sha256sum -c 2>/dev/null || shasum -a 256 -c; }; }; then
  echo "::warning::nuget.exe download/verify failed — this leg runs without the binary cache (ports build from source)."
  exit 0
fi
chmod +x "$NUGET"

# vcpkg runs nuget.exe natively on Windows and through mono elsewhere.
# The FORCE_SYSTEM_BINARIES PATH lookup searches the bare stem `nuget` —
# on Windows nuget.exe itself already matches (and a bare-name copy would
# be the same file on the case-insensitive FS).
MONO="mono"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) MONO="" ;;
  *) cp "$NUGET" "$NUGET_DIR/nuget" ;;
esac
echo "$NUGET_DIR" >> "$GITHUB_PATH"

if ! $MONO "$NUGET" sources add -source "$FEED" -name GitHub \
       -username tebako-ci -password "$GH_TOKEN" -storepasswordincleartext > /dev/null 2>&1 ||
   ! $MONO "$NUGET" setapikey "$GH_TOKEN" -source "$FEED" > /dev/null 2>&1; then
  # vcpkg's nuget push passes no -ApiKey; the stored key authenticates it.
  echo "::warning::nuget feed registration failed — this leg runs without the binary cache (ports build from source)."
  exit 0
fi

echo "VCPKG_BINARY_SOURCES=clear;nuget,GitHub,readwrite" >> "$GITHUB_ENV"
echo "vcpkg binary cache: nuget feed $FEED (readwrite)"

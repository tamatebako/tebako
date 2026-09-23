#!/bin/bash
# release-upload.sh — the skip-if-current release uploader (tebako#189).
#
# Replaces blanket `gh release upload --clobber` in release.yml. A
# deleted asset name 422-blocks re-uploads on a young release object for
# hours (probe-proven 2026-09-23 on tamatebako/tebako-runtime-ruby), so
# this script never deletes on the payload path:
#
#   payload mode (default — binaries, link-unit tarballs):
#     name absent              → upload
#     same sha256 digest       → skip (byte-identical re-run)
#     different/unknown digest → ::warning:: and KEEP the served bytes;
#                                never delete. A drifted payload under
#                                the same name is a finding to surface,
#                                not something to paper over (#189:
#                                payload names are byte-immutable).
#
#   metadata mode (--metadata — SHA256SUMS, manifest.json, .sha256
#   sidecars, .asc signatures, the MSI/pkg installer containers):
#     name absent        → upload
#     same digest        → skip
#     drifted            → single-asset replace with bounded convergence:
#                          delete, poll the asset list until the name is
#                          gone, upload without --clobber (422s retried),
#                          verify the served digest. Budget exhaust is a
#                          named failure — never a silent stale serve.
#
# Usage: bash .github/workflows/lib/release-upload.sh [--metadata] <tag> <file>...
# Env:   GH_TOKEN (required by gh), GITHUB_REPOSITORY (set by Actions).
# Path note: platform/installer jobs check out into tebako-rs/, so they
# call this as tebako-rs/.github/workflows/lib/release-upload.sh; the
# finalize job checks out at the workspace root.
set -uo pipefail

MODE=payload
if [ "${1:-}" = "--metadata" ]; then
  MODE=metadata
  shift
fi
TAG="${1:-}"
shift || true
if [ -z "$TAG" ] || [ "$#" -eq 0 ]; then
  echo "::error::release-upload: usage: release-upload.sh [--metadata] <tag> <file>..."
  exit 1
fi

DELETE_POLL_BUDGET=180    # s — asset-list convergence after a delete
UPLOAD_RETRY_BUDGET=180   # s — 422 name-block convergence
DIGEST_GRACE=60           # s — served-digest population after upload

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    openssl dgst -sha256 -r "$1" | cut -d' ' -f1
  fi
}

release_id() {
  gh api "repos/$GITHUB_REPOSITORY/releases/tags/$TAG" --jq .id
}

# Echoes "<name> <sha256-or-> <size>" per asset, one per line.
asset_list() {
  gh api --paginate "repos/$GITHUB_REPOSITORY/releases/$RELEASE_ID/assets" \
    --jq '.[] | .name + " " + ((.digest // "") | sub("^sha256:"; "") | if . == "" then "-" else . end) + " " + (.size | tostring)'
}

served_digest() { # $1=name → digest or "-" or empty (absent)
  asset_list | awk -v n="$1" '$1 == n { print $2; found=1 } END { if (!found) print "" }' | head -1
}

upload_one() { # $1=file
  local file="$1" name local_sha remote tries delay
  name=$(basename "$file")
  if [ ! -f "$file" ]; then
    echo "::error::release-upload: local file missing: $file"
    return 1
  fi
  local_sha=$(sha256_of "$file")
  remote=$(served_digest "$name")
  if [ "$remote" = "$local_sha" ]; then
    echo "release-upload: $name already serves sha256:$local_sha — skip"
    return 0
  fi
  if [ -z "$remote" ]; then
    gh release upload "$TAG" "$file"
    echo "release-upload: $name uploaded (sha256:$local_sha)"
    return 0
  fi
  # Name occupied by different (or unknown-digest) bytes.
  if [ "$MODE" = payload ]; then
    echo "::warning::release-upload: $name already exists with served digest $remote (local $local_sha) — payload names are byte-immutable (tebako#189); keeping the served bytes, NOT re-uploading"
    return 0
  fi
  # metadata mode: bounded single-asset replace.
  echo "release-upload: $name drifted (served $remote, local $local_sha) — replacing (metadata mode)"
  gh release delete-asset "$TAG" "$name" --yes
  tries=0
  while [ -n "$(served_digest "$name")" ]; do
    if [ "$tries" -ge "$DELETE_POLL_BUDGET" ]; then
      echo "::error::release-upload: $name still listed ${DELETE_POLL_BUDGET}s after delete — release object not converging; re-run this job later"
      return 1
    fi
    sleep 5; tries=$(( tries + 5 ))
  done
  tries=0; delay=5
  while true; do
    if gh release upload "$TAG" "$file"; then
      break
    fi
    if [ "$tries" -ge "$UPLOAD_RETRY_BUDGET" ]; then
      echo "::error::release-upload: $name still un-uploadable ${UPLOAD_RETRY_BUDGET}s after delete (name-blocked young object) — re-run this job later"
      return 1
    fi
    sleep "$delay"; tries=$(( tries + delay ))
    [ "$delay" -lt 80 ] && delay=$(( delay * 2 ))
  done
  # Verify the served digest (short grace: the digest field populates
  # asynchronously on fresh uploads).
  tries=0
  while true; do
    remote=$(served_digest "$name")
    if [ "$remote" = "$local_sha" ]; then
      echo "release-upload: $name replaced and verified (sha256:$local_sha)"
      return 0
    fi
    if [ -n "$remote" ] && [ "$remote" != "-" ]; then
      echo "::error::release-upload: $name serves $remote after replace, expected $local_sha — upload corruption"
      return 1
    fi
    if [ "$tries" -ge "$DIGEST_GRACE" ]; then
      echo "::warning::release-upload: $name digest not populated ${DIGEST_GRACE}s after upload; size cross-check only"
      local want got
      want=$(stat -c %s "$file" 2>/dev/null || stat -f %z "$file")
      got=$(asset_list | awk -v n="$name" '$1 == n { print $3 }' | head -1)
      if [ "$got" = "$want" ]; then
        return 0
      fi
      echo "::error::release-upload: $name size mismatch after replace (served $got, local $want)"
      return 1
    fi
    sleep 5; tries=$(( tries + 5 ))
  done
}

RELEASE_ID=$(release_id) || {
  echo "::error::release-upload: no release object for tag $TAG"
  exit 1
}

rc=0
for f in "$@"; do
  upload_one "$f" || rc=1
done
exit "$rc"

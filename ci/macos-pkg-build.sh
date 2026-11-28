#!/bin/bash
# macos-pkg-build.sh — build the system pkg installer from the release's
# own staged macOS binaries (roadmap 83, spec 16 §7 / spec 31 §5a).
# pkgbuild (component) → productbuild (distribution) → [armed] productsign
# (Developer ID **Installer** — NOT the Application cert) → notarize →
# STAPLE (a pkg carries the offline Gatekeeper ticket bare exes cannot).
# Sign-then-hash is honored by construction: signing is IN-SCRIPT before
# the fragment is written (the macOS pattern — spec 34 §1.3's two-phase
# dance is the Windows exception, needed only because its signing is a
# workflow action).
#
# Required env: VERSION (tag minus v), PLATFORM (macos-arm64|macos-x86_64),
# SRC_DIR (the 4 staged binaries), FRAG_DIR (the downloaded
# frag-<platform> artifact — the sign-then-hash anchors; THIS release's
# SHA256SUMS does not exist yet, finalize makes it AFTER us), RUNNER_TEMP.
# Optional: PRODUCT_NAME (tebako), ORG_ID (org.tamatebako),
# INSTALL_ROOT (/opt/tebako), MIN_MACOS (12.0). Web-bootstrapper (spec 16
# §7): BOOTSTRAP_REGISTRY (the client's registry ref) + BOOTSTRAP_PAYLOADS
# (space-separated names) — stages <root>/bootstrap-seed.sh (self-locating,
# user-re-runnable) + the home/shims grammar marker (spec 05 §3.1); the
# postinstall launches the seed DETACHED (PackageKit's script budget must
# never kill a network prefetch), logged to home/seed.log. Optional third
# knob BOOTSTRAP_WARM (a subset of BOOTSTRAP_PAYLOADS): the seed also
# dispatches
# each named shim once, pulling its RUNTIME into the shared home at
# install time (as the install user). After a warm, every user's dispatch
# is read-only against the root-owned home — the runtime download is the
# only dispatch-time write (the registry refresh degrades to loud
# stale-serve, the journal is best-effort). Warm only bounded,
# print-and-exit entrypoints: the seed runs no timeout. The rehearsal
# re-dispatches the warmed shim with BOOTSTRAP_WARM_CMD (default
# --version).
# Gate env (the workflow's setup step): APPLE_INSTALLER_SIGNING_ENABLED,
# INSTALLER_SIGN_HASH, INSTALLER_KEYCHAIN; notary: APPLE_ASC_KEY_P8,
# APPLE_ASC_KEY_ID, APPLE_ASC_ISSUER_ID.
#
# Ship gates (all run in-script, before any upload): pkgutil
# --check-signature (armed), stapler validate + spctl --assess -t install
# (armed), and ALWAYS a real install rehearsal (sudo installer -target /)
# proving the payload lands and postinstall writes the paths.d entry.
set -euo pipefail

: "${VERSION:?VERSION is required (the release version, tag minus v)}"
: "${PLATFORM:?PLATFORM is required (macos-arm64|macos-x86_64)}"
: "${SRC_DIR:?SRC_DIR is required (staged binaries + SHA256SUMS)}"
PRODUCT_NAME="${PRODUCT_NAME:-tebako}"
ORG_ID="${ORG_ID:-org.tamatebako}"
INSTALL_ROOT="${INSTALL_ROOT:-/opt/tebako}"
MIN_MACOS="${MIN_MACOS:-12.0}"
TOOLS="tebako tebako-shim tfs tebako-pkg"
ASSET="${PRODUCT_NAME}-setup-${VERSION}-${PLATFORM}.pkg"

# ---- 1. stage + verify the binaries against the leg's fragments ---------
# The macOS leg's frags were computed over the SIGNED+notarized bytes
# (sign-then-hash, spec 31 §5) — the pkg wraps EXACTLY the published
# bytes; a corrupted download inside a signed installer is the failure
# this precludes.
: "${FRAG_DIR:?FRAG_DIR is required (the downloaded frag-$PLATFORM artifact)}"
rm -rf pkg-root build out
mkdir -p "pkg-root$INSTALL_ROOT/bin" build/scripts out
for tool in $TOOLS; do
  bin="${tool}-${VERSION}-${PLATFORM}"
  [ -f "$SRC_DIR/$bin" ] || { echo "::error::$bin not staged in $SRC_DIR"; exit 1; }
  [ -f "$FRAG_DIR/${tool}-${PLATFORM}.sha256" ] || { echo "::error::no fragment ${tool}-${PLATFORM}.sha256 in $FRAG_DIR — the macos leg did not complete"; exit 1; }
  want=$(cat "$FRAG_DIR/${tool}-${PLATFORM}.sha256")
  got=$(shasum -a 256 "$SRC_DIR/$bin" | cut -d' ' -f1)
  [ "$got" = "$want" ] || { echo "::error::sha256 mismatch on $bin — got $got want $want"; exit 1; }
  cp "$SRC_DIR/$bin" "pkg-root$INSTALL_ROOT/bin/$tool"
  chmod 755 "pkg-root$INSTALL_ROOT/bin/$tool"
done
echo "staged + verified against the leg's signed-byte fragments: $TOOLS"

# ---- 2. render the template tokens (productbuild has no binds) ----------
# pkgbuild/productbuild versions are numeric triplets — a roadmap-89
# rehearsal's VERSION is a PR ref ("590-merge"), not a product version.
PKG_VERSION="$VERSION"
[[ "$PKG_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || PKG_VERSION="0.0.0"
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s/@VERSION@/$PKG_VERSION/g" \
    -e "s/@ORG_ID@/$ORG_ID/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
    templates/installers/macos/distribution.xml > build/distribution.xml
sed -e "s/@PRODUCT_NAME@/$PRODUCT_NAME/g" -e "s|@INSTALL_ROOT@|$INSTALL_ROOT|g" \
    templates/installers/macos/scripts/postinstall > build/scripts/postinstall
# Optional web-bootstrapper seed (spec 16 §7; the MSI leg's contract,
# mirrored): a self-locating, user-re-runnable seed script at the install
# root + the home/shims store-grammar marker (spec 05 §3.1); the
# postinstall splices the detached-seed hook at @BOOTSTRAP_HOOK@ (the
# placeholder is deleted outright when unbound).
if [ -n "${BOOTSTRAP_REGISTRY:-}" ]; then
  : "${BOOTSTRAP_PAYLOADS:?BOOTSTRAP_PAYLOADS is required when BOOTSTRAP_REGISTRY is set (space-separated payload names)}"
  {
    echo '#!/bin/bash'
    echo "# $PRODUCT_NAME bootstrap seed (spec 16 §7) — composed at build time; safe to re-run (tebako install is idempotent)."
    # set -euo pipefail + the ERR trap: a failed add-registry/install must fail the
    # seed LOUDLY (the "seed: failed" marker the rehearsal greps) — the old 'set
    # -u' let a dead fetch fall through to a misleading "seed: complete"
    # (tebako#1078).
    echo 'set -euo pipefail'
    echo 'trap '"'"'echo "seed: failed" >&2'"'"' ERR'
    echo '# The seed may run detached from a PackageKit sandbox whose cwd is torn down at'
    echo '# install completion — never inherit a cwd (the shim dies on getcwd() otherwise).'
    echo 'cd /'
    echo 'ROOT="$(cd "$(dirname "$0")" && pwd)"'
    echo 'export TEBAKO_HOME="$ROOT/home"'
    # The detached run ALSO inherits the sandbox TMPDIR, torn down with it —
    # the add-registry git fetch died there (tebako#1078, the pkg-leg race on
    # macos-15). A tmpdir under the home survives the teardown.
    echo 'export TMPDIR="$TEBAKO_HOME/tmp"; mkdir -p "$TMPDIR"'
    echo 'mkdir -p "$TEBAKO_HOME/shims"'
    echo "echo \"seeding the $PRODUCT_NAME payload set into \$TEBAKO_HOME (network required)…\""
    echo "\"\$ROOT/bin/tebako\" add-registry \"$BOOTSTRAP_REGISTRY\""
    for p in $BOOTSTRAP_PAYLOADS; do
      echo "\"\$ROOT/bin/tebako\" install \"$p\""
    done
    # Optional warm (spec 16 §7): dispatch each named shim once so its
    # RUNTIME lands in the shared home at install time — after that a
    # user dispatch is read-only against the root-owned home.
    for w in ${BOOTSTRAP_WARM:-}; do
      case " $BOOTSTRAP_PAYLOADS " in
        *" $w "*) ;;
        *) echo "::error::BOOTSTRAP_WARM entry $w is not in BOOTSTRAP_PAYLOADS — warm is a subset"; exit 1 ;;
      esac
      echo "\"\$ROOT/home/shims/$w\" || echo \"warning: warm dispatch of $w failed (offline?) — the first user run downloads its runtime\" >&2"
    done
    # Completion marker (the rehearsal poll's readiness signal): the
    # dependency closure's spawned runtimes land during `tebako install`,
    # so the store's effects alone do NOT mean the warm finished — a
    # dispatch that races the warm hits the root-held runtime install
    # locks (EACCES in the root-owned home). The line lands in
    # home/seed.log on the detached install-time run.
    echo 'echo "seed: complete"'
  } > "pkg-root$INSTALL_ROOT/bootstrap-seed.sh"
  chmod 755 "pkg-root$INSTALL_ROOT/bootstrap-seed.sh"
  mkdir -p "pkg-root$INSTALL_ROOT/home/shims"
  cat > build/bootstrap-hook <<EOF

# --- install-time seed, detached: PackageKit kills postinstall scripts that
# overrun its budget, so the network prefetch runs in the background (log:
# $INSTALL_ROOT/home/seed.log). A failed seed never fails the install —
# the first dispatch self-heals; the user may re-run the seed manually.
printf '%s\n' "$INSTALL_ROOT/home/shims" >> "/etc/paths.d/$PRODUCT_NAME"
nohup "$INSTALL_ROOT/bootstrap-seed.sh" > "$INSTALL_ROOT/home/seed.log" 2>&1 & disown
EOF
  echo "bootstrap seed staged: registry $BOOTSTRAP_REGISTRY — payloads: $BOOTSTRAP_PAYLOADS"
else
  : > build/bootstrap-hook
fi
sed -e "/@BOOTSTRAP_HOOK@/r build/bootstrap-hook" -e "/@BOOTSTRAP_HOOK@/d" \
    build/scripts/postinstall > build/scripts/postinstall.rendered
mv build/scripts/postinstall.rendered build/scripts/postinstall
# The splice recreates the file (umask 644) — the exec bit goes LAST, on
# the file that actually ships (an un-executable postinstall fails the
# install: "an error occurred while running scripts from the package").
chmod +x build/scripts/postinstall
if grep -qE '@(PRODUCT_NAME|VERSION|ORG_ID|MIN_MACOS|INSTALL_ROOT|BOOTSTRAP_HOOK)@' build/distribution.xml build/scripts/postinstall; then
  echo "::error::unrendered template token left in the pkg inputs — the template drifted from this script"; exit 1
fi

# ---- 3. pkgbuild → productbuild ------------------------------------------
# --root is pkg-root, not pkg-root$INSTALL_ROOT: the payload must KEEP its
# opt/<name>/bin prefix under --install-location / (landing at
# /opt/<name>/bin on the data volume). Rooting at the INSTALL_ROOT subdir
# drops that prefix and the payload lands at /bin — the sealed system
# volume, which installer(8) refuses outright (the v2.8.0 legs).
pkgbuild --root pkg-root --install-location / \
  --identifier "$ORG_ID.$PRODUCT_NAME.pkg" --version "$PKG_VERSION" \
  --scripts build/scripts \
  "build/$PRODUCT_NAME-component.pkg"
productbuild --distribution build/distribution.xml \
  --package-path build "out/$ASSET.unsigned"
mv "out/$ASSET.unsigned" "out/$ASSET"

# ---- 4. sign + notarize + staple (the armed path) -------------------------
if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-false}" = "true" ]; then
  : "${INSTALLER_SIGN_HASH:?the setup step did not export INSTALLER_SIGN_HASH}"
  : "${INSTALLER_KEYCHAIN:?the setup step did not export INSTALLER_KEYCHAIN}"
  mv "out/$ASSET" "out/$ASSET.unsigned"
  # productsign reaches Apple's timestamp authority; a transient blip
  # fails the CMS encoding with -67882 ("A timestamp was expected but
  # was not found") and kills the leg — the metanorma v1.18.6-1 respin's
  # x86_64 pkg leg lost twice to it (tebako#1084). Retry only that
  # signature, on a short ladder; any other failure stands immediately
  # (a broken identity or unsigned input must not hide behind retries).
  ps_attempt=1
  while true; do
    if productsign --keychain "$INSTALLER_KEYCHAIN" --sign "$INSTALLER_SIGN_HASH" \
      "out/$ASSET.unsigned" "out/$ASSET" 2>&1 | tee build/productsign.txt; then
      break
    fi
    if grep -qE -- '-67882|timestamp' build/productsign.txt && [ "$ps_attempt" -lt 5 ]; then
      echo "productsign hit the timestamp blip (-67882) — retry $ps_attempt of 4 in 45 s"
      ps_attempt=$((ps_attempt + 1))
      sleep 45
      continue
    fi
    echo "::error::productsign failed for $ASSET (not the timestamp blip, or 5 attempts exhausted)"
    exit 1
  done
  pkgutil --check-signature "out/$ASSET" | tee build/pkg-sig.txt
  grep -q "Developer ID Installer" build/pkg-sig.txt \
    || { echo "::error::pkg not signed by a Developer ID Installer identity"; exit 1; }
  grep -q 'Status: signed' build/pkg-sig.txt || { echo "::error::pkgutil reports the pkg unsigned"; exit 1; }

  work="$RUNNER_TEMP/tebako-pkg-notarize"
  mkdir -p "$work"
  printf '%s' "$APPLE_ASC_KEY_P8" > "$work/AuthKey.p8"
  xcrun notarytool submit "out/$ASSET" --key "$work/AuthKey.p8" \
    --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" \
    --wait --timeout 20m | tee "$work/notary.txt"
  grep -q 'status: Accepted' "$work/notary.txt" || {
    xcrun notarytool log "$(grep -m1 -oE '[0-9a-f-]{36}' "$work/notary.txt")" \
      --key "$work/AuthKey.p8" --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" 2>&1 | tail -20
    echo "::error::notarytool did not Accept the pkg"; exit 1; }
  # staple fetches the online ticket — the same Accepted≠visible race as
  # the binaries' notarization check: bounded poll, then a named failure.
  staple_deadline=$(( $(date +%s) + 300 ))
  until xcrun stapler staple "out/$ASSET"; do
    [ "$(date +%s)" -lt "$staple_deadline" ] || { echo "::error::stapler could not fetch the ticket for $ASSET within 300s"; exit 1; }
    echo "ticket not yet fetchable for $ASSET — polling again in 10 s"
    sleep 10
  done
  # validate re-checks the stapled ticket against Apple's ticket-delivery
  # service (api.apple-cloudkit.com) and dies on transient network blips
  # (NSURLError -1005 — the v2.8.30 and v2.8.32 x86_64 pkg legs both lost
  # to it). Retry the network signature; any other verdict stands
  # immediately (a genuinely bad staple must not hide behind retries).
  vdeadline=$(( $(date +%s) + 300 ))
  while true; do
    xcrun stapler validate "out/$ASSET" 2>&1 | tee "$work/validate.txt" || true
    grep -q "The validate action worked" "$work/validate.txt" && break
    grep -qE "NSURLError|network connection" "$work/validate.txt" || {
      echo "::error::stapler validate failed for $ASSET (not a network blip) — the verdict stands"; exit 1; }
    [ "$(date +%s)" -lt "$vdeadline" ] || {
      echo "::error::stapler validate for $ASSET kept hitting the ticket-delivery network for 300s"; exit 1; }
    echo "validate hit the ticket-delivery network blip — retrying in 15 s"
    sleep 15
  done
  # §6.3a: the pkg's assessment IS the right spctl gate (unlike bare
  # exes): notarized + stapled must Assess accepted, source=Notarized
  # Developer ID. (spctl prints its verdict on STDERR — 2>&1 or the tee
  # captures nothing.)
  spctl --assess -t install -v "out/$ASSET" 2>&1 | tee build/spctl.txt
  grep -q "accepted" build/spctl.txt || { echo "::error::spctl rejected the notarized pkg"; exit 1; }
  echo "signed + notarized + STAPLED: $ASSET"
else
  echo "::notice::spec 31 §5a gate disarmed — the pkg is UNSIGNED (spec 00 invariant 7); the install rehearsal still gates it"
fi

# ---- 5. the install rehearsal ----------------------------------------------
# Signed pkgs (the armed path): the full install rehearsal — the trust path
# runs. Unsigned pkgs (PR legs): the rehearsal still gates the mechanics, but
# macOS 26's installer(8) refuses unsigned packages at trust evaluation
# (PKInstallErrorDomain Code=101 — the runner image moved macos-latest to
# macos-26, 20261123.0526.1; macOS 15's installer accepted them). The trust
# path is the signed legs' surface; the unsigned arm runs the STRUCTURAL
# rehearsal (the pkg's contents + the postinstall contract) and skips the
# install-time assertions (nothing installed). A refusal that is NOT the
# unsigned-signature class is a hard fail either way.
STRUCTURAL=0
if ! install_out=$(sudo installer -pkg "out/$ASSET" -target / 2>&1); then
  if printf '%s' "$install_out" | grep -qE 'is not signed|PKInstallErrorDomain Code=101'; then
    STRUCTURAL=1
    echo "::notice::unsigned pkg refused by installer(8) on this OS (trust evaluation Code=101) — the structural rehearsal runs; the trust path is the signed legs' surface"
    struct_root="$RUNNER_TEMP/pkg-struct"
    rm -rf "$struct_root"; mkdir -p "$struct_root"
    pkgutil --expand-full "out/$ASSET" "$struct_root"
    component="$struct_root/$PRODUCT_NAME-component.pkg"
    # expand-full leaves Payload as an unpacked DIRECTORY tree (not the
    # gzipped cpio) — assert the tree directly, never re-decompress it.
    [ -x "$component/Payload/opt/$PRODUCT_NAME/bin/tebako" ] || { echo "::error::the pkg Payload carries no executable opt/$PRODUCT_NAME/bin/tebako"; exit 1; }
    for tool in tebako-shim tfs tebako-pkg; do
      [ -x "$component/Payload/opt/$PRODUCT_NAME/bin/$tool" ] || { echo "::error::the pkg Payload is missing opt/$PRODUCT_NAME/bin/$tool"; exit 1; }
    done
    [ -f "$component/Scripts/postinstall" ] || { echo "::error::the pkg carries no postinstall script"; exit 1; }
    echo "structural rehearsal OK (the pkg carries the four PATH tools + the postinstall contract; the install path is the signed legs' proof)"
  else
    printf '%s\n' "$install_out"
    echo "::error::installer refused $ASSET (not the unsigned-signature class)"
    exit 1
  fi
fi
if [ "$STRUCTURAL" = 0 ]; then
# The payload lays down the toolset as bin/tebako in EVERY product shape:
# in the product's own shape (PRODUCT_NAME=tebako) and in a client
# product's alike — a client's product name lives on its seeded shims
# (home/shims/<product>), never on a bin/<product> binary.
[ -x "$INSTALL_ROOT/bin/tebako" ] || { echo "::error::$INSTALL_ROOT/bin/tebako missing after install"; exit 1; }
"$INSTALL_ROOT/bin/tebako" --version
[ -f "/etc/paths.d/$PRODUCT_NAME" ] || { echo "::error::/etc/paths.d/$PRODUCT_NAME missing — postinstall did not run"; exit 1; }
grep -q "$INSTALL_ROOT/bin" "/etc/paths.d/$PRODUCT_NAME" || { echo "::error::/etc/paths.d/$PRODUCT_NAME content wrong"; exit 1; }
if [ -n "${BOOTSTRAP_REGISTRY:-}" ]; then
  [ -x "$INSTALL_ROOT/bootstrap-seed.sh" ] || { echo "::error::bootstrap-seed.sh missing after install"; exit 1; }
  grep -q "$INSTALL_ROOT/home/shims" "/etc/paths.d/$PRODUCT_NAME" || { echo "::error::/etc/paths.d/$PRODUCT_NAME lacks the shims line"; exit 1; }
  # tebako#290: the detached seed rides UNAUTHENTICATED api.github.com, and
  # a shared CI egress IP exhausts the 60/hr anonymous budget — the
  # registry fetch dies throttled-403 (six backoff rounds), the effects
  # never land, and the poll below burns its 1800s budget (PR #288's pkg
  # leg, run 43957753605; pmn#292's x86_64 leg, run 44261962036).
  # tebako-http already honors GITHUB_TOKEN for api.github.com reads, so
  # when the workflow env carries one, run the same steps authenticated,
  # as root, BEFORE polling: `tebako install` is idempotent, the store's
  # flocks serialize the two passes, and the pass warms the 24h registry
  # cache — the detached seed's own add-registry then reads the cache and
  # completes. The pass is best-effort: a persistent failure falls through
  # to the poll, which stays the gate. On a real user's machine no token
  # exists and nothing here runs — the detached seed is the only path.
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    echo "== authenticated seed pass (CI de-flake, tebako#290; the detached seed still proves the real-user path)"
    seed_try() { # <cmd...> — 6 attempts with linear backoff, then soft-fail
      local i
      for i in 1 2 3 4 5 6; do
        sudo TEBAKO_HOME="$INSTALL_ROOT/home" TEBAKO_GITHUB_TOKEN="$GITHUB_TOKEN" "$@" && return 0
        sleep $((i * 10))
      done
      return 1
    }
    seed_try "$INSTALL_ROOT/bin/tebako" add-registry "$BOOTSTRAP_REGISTRY" || echo "::warning::authenticated add-registry did not land — the poll remains the gate"
    for p in $BOOTSTRAP_PAYLOADS; do
      seed_try "$INSTALL_ROOT/bin/tebako" install "$p" || echo "::warning::authenticated install of $p did not land — the poll remains the gate"
    done
    for w in ${BOOTSTRAP_WARM:-}; do
      [ -x "$INSTALL_ROOT/home/shims/$w" ] && seed_try "$INSTALL_ROOT/home/shims/$w" ${BOOTSTRAP_WARM_CMD:---version} || echo "::warning::authenticated warm of $w did not land — the poll remains the gate"
    done
  fi
  # The seed runs DETACHED at install time (PackageKit's script budget), so
  # its effects land asynchronously — poll for them with a generous budget,
  # and on timeout dump the seed log + the installer log (the runner is
  # networked). The effects alone race the WARM: the closure's spawned
  # runtimes land during `tebako install`, so a non-empty runtimes/ does
  # not mean the warm finished — a rehearsal dispatch that fires mid-warm
  # hits the root-held runtime install lock and dies on EACCES in the
  # root-owned home. The seed's "seed: complete" line in seed.log is the
  # readiness signal (a heavy payload closure can take many minutes).
  # Budget: the v1.18.6-2 tag build's arm64 leg burned the full 1800s
  # mid-install (run 44276073858 — the seed was healthy, the closure is
  # just that big on a loaded runner: the runtime pair + the payload +
  # the spawned openjdk/python pairs + the warm dispatch's first-run
  # resolution). 3600s, overridable for a bigger future closure.
  deadline=$(( $(date +%s) + ${SEED_BUDGET:-3600} ))
  while :; do
    seeded=1
    [ -f "$INSTALL_ROOT/home/config.yaml" ] || seeded=0
    grep -q 'seed: complete' "$INSTALL_ROOT/home/seed.log" 2>/dev/null || seeded=0
    for p in $BOOTSTRAP_PAYLOADS; do
      [ -d "$INSTALL_ROOT/home/payloads/$p" ] || seeded=0
    done
    for w in ${BOOTSTRAP_WARM:-}; do
      [ -x "$INSTALL_ROOT/home/shims/$w" ] || seeded=0
      { [ -d "$INSTALL_ROOT/home/runtimes" ] && [ -n "$(ls -A "$INSTALL_ROOT/home/runtimes" 2>/dev/null)" ]; } || seeded=0
    done
    [ "$seeded" = 1 ] && break
    # A loudly-failed seed (the ERR trap's marker) fails the rehearsal NOW —
    # never spin the full budget on a dead seed (tebako#1078).
    if grep -q 'seed: failed' "$INSTALL_ROOT/home/seed.log" 2>/dev/null; then
      echo "== the detached seed FAILED — seed.log tail:"; sudo tail -30 "$INSTALL_ROOT/home/seed.log" 2>/dev/null || true
      echo "::error::the detached seed failed (seed.log above) — the rehearsal's store is incomplete"; exit 1
    fi
    if [ "$(date +%s)" -ge "$deadline" ]; then
      echo "== the detached seed did not land within ${SEED_BUDGET:-3600}s — seed.log tail:"; sudo tail -30 "$INSTALL_ROOT/home/seed.log" 2>/dev/null || true
      echo "== install.log tail:"; sudo grep -a "$PRODUCT_NAME" /var/log/install.log 2>/dev/null | tail -20 || true
      # tebako#1147 forensics: name the seed's STATE, never a bare
      # timeout. A silent full-budget stall is the anonymous
      # api.github.com throttle — the shared CI egress drains the 60/h
      # budget and the seed sleeps in tebako-http's backoff honoring the
      # server's reset window (its wait lines land in seed.log). The
      # rate_limit probe is anonymous too: remaining=0 + the reset epoch
      # confirm the drained budget at this egress.
      echo "== seed process state:"; pgrep -fl bootstrap-seed 2>/dev/null || echo "no bootstrap-seed process — the seed died without writing a marker"
      ps -axo pid,ppid,stat,etime,command | grep -E 'bootstrap-seed|bin/tebako' | grep -v grep || true
      echo "== home lock state:"; sudo ls -l "$INSTALL_ROOT/home/locks" 2>/dev/null || true
      echo "== anonymous api.github.com budget at this egress:"; curl -s --max-time 15 https://api.github.com/rate_limit 2>/dev/null | grep -E '"remaining"|"reset"' | head -8 || true
      if grep -q 'throttled by the host' "$INSTALL_ROOT/home/seed.log" 2>/dev/null; then
        echo "::error::the detached seed is THROTTLE-STALLED (seed.log shows tebako's rate-limit wait) — the anonymous api.github.com budget is drained at this egress IP; the window resets hourly, the authenticated pass does not drain it"; exit 1
      fi
      echo "::error::the detached seed's effects did not land within ${SEED_BUDGET:-3600}s (forensics above)"; exit 1
    fi
    sleep 5
  done
  if [ -n "${BOOTSTRAP_WARM:-}" ]; then
    # The warm dispatch cached each warmed payload's RUNTIME at install
    # time: the shared home is complete and a user's dispatch is read-only.
    # The warmed shim is the client product's user-facing command: it must
    # dispatch read-only against the warmed home (the poll above proved the
    # cold path; this proves the installed state).
    for w in $BOOTSTRAP_WARM; do
      TEBAKO_HOME="$INSTALL_ROOT/home" "$INSTALL_ROOT/home/shims/$w" ${BOOTSTRAP_WARM_CMD:---version} >/dev/null \
        || { echo "::error::the warmed shim $w did not dispatch (${BOOTSTRAP_WARM_CMD:---version})"; exit 1; }
    done
    echo "warm rehearsal OK (runtimes cached: $(ls "$INSTALL_ROOT/home/runtimes" | tr '\n' ' '); shims dispatch: $BOOTSTRAP_WARM)"
  fi
  echo "bootstrap seed rehearsal OK (registry registered, payloads cached, shims on PATH)"
fi
sudo rm -rf "$INSTALL_ROOT" "/etc/paths.d/$PRODUCT_NAME"
echo "install rehearsal OK (payload lands, postinstall writes the paths.d entry, the installed tool runs)"
fi

# ---- 6. the fragment over the FINAL bytes (ship-armed runs only) --------
# A disarmed run still builds + rehearses the pkg (the template's CI
# signal) but ships NOTHING — an unsigned installer container never lands
# on the release (the container's audience is exactly the one SmartScreen
# / Gatekeeper warnings burn; the loose binaries remain the
# unsigned-first-class path, spec 00 invariant 7). No ship → no frag:
# finalize folds whatever frags exist into SHA256SUMS, and a frag without
# its asset is broken data.
if [ "${APPLE_INSTALLER_SIGNING_ENABLED:-false}" = "true" ]; then
  mkdir -p "fragments/frag-installers-$PLATFORM"
  shasum -a 256 "out/$ASSET" | cut -d' ' -f1 > "fragments/frag-installers-$PLATFORM/$ASSET.sha256"
  stat -f %z "out/$ASSET" > "fragments/frag-installers-$PLATFORM/$ASSET.size"
  echo "frag: $(cat fragments/frag-installers-$PLATFORM/$ASSET.sha256)  $ASSET"
fi

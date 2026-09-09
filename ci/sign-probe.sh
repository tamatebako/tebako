#!/bin/bash
# ci/sign-probe.sh — spec 31 credential pre-flight (tebako#522 follow-up).
# Proves: p12 imports, codesign+hardened runtime works, notarytool accepts,
# Gatekeeper assesses Notarized Developer ID, and the SIGNED runtime still
# JITs (the allow-jit canary). Throwaway: runs only on the ci/sign-probe
# branch; nothing here touches the release workflows or the enablement gate.
set -euo pipefail
cd "$RUNNER_TEMP"
step() { echo; echo "=== $*"; }
fail() { echo "SIGN-PROBE FAIL: $*" >&2; exit 1; }

step "1. fetch the artifacts (bootstrap 2.5.0 + ruby runtime 0.16.22-4.0.6)"
GH_TOKEN="$TEBAKO_CI_PAT_TOKEN" gh release download v2.5.0 --repo tamatebako/tebako \
  --pattern 'tebako-bootstrap-2.5.0-macos-arm64*' --dir . --clobber
GH_TOKEN="$TEBAKO_CI_PAT_TOKEN" gh release download v0.16.22 \
  --repo tamatebako/tebako-runtime-ruby \
  --pattern 'tebako-runtime-0.16.22-4.0.6-macos-arm64*' --dir . --clobber
shasum -a 256 -c tebako-bootstrap-2.5.0-macos-arm64.sha256
shasum -a 256 -c tebako-runtime-0.16.22-4.0.6-macos-arm64.sha256
shasum -a 256 -c tebako-runtime-0.16.22-4.0.6-macos-arm64.tfs.sha256
chmod +x tebako-bootstrap-2.5.0-macos-arm64 tebako-runtime-0.16.22-4.0.6-macos-arm64

step "2. keychain + identity"
printf '%s' "$APPLE_DEVELOPER_ID_P12" | base64 -d > devid.p12
KC="$RUNNER_TEMP/probe.keychain"
security create-keychain -p probe-kc-pass "$KC"
security unlock-keychain -p probe-kc-pass "$KC"
# codesign resolves identities through the user search list — a freshly
# created keychain is not on it (the attempt-2 "item could not be found").
security list-keychains -d user -s "$KC" $(security list-keychains -d user | tr -d '"')
security import devid.p12 -k "$KC" -P "$APPLE_DEVELOPER_ID_P12_PASSWORD" \
  -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple: -k probe-kc-pass "$KC" >/dev/null
IDENT=$(security find-identity -v -p codesigning "$KC" | grep -o '"Developer ID Application[^"]*"' | tr -d '"' | head -1)
[ -n "$IDENT" ] || fail "no Developer ID Application identity in the p12"
echo "identity: $IDENT"

step "3. entitlements (spec 31 §3 vocabulary — the runtime pair)"
cat > runtime.entitlements <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>com.apple.security.cs.disable-library-validation</key><true/>
  <key>com.apple.security.cs.allow-jit</key><true/>
</dict></plist>
XML

step "4. baseline functional (unsigned)"
export TEBAKO_OFFLINE=1 TEBAKO_RUNTIME_IMAGE="$RUNNER_TEMP/tebako-runtime-0.16.22-4.0.6-macos-arm64.tfs"
RUBY_YJIT_ENABLE=1 ./tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  -e 'puts "unsigned yjit=#{RubyVM::YJIT.enabled?}"'
RUBY_ZJIT_ENABLE=1 ./tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  -e 'puts "unsigned zjit=#{defined?(RubyVM::ZJIT) ? RubyVM::ZJIT.enabled? : "not-compiled"}"'
./tebako-bootstrap-2.5.0-macos-arm64 --version > baseline.boot.txt 2>&1 || echo "bootstrap rc=$?" >> baseline.boot.txt
cat baseline.boot.txt

step "5. codesign (hardened runtime; runtime exe gets the entitlements)"
# sign by cert SHA-1, never by name — a duplicate identity in any search-listed
# keychain makes name resolution ambiguous (proven locally on the owner's box).
HASH=$(security find-identity -v -p codesigning "$KC" | awk '/Developer ID Application/ {print $2; exit}')
[ -n "$HASH" ] || fail "no Developer ID codesigning hash"
echo "signing with cert hash: ${HASH:0:10}…"
codesign --force --options runtime --timestamp --keychain "$KC" \
  --sign "$HASH" tebako-bootstrap-2.5.0-macos-arm64
codesign --force --options runtime --timestamp --keychain "$KC" \
  --entitlements runtime.entitlements --sign "$HASH" tebako-runtime-0.16.22-4.0.6-macos-arm64
codesign --verify --deep --strict --verbose=1 tebako-bootstrap-2.5.0-macos-arm64
codesign --verify --deep --strict --verbose=1 tebako-runtime-0.16.22-4.0.6-macos-arm64
codesign -d --entitlements :- tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  > embedded.entitlements.xml 2> embedded.entitlements.err || true
echo "--- embedded entitlements (stdout) ---"; cat embedded.entitlements.xml
echo "--- embedded entitlements (stderr) ---"; cat embedded.entitlements.err
TRUES=$(grep -o '<true/>' embedded.entitlements.xml | wc -l | tr -d ' ')
echo "embedded <true/> count: $TRUES"
[ "$TRUES" = "2" ] || fail "entitlements not embedded"

step "6. functional on SIGNED binaries (pre-notary)"
RUBY_YJIT_ENABLE=1 ./tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  -e 'puts "signed yjit=#{RubyVM::YJIT.enabled?}"' | tee signed.yjit.txt
grep -q 'signed yjit=true' signed.yjit.txt || fail "allow-jit entitlement NOT effective on signed exe"
RUBY_ZJIT_ENABLE=1 ./tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  -e 'puts "signed zjit=#{RubyVM::ZJIT.enabled?}"' | tee signed.zjit.txt
grep -q 'signed zjit=true' signed.zjit.txt || fail "zjit dead on signed exe"
./tebako-bootstrap-2.5.0-macos-arm64 --version > signed.boot.txt 2>&1 || echo "bootstrap rc=$?" >> signed.boot.txt
diff baseline.boot.txt signed.boot.txt || fail "signed bootstrap behavior drifted"

step "7. notarize (the credential end-to-end)"
printf '%s' "$APPLE_ASC_KEY_P8" > AuthKey.p8
zip -q probe.zip tebako-bootstrap-2.5.0-macos-arm64 tebako-runtime-0.16.22-4.0.6-macos-arm64
xcrun notarytool submit probe.zip --key AuthKey.p8 --key-id "$APPLE_ASC_KEY_ID" \
  --issuer "$APPLE_ASC_ISSUER_ID" --wait --timeout 20m | tee notary.txt
grep -q 'status: Accepted' notary.txt || { xcrun notarytool log "$(grep -m1 -oE '[0-9a-f-]{36}' notary.txt)" --key AuthKey.p8 --key-id "$APPLE_ASC_KEY_ID" --issuer "$APPLE_ASC_ISSUER_ID" 2>&1 | tail -20; fail "notarytool not Accepted"; }

step "8. staple + Gatekeeper assess"
# stapler cannot attach tickets to bare Mach-O binaries ("incapable of working
# with Document files") — stapling exists for bundles/pkg/dmg containers.
# Gatekeeper still assesses our binaries as Notarized Developer ID ONLINE via
# the accepted submission ticket (that assessment is the credential proof).
# If offline-stapled distribution ever matters, the artifact shape would have
# to change to a .pkg container — not our three-part exe model.
spctl -a -vvv -t execute tebako-bootstrap-2.5.0-macos-arm64 2>&1 | tee spctl.boot.txt
spctl -a -vvv -t execute tebako-runtime-0.16.22-4.0.6-macos-arm64 2>&1 | tee spctl.rt.txt
grep -q 'source=Notarized Developer ID' spctl.boot.txt || fail "bootstrap not Notarized Developer ID"
grep -q 'source=Notarized Developer ID' spctl.rt.txt || fail "runtime exe not Notarized Developer ID"

step "9. functional post-notary (stapled)"
RUBY_YJIT_ENABLE=1 ./tebako-runtime-0.16.22-4.0.6-macos-arm64 \
  -e 'puts "stapled yjit=#{RubyVM::YJIT.enabled?}"' | grep -q 'yjit=true' || fail "yjit died post-staple"

echo
echo "SIGN-PROBE PASS: credentials valid; sign->notarize->staple->assess green; yjit/zjit alive on the signed runtime"

# Test fixtures

`subkeyed-release-shape.{key,pub}.asc` — an OpenPGP keypair in the
release key's shape: a certify-only Ed25519 primary
(`58CF65380FB5C5A8FA7239DD50849A8E5658E47A`) plus a signing Ed25519
subkey (`2B09411497507C279F85F41E2D6DF607DFC5AAD5`). No passphrase,
test-only, used by nothing else.

Why a fixture: librnp's own keygen leaves an EdDSA primary
signing-capable regardless of usage flags, so an in-process mint cannot
reproduce the release shape; gpg's can. Regenerate with:

```
gpg --batch --gen-key <<'EOF'
Key-Type: eddsa
Key-Curve: ed25519
Key-Usage: cert
Name-Real: tebako-signer test fixture (subkeyed release shape)
Name-Email: fixture@tebako.invalid
Expire-Date: 0
Subkey-Type: eddsa
Subkey-Curve: ed25519
Subkey-Usage: sign
%no-protection
%commit
EOF
```

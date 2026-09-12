# Installer templates (roadmap 83, spec 16 §7)

One parameterized template per OS-native installer container. TWO
identities consume the SAME files — no forks:

- **Case A — tebako the system:** the tamatebako/tebako release pipeline
  (`installer-msi` / `installer-pkg` legs in release.yml) binds the
  tamatebako identity and signs with the tamatebako Azure Trusted Signing
  account / Apple Developer ID Installer certificate.
- **Case B — a client product (packed-mn class):** the client's CI binds
  its own product name, identifiers, upgrade code, install root — and
  signs all three surfaces with ITS OWN credentials (its Azure account,
  its Apple Developer ID certs, its OpenPGP key). Nothing in these
  templates names tamatebako except defaults in the pipeline scripts.

## windows/tebako.wxs (WiX v4/v5)

Build: `wix build -arch x64 tebako.wxs -d ProductName=… -d
ProductVersion=… -d Manufacturer=… -d UpgradeCode=… -d BinDir=<dir of
staged *.exe> -o <name>.msi`

| Bind | tebako value | Client MUST change |
|---|---|---|
| ProductName | `tebako` | yes (their product) |
| ProductVersion | release version (3-part) | theirs |
| Manufacturer | `tamatebako` | yes (registry key root) |
| UpgradeCode | `9A404514-715E-44F6-B02E-B800845759A9` (generated 2026-09-12, LOCKED — never edit) | **yes — mint a new GUID and lock it forever** |
| BinDir | the release's signed exes | their staged bundle bin/ |

Everything in `BinDir/*.exe` installs to `%ProgramFiles%\<ProductName>\bin`
and the system PATH gains that dir (part=last, removed on uninstall).

## macos/distribution.xml + macos/scripts/postinstall

`ci/macos-pkg-build.sh` renders the `@TOKENS@` (productbuild has no bind
mechanism), then: pkgbuild (component pkg from the staged root
`opt/<name>/bin/*` + the postinstall that writes
`/etc/paths.d/<name>`) → productbuild → (armed) productsign with the
**Developer ID Installer** identity → notarize → **staple**.

| Token / knob | tebako value | Client MUST change |
|---|---|---|
| `@PRODUCT_NAME@` | `tebako` | yes |
| `@VERSION@` | release version | theirs |
| `@ORG_ID@` | `org.tamatebako` | yes (pkg identifier prefix) |
| `@MIN_MACOS@` | `12.0` | theirs |
| `@INSTALL_ROOT@` | `/opt/tebako` | theirs |

## The signing credentials (owner provisioning, per identity)

Windows (per publisher): an Azure Trusted Signing account + cert profile
+ the Entra app registration with a federated credential scoped to
`<org>@<org-id>/<repo>@<repo-id>:environment:windows-signing` (the
immutable-subject form). Repo: secrets `AZURE_CLIENT_ID` /
`AZURE_TENANT_ID` / `AZURE_SUBSCRIPTION_ID`, vars `AZURE_ENDPOINT` /
`AZURE_SIGNING_ACCOUNT_NAME` / `AZURE_SIGNING_CERT_PROFILE`, gate var
`WINDOWS_SIGNING_ENABLED=true`.

macOS (per publisher): **two distinct certificates** — Developer ID
Application (signs the binaries inside; spec 31 §5's existing
`APPLE_DEVELOPER_ID_P12`) and **Developer ID Installer** (productsign's
identity — the pkg container). Plus the App Store Connect API key for
notarytool. Repo: secrets `APPLE_DEVELOPER_ID_INSTALLER_P12` /
`APPLE_DEVELOPER_ID_INSTALLER_P12_PASSWORD` beside the spec-31 set, gate
var `APPLE_INSTALLER_SIGNING_ENABLED=true` (pkg ships only when BOTH
Apple gates are armed — a signed-but-unnotarized pkg never ships).

### Provisioning the Developer ID Installer certificate (owner, Apple portal)

1. developer.apple.com → Certificates, Identifiers & Profiles →
   Certificates → **+**.
2. Choose **Developer ID Installer** (NOT Application — the two are
   different types; the Application one cannot productsign).
3. Upload a NEW CSR (Keychain Access → Certificate Assistant → Request
   from a CA — one CSR per certificate; Apple rejects a reused CSR).
4. Download the .cer; in Keychain Access export cert+key as .p12 with a
   password; `base64 -i cert.p12` → the `APPLE_DEVELOPER_ID_INSTALLER_P12`
   secret; the password → `APPLE_DEVELOPER_ID_INSTALLER_P12_PASSWORD`.

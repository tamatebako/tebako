# Enterprise networking — proxies and custom CAs

Everything tebako downloads (runtimes, payloads, registry indexes) rides
one HTTP client with one rule set. If your machine sits behind a
corporate proxy or a TLS-intercepting appliance, this page is the whole
story.

You need this if `tebako install` fails with a connection error, a 407,
or a certificate verification error on a network where your browser
works fine.

## Proxies

The standard environment variables are honored, upper or lower case:

```
HTTPS_PROXY=http://proxy.corp:3128
HTTP_PROXY=http://proxy.corp:3128        # plain-http endpoints
ALL_PROXY=http://proxy.corp:3128         # fallback for any scheme
NO_PROXY=localhost,127.0.0.1,.internal.corp
```

- http/https proxies via CONNECT. SOCKS URLs are refused with a named
  error — connect a SOCKS-only network through a local CONNECT relay
  (e.g. your IT department's) instead.
- `NO_PROXY` takes a comma list of exact hosts, `.suffix` entries
  (matches the suffix's subdomains, not a lookalike domain), or `*`.
  localhost is always direct.
- Proxy credentials ride the URL: `http://user:pass@proxy.corp:3128`.
  If the proxy answers 407 without them, the fetch fails with a named
  `proxy authentication required` error that tells you exactly where to
  put the credentials.

Prefer a file over environment? The same knobs live in
`~/.tebako/config.yaml`:

```yaml
network:
  proxy: http://user:pass@proxy.corp:3128
```

Environment wins over the file, per key. With nothing set anywhere, the
default is a direct connection — unchanged from before.

## Custom certificate authorities

TLS trust is never *less* than the bundled Mozilla root set. There is no
verify-off spelling, anywhere. You have two ways to add trust, and they
are mutually exclusive by design (a named error, not a silent pick):

**Add your CA to the bundled roots** — for a fixed corporate root you
can distribute as a file:

```
TEBAKO_EXTRA_CA=/etc/pki/corp-root.pem                    # one file
TEBAKO_EXTRA_CA=/etc/pki/corp-root.pem:/etc/pki/corp-intermediate.pem
```

The value is an OS path list (`:`-separated on unix/macOS, `;`-separated
on Windows). Each file is PEM; a file holding a chain (root +
intermediates) parses whole. An unreadable or malformed file is a named
startup error, never a mid-fetch surprise.

**Trust the OS store** — for roots your fleet management already pushes
(GPO on Windows, MDM on macOS):

```
TEBAKO_TLS_PLATFORM_ROOTS=1
```

The platform verifier answers exactly what the OS answers, so
enterprise-managed roots just work.

Both spellings exist in the config file too:

```yaml
network:
  tls_roots: platform            # the OS store
  # — OR —
  extra_ca: [/etc/pki/corp-root.pem]
```

`tls_roots: platform` and `extra_ca` together are refused: the platform
verifier cannot take added roots, so pick one trust story.

## The trust bridge — your payloads follow the same roots

The resolution you configure for downloads does not stop at the loader.
The effective proxy/TLS verdict rides the runtime handoff environment,
so the payload's own TLS stacks (a Ruby gem fetching over HTTPS, a
Python library) resolve against the same roots. No runtime rebuild, no
payload-side configuration — set it once for tebako and the whole
composition follows.

## Diagnosis

`tebako doctor` probes TLS against your effective configuration and
names what it finds:

- chain verifies → you're done;
- rejected by the effective roots but accepted by the platform store →
  a TLS-intercepting proxy is in the path; the doctor prints the
  remediation (`tls_roots: platform` or `TEBAKO_EXTRA_CA`);
- verifies under neither → do not bypass; investigate before trusting.

Every non-default resolution is also journaled at install time, so a
support ticket can start from the audit lines instead of guesswork.

## The size-gated bootstrap

The standalone bootstrap binary (the < 3 MB loader inside stitched
packages) compiles this feature out to hold its size gate. If a proxy or
CA variable is set when it runs, it stops with a named
`networking compiled out` error instead of silently ignoring your
configuration — pre-seed the store with the full `tebako` toolchain
(which honors everything on this page), or let it run where the default
direct connection works.

# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 121.250 | 120.247 | 158.781 | 19.884 | 135.247 | 149.766 | 1205.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 143.117 | 142.646 | 143.588 | 0.666 | 143.117 | 158.964 | 1305.9 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 61.742 | 61.520 | 97.369 | 15.925 | 68.886 | 80.499 | 894.1 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 97.942 | 97.711 | 98.173 | 0.327 | 97.942 | 86.690 | 895.9 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-gnu-x86_64

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 69) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 69) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log

Failed runs:

- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 146.646 | 126.901 | 150.901 | 9.461 | 142.103 | 154.226 | 2010.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 173.515 | 167.373 | 179.657 | 8.687 | 173.515 | 145.734 | 2046.1 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 139.275 | 135.721 | 178.206 | 20.478 | 151.717 | 168.266 | 1480.0 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 161.178 | 159.862 | 162.494 | 1.861 | 161.178 | 177.773 | 1666.6 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.72/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.72/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 75) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.72/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.72/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 75) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

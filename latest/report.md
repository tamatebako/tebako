# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 103.496 | 103.125 | 105.669 | 1.052 | 103.873 | 122.227 | 1159.5 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 111.082 | 110.615 | 111.549 | 0.661 | 111.082 | 130.024 | 1252.0 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.697 | 50.138 | 51.287 | 0.458 | 50.740 | 62.126 | 845.4 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.002 | 54.861 | 55.143 | 0.199 | 55.002 | 67.259 | 848.3 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 112.953 | 111.378 | 114.301 | 1.128 | 113.084 | 139.211 | 1202.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 124.469 | 123.608 | 125.329 | 1.217 | 124.469 | 150.769 | 1330.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 57.576 | 57.106 | 58.651 | 0.607 | 57.733 | 75.027 | 893.4 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 63.826 | 63.418 | 64.234 | 0.577 | 63.826 | 82.232 | 894.3 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 183.288 | 160.979 | 237.956 | 34.801 | 199.033 | 191.073 | 2052.8 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 178.817 | 177.058 | 180.576 | 2.488 | 178.817 | 187.144 | 2091.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 127.623 | 116.315 | 143.297 | 11.852 | 129.358 | 130.821 | 1572.5 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 156.493 | 143.560 | 169.425 | 18.289 | 156.493 | 160.285 | 1609.3 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 126.287 | 125.726 | 128.004 | 1.018 | 126.636 | 155.344 | 1559.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 135.911 | 135.247 | 136.575 | 0.939 | 135.911 | 164.953 | 1588.9 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 61.801 | 59.400 | 67.041 | 3.122 | 62.183 | 86.250 | 1592.5 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 68.379 | 67.686 | 69.072 | 0.980 | 68.379 | 89.125 | 1561.5 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

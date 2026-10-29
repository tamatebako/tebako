# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 105.192 | 104.858 | 108.212 | 1.377 | 105.914 | 123.572 | 1165.6 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 113.970 | 113.510 | 114.429 | 0.650 | 113.970 | 131.887 | 1251.9 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.663 | 51.349 | 51.784 | 0.185 | 51.601 | 63.778 | 848.7 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.813 | 56.649 | 56.977 | 0.232 | 56.813 | 69.032 | 846.2 | 1.00× |

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
| v1-packed-mn | 5 | 126.582 | 125.372 | 127.813 | 1.061 | 126.597 | 155.886 | 1200.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 135.868 | 135.346 | 136.390 | 0.738 | 135.868 | 165.655 | 1325.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 64.087 | 63.062 | 64.492 | 0.538 | 63.973 | 83.563 | 894.1 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 69.837 | 69.704 | 69.971 | 0.189 | 69.837 | 91.113 | 896.4 | 1.00× |

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
| v1-packed-mn | 5 | 170.558 | 153.345 | 184.518 | 11.296 | 169.646 | 180.695 | 2060.6 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 158.145 | 156.311 | 159.979 | 2.594 | 158.145 | 166.058 | 2068.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 124.790 | 112.621 | 145.627 | 12.386 | 126.073 | 128.812 | 1574.6 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 136.458 | 135.606 | 137.309 | 1.204 | 136.458 | 140.609 | 1599.0 | 1.00× |

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
| v1-packed-mn | 5 | 133.192 | 128.860 | 133.918 | 2.076 | 132.409 | 163.594 | 1591.1 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 141.823 | 141.611 | 142.035 | 0.300 | 141.823 | 174.094 | 1572.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 66.423 | 65.472 | 72.348 | 3.146 | 68.028 | 90.562 | 1563.5 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 73.392 | 73.081 | 73.702 | 0.439 | 73.392 | 98.508 | 1589.8 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/download/v2.8.67/SHA256SUMS

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

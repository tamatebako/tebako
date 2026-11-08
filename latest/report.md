# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 103.222 | 102.444 | 103.875 | 0.538 | 103.214 | 120.543 | 1166.1 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 112.029 | 110.660 | 113.397 | 1.936 | 112.029 | 127.344 | 1252.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.282 | 49.432 | 50.473 | 0.431 | 50.064 | 61.466 | 847.6 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 57.101 | 55.759 | 58.442 | 1.897 | 57.101 | 66.788 | 846.3 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 121.985 | 121.421 | 123.410 | 0.770 | 122.142 | 150.740 | 1195.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 133.468 | 132.793 | 134.144 | 0.955 | 133.468 | 162.548 | 1350.1 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 62.093 | 61.477 | 63.116 | 0.626 | 62.212 | 81.311 | 894.6 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 67.991 | 66.830 | 69.153 | 1.642 | 67.991 | 87.814 | 894.6 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 104.305 | 102.521 | 105.949 | 1.220 | 104.252 | 126.328 | 1594.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 113.973 | 111.973 | 115.973 | 2.828 | 113.973 | 136.203 | 1551.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 52.094 | 51.689 | 54.021 | 0.941 | 52.388 | 70.656 | 1599.1 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 62.596 | 59.781 | 65.412 | 3.982 | 62.596 | 74.617 | 1581.1 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

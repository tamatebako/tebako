# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.995 | 102.717 | 103.495 | 0.365 | 103.092 | 120.599 | 1155.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 110.521 | 110.220 | 110.823 | 0.426 | 110.521 | 128.417 | 1272.9 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 48.772 | 48.494 | 50.267 | 0.728 | 49.069 | 59.912 | 841.0 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.790 | 53.356 | 56.223 | 2.027 | 54.790 | 64.958 | 841.2 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15988.7 GiB
Versions: tebako 2.8.25 · runtime 0.16.32-3.3.12 · payload 1.16.9-17 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 112.473 | 108.411 | 112.960 | 1.847 | 111.511 | 137.474 | 1208.3 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 122.213 | 121.384 | 123.042 | 1.172 | 122.213 | 142.846 | 1307.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 57.712 | 56.703 | 58.267 | 0.595 | 57.560 | 74.826 | 907.5 | 1.00× |
| v2-fat | 5 | 40.997 | 39.667 | 41.836 | 0.849 | 40.982 | 60.581 | 847.0 | 1.41× |
| v2-shim | 5 | 42.084 | 40.251 | 43.526 | 1.209 | 42.081 | 62.593 | 964.2 | 1.37× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 65.011 | 64.887 | 65.135 | 0.176 | 65.011 | 84.001 | 900.7 | 1.00× |
| v2-shim | 2 | 50.382 | 49.752 | 51.012 | 0.891 | 50.382 | 67.678 | 970.5 | 1.29× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit 75): failed — exit 75 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit 75): failed — exit 75 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 75): failed — warmup: exit 75 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-musl-arm64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-musl-arm64 — no published payload
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-musl-arm64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-musl-arm64 — no published payload

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-musl-x86_64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-musl-x86_64 — no published payload
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-musl-x86_64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-musl-x86_64 — no published payload

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.25 · runtime 0.16.32-3.3.12 · payload 1.16.9-17 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 180.215 | 165.514 | 191.037 | 9.604 | 178.509 | 186.505 | 2061.3 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 181.433 | 180.103 | 182.763 | 1.881 | 181.433 | 189.144 | 2016.5 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 123.145 | 107.849 | 151.751 | 16.068 | 125.369 | 126.029 | 1669.4 | 1.00× |
| v2-fat | 5 | 60.440 | 54.498 | 64.047 | 3.551 | 59.785 | 71.415 | 999.1 | 2.04× |
| v2-shim | 5 | 52.910 | 46.516 | 58.306 | 4.531 | 52.922 | 63.949 | 1147.1 | 2.33× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 124.004 | 121.927 | 126.082 | 2.938 | 124.004 | 126.557 | 1645.8 | 1.00× |
| v2-shim | 2 | 56.411 | 52.503 | 60.319 | 5.526 | 56.411 | 66.293 | 1252.0 | 2.20× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit 75): failed — exit 75 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit 75): failed — exit 75 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 75): failed — warmup: exit 75 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for macos-x86_64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for macos-x86_64 — no published payload
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for macos-x86_64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for macos-x86_64 — no published payload

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.25 · runtime 0.16.32-3.3.12 · payload 1.16.9-17 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 129.206 | 123.637 | 145.831 | 8.599 | 131.347 | 14.188 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 133.221 | 132.326 | 134.115 | 1.265 | 133.221 | 14.164 | 642.1 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 63.370 | 61.845 | 65.382 | 1.327 | 63.674 | 14.250 | 642.2 | 1.00× |
| v2-fat | 5 | 47.630 | 46.248 | 49.938 | 1.534 | 48.157 | 0.016 | 5.7 | 1.33× |
| v2-shim | 5 | 50.288 | 49.233 | 51.545 | 0.876 | 50.312 | 0.031 | 8.7 | 1.26× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 74.473 | 70.465 | 78.481 | 5.668 | 74.473 | 14.469 | 642.2 | 1.00× |
| v2-shim | 2 | 61.923 | 61.503 | 62.343 | 0.594 | 61.923 | 0.812 | 64.8 | 1.20× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit 75): failed — exit 75 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit 75): failed — exit 75 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 75): failed — warmup: exit 75 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15988.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

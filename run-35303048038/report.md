# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 98.377 | 97.777 | 99.532 | 0.754 | 98.558 | 115.518 | 1158.5 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 107.266 | 106.651 | 107.881 | 0.870 | 107.266 | 124.384 | 1284.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 48.105 | 47.174 | 48.558 | 0.506 | 47.979 | 58.674 | 840.3 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 52.188 | 51.681 | 52.694 | 0.717 | 52.188 | 62.043 | 839.2 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.9 · runtime 0.16.26-3.3.12 · payload 1.16.9-16 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 124.713 | 124.133 | 125.613 | 0.583 | 124.776 | 153.431 | 1198.4 | 1.00× |
| v2-fat | 5 | 110.592 | 109.191 | 114.483 | 1.984 | 111.130 | 136.077 | 1475.6 | 1.13× |
| v2-shim | 5 | 113.222 | 111.389 | 114.920 | 1.546 | 113.194 | 131.539 | 1559.8 | 1.10× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 132.667 | 132.648 | 132.687 | 0.028 | 132.667 | 162.141 | 1349.8 | 1.00× |
| v2-shim | 2 | 133.689 | 132.480 | 134.898 | 1.709 | 133.689 | 153.789 | 1546.7 | 0.99× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 62.780 | 62.344 | 63.500 | 0.459 | 62.913 | 81.945 | 907.2 | 1.00× |
| v2-fat | 5 | 42.944 | 42.216 | 43.863 | 0.614 | 43.034 | 52.768 | 857.6 | 1.46× |
| v2-shim | 5 | 45.362 | 44.895 | 45.941 | 0.405 | 45.322 | 56.127 | 901.6 | 1.38× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 68.221 | 67.721 | 68.721 | 0.707 | 68.221 | 89.025 | 905.7 | 1.00× |
| v2-fat | 2 | 37.781 | 36.939 | 38.623 | 1.190 | 37.781 | 37.759 | 850.4 | 1.81× |
| v2-shim | 2 | 53.320 | 52.210 | 54.429 | 1.570 | 53.320 | 61.192 | 913.4 | 1.28× |

Failed runs:

- compile-medium-rice / v2-fat [cold] #1 (exit 1): failed — exit 1 (expected 0)
- compile-medium-rice / v2-fat [cold] #2 (exit 1): failed — exit 1 (expected 0)

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
Versions: tebako 2.8.9 · runtime 0.16.26-3.3.12 · payload 1.16.9-16 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 150.808 | 137.457 | 173.983 | 13.297 | 153.030 | 157.131 | 2082.6 | 1.00× |
| v2-fat | 5 | 94.120 | 88.806 | 97.998 | 4.256 | 93.244 | 102.832 | 1647.0 | 1.60× |
| v2-shim | 5 | 82.648 | 77.563 | 96.527 | 7.544 | 85.568 | 86.802 | 1814.9 | 1.82× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 145.970 | 142.351 | 149.589 | 5.118 | 145.970 | 150.966 | 2003.7 | 1.00× |
| v2-shim | 2 | 105.755 | 98.485 | 113.026 | 10.282 | 105.755 | 109.993 | 1774.4 | 1.38× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 90.464 | 79.778 | 108.773 | 11.403 | 90.619 | 92.863 | 1707.5 | 1.00× |
| v2-fat | 5 | 39.095 | 37.479 | 45.181 | 3.132 | 40.574 | 42.548 | 1008.5 | 2.31× |
| v2-shim | 5 | 36.351 | 31.560 | 40.452 | 3.284 | 36.596 | 38.885 | 1136.5 | 2.49× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 107.080 | 101.230 | 112.930 | 8.273 | 107.080 | 111.430 | 1647.0 | 1.00× |
| v2-fat | 2 | 36.753 | 35.714 | 37.792 | 1.470 | 36.753 | 36.018 | 904.6 | 2.91× |
| v2-shim | 2 | 52.622 | 50.713 | 54.531 | 2.700 | 52.622 | 53.425 | 1123.0 | 2.03× |

Failed runs:

- compile-medium-rice / v2-fat [cold] #1 (exit 1): failed — exit 1 (expected 0)
- compile-medium-rice / v2-fat [cold] #2 (exit 1): failed — exit 1 (expected 0)

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
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: the runtime cache entry \\?\D:\a\tebako\tebako\tebako-rs\out\home\.tebako\runtimes\ruby-3.3.12-0.16.26-windows-ucrt64 has no tebako-runtime-0.16.26-3.3.12-windows-ucrt64.exe
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: the runtime cache entry \\?\D:\a\tebako\tebako\tebako-rs\out\home\.tebako\runtimes\ruby-3.3.12-0.16.26-windows-ucrt64 has no tebako-runtime-0.16.26-3.3.12-windows-ucrt64.exe
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: the runtime cache entry \\?\D:\a\tebako\tebako\tebako-rs\out\home\.tebako\runtimes\ruby-3.3.12-0.16.26-windows-ucrt64 has no tebako-runtime-0.16.26-3.3.12-windows-ucrt64.exe
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: the runtime cache entry \\?\D:\a\tebako\tebako\tebako-rs\out\home\.tebako\runtimes\ruby-3.3.12-0.16.26-windows-ucrt64 has no tebako-runtime-0.16.26-3.3.12-windows-ucrt64.exe

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

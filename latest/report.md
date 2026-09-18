# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 100.691 | 100.296 | 101.558 | 0.555 | 100.924 | 118.432 | 1158.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 110.625 | 108.198 | 113.051 | 3.431 | 110.625 | 126.199 | 1244.9 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.092 | 48.601 | 49.663 | 0.466 | 49.088 | 59.993 | 839.6 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.863 | 53.850 | 55.876 | 1.433 | 54.863 | 65.815 | 836.9 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-small-iso / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-shim: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload
- compile-medium-rice / v2-fat: unavailable — platforms.yaml: v2_payload is false for linux-gnu-arm64 — no published payload

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 111.207 | 109.995 | 112.010 | 0.725 | 111.100 | 137.124 | 1199.1 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 121.541 | 121.294 | 121.787 | 0.349 | 121.541 | 147.024 | 1307.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 56.008 | 55.929 | 56.502 | 0.251 | 56.135 | 72.924 | 894.0 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 65.191 | 63.775 | 66.608 | 2.003 | 65.191 | 80.209 | 892.8 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB

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
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 120.983 | 116.202 | 131.240 | 6.008 | 123.419 | 127.189 | 2039.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 150.701 | 144.132 | 157.269 | 9.289 | 150.701 | 156.199 | 2033.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 81.887 | 75.124 | 91.569 | 6.673 | 81.429 | 86.642 | 1740.5 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 93.545 | 92.514 | 94.575 | 1.458 | 93.545 | 97.167 | 1668.9 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: 2 runtime entries in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — the bench home is shared or stale; refusing to guess

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

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: runtime cache entry 'java-21.0.12-2.7.0-windows-ucrt64' is not ruby-<lang-ver>-<tebako-ver>-windows-ucrt64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: runtime cache entry 'java-21.0.12-2.7.0-windows-ucrt64' is not ruby-<lang-ver>-<tebako-ver>-windows-ucrt64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: runtime cache entry 'java-21.0.12-2.7.0-windows-ucrt64' is not ruby-<lang-ver>-<tebako-ver>-windows-ucrt64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: runtime cache entry 'java-21.0.12-2.7.0-windows-ucrt64' is not ruby-<lang-ver>-<tebako-ver>-windows-ucrt64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

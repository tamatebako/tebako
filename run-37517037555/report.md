# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 103.671 | 102.224 | 105.610 | 1.569 | 103.957 | 120.596 | 1149.7 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 110.325 | 109.857 | 110.793 | 0.662 | 110.325 | 128.016 | 1245.0 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.475 | 50.244 | 51.332 | 0.461 | 50.706 | 62.531 | 839.3 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.514 | 55.470 | 55.557 | 0.061 | 55.514 | 67.486 | 841.4 | 1.00× |

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
| v1-packed-mn | 5 | 94.014 | 91.239 | 95.262 | 1.510 | 93.755 | 113.228 | 1204.6 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 101.065 | 100.198 | 101.932 | 1.226 | 101.065 | 120.625 | 1307.6 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 45.488 | 44.484 | 45.639 | 0.465 | 45.259 | 59.174 | 894.1 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 51.154 | 50.941 | 51.367 | 0.301 | 51.154 | 64.336 | 893.7 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-gnu-x86_64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-gnu-x86_64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-gnu-x86_64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-gnu-x86_64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log

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
| v1-packed-mn | 5 | 181.947 | 158.689 | 211.291 | 20.425 | 181.133 | 189.805 | 2033.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 195.121 | 177.485 | 212.758 | 24.942 | 195.121 | 201.963 | 2056.3 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 108.638 | 104.792 | 113.555 | 3.147 | 109.121 | 111.886 | 1563.0 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.153 | 98.011 | 120.294 | 15.757 | 109.153 | 112.951 | 1660.8 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for macos-arm64 in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for macos-arm64 in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for macos-arm64 in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for macos-arm64 in /Users/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log

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
Versions: tebako 2.8.26 · runtime 0.17.1-3.3.12 · payload 1.16.9-17 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image dwarfs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 139.721 | 135.862 | 150.036 | 5.367 | 141.090 | 14.312 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 140.359 | 140.092 | 140.625 | 0.377 | 140.359 | 13.891 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 67.661 | 65.920 | 70.096 | 1.631 | 67.692 | 13.938 | 642.2 | 1.00× |
| v2-fat | 5 | 52.688 | 51.350 | 53.901 | 1.016 | 52.536 | 0.016 | 5.7 | 1.28× |
| v2-shim | 5 | 55.578 | 53.200 | 58.224 | 1.898 | 55.836 | 0.031 | 8.6 | 1.22× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 78.237 | 75.862 | 80.613 | 3.360 | 78.237 | 14.203 | 642.2 | 1.00× |
| v2-shim | 2 | 62.571 | 62.131 | 63.012 | 0.623 | 62.571 | 0.570 | 111.7 | 1.25× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit 75): failed — exit 75 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit 75): failed — exit 75 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 75): failed — warmup: exit 75 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

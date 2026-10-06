# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 98.787 | 96.678 | 103.419 | 2.556 | 99.378 | 115.887 | 1160.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 105.879 | 105.794 | 105.964 | 0.120 | 105.879 | 121.829 | 1350.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.690 | 51.442 | 52.601 | 0.473 | 51.786 | 63.763 | 840.0 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 57.133 | 56.906 | 57.360 | 0.321 | 57.133 | 68.373 | 839.3 | 1.00× |

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
| v1-packed-mn | 5 | 73.869 | 73.316 | 77.978 | 1.946 | 74.662 | 90.648 | 1195.7 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 79.697 | 79.196 | 80.199 | 0.709 | 79.697 | 96.249 | 1328.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 39.821 | 38.306 | 40.728 | 1.017 | 39.510 | 52.641 | 894.0 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 43.923 | 43.175 | 44.672 | 1.059 | 43.923 | 56.616 | 896.2 | 1.00× |

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
| v1-packed-mn | 5 | 129.515 | 127.469 | 133.008 | 2.278 | 129.896 | 134.887 | 2067.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 162.398 | 141.884 | 182.912 | 29.011 | 162.398 | 168.666 | 2078.3 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 88.881 | 86.365 | 91.177 | 1.880 | 88.993 | 91.156 | 1592.4 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 98.792 | 96.108 | 101.477 | 3.797 | 98.792 | 101.959 | 1588.0 | 1.00× |

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
| v1-packed-mn | 5 | 138.626 | 133.603 | 143.220 | 4.096 | 139.025 | 14.219 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 146.827 | 146.721 | 146.933 | 0.150 | 146.827 | 14.156 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 70.958 | 68.351 | 71.512 | 1.415 | 70.300 | 14.297 | 642.2 | 1.00× |
| v2-fat | 5 | 57.507 | 54.198 | 58.635 | 2.102 | 56.517 | 0.016 | 5.7 | 1.23× |
| v2-shim | 5 | 57.907 | 56.491 | 58.087 | 0.655 | 57.602 | 0.047 | 8.7 | 1.23× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 78.513 | 77.970 | 79.056 | 0.768 | 78.513 | 14.531 | 642.2 | 1.00× |
| v2-shim | 2 | 67.614 | 63.868 | 71.360 | 5.297 | 67.614 | 0.625 | 111.7 | 1.16× |

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

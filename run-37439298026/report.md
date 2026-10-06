# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.848 | 101.381 | 102.515 | 0.477 | 101.841 | 119.189 | 1152.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.930 | 108.819 | 111.041 | 1.572 | 109.930 | 126.937 | 1272.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.507 | 49.294 | 50.175 | 0.336 | 49.621 | 60.751 | 839.2 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.473 | 54.442 | 54.505 | 0.044 | 54.473 | 65.400 | 841.1 | 1.00× |

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
| v1-packed-mn | 5 | 123.358 | 122.917 | 124.569 | 0.664 | 123.574 | 151.486 | 1199.3 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 131.431 | 129.908 | 132.954 | 2.154 | 131.431 | 161.099 | 1307.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 63.172 | 62.113 | 63.550 | 0.596 | 62.885 | 82.924 | 893.6 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 68.351 | 67.871 | 68.831 | 0.679 | 68.351 | 89.926 | 893.2 | 1.00× |

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15988.7 GiB
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
| v1-packed-mn | 5 | 128.487 | 128.029 | 135.182 | 3.315 | 130.584 | 133.689 | 2052.3 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 137.272 | 136.483 | 138.060 | 1.115 | 137.272 | 142.065 | 2037.0 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 87.748 | 84.261 | 95.247 | 4.223 | 88.513 | 90.298 | 1589.3 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 93.134 | 92.959 | 93.308 | 0.247 | 93.134 | 94.720 | 1639.3 | 1.00× |

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
| v1-packed-mn | 5 | 136.112 | 134.426 | 143.536 | 3.778 | 137.973 | 14.094 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 145.542 | 144.428 | 146.656 | 1.575 | 145.542 | 14.094 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 71.899 | 70.040 | 74.339 | 1.559 | 71.990 | 14.266 | 642.2 | 1.00× |
| v2-fat | 5 | 55.224 | 52.675 | 60.631 | 3.067 | 55.970 | 0.016 | 5.7 | 1.30× |
| v2-shim | 5 | 56.868 | 55.712 | 59.269 | 1.299 | 57.202 | 0.031 | 8.6 | 1.26× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 81.708 | 79.961 | 83.455 | 2.470 | 81.708 | 14.461 | 642.2 | 1.00× |
| v2-shim | 2 | 68.670 | 67.933 | 69.406 | 1.042 | 68.670 | 0.617 | 113.6 | 1.19× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit 75): failed — exit 75 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit 75): failed — exit 75 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 75): failed — warmup: exit 75 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15988.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

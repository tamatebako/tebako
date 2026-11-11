# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.884 | 102.245 | 103.498 | 0.546 | 102.888 | 119.537 | 1166.9 | 1.00× |
| v2-fat | 5 | 210.379 | 209.560 | 210.712 | 0.501 | 210.233 | 221.935 | 1713.6 | 0.49× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 111.672 | 110.781 | 112.563 | 1.260 | 111.672 | 126.538 | 1252.6 | 1.00× |
| v2-fat | 2 | 226.138 | 224.483 | 227.794 | 2.341 | 226.138 | 236.947 | 1798.8 | 0.49× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.746 | 49.386 | 51.214 | 0.740 | 50.120 | 60.238 | 853.4 | 1.00× |
| v2-fat | 5 | 122.339 | 120.285 | 122.563 | 0.962 | 121.868 | 195.593 | 1213.3 | 0.41× |
| v2-shim | 5 | 122.159 | 121.261 | 123.451 | 0.822 | 122.354 | 195.947 | 1224.4 | 0.41× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.689 | 54.952 | 56.427 | 1.043 | 55.689 | 66.033 | 850.1 | 1.00× |
| v2-fat | 2 | 131.620 | 131.126 | 132.114 | 0.699 | 131.620 | 202.107 | 1167.3 | 0.42× |
| v2-shim | 2 | 136.635 | 136.477 | 136.793 | 0.223 | 136.635 | 204.403 | 1182.1 | 0.41× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 93.451 | 92.233 | 96.064 | 1.693 | 94.109 | 114.287 | 1204.3 | 1.00× |
| v2-fat | 5 | 158.261 | 157.163 | 161.302 | 1.642 | 158.757 | 176.639 | 1685.8 | 0.59× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 108.699 | 108.433 | 108.965 | 0.376 | 108.699 | 123.661 | 1307.9 | 1.00× |
| v2-fat | 2 | 184.068 | 182.816 | 185.319 | 1.769 | 184.068 | 194.291 | 1841.3 | 0.59× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 47.626 | 47.372 | 48.255 | 0.337 | 47.758 | 62.455 | 908.2 | 1.00× |
| v2-fat | 5 | 92.028 | 89.638 | 95.154 | 2.558 | 92.314 | 160.742 | 1196.1 | 0.52× |
| v2-shim | 5 | 91.108 | 90.148 | 94.475 | 1.808 | 91.777 | 158.333 | 1227.1 | 0.52× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.308 | 55.409 | 57.207 | 1.271 | 56.308 | 65.867 | 901.3 | 1.00× |
| v2-fat | 2 | 112.120 | 109.402 | 114.837 | 3.843 | 112.120 | 170.662 | 1175.0 | 0.50× |
| v2-shim | 2 | 113.607 | 112.720 | 114.495 | 1.255 | 113.607 | 175.217 | 1166.0 | 0.50× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · image limnifs

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 156.698 | 143.181 | 163.069 | 7.898 | 154.353 | 163.811 | 2017.3 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 183.597 | 179.224 | 187.970 | 6.184 | 183.597 | 189.971 | 2026.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.632 | 86.575 | 171.341 | 32.921 | 114.992 | 107.066 | 1721.9 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 98.706 | 90.350 | 107.062 | 11.818 | 98.706 | 101.682 | 1690.5 | 1.00× |

Failed runs:

- compile-small-iso / v2-shim [warm] #0 (exit 69): failed — warmup: exit 69 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 69): failed — warmup: exit 69 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 338.242 | 290.698 | 379.454 | 32.191 | 338.371 | 386.226 | 1329.7 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 395.816 | 375.072 | 416.560 | 29.336 | 395.816 | 428.712 | 1350.4 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 132.465 | 129.708 | 134.422 | 2.122 | 131.916 | 239.537 | 777.5 | — |
| v2-shim | 5 | 131.299 | 129.936 | 190.064 | 25.860 | 144.418 | 241.457 | 780.3 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 207.168 | 198.837 | 215.499 | 11.782 | 207.168 | 364.063 | 767.4 | — |
| v2-shim | 2 | 174.013 | 173.436 | 174.590 | 0.816 | 174.013 | 310.058 | 749.1 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.76 · runtime 0.17.7-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 129.062 | 127.803 | 130.971 | 1.500 | 129.250 | 159.906 | 1552.8 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 137.793 | 137.452 | 138.134 | 0.482 | 137.793 | 169.469 | 1672.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 64.151 | 63.590 | 66.382 | 1.201 | 64.626 | 87.188 | 1562.5 | 1.00× |
| v2-fat | 5 | 65.643 | 64.981 | 67.491 | 0.973 | 65.943 | 91.156 | 2484.5 | 0.98× |
| v2-shim | 5 | 67.716 | 66.218 | 68.696 | 0.949 | 67.586 | 94.203 | 2468.4 | 0.95× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 72.503 | 72.434 | 72.571 | 0.097 | 72.503 | 98.273 | 1588.8 | 1.00× |
| v2-shim | 2 | 75.385 | 74.531 | 76.238 | 1.207 | 75.385 | 99.008 | 2499.8 | 0.96× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

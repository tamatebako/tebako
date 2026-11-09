# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.512 | 102.331 | 104.426 | 0.880 | 102.993 | 120.485 | 1170.4 | 1.00× |
| v2-fat | 5 | 202.341 | 201.067 | 202.981 | 0.730 | 202.113 | 214.314 | 1340.1 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.908 | 109.278 | 110.539 | 0.892 | 109.908 | 127.111 | 1252.4 | 1.00× |
| v2-fat | 2 | 218.621 | 217.330 | 219.911 | 1.825 | 218.621 | 231.385 | 1453.5 | 0.50× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.165 | 49.356 | 50.490 | 0.436 | 50.043 | 60.949 | 853.9 | 1.00× |
| v2-fat | 5 | 112.357 | 111.452 | 112.478 | 0.458 | 112.089 | 185.782 | 865.1 | 0.45× |
| v2-shim | 5 | 112.651 | 111.736 | 113.370 | 0.719 | 112.538 | 186.823 | 878.5 | 0.45× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.586 | 55.796 | 57.376 | 1.118 | 56.586 | 67.374 | 848.6 | 1.00× |
| v2-fat | 2 | 121.910 | 121.902 | 121.918 | 0.011 | 121.910 | 193.767 | 814.9 | 0.46× |
| v2-shim | 2 | 143.458 | 125.657 | 161.259 | 25.174 | 143.458 | 197.428 | 833.6 | 0.39× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 124.054 | 123.020 | 124.923 | 0.717 | 124.099 | 151.223 | 1196.7 | 1.00× |
| v2-fat | 5 | 195.376 | 193.733 | 196.111 | 1.077 | 195.101 | 222.144 | 1302.6 | 0.63× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 134.946 | 134.077 | 135.815 | 1.229 | 134.946 | 160.949 | 1306.2 | 1.00× |
| v2-fat | 2 | 220.316 | 220.309 | 220.322 | 0.010 | 220.316 | 241.156 | 1442.8 | 0.61× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 61.508 | 61.357 | 62.266 | 0.389 | 61.719 | 80.447 | 908.1 | 1.00× |
| v2-fat | 5 | 105.090 | 104.282 | 113.316 | 4.315 | 107.716 | 199.258 | 859.2 | 0.59× |
| v2-shim | 5 | 106.162 | 102.670 | 110.820 | 3.043 | 106.225 | 200.989 | 876.4 | 0.58× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 68.708 | 68.181 | 69.235 | 0.746 | 68.708 | 87.618 | 902.9 | 1.00× |
| v2-fat | 2 | 122.504 | 121.636 | 123.371 | 1.227 | 122.504 | 210.725 | 824.0 | 0.56× |
| v2-shim | 2 | 130.068 | 129.986 | 130.151 | 0.117 | 130.068 | 215.967 | 821.3 | 0.53× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 161.429 | 152.000 | 167.088 | 5.751 | 159.905 | 165.258 | 2026.1 | 1.00× |
| v2-fat | 5 | 180.486 | 164.517 | 186.200 | 9.006 | 177.764 | 186.887 | 1617.3 | 0.89× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 166.642 | 155.257 | 178.026 | 16.100 | 166.642 | 171.954 | 2061.6 | 1.00× |
| v2-fat | 2 | 193.518 | 193.348 | 193.688 | 0.241 | 193.518 | 200.241 | 1655.0 | 0.86× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 82.538 | 78.299 | 102.704 | 10.048 | 85.966 | 85.902 | 1717.0 | 1.00× |
| v2-fat | 5 | 91.807 | 88.709 | 105.806 | 7.043 | 93.616 | 165.753 | 988.4 | 0.90× |
| v2-shim | 5 | 86.528 | 84.944 | 98.690 | 5.707 | 88.680 | 158.558 | 960.8 | 0.95× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 129.433 | 114.695 | 144.171 | 20.843 | 129.433 | 133.101 | 1711.2 | 1.00× |
| v2-fat | 2 | 121.153 | 117.127 | 125.179 | 5.694 | 121.153 | 212.683 | 945.6 | 1.07× |
| v2-shim | 2 | 123.384 | 123.126 | 123.642 | 0.364 | 123.384 | 215.369 | 951.8 | 1.05× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 311.677 | 311.399 | 324.854 | 5.942 | 314.228 | 347.705 | 1310.6 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 332.304 | 330.266 | 334.342 | 2.882 | 332.304 | 369.234 | 1328.9 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 143.383 | 141.443 | 146.260 | 1.935 | 143.754 | 268.787 | 785.8 | — |
| v2-shim | 5 | 144.190 | 142.656 | 145.752 | 1.440 | 144.173 | 271.401 | 777.0 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 173.481 | 168.043 | 178.919 | 7.690 | 173.481 | 313.213 | 764.9 | — |
| v2-shim | 2 | 167.999 | 167.909 | 168.089 | 0.127 | 167.999 | 298.101 | 759.5 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.000 | 101.179 | 103.445 | 0.933 | 102.006 | 123.406 | 1537.5 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.108 | 108.532 | 109.684 | 0.815 | 109.108 | 131.148 | 1716.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.259 | 49.580 | 52.942 | 1.250 | 51.386 | 68.484 | 1593.3 | 1.00× |
| v2-fat | 5 | 48.264 | 46.836 | 50.568 | 1.686 | 48.522 | 68.391 | 2481.1 | 1.06× |
| v2-shim | 5 | 49.278 | 48.244 | 50.355 | 0.757 | 49.245 | 68.000 | 2489.5 | 1.04× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 59.367 | 57.675 | 61.058 | 2.393 | 59.367 | 74.898 | 1578.7 | 1.00× |
| v2-shim | 2 | 58.144 | 56.902 | 59.387 | 1.757 | 58.144 | 73.891 | 2308.4 | 1.02× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

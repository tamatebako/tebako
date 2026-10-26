# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.774 | 101.414 | 102.085 | 0.312 | 101.756 | 118.849 | 1162.0 | 1.00× |
| v2-fat | 5 | 193.740 | 192.104 | 194.525 | 0.998 | 193.483 | 209.207 | 1355.7 | 0.53× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 110.266 | 110.068 | 110.464 | 0.280 | 110.266 | 127.759 | 1266.0 | 1.00× |
| v2-fat | 2 | 209.697 | 209.054 | 210.339 | 0.908 | 209.697 | 221.927 | 1463.2 | 0.53× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.752 | 49.554 | 50.185 | 0.239 | 49.829 | 60.701 | 845.7 | 1.00× |
| v2-fat | 5 | 105.974 | 105.389 | 106.300 | 0.331 | 105.921 | 178.628 | 865.3 | 0.47× |
| v2-shim | 5 | 106.743 | 105.993 | 107.006 | 0.382 | 106.617 | 179.618 | 875.4 | 0.47× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.759 | 54.757 | 54.761 | 0.002 | 54.759 | 64.714 | 840.7 | 1.00× |
| v2-fat | 2 | 114.301 | 113.234 | 115.368 | 1.509 | 114.301 | 186.081 | 829.5 | 0.48× |
| v2-shim | 2 | 115.098 | 114.963 | 115.233 | 0.191 | 115.098 | 187.433 | 837.9 | 0.48× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 121.800 | 120.902 | 124.122 | 1.306 | 122.141 | 147.553 | 1229.4 | 1.00× |
| v2-fat | 5 | 185.238 | 183.436 | 187.912 | 1.632 | 185.382 | 203.286 | 1345.4 | 0.66× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 130.617 | 127.604 | 133.630 | 4.261 | 130.617 | 157.708 | 1308.0 | 1.00× |
| v2-fat | 2 | 206.977 | 206.806 | 207.147 | 0.241 | 206.977 | 222.397 | 1452.2 | 0.63× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 61.706 | 60.393 | 61.953 | 0.732 | 61.293 | 80.519 | 907.7 | 1.00× |
| v2-fat | 5 | 98.509 | 96.731 | 106.165 | 4.418 | 100.929 | 190.883 | 879.6 | 0.63× |
| v2-shim | 5 | 107.187 | 99.987 | 107.501 | 3.592 | 104.774 | 188.193 | 889.6 | 0.58× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 67.532 | 67.381 | 67.682 | 0.212 | 67.532 | 87.178 | 900.1 | 1.00× |
| v2-fat | 2 | 119.776 | 118.974 | 120.579 | 1.135 | 119.776 | 202.446 | 834.4 | 0.56× |
| v2-shim | 2 | 123.626 | 123.031 | 124.220 | 0.841 | 123.626 | 204.992 | 836.5 | 0.55× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · image limnifs

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 156.287 | 139.984 | 172.286 | 13.313 | 153.856 | 164.525 | 2106.9 | 1.00× |
| v2-fat | 5 | 170.341 | 164.137 | 188.407 | 9.349 | 173.375 | 177.078 | 1744.9 | 0.92× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 198.253 | 179.044 | 217.462 | 27.166 | 198.253 | 206.021 | 2015.1 | 1.00× |
| v2-fat | 2 | 195.532 | 195.102 | 195.962 | 0.608 | 195.532 | 201.806 | 1687.8 | 1.01× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 103.063 | 90.016 | 128.074 | 17.152 | 106.488 | 107.761 | 1636.8 | 1.00× |
| v2-fat | 5 | 98.629 | 94.078 | 120.910 | 10.651 | 103.571 | 186.426 | 983.3 | 1.04× |
| v2-shim | 5 | 99.439 | 86.267 | 104.554 | 7.521 | 96.572 | 181.534 | 972.1 | 1.04× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 89.483 | 86.004 | 92.962 | 4.920 | 89.483 | 93.487 | 1698.3 | 1.00× |
| v2-fat | 2 | 98.058 | 97.405 | 98.711 | 0.924 | 98.058 | 171.383 | 948.1 | 0.91× |
| v2-shim | 2 | 97.453 | 93.233 | 101.672 | 5.967 | 97.453 | 175.179 | 938.4 | 0.92× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 444.852 | 434.126 | 460.063 | 10.064 | 446.420 | 494.403 | 1323.9 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 207.698 | 178.414 | 230.709 | 21.848 | 207.666 | 387.784 | 777.7 | — |
| v2-shim | 5 | 203.770 | 176.536 | 230.794 | 20.012 | 201.868 | 364.831 | 783.2 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 239.866 | 232.414 | 247.318 | 10.539 | 239.866 | 421.039 | 771.8 | — |
| v2-shim | 2 | 250.614 | 244.214 | 257.015 | 9.052 | 250.614 | 441.476 | 759.0 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [cold] #1 (exit 69): failed — exit 69 (expected 0)
- compile-medium-rice / v2-fat [cold] #2 (exit 69): failed — exit 69 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.65 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 130.842 | 126.048 | 135.609 | 4.006 | 131.378 | 13.531 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 149.592 | 146.273 | 152.910 | 4.693 | 149.592 | 14.602 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 76.477 | 71.283 | 78.906 | 3.515 | 75.574 | 14.844 | 642.2 | 1.00× |
| v2-fat | 5 | 67.589 | 62.267 | 69.872 | 3.401 | 66.170 | 0.016 | 5.8 | 1.13× |
| v2-shim | 5 | 69.789 | 63.581 | 71.572 | 3.874 | 68.000 | 0.031 | 8.7 | 1.10× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 83.433 | 83.175 | 83.692 | 0.366 | 83.433 | 15.867 | 642.2 | 1.00× |
| v2-shim | 2 | 66.505 | 63.129 | 69.881 | 4.775 | 66.505 | 0.602 | 113.8 | 1.25× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

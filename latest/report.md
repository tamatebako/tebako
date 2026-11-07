# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.810 | 101.649 | 105.141 | 1.360 | 102.892 | 120.062 | 1162.5 | 1.00× |
| v2-fat | 5 | 200.983 | 200.206 | 203.201 | 1.152 | 201.226 | 212.293 | 1351.0 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 112.898 | 112.364 | 113.433 | 0.756 | 112.898 | 130.981 | 1273.7 | 1.00× |
| v2-fat | 2 | 219.221 | 217.928 | 220.515 | 1.829 | 219.221 | 232.062 | 1528.9 | 0.51× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.878 | 50.723 | 52.808 | 0.869 | 51.633 | 63.184 | 853.4 | 1.00× |
| v2-fat | 5 | 112.311 | 111.884 | 112.572 | 0.267 | 112.261 | 186.350 | 864.0 | 0.46× |
| v2-shim | 5 | 112.873 | 112.217 | 112.913 | 0.301 | 112.752 | 187.538 | 876.5 | 0.46× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.326 | 54.017 | 56.635 | 1.851 | 55.326 | 65.994 | 851.5 | 1.00× |
| v2-fat | 2 | 121.293 | 121.147 | 121.438 | 0.205 | 121.293 | 193.035 | 821.4 | 0.46× |
| v2-shim | 2 | 123.253 | 122.911 | 123.595 | 0.484 | 123.253 | 195.219 | 831.2 | 0.45× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 122.261 | 119.890 | 125.856 | 2.353 | 122.603 | 149.364 | 1202.0 | 1.00× |
| v2-fat | 5 | 194.529 | 193.008 | 196.396 | 1.258 | 194.727 | 212.966 | 1329.8 | 0.63× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 130.314 | 130.297 | 130.331 | 0.024 | 130.314 | 160.079 | 1351.4 | 1.00× |
| v2-fat | 2 | 217.396 | 216.508 | 218.284 | 1.256 | 217.396 | 238.813 | 1443.4 | 0.60× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 61.681 | 61.054 | 62.605 | 0.602 | 61.720 | 80.320 | 907.8 | 1.00× |
| v2-fat | 5 | 105.175 | 104.606 | 105.264 | 0.270 | 105.081 | 201.259 | 860.2 | 0.59× |
| v2-shim | 5 | 106.105 | 105.505 | 116.246 | 5.189 | 109.468 | 202.358 | 873.7 | 0.58× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 66.772 | 66.658 | 66.885 | 0.160 | 66.772 | 86.203 | 901.0 | 1.00× |
| v2-fat | 2 | 126.657 | 125.246 | 128.069 | 1.996 | 126.657 | 210.262 | 820.4 | 0.53× |
| v2-shim | 2 | 127.740 | 126.949 | 128.531 | 1.119 | 127.740 | 216.552 | 826.7 | 0.52× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 138.958 | 129.614 | 162.386 | 14.710 | 144.023 | 148.078 | 1995.4 | 1.00× |
| v2-fat | 5 | 170.907 | 146.141 | 184.687 | 14.959 | 167.526 | 178.287 | 1629.2 | 0.81× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 123.781 | 118.717 | 128.846 | 7.162 | 123.781 | 127.172 | 2052.6 | 1.00× |
| v2-fat | 2 | 155.184 | 152.946 | 157.422 | 3.165 | 155.184 | 158.654 | 1689.5 | 0.80× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 87.678 | 83.263 | 94.184 | 3.962 | 88.319 | 92.347 | 1747.0 | 1.00× |
| v2-fat | 5 | 94.434 | 87.717 | 106.911 | 8.230 | 95.495 | 172.480 | 987.2 | 0.93× |
| v2-shim | 5 | 91.985 | 87.877 | 96.066 | 3.788 | 91.723 | 171.129 | 963.8 | 0.95× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 87.660 | 84.375 | 90.945 | 4.645 | 87.660 | 91.924 | 1720.4 | 1.00× |
| v2-fat | 2 | 118.113 | 116.502 | 119.723 | 2.277 | 118.113 | 203.303 | 948.5 | 0.74× |
| v2-shim | 2 | 92.465 | 90.630 | 94.301 | 2.595 | 92.465 | 158.849 | 944.0 | 0.95× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 275.598 | 272.460 | 342.230 | 30.431 | 287.892 | 303.483 | 1316.1 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 394.982 | 387.231 | 402.732 | 10.961 | 394.982 | 432.118 | 1401.5 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 177.547 | 132.265 | 199.132 | 28.011 | 166.665 | 322.824 | 778.7 | — |
| v2-shim | 5 | 166.925 | 130.997 | 200.662 | 32.343 | 165.063 | 317.248 | 781.4 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 191.529 | 184.772 | 198.285 | 9.555 | 191.529 | 337.902 | 760.3 | — |
| v2-shim | 2 | 160.649 | 154.080 | 167.217 | 9.290 | 160.649 | 286.974 | 753.6 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.72 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 127.273 | 121.703 | 128.146 | 2.671 | 126.381 | 157.500 | 1521.9 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 137.724 | 136.774 | 138.674 | 1.344 | 137.724 | 168.578 | 1664.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 63.295 | 62.747 | 63.785 | 0.368 | 63.280 | 86.859 | 1599.1 | 1.00× |
| v2-fat | 5 | 62.859 | 62.607 | 62.985 | 0.168 | 62.799 | 86.688 | 2470.2 | 1.01× |
| v2-shim | 5 | 64.663 | 63.862 | 65.272 | 0.572 | 64.632 | 89.812 | 2498.9 | 0.98× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 70.667 | 68.536 | 72.798 | 3.013 | 70.667 | 94.438 | 1567.8 | 1.00× |
| v2-shim | 2 | 73.610 | 72.291 | 74.928 | 1.865 | 73.610 | 97.766 | 2479.6 | 0.96× |

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

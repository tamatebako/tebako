# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 104.317 | 104.074 | 105.376 | 0.602 | 104.631 | 122.891 | 1159.0 | 1.00× |
| v2-fat | 5 | 195.315 | 194.049 | 196.469 | 1.129 | 195.310 | 211.343 | 1363.1 | 0.53× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 112.524 | 112.496 | 112.553 | 0.040 | 112.524 | 130.878 | 1244.0 | 1.00× |
| v2-fat | 2 | 213.382 | 212.906 | 213.858 | 0.673 | 213.382 | 227.709 | 1508.7 | 0.53× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.590 | 50.137 | 50.675 | 0.218 | 50.522 | 61.745 | 845.5 | 1.00× |
| v2-fat | 5 | 105.745 | 105.483 | 106.516 | 0.476 | 105.920 | 179.153 | 865.7 | 0.48× |
| v2-shim | 5 | 106.343 | 105.984 | 106.350 | 0.157 | 106.248 | 179.780 | 875.8 | 0.48× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.430 | 53.744 | 55.116 | 0.970 | 54.430 | 65.601 | 843.3 | 1.00× |
| v2-fat | 2 | 114.655 | 113.864 | 115.447 | 1.119 | 114.655 | 187.723 | 834.7 | 0.47× |
| v2-shim | 2 | 115.958 | 115.818 | 116.098 | 0.198 | 115.958 | 188.584 | 827.8 | 0.47× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.944 | 100.841 | 102.837 | 0.719 | 101.851 | 125.526 | 1207.0 | 1.00× |
| v2-fat | 5 | 157.808 | 156.383 | 159.015 | 1.103 | 157.612 | 171.609 | 1342.1 | 0.65× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 112.469 | 110.735 | 114.203 | 2.453 | 112.469 | 138.016 | 1307.9 | 1.00× |
| v2-fat | 2 | 178.180 | 177.440 | 178.920 | 1.047 | 178.180 | 194.699 | 1464.0 | 0.63× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.183 | 50.602 | 52.616 | 0.765 | 51.332 | 66.716 | 907.6 | 1.00× |
| v2-fat | 5 | 82.128 | 81.491 | 87.487 | 2.954 | 83.993 | 158.139 | 879.3 | 0.62× |
| v2-shim | 5 | 89.007 | 82.504 | 90.377 | 3.346 | 87.164 | 158.373 | 892.7 | 0.58× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.134 | 55.710 | 56.559 | 0.600 | 56.134 | 73.199 | 900.2 | 1.00× |
| v2-fat | 2 | 100.379 | 100.313 | 100.445 | 0.094 | 100.379 | 168.049 | 846.3 | 0.56× |
| v2-shim | 2 | 99.429 | 94.275 | 104.582 | 7.288 | 99.429 | 168.480 | 845.0 | 0.56× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 171.658 | 145.185 | 207.787 | 24.831 | 170.063 | 178.376 | 2035.4 | 1.00× |
| v2-fat | 5 | 176.035 | 162.326 | 188.923 | 10.087 | 175.737 | 184.703 | 1621.9 | 0.98× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 171.802 | 163.055 | 180.548 | 12.369 | 171.802 | 176.757 | 2035.3 | 1.00× |
| v2-fat | 2 | 178.071 | 171.494 | 184.647 | 9.300 | 178.071 | 184.428 | 1746.0 | 0.96× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 119.491 | 99.764 | 148.854 | 19.163 | 120.587 | 124.405 | 1672.4 | 1.00× |
| v2-fat | 5 | 103.221 | 99.856 | 117.078 | 7.195 | 106.248 | 192.951 | 984.2 | 1.16× |
| v2-shim | 5 | 99.807 | 96.754 | 102.051 | 1.904 | 99.674 | 185.862 | 968.9 | 1.20× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 90.578 | 85.070 | 96.085 | 7.789 | 90.578 | 95.891 | 1700.1 | 1.00× |
| v2-fat | 2 | 115.466 | 113.180 | 117.752 | 3.233 | 115.466 | 203.823 | 940.4 | 0.78× |
| v2-shim | 2 | 110.650 | 107.553 | 113.748 | 4.381 | 110.650 | 196.261 | 941.9 | 0.82× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 373.717 | 346.213 | 389.672 | 16.837 | 369.039 | 416.185 | 1329.0 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 408.419 | 373.428 | 443.409 | 49.484 | 408.419 | 448.775 | 1358.3 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 174.268 | 140.534 | 197.575 | 22.010 | 175.034 | 326.695 | 776.0 | — |
| v2-shim | 5 | 150.668 | 144.987 | 200.760 | 26.209 | 167.628 | 290.630 | 767.6 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 188.783 | 180.968 | 196.599 | 11.053 | 188.783 | 339.399 | 750.9 | — |
| v2-shim | 2 | 201.561 | 178.702 | 224.421 | 32.328 | 201.561 | 342.816 | 740.2 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.64 · runtime 0.17.3-4.0.7 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 139.888 | 138.540 | 140.787 | 0.896 | 139.665 | 14.172 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 147.903 | 145.815 | 149.990 | 2.953 | 147.903 | 14.086 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 70.817 | 69.991 | 72.054 | 0.769 | 70.873 | 14.391 | 642.2 | 1.00× |
| v2-fat | 5 | 61.918 | 59.577 | 63.409 | 1.409 | 61.595 | 0.016 | 5.7 | 1.14× |
| v2-shim | 5 | 62.947 | 60.524 | 65.504 | 2.018 | 63.006 | 0.031 | 8.7 | 1.13× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 74.200 | 73.849 | 74.550 | 0.496 | 74.200 | 14.281 | 642.2 | 1.00× |
| v2-shim | 2 | 68.187 | 66.265 | 70.108 | 2.717 | 68.187 | 0.523 | 113.9 | 1.09× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

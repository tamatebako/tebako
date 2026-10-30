# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.958 | 102.421 | 106.207 | 1.578 | 103.720 | 119.841 | 1163.7 | 1.00× |
| v2-fat | 5 | 200.307 | 199.572 | 201.866 | 0.963 | 200.704 | 214.139 | 1340.8 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 112.244 | 111.897 | 112.591 | 0.491 | 112.244 | 129.135 | 1251.8 | 1.00× |
| v2-fat | 2 | 220.069 | 218.713 | 221.425 | 1.918 | 220.069 | 231.684 | 1449.2 | 0.51× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.411 | 51.148 | 51.898 | 0.290 | 51.423 | 62.829 | 853.1 | 1.00× |
| v2-fat | 5 | 111.353 | 110.897 | 111.581 | 0.258 | 111.323 | 185.796 | 863.3 | 0.46× |
| v2-shim | 5 | 112.042 | 111.479 | 112.305 | 0.347 | 111.949 | 186.681 | 880.0 | 0.46× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.079 | 55.819 | 56.339 | 0.367 | 56.079 | 67.513 | 848.6 | 1.00× |
| v2-fat | 2 | 120.023 | 119.450 | 120.596 | 0.810 | 120.023 | 192.750 | 818.3 | 0.47× |
| v2-shim | 2 | 122.496 | 121.804 | 123.187 | 0.978 | 122.496 | 194.290 | 833.6 | 0.46× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 75.347 | 71.587 | 78.928 | 2.985 | 75.777 | 91.475 | 1210.8 | 1.00× |
| v2-fat | 5 | 131.722 | 126.865 | 134.959 | 3.360 | 130.810 | 141.358 | 1337.6 | 0.57× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 78.577 | 77.117 | 80.038 | 2.066 | 78.577 | 93.537 | 1307.8 | 1.00× |
| v2-fat | 2 | 149.740 | 149.303 | 150.176 | 0.617 | 149.740 | 161.723 | 1473.0 | 0.52× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 38.349 | 37.200 | 39.937 | 1.001 | 38.369 | 50.704 | 908.1 | 1.00× |
| v2-fat | 5 | 72.321 | 70.101 | 75.997 | 2.147 | 72.613 | 137.210 | 865.3 | 0.53× |
| v2-shim | 5 | 73.388 | 71.801 | 78.565 | 3.226 | 74.764 | 136.792 | 878.2 | 0.52× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 47.112 | 46.677 | 47.548 | 0.616 | 47.112 | 58.936 | 903.3 | 1.00× |
| v2-fat | 2 | 85.666 | 85.177 | 86.155 | 0.691 | 85.666 | 148.391 | 819.7 | 0.55× |
| v2-shim | 2 | 90.320 | 89.150 | 91.490 | 1.655 | 90.320 | 147.441 | 814.0 | 0.52× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 147.076 | 142.056 | 162.961 | 7.988 | 149.463 | 153.178 | 2087.5 | 1.00× |
| v2-fat | 5 | 170.557 | 157.518 | 180.570 | 9.559 | 168.327 | 176.826 | 1627.9 | 0.86× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 168.055 | 161.325 | 174.784 | 9.516 | 168.055 | 173.333 | 2093.5 | 1.00× |
| v2-fat | 2 | 184.883 | 183.086 | 186.681 | 2.542 | 184.883 | 189.564 | 1668.0 | 0.91× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 88.857 | 84.564 | 136.963 | 21.661 | 99.814 | 92.380 | 1636.7 | 1.00× |
| v2-fat | 5 | 101.769 | 90.805 | 114.392 | 9.042 | 103.645 | 189.776 | 983.8 | 0.87× |
| v2-shim | 5 | 92.830 | 88.117 | 109.345 | 8.269 | 95.825 | 178.371 | 972.7 | 0.96× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 94.608 | 91.289 | 97.928 | 4.695 | 94.608 | 98.696 | 1753.4 | 1.00× |
| v2-fat | 2 | 106.614 | 106.287 | 106.941 | 0.463 | 106.614 | 184.400 | 987.4 | 0.89× |
| v2-shim | 2 | 108.286 | 101.930 | 114.642 | 8.989 | 108.286 | 192.582 | 942.5 | 0.87× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 348.645 | 300.348 | 371.128 | 33.636 | 339.684 | 396.670 | 1318.3 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 418.283 | 389.927 | 446.638 | 40.100 | 418.283 | 453.027 | 1366.0 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 197.307 | 183.053 | 212.559 | 11.987 | 199.641 | 359.520 | 772.6 | — |
| v2-shim | 5 | 183.767 | 178.283 | 203.323 | 12.449 | 189.427 | 337.795 | 778.2 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 180.909 | 161.340 | 200.478 | 27.675 | 180.909 | 311.835 | 757.6 | — |
| v2-shim | 2 | 208.609 | 184.916 | 232.302 | 33.507 | 208.609 | 356.994 | 756.1 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.195 | 98.020 | 103.803 | 2.463 | 101.305 | 122.234 | 1610.6 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 113.591 | 112.136 | 115.047 | 2.058 | 113.591 | 135.484 | 1658.4 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.206 | 50.767 | 51.617 | 0.346 | 51.135 | 69.234 | 1595.3 | 1.00× |
| v2-fat | 5 | 46.233 | 46.001 | 46.693 | 0.258 | 46.285 | 65.844 | 2512.0 | 1.11× |
| v2-shim | 5 | 47.600 | 46.507 | 49.218 | 1.177 | 47.947 | 66.266 | 2510.2 | 1.08× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 58.951 | 56.363 | 61.538 | 3.659 | 58.951 | 75.117 | 1614.0 | 1.00× |
| v2-shim | 2 | 54.865 | 54.671 | 55.058 | 0.274 | 54.865 | 72.742 | 2530.1 | 1.07× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

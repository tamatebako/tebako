# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.433 | 101.224 | 102.680 | 0.626 | 101.777 | 118.813 | 1173.4 | 1.00× |
| v2-fat | 5 | 209.505 | 205.863 | 210.264 | 1.742 | 208.922 | 221.802 | 1717.1 | 0.48× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 111.291 | 111.179 | 111.402 | 0.158 | 111.291 | 129.009 | 1273.7 | 1.00× |
| v2-fat | 2 | 228.082 | 227.517 | 228.647 | 0.799 | 228.082 | 241.233 | 1825.0 | 0.49× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.804 | 49.711 | 52.005 | 0.924 | 50.761 | 62.183 | 853.7 | 1.00× |
| v2-fat | 5 | 121.702 | 120.755 | 122.536 | 0.686 | 121.645 | 195.142 | 1219.1 | 0.42× |
| v2-shim | 5 | 121.980 | 120.926 | 122.645 | 0.657 | 121.933 | 195.989 | 1222.1 | 0.42× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 56.856 | 56.763 | 56.949 | 0.131 | 56.856 | 68.059 | 848.9 | 1.00× |
| v2-fat | 2 | 131.247 | 130.992 | 131.503 | 0.361 | 131.247 | 203.246 | 1172.2 | 0.43× |
| v2-shim | 2 | 132.911 | 132.527 | 133.295 | 0.543 | 132.911 | 205.421 | 1178.0 | 0.43× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 91.979 | 91.663 | 92.731 | 0.486 | 92.145 | 111.798 | 1199.9 | 1.00× |
| v2-fat | 5 | 156.727 | 155.615 | 157.560 | 0.756 | 156.753 | 175.268 | 1677.3 | 0.59× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 102.076 | 101.731 | 102.422 | 0.489 | 102.076 | 122.969 | 1308.3 | 1.00× |
| v2-fat | 2 | 176.720 | 176.628 | 176.811 | 0.130 | 176.720 | 190.295 | 1778.6 | 0.58× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 45.910 | 45.121 | 46.965 | 0.694 | 45.879 | 59.866 | 908.5 | 1.00× |
| v2-fat | 5 | 92.728 | 87.117 | 93.759 | 3.324 | 90.821 | 155.786 | 1206.6 | 0.50× |
| v2-shim | 5 | 89.045 | 87.105 | 92.370 | 2.212 | 89.886 | 156.041 | 1225.4 | 0.52× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 50.244 | 49.813 | 50.675 | 0.610 | 50.244 | 64.409 | 903.8 | 1.00× |
| v2-fat | 2 | 99.291 | 98.454 | 100.128 | 1.184 | 99.291 | 166.424 | 1175.0 | 0.51× |
| v2-shim | 2 | 105.576 | 104.310 | 106.842 | 1.790 | 105.576 | 172.471 | 1170.3 | 0.48× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 156.388 | 141.265 | 187.020 | 17.201 | 158.687 | 164.426 | 2029.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 167.577 | 164.597 | 170.556 | 4.214 | 167.577 | 174.430 | 2024.3 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 111.955 | 86.134 | 124.918 | 15.469 | 109.887 | 115.471 | 1576.7 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 99.494 | 99.491 | 99.497 | 0.004 | 99.494 | 102.887 | 1746.4 | 1.00× |

Failed runs:

- compile-small-iso / v2-shim [warm] #0 (exit 69): failed — warmup: exit 69 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 69): failed — warmup: exit 69 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 387.620 | 341.613 | 391.489 | 20.820 | 378.587 | 432.457 | 1320.9 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 425.229 | 393.998 | 456.459 | 44.167 | 425.229 | 457.855 | 1379.4 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 200.979 | 145.004 | 231.552 | 37.904 | 185.334 | 372.914 | 766.4 | — |
| v2-shim | 5 | 187.743 | 145.953 | 229.643 | 32.506 | 188.482 | 347.073 | 785.0 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 215.647 | 213.589 | 217.705 | 2.911 | 215.647 | 373.185 | 755.2 | — |
| v2-shim | 2 | 197.630 | 171.772 | 223.488 | 36.569 | 197.630 | 338.714 | 761.8 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.75 · runtime 0.17.6-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 134.844 | 132.411 | 137.821 | 1.973 | 134.816 | 165.734 | 1584.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 146.628 | 146.535 | 146.721 | 0.131 | 146.628 | 181.133 | 1497.3 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 68.013 | 67.302 | 70.398 | 1.216 | 68.342 | 92.172 | 1585.3 | 1.00× |
| v2-fat | 5 | 64.028 | 62.592 | 65.518 | 1.187 | 63.901 | 90.812 | 2494.2 | 1.06× |
| v2-shim | 5 | 65.683 | 65.026 | 66.538 | 0.625 | 65.677 | 90.844 | 2481.8 | 1.04× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 73.212 | 72.296 | 74.129 | 1.296 | 73.212 | 97.453 | 1546.8 | 1.00× |
| v2-shim | 2 | 73.137 | 72.421 | 73.853 | 1.012 | 73.137 | 100.477 | 2479.4 | 1.00× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

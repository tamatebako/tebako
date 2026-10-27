# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.840 | 102.147 | 106.114 | 1.562 | 103.526 | 119.459 | 1161.8 | 1.00× |
| v2-fat | 5 | 200.624 | 199.787 | 202.358 | 0.972 | 200.821 | 213.988 | 1350.8 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 110.070 | 108.215 | 111.925 | 2.623 | 110.070 | 125.571 | 1299.2 | 1.00× |
| v2-fat | 2 | 218.632 | 218.144 | 219.120 | 0.690 | 218.632 | 228.605 | 1416.4 | 0.50× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.704 | 49.455 | 50.426 | 0.388 | 49.799 | 60.782 | 845.8 | 1.00× |
| v2-fat | 5 | 110.581 | 110.424 | 111.198 | 0.323 | 110.735 | 183.838 | 862.1 | 0.45× |
| v2-shim | 5 | 111.854 | 110.055 | 112.069 | 0.833 | 111.491 | 185.048 | 877.3 | 0.44× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.894 | 53.898 | 55.891 | 1.410 | 54.894 | 64.420 | 844.5 | 1.00× |
| v2-fat | 2 | 121.707 | 120.477 | 122.936 | 1.739 | 121.707 | 191.942 | 824.0 | 0.45× |
| v2-shim | 2 | 125.742 | 123.213 | 128.271 | 3.577 | 125.742 | 192.819 | 839.2 | 0.44× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 91.638 | 90.164 | 93.145 | 1.182 | 91.845 | 112.023 | 1204.9 | 1.00× |
| v2-fat | 5 | 146.869 | 145.524 | 148.469 | 1.064 | 146.949 | 157.558 | 1355.4 | 0.62× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 99.772 | 98.514 | 101.029 | 1.779 | 99.772 | 118.286 | 1326.2 | 1.00× |
| v2-fat | 2 | 166.677 | 165.315 | 168.038 | 1.926 | 166.677 | 174.968 | 1479.7 | 0.60× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 44.791 | 44.485 | 46.838 | 1.031 | 45.369 | 58.562 | 907.6 | 1.00× |
| v2-fat | 5 | 78.961 | 76.007 | 83.029 | 3.144 | 79.272 | 144.892 | 881.9 | 0.57× |
| v2-shim | 5 | 79.365 | 76.748 | 82.980 | 2.345 | 79.353 | 143.888 | 890.0 | 0.56× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 52.050 | 51.982 | 52.117 | 0.095 | 52.050 | 63.246 | 901.1 | 1.00× |
| v2-fat | 2 | 93.691 | 92.108 | 95.273 | 2.238 | 93.691 | 155.903 | 842.3 | 0.56× |
| v2-shim | 2 | 99.346 | 96.403 | 102.289 | 4.162 | 99.346 | 159.563 | 841.7 | 0.52× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 139.402 | 138.491 | 184.230 | 22.924 | 155.571 | 143.990 | 2085.0 | 1.00× |
| v2-fat | 5 | 172.501 | 159.004 | 189.942 | 13.073 | 174.544 | 181.231 | 1612.0 | 0.81× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 158.031 | 154.231 | 161.832 | 5.375 | 158.031 | 162.894 | 2011.8 | 1.00× |
| v2-fat | 2 | 204.507 | 196.123 | 212.892 | 11.857 | 204.507 | 213.055 | 1723.3 | 0.77× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 128.178 | 101.485 | 133.811 | 13.103 | 123.206 | 131.690 | 1697.2 | 1.00× |
| v2-fat | 5 | 110.899 | 104.728 | 125.112 | 8.127 | 113.984 | 204.430 | 983.8 | 1.16× |
| v2-shim | 5 | 109.185 | 96.552 | 115.939 | 8.103 | 106.258 | 194.511 | 967.5 | 1.17× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 126.780 | 124.403 | 129.158 | 3.362 | 126.780 | 134.073 | 1683.5 | 1.00× |
| v2-fat | 2 | 116.074 | 115.986 | 116.163 | 0.125 | 116.074 | 205.469 | 948.8 | 1.09× |
| v2-shim | 2 | 117.947 | 115.219 | 120.675 | 3.858 | 117.947 | 209.632 | 949.3 | 1.07× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 252.937 | 251.977 | 256.596 | 1.899 | 253.333 | 276.907 | 1329.3 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 303.108 | 291.685 | 314.530 | 16.154 | 303.108 | 339.054 | 1355.3 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 122.705 | 118.202 | 148.817 | 12.681 | 128.611 | 219.931 | 771.8 | — |
| v2-shim | 5 | 124.151 | 118.056 | 152.777 | 16.304 | 132.352 | 224.418 | 768.1 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 170.378 | 167.070 | 173.686 | 4.678 | 170.378 | 297.019 | 753.1 | — |
| v2-shim | 2 | 156.455 | 149.945 | 162.966 | 9.207 | 156.455 | 274.839 | 773.7 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.66 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 117.997 | 111.720 | 120.061 | 3.377 | 117.281 | 13.906 | 642.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 129.717 | 129.007 | 130.426 | 1.003 | 129.717 | 14.023 | 642.2 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 59.762 | 59.546 | 61.377 | 0.841 | 60.246 | 13.875 | 642.2 | 1.00× |
| v2-fat | 5 | 56.396 | 55.683 | 57.267 | 0.600 | 56.418 | 0.016 | 5.7 | 1.06× |
| v2-shim | 5 | 57.932 | 57.580 | 58.044 | 0.177 | 57.868 | 0.016 | 8.7 | 1.03× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 68.113 | 67.309 | 68.916 | 1.137 | 68.113 | 13.930 | 642.2 | 1.00× |
| v2-shim | 2 | 67.659 | 67.147 | 68.171 | 0.725 | 67.659 | 0.461 | 114.1 | 1.01× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 99.987 | 99.042 | 100.446 | 0.521 | 99.860 | 117.217 | 1170.1 | 1.00× |
| v2-fat | 5 | 196.936 | 195.193 | 197.732 | 0.976 | 196.638 | 210.133 | 1339.8 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 107.087 | 105.797 | 108.376 | 1.823 | 107.087 | 124.279 | 1316.9 | 1.00× |
| v2-fat | 2 | 216.104 | 215.762 | 216.446 | 0.484 | 216.104 | 229.897 | 1495.3 | 0.50× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.030 | 48.781 | 49.255 | 0.174 | 49.032 | 59.794 | 853.4 | 1.00× |
| v2-fat | 5 | 109.483 | 108.911 | 111.312 | 0.923 | 109.720 | 182.483 | 871.1 | 0.45× |
| v2-shim | 5 | 110.105 | 109.031 | 111.003 | 0.818 | 110.008 | 183.109 | 872.9 | 0.45× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 53.376 | 52.940 | 53.812 | 0.616 | 53.376 | 64.880 | 853.6 | 1.00× |
| v2-fat | 2 | 118.339 | 117.995 | 118.683 | 0.486 | 118.339 | 190.590 | 833.5 | 0.45× |
| v2-shim | 2 | 121.170 | 120.672 | 121.669 | 0.705 | 121.170 | 194.279 | 836.1 | 0.44× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 125.357 | 123.451 | 125.910 | 0.970 | 125.065 | 153.412 | 1203.7 | 1.00× |
| v2-fat | 5 | 194.723 | 193.914 | 196.178 | 0.911 | 194.924 | 223.046 | 1304.7 | 0.64× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 133.549 | 132.650 | 134.448 | 1.272 | 133.549 | 163.791 | 1348.9 | 1.00× |
| v2-fat | 2 | 216.878 | 216.305 | 217.451 | 0.811 | 216.878 | 239.629 | 1441.3 | 0.62× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 63.219 | 62.584 | 64.931 | 0.946 | 63.533 | 82.864 | 908.0 | 1.00× |
| v2-fat | 5 | 104.247 | 102.134 | 111.264 | 4.365 | 106.302 | 197.540 | 866.8 | 0.61× |
| v2-shim | 5 | 103.546 | 103.074 | 111.467 | 3.597 | 105.649 | 199.304 | 869.5 | 0.61× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 67.990 | 67.462 | 68.517 | 0.746 | 67.990 | 88.662 | 905.4 | 1.00× |
| v2-fat | 2 | 118.072 | 117.611 | 118.532 | 0.651 | 118.072 | 213.814 | 823.0 | 0.58× |
| v2-shim | 2 | 126.946 | 123.391 | 130.500 | 5.027 | 126.946 | 213.924 | 825.7 | 0.54× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
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
| v1-packed-mn | 5 | 163.307 | 129.106 | 178.043 | 19.822 | 156.750 | 172.543 | 2080.4 | 1.00× |
| v2-fat | 5 | 179.168 | 168.963 | 200.259 | 12.211 | 180.583 | 187.878 | 1620.3 | 0.91× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 172.673 | 166.376 | 178.971 | 8.906 | 172.673 | 181.162 | 1992.2 | 1.00× |
| v2-fat | 2 | 192.789 | 192.305 | 193.272 | 0.683 | 192.789 | 200.781 | 1673.9 | 0.90× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 96.954 | 87.245 | 97.886 | 4.438 | 95.054 | 99.743 | 1726.0 | 1.00× |
| v2-fat | 5 | 100.083 | 87.462 | 111.113 | 9.479 | 98.022 | 188.134 | 988.1 | 0.97× |
| v2-shim | 5 | 90.571 | 82.285 | 93.840 | 4.444 | 89.581 | 166.720 | 976.2 | 1.07× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 95.459 | 93.118 | 97.801 | 3.312 | 95.459 | 98.451 | 1680.6 | 1.00× |
| v2-fat | 2 | 109.156 | 107.398 | 110.915 | 2.487 | 109.156 | 190.219 | 954.8 | 0.87× |
| v2-shim | 2 | 109.038 | 103.081 | 114.996 | 8.425 | 109.038 | 193.789 | 934.3 | 0.88× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.67 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 262.189 | 260.016 | 359.695 | 44.146 | 291.171 | 289.152 | 1303.0 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 404.411 | 401.487 | 407.334 | 4.134 | 404.411 | 438.513 | 1425.7 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 166.129 | 145.956 | 183.320 | 13.390 | 166.269 | 301.808 | 784.5 | — |
| v2-shim | 5 | 165.369 | 131.762 | 171.885 | 19.529 | 154.899 | 302.511 | 767.9 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 172.347 | 164.697 | 179.997 | 10.818 | 172.347 | 295.121 | 752.4 | — |
| v2-shim | 2 | 183.134 | 183.015 | 183.253 | 0.168 | 183.134 | 317.950 | 755.5 | — |

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
| v1-packed-mn | 5 | 135.678 | 134.034 | 137.257 | 1.171 | 135.706 | 167.266 | 1556.8 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 144.566 | 144.430 | 144.702 | 0.193 | 144.566 | 178.508 | 1593.1 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 68.478 | 68.225 | 69.024 | 0.317 | 68.522 | 92.062 | 1574.5 | 1.00× |
| v2-fat | 5 | 62.211 | 62.002 | 62.352 | 0.142 | 62.173 | 88.016 | 2508.8 | 1.10× |
| v2-shim | 5 | 64.148 | 62.788 | 65.050 | 0.869 | 64.048 | 91.406 | 2490.2 | 1.07× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 73.868 | 72.732 | 75.004 | 1.607 | 73.868 | 98.930 | 1566.5 | 1.00× |
| v2-shim | 2 | 70.395 | 69.538 | 71.253 | 1.213 | 70.395 | 99.195 | 2512.9 | 1.05× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

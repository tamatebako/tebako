# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.071 | 99.446 | 104.737 | 2.011 | 101.388 | 117.919 | 1159.7 | 1.00× |
| v2-fat | 5 | 199.305 | 196.012 | 199.994 | 1.589 | 198.791 | 211.215 | 1351.7 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.163 | 108.712 | 109.614 | 0.638 | 109.163 | 127.123 | 1288.6 | 1.00× |
| v2-fat | 2 | 215.475 | 215.038 | 215.912 | 0.618 | 215.475 | 228.524 | 1490.3 | 0.51× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.168 | 48.497 | 50.580 | 0.782 | 49.278 | 59.357 | 853.7 | 1.00× |
| v2-fat | 5 | 109.615 | 108.930 | 110.542 | 0.648 | 109.656 | 181.648 | 869.9 | 0.45× |
| v2-shim | 5 | 110.627 | 109.623 | 111.851 | 0.796 | 110.713 | 183.687 | 883.1 | 0.44× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 52.947 | 52.662 | 53.231 | 0.403 | 52.947 | 63.608 | 851.9 | 1.00× |
| v2-fat | 2 | 119.138 | 119.125 | 119.151 | 0.018 | 119.138 | 191.176 | 829.7 | 0.44× |
| v2-shim | 2 | 120.929 | 120.221 | 121.637 | 1.002 | 120.929 | 193.406 | 838.7 | 0.44× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 118.104 | 116.053 | 123.602 | 3.010 | 118.788 | 144.919 | 1201.0 | 1.00× |
| v2-fat | 5 | 179.665 | 178.211 | 182.932 | 2.233 | 180.551 | 203.850 | 1330.2 | 0.66× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 125.895 | 124.190 | 127.599 | 2.411 | 125.895 | 154.117 | 1356.2 | 1.00× |
| v2-fat | 2 | 201.883 | 201.599 | 202.166 | 0.401 | 201.883 | 221.113 | 1456.7 | 0.62× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 57.655 | 57.360 | 58.356 | 0.483 | 57.820 | 75.149 | 908.2 | 1.00× |
| v2-fat | 5 | 96.864 | 94.660 | 102.441 | 3.685 | 98.464 | 183.979 | 863.9 | 0.60× |
| v2-shim | 5 | 100.146 | 94.993 | 104.236 | 3.659 | 100.543 | 180.935 | 872.5 | 0.58× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 63.965 | 63.055 | 64.876 | 1.287 | 63.965 | 82.492 | 907.3 | 1.00× |
| v2-fat | 2 | 112.369 | 111.092 | 113.646 | 1.806 | 112.369 | 198.368 | 827.7 | 0.57× |
| v2-shim | 2 | 111.178 | 110.845 | 111.510 | 0.470 | 111.178 | 200.775 | 821.8 | 0.58× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 129.416 | 128.335 | 130.954 | 1.220 | 129.592 | 133.900 | 2051.5 | 1.00× |
| v2-fat | 5 | 152.038 | 151.437 | 153.405 | 0.752 | 152.222 | 156.691 | 1645.5 | 0.85× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 137.613 | 135.284 | 139.942 | 3.294 | 137.613 | 141.032 | 2056.5 | 1.00× |
| v2-fat | 2 | 169.927 | 167.426 | 172.428 | 3.537 | 169.927 | 172.711 | 1762.3 | 0.81× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.357 | 77.166 | 138.663 | 26.691 | 111.360 | 106.656 | 1660.3 | 1.00× |
| v2-fat | 5 | 103.120 | 87.426 | 131.110 | 16.697 | 104.196 | 181.028 | 982.4 | 0.98× |
| v2-shim | 5 | 100.729 | 82.469 | 118.779 | 12.898 | 100.489 | 188.250 | 971.2 | 1.01× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 82.597 | 79.874 | 85.320 | 3.851 | 82.597 | 85.857 | 1667.4 | 1.00× |
| v2-fat | 2 | 93.235 | 92.821 | 93.649 | 0.586 | 93.235 | 159.242 | 939.9 | 0.89× |
| v2-shim | 2 | 90.190 | 87.019 | 93.362 | 4.485 | 90.190 | 155.344 | 939.6 | 0.92× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 330.254 | 285.299 | 365.037 | 32.677 | 327.840 | 369.765 | 1320.3 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 449.247 | 442.380 | 456.115 | 9.712 | 449.247 | 481.899 | 1388.3 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 195.308 | 182.460 | 209.197 | 11.171 | 194.798 | 361.281 | 767.6 | — |
| v2-shim | 5 | 193.602 | 185.270 | 194.143 | 3.898 | 191.229 | 351.468 | 775.7 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 187.215 | 175.979 | 198.450 | 15.890 | 187.215 | 335.298 | 755.1 | — |
| v2-shim | 2 | 188.429 | 157.037 | 219.821 | 44.395 | 188.429 | 325.914 | 750.9 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.68 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 135.004 | 131.451 | 136.982 | 2.011 | 134.697 | 166.406 | 1533.8 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 144.490 | 143.968 | 145.011 | 0.737 | 144.490 | 177.352 | 1513.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 67.578 | 66.969 | 69.186 | 1.040 | 67.981 | 92.891 | 1576.6 | 1.00× |
| v2-fat | 5 | 61.517 | 60.874 | 61.785 | 0.337 | 61.437 | 87.328 | 2483.6 | 1.10× |
| v2-shim | 5 | 63.748 | 62.878 | 64.994 | 0.870 | 63.694 | 90.109 | 2490.4 | 1.06× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 73.833 | 72.437 | 75.228 | 1.974 | 73.833 | 100.516 | 1586.0 | 1.00× |
| v2-shim | 2 | 69.699 | 68.515 | 70.882 | 1.674 | 69.699 | 97.086 | 2516.3 | 1.06× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

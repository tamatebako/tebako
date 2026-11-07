# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 104.471 | 104.275 | 105.487 | 0.494 | 104.635 | 121.966 | 1172.6 | 1.00× |
| v2-fat | 5 | 203.781 | 203.432 | 203.960 | 0.206 | 203.750 | 215.852 | 1343.2 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 111.458 | 110.516 | 112.400 | 1.333 | 111.458 | 129.442 | 1272.3 | 1.00× |
| v2-fat | 2 | 222.422 | 221.525 | 223.318 | 1.268 | 222.422 | 234.915 | 1436.2 | 0.50× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 51.135 | 50.726 | 52.127 | 0.575 | 51.212 | 62.748 | 852.9 | 1.00× |
| v2-fat | 5 | 112.509 | 112.112 | 113.547 | 0.576 | 112.573 | 186.677 | 859.4 | 0.45× |
| v2-shim | 5 | 112.962 | 112.668 | 114.097 | 0.556 | 113.175 | 187.392 | 874.1 | 0.45× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.635 | 55.189 | 56.080 | 0.630 | 55.635 | 66.877 | 851.0 | 1.00× |
| v2-fat | 2 | 122.549 | 122.266 | 122.832 | 0.401 | 122.549 | 194.289 | 827.1 | 0.45× |
| v2-shim | 2 | 124.745 | 124.512 | 124.977 | 0.329 | 124.745 | 198.294 | 822.8 | 0.45× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 91.529 | 89.140 | 92.501 | 1.583 | 90.989 | 111.976 | 1202.2 | 1.00× |
| v2-fat | 5 | 147.113 | 145.905 | 150.308 | 1.654 | 147.562 | 161.654 | 1324.3 | 0.62× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 98.364 | 98.237 | 98.490 | 0.179 | 98.364 | 118.245 | 1349.7 | 1.00× |
| v2-fat | 2 | 167.052 | 166.086 | 168.018 | 1.366 | 167.052 | 179.069 | 1448.4 | 0.59× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 44.639 | 44.203 | 46.268 | 0.832 | 44.864 | 58.666 | 908.2 | 1.00× |
| v2-fat | 5 | 82.704 | 77.612 | 84.423 | 2.633 | 81.598 | 145.571 | 861.5 | 0.54× |
| v2-shim | 5 | 77.800 | 77.497 | 83.468 | 2.523 | 79.009 | 146.497 | 876.3 | 0.57× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 49.653 | 49.450 | 49.857 | 0.288 | 49.653 | 63.705 | 901.1 | 1.00× |
| v2-fat | 2 | 93.326 | 91.773 | 94.878 | 2.196 | 93.326 | 155.440 | 822.1 | 0.53× |
| v2-shim | 2 | 100.496 | 100.229 | 100.763 | 0.377 | 100.496 | 163.056 | 820.2 | 0.49× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 394.849 | 385.638 | 411.650 | 11.022 | 397.609 | 447.047 | 1335.2 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 426.899 | 425.859 | 427.938 | 1.470 | 426.899 | 467.491 | 1387.2 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 210.003 | 183.072 | 217.033 | 16.333 | 202.378 | 383.461 | 780.5 | — |
| v2-shim | 5 | 232.770 | 164.297 | 255.854 | 34.853 | 223.457 | 431.181 | 781.2 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 233.010 | 225.256 | 240.765 | 10.966 | 233.010 | 401.125 | 745.8 | — |
| v2-shim | 2 | 225.074 | 218.909 | 231.239 | 8.719 | 225.074 | 403.116 | 757.8 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.71 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 81.817 | 78.547 | 83.363 | 2.152 | 80.833 | 97.891 | 1557.8 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 87.060 | 85.557 | 88.562 | 2.124 | 87.060 | 104.211 | 1533.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 41.358 | 39.477 | 43.612 | 1.758 | 41.809 | 53.594 | 1603.3 | 1.00× |
| v2-fat | 5 | 38.308 | 37.923 | 43.825 | 2.491 | 39.431 | 53.016 | 2483.6 | 1.08× |
| v2-shim | 5 | 39.118 | 38.473 | 40.785 | 0.870 | 39.344 | 52.406 | 2485.9 | 1.06× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 45.530 | 44.750 | 46.309 | 1.102 | 45.530 | 57.992 | 1589.5 | 1.00× |
| v2-shim | 2 | 45.946 | 45.082 | 46.810 | 1.221 | 45.946 | 59.109 | 2293.9 | 0.99× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

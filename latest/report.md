# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 102.342 | 101.633 | 103.817 | 0.809 | 102.476 | 119.793 | 1168.0 | 1.00× |
| v2-fat | 5 | 202.068 | 200.718 | 203.702 | 1.122 | 202.171 | 214.335 | 1349.3 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 109.581 | 108.858 | 110.303 | 1.022 | 109.581 | 127.583 | 1292.6 | 1.00× |
| v2-fat | 2 | 218.742 | 218.322 | 219.162 | 0.594 | 218.742 | 232.001 | 1431.8 | 0.50× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 50.489 | 49.840 | 50.855 | 0.397 | 50.439 | 61.890 | 854.0 | 1.00× |
| v2-fat | 5 | 112.966 | 112.300 | 113.157 | 0.363 | 112.786 | 186.733 | 862.5 | 0.45× |
| v2-shim | 5 | 113.043 | 112.655 | 113.349 | 0.307 | 112.985 | 186.504 | 878.2 | 0.45× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.191 | 53.473 | 54.909 | 1.015 | 54.191 | 65.809 | 851.4 | 1.00× |
| v2-fat | 2 | 120.953 | 120.626 | 121.279 | 0.462 | 120.953 | 194.889 | 826.2 | 0.45× |
| v2-shim | 2 | 123.408 | 123.352 | 123.464 | 0.079 | 123.408 | 196.379 | 829.9 | 0.44× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 127.896 | 126.300 | 131.159 | 2.006 | 128.207 | 155.297 | 1192.5 | 1.00× |
| v2-fat | 5 | 198.538 | 197.933 | 199.989 | 0.842 | 198.643 | 223.806 | 1319.8 | 0.64× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 135.767 | 135.222 | 136.312 | 0.771 | 135.767 | 165.367 | 1307.8 | 1.00× |
| v2-fat | 2 | 222.069 | 221.381 | 222.756 | 0.972 | 222.069 | 246.518 | 1436.9 | 0.61× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 63.860 | 62.578 | 65.552 | 1.130 | 63.834 | 82.781 | 907.7 | 1.00× |
| v2-fat | 5 | 113.932 | 105.368 | 115.049 | 5.028 | 110.909 | 197.075 | 854.9 | 0.56× |
| v2-shim | 5 | 105.720 | 105.071 | 115.885 | 5.349 | 109.296 | 202.028 | 863.2 | 0.60× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 69.647 | 69.484 | 69.811 | 0.231 | 69.647 | 90.072 | 901.1 | 1.00× |
| v2-fat | 2 | 129.255 | 126.269 | 132.240 | 4.222 | 129.255 | 213.381 | 819.7 | 0.54× |
| v2-shim | 2 | 128.724 | 126.268 | 131.179 | 3.473 | 128.724 | 222.179 | 819.7 | 0.54× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 302.987 | 287.994 | 312.058 | 11.607 | 300.629 | 339.211 | 1300.7 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 344.472 | 338.807 | 350.137 | 8.012 | 344.472 | 373.580 | 1419.5 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 167.897 | 152.568 | 174.505 | 8.881 | 165.963 | 316.317 | 772.8 | — |
| v2-shim | 5 | 157.427 | 135.279 | 170.607 | 13.245 | 153.940 | 289.161 | 772.1 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 170.304 | 163.353 | 177.254 | 9.830 | 170.304 | 303.880 | 744.8 | — |
| v2-shim | 2 | 176.862 | 169.188 | 184.537 | 10.854 | 176.862 | 317.871 | 772.7 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.69 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 138.110 | 138.001 | 138.733 | 0.361 | 138.326 | 170.250 | 1574.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 148.108 | 147.420 | 148.797 | 0.974 | 148.108 | 181.469 | 1658.0 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 68.904 | 68.062 | 69.864 | 0.792 | 69.028 | 93.188 | 1562.7 | 1.00× |
| v2-fat | 5 | 64.640 | 63.831 | 64.978 | 0.468 | 64.607 | 92.328 | 2480.0 | 1.07× |
| v2-shim | 5 | 66.531 | 66.040 | 66.623 | 0.232 | 66.428 | 92.906 | 2494.3 | 1.04× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 76.156 | 75.315 | 76.998 | 1.190 | 76.156 | 105.102 | 1601.7 | 1.00× |
| v2-shim | 2 | 73.762 | 73.278 | 74.246 | 0.684 | 73.762 | 102.023 | 2512.9 | 1.03× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

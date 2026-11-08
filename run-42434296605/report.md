# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 99.970 | 97.761 | 101.831 | 1.542 | 99.934 | 116.507 | 1167.4 | 1.00× |
| v2-fat | 5 | 196.718 | 194.970 | 198.576 | 1.417 | 196.549 | 212.442 | 1352.9 | 0.51× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 111.803 | 111.369 | 112.237 | 0.614 | 111.803 | 125.421 | 1252.1 | 1.00× |
| v2-fat | 2 | 214.885 | 214.616 | 215.154 | 0.381 | 214.885 | 225.308 | 1496.8 | 0.52× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 48.751 | 47.968 | 49.949 | 0.735 | 48.939 | 59.426 | 853.2 | 1.00× |
| v2-fat | 5 | 110.667 | 109.729 | 111.384 | 0.658 | 110.580 | 183.197 | 859.3 | 0.44× |
| v2-shim | 5 | 110.948 | 110.718 | 112.395 | 0.696 | 111.287 | 183.465 | 869.8 | 0.44× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 55.408 | 52.567 | 58.250 | 4.018 | 55.408 | 64.191 | 851.1 | 1.00× |
| v2-fat | 2 | 121.540 | 120.707 | 122.372 | 1.178 | 121.540 | 191.611 | 827.8 | 0.46× |
| v2-shim | 2 | 126.026 | 123.545 | 128.507 | 3.509 | 126.026 | 195.139 | 819.6 | 0.44× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 93.707 | 93.499 | 95.346 | 0.809 | 94.146 | 115.763 | 1208.7 | 1.00× |
| v2-fat | 5 | 151.543 | 150.426 | 153.736 | 1.358 | 151.806 | 171.724 | 1330.4 | 0.62× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 105.317 | 104.865 | 105.770 | 0.640 | 105.317 | 123.159 | 1308.0 | 1.00× |
| v2-fat | 2 | 175.061 | 173.143 | 176.979 | 2.713 | 175.061 | 190.222 | 1472.0 | 0.60× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 47.048 | 46.328 | 48.213 | 0.712 | 47.125 | 61.469 | 908.0 | 1.00× |
| v2-fat | 5 | 79.915 | 78.567 | 84.550 | 2.454 | 81.135 | 148.677 | 861.6 | 0.59× |
| v2-shim | 5 | 79.604 | 79.144 | 85.264 | 3.194 | 81.743 | 149.536 | 871.7 | 0.59× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 52.775 | 52.616 | 52.934 | 0.225 | 52.775 | 65.615 | 903.2 | 1.00× |
| v2-fat | 2 | 95.434 | 92.289 | 98.578 | 4.447 | 95.434 | 158.377 | 819.6 | 0.55× |
| v2-shim | 2 | 105.851 | 105.631 | 106.071 | 0.311 | 105.851 | 165.923 | 815.8 | 0.50× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

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
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 145.454 | 120.906 | 176.598 | 22.503 | 143.034 | 150.304 | 2048.2 | 1.00× |
| v2-fat | 5 | 156.285 | 146.541 | 194.856 | 19.348 | 161.504 | 161.623 | 1656.2 | 0.93× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 155.844 | 142.514 | 169.173 | 18.851 | 155.844 | 160.629 | 2070.5 | 1.00× |
| v2-fat | 2 | 188.719 | 178.521 | 198.917 | 14.422 | 188.719 | 194.406 | 1674.8 | 0.83× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 93.629 | 89.850 | 102.069 | 5.222 | 95.136 | 99.465 | 1681.6 | 1.00× |
| v2-fat | 5 | 95.888 | 92.565 | 109.964 | 7.393 | 99.804 | 170.133 | 996.1 | 0.98× |
| v2-shim | 5 | 98.209 | 96.386 | 101.594 | 2.020 | 98.787 | 183.314 | 968.3 | 0.95× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 104.539 | 100.964 | 108.114 | 5.056 | 104.539 | 107.518 | 1730.4 | 1.00× |
| v2-fat | 2 | 97.879 | 95.096 | 100.661 | 3.935 | 97.879 | 168.069 | 948.3 | 1.07× |
| v2-shim | 2 | 119.575 | 117.764 | 121.386 | 2.562 | 119.575 | 205.933 | 945.0 | 0.87× |

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 370.458 | 312.141 | 374.459 | 26.481 | 355.215 | 402.049 | 1337.1 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 470.183 | 450.854 | 489.512 | 27.335 | 470.183 | 506.520 | 1398.0 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 158.413 | 143.737 | 195.116 | 22.189 | 164.329 | 292.795 | 764.0 | — |
| v2-shim | 5 | 155.776 | 142.968 | 202.838 | 24.507 | 163.297 | 287.762 | 765.0 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 231.726 | 227.303 | 236.149 | 6.255 | 231.726 | 406.303 | 756.8 | — |
| v2-shim | 2 | 214.421 | 207.469 | 221.374 | 9.832 | 214.421 | 378.285 | 756.9 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.73 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 133.769 | 129.092 | 134.112 | 2.089 | 132.777 | 164.656 | 1513.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 145.003 | 143.749 | 146.257 | 1.774 | 145.003 | 175.805 | 1540.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 68.755 | 67.001 | 69.516 | 1.025 | 68.353 | 94.172 | 1602.5 | 1.00× |
| v2-fat | 5 | 62.941 | 62.883 | 64.217 | 0.566 | 63.259 | 88.828 | 2485.2 | 1.09× |
| v2-shim | 5 | 64.566 | 64.289 | 65.325 | 0.420 | 64.659 | 90.422 | 2498.6 | 1.06× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 74.695 | 74.662 | 74.727 | 0.046 | 74.695 | 98.594 | 1565.8 | 1.00× |
| v2-shim | 2 | 74.354 | 71.478 | 77.230 | 4.067 | 74.354 | 98.328 | 2479.1 | 1.00× |

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

# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB · image ubuntu24-arm64/20261122.193.1
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 0.733 | 0.688 | 0.750 | 0.024 | 0.729 | 0.268 | 176.2 | — |

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 101.397 | 100.613 | 102.509 | 0.832 | 101.654 | 119.394 | 1173.3 | 1.00× |
| v2-fat | 5 | 85.780 | 85.553 | 86.684 | 0.448 | 85.911 | 97.298 | 1711.6 | 1.18× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 108.616 | 108.017 | 109.215 | 0.848 | 108.616 | 126.246 | 1252.6 | 1.00× |
| v2-fat | 2 | 102.538 | 102.535 | 102.542 | 0.005 | 102.538 | 114.770 | 1824.3 | 1.06× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 49.314 | 48.997 | 49.640 | 0.236 | 49.311 | 59.718 | 853.9 | 1.00× |
| v2-fat | 5 | 28.973 | 28.912 | 29.338 | 0.181 | 29.061 | 42.691 | 1220.9 | 1.70× |
| v2-shim | 5 | 29.240 | 28.376 | 29.749 | 0.506 | 29.170 | 42.981 | 1221.9 | 1.69× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 54.167 | 53.522 | 54.813 | 0.913 | 54.167 | 65.915 | 848.7 | 1.00× |
| v2-fat | 2 | 37.773 | 37.472 | 38.075 | 0.426 | 37.773 | 50.606 | 1170.5 | 1.43× |
| v2-shim | 2 | 39.573 | 39.062 | 40.084 | 0.723 | 39.573 | 53.468 | 1186.8 | 1.37× |

Unavailable arms:

- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- author-repress / v2-tools [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB · image ubuntu24/20261122.23.1
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 0.585 | 0.578 | 0.596 | 0.007 | 0.585 | 0.253 | 179.2 | — |

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 85.033 | 83.278 | 86.875 | 1.400 | 85.269 | 103.768 | 1209.7 | 1.00× |
| v2-fat | 5 | 68.119 | 67.138 | 68.453 | 0.504 | 67.979 | 83.085 | 1699.0 | 1.25× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 91.607 | 91.584 | 91.629 | 0.032 | 91.607 | 109.181 | 1335.9 | 1.00× |
| v2-fat | 2 | 81.350 | 80.207 | 82.493 | 1.616 | 81.350 | 90.937 | 1861.2 | 1.13× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 42.751 | 42.258 | 43.274 | 0.388 | 42.818 | 55.002 | 908.5 | 1.00× |
| v2-fat | 5 | 25.706 | 25.557 | 25.957 | 0.156 | 25.730 | 42.529 | 1211.7 | 1.66× |
| v2-shim | 5 | 25.884 | 25.307 | 26.257 | 0.367 | 25.878 | 42.616 | 1212.1 | 1.65× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 47.410 | 46.747 | 48.073 | 0.937 | 47.410 | 60.214 | 903.3 | 1.00× |
| v2-fat | 2 | 35.735 | 34.791 | 36.679 | 1.335 | 35.735 | 51.626 | 1173.2 | 1.33× |
| v2-shim | 2 | 37.588 | 36.751 | 38.425 | 1.184 | 37.588 | 54.037 | 1171.4 | 1.26× |

Unavailable arms:

- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- author-repress / v2-tools [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 0.249 | 0.249 | 0.251 | 0.001 | 0.249 | 0.248 | 177.8 | — |

### author-repress — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 6.463 | 6.452 | 6.479 | 0.011 | 6.463 | 1.726 | 128.1 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- author-repress / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm
- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 0.297 | 0.295 | 0.305 | 0.004 | 0.299 | 0.296 | 175.9 | — |

### author-repress — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 5.947 | 5.943 | 5.951 | 0.004 | 5.947 | 1.396 | 129.0 | — |

Unavailable arms:

- author-repress / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm
- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB · image macos15/20261118.0550.1
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 1.835 | 1.768 | 1.909 | 0.058 | 1.827 | 0.690 | 181.2 | — |

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 179.903 | 160.519 | 199.997 | 15.589 | 181.024 | 187.646 | 1944.9 | 1.00× |
| v2-fat | 5 | 102.197 | 97.437 | 103.696 | 2.750 | 101.045 | 112.110 | 1927.1 | 1.76× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 192.652 | 182.742 | 202.562 | 14.015 | 192.652 | 200.513 | 2016.0 | 1.00× |
| v2-fat | 2 | 113.972 | 110.295 | 117.649 | 5.200 | 113.972 | 122.043 | 2009.9 | 1.69× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 105.661 | 95.771 | 133.912 | 14.718 | 108.809 | 107.459 | 1708.8 | 1.00× |
| v2-fat | 5 | 49.890 | 46.088 | 54.602 | 3.370 | 49.530 | 62.078 | 1242.1 | 2.12× |
| v2-shim | 5 | 44.668 | 40.037 | 45.929 | 2.399 | 43.820 | 54.662 | 1218.7 | 2.37× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 101.434 | 94.618 | 108.250 | 9.639 | 101.434 | 104.290 | 1750.6 | 1.00× |
| v2-fat | 2 | 60.363 | 60.022 | 60.703 | 0.482 | 60.363 | 70.393 | 1235.8 | 1.68× |
| v2-shim | 2 | 55.822 | 55.594 | 56.049 | 0.322 | 55.822 | 63.404 | 1227.7 | 1.82× |

Unavailable arms:

- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- author-repress / v2-tools [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB · image macos15/20260824.0482.1
Versions: tebako 2.8.86 · runtime 0.17.15-4.0.9 · payload 1.17.0-6 · image limnifs

### author-first-run — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-tools | 5 | 1.821 | 1.703 | 2.367 | 0.266 | 1.940 | 0.421 | 80.8 | — |

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 276.239 | 269.047 | 311.802 | 17.069 | 281.984 | 326.867 | 1280.8 | — |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 305.323 | 292.429 | 318.216 | 18.234 | 305.323 | 341.153 | 1342.2 | — |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 5 | 77.877 | 65.993 | 100.917 | 15.833 | 80.781 | 109.897 | 733.0 | — |
| v2-shim | 5 | 84.173 | 64.096 | 98.241 | 14.596 | 79.724 | 114.332 | 709.8 | — |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v2-fat | 2 | 127.267 | 117.882 | 136.652 | 13.273 | 127.267 | 166.566 | 725.3 | — |
| v2-shim | 2 | 130.182 | 124.834 | 135.529 | 7.562 | 130.182 | 170.456 | 738.8 | — |

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- author-first-run / v2-tools [cold]: unavailable — v2-tools arms have no cold story — a cold press would re-resolve the runtime (a download) inside the measured span and the matrix is offline; the author-loop cells are warm

Failed runs:

- compile-medium-rice / v2-shim [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- author-repress / v2-tools [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB · image win25-vs2026/20261123.321.1
Versions: tebako 2.8.86 · runtime 0.17.16-4.0.9 · payload 1.17.0-6 · packed-mn v1.14.4 (metanorma-cli 1.14.4) · image limnifs

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 135.736 | 134.684 | 136.509 | 0.673 | 135.638 | 166.609 | 1576.7 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 147.796 | 147.563 | 148.029 | 0.330 | 147.796 | 178.180 | 1597.7 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 71.282 | 70.475 | 76.899 | 2.633 | 72.228 | 97.703 | 1578.4 | 1.00× |
| v2-fat | 5 | 44.490 | 43.202 | 45.424 | 0.827 | 44.422 | 72.406 | 2580.8 | 1.60× |
| v2-shim | 5 | 46.415 | 45.871 | 47.113 | 0.495 | 46.357 | 75.781 | 2590.5 | 1.54× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 76.553 | 75.778 | 77.328 | 1.096 | 76.553 | 101.531 | 1561.2 | 1.00× |
| v2-shim | 2 | 53.244 | 51.574 | 54.914 | 2.362 | 53.244 | 83.805 | 2599.4 | 1.44× |

Failed runs:

- compile-small-iso / v2-fat [cold] #1 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-small-iso / v2-fat [cold] #2 (exit -1073741515): failed — exit -1073741515 (expected 0)
- compile-medium-rice / v2-shim [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)
- author-repress / v2-tools [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- author-first-run / v2-tools [warm] #0 (exit -1073741515): failed — warmup: exit -1073741515 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

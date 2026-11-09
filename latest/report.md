# tebako benchmark report — metanorma-v1-vs-v2

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-gnu-arm64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-linux-gnu-arm64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 107.092 | 106.857 | 107.392 | 0.231 | 107.097 | 134.432 | 1198.0 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 162.639 | 144.043 | 181.235 | 26.299 | 162.639 | 141.541 | 1308.1 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 71) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 69) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 71) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: `/home/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 69) — see /home/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.74 · runtime 0.17.4-4.0.9 · payload 1.17.0-6 · image limnifs

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-musl-arm64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for linux-musl-arm64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: the priming dispatch left no ruby runtime for linux-musl-arm64 in /home/runner/work/tebako/tebako/tebako-rs/out/home/.tebako/runtimes — see logs/acquire-prime-runtime.log

Failed runs:

- compile-small-iso / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v2-fat [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-bootstrap-2.8.74-linux-musl-x86_64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-2.8.74-linux-musl-x86_64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- compile-medium-rice / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 146.646 | 126.901 | 150.901 | 9.461 | 142.103 | 154.226 | 2010.4 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 173.515 | 167.373 | 179.657 | 8.687 | 173.515 | 145.734 | 2046.1 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/tebako-shim-2.8.74-macos-arm64

Failed runs:

- compile-small-iso / v1-packed-mn [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- compile-small-iso / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v1-packed-mn: unavailable — platforms.yaml: v1_asset is null for macos-x86_64 — packed-mn shipped no asset for this triplet
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: `/Users/runner/work/tebako/tebako/tebako-rs/out/bin/tebako install metanorma@1.17.0` failed (exit 75) — see /Users/runner/work/tebako/tebako/tebako-rs/out/logs/acquire-install.log

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: packed-mn v1.14.4 (metanorma-cli 1.14.4)

### compile-medium-rice — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 82.406 | 81.816 | 107.566 | 11.187 | 87.655 | 98.891 | 1631.2 | 1.00× |

### compile-medium-rice — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 155.397 | 153.748 | 157.047 | 2.332 | 155.397 | 107.250 | 1613.8 | 1.00× |

### compile-small-iso — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 5 | 44.975 | 43.486 | 79.996 | 15.926 | 51.553 | 59.203 | 1617.3 | 1.00× |

### compile-small-iso — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs v1 |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| v1-packed-mn | 2 | 59.898 | 42.968 | 76.829 | 23.944 | 59.898 | 56.234 | 1578.4 | 1.00× |

Unavailable arms:

- compile-small-iso / v2-shim: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 69) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log
- compile-small-iso / v2-fat: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 75) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log
- compile-medium-rice / v2-shim: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 69) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log
- compile-medium-rice / v2-fat: unavailable — v2 acquisition failed: acquire: `D:\a\tebako\tebako\tebako-rs\out\bin\tebako.exe install metanorma@1.17.0` failed (exit 75) — see D:\a\tebako\tebako\tebako-rs\out\logs\acquire-install.log

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS
- boot / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256
- fib / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS
- fib / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256
- ioread / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS
- ioread / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256
- treewalk / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS: 504 fetching https://github.com/tamatebako/tebako/releases/download/v2.8.74/SHA256SUMS
- treewalk / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-gnu/tebako-runtime-0.17.1-ruby-3.3.12-linux-gnu-arm64.sha256

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.280 | 0.278 | 0.284 | 0.002 | 0.280 | 0.280 | 79.6 | 1.00× |
| lazy-ruby | 5 | 0.280 | 0.278 | 0.286 | 0.003 | 0.281 | 0.279 | 79.8 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.280 | 0.280 | 0.286 | 0.003 | 0.282 | 0.279 | 79.4 | 1.00× |
| lazy-ruby | 3 | 6.850 | 6.820 | 6.901 | 0.041 | 6.857 | 6.911 | 51.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.500 | 0.498 | 0.516 | 0.008 | 0.504 | 0.500 | 79.5 | 1.00× |
| lazy-ruby | 5 | 0.502 | 0.496 | 0.512 | 0.006 | 0.503 | 0.501 | 79.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.506 | 0.504 | 0.531 | 0.015 | 0.514 | 0.504 | 79.9 | 1.00× |
| lazy-ruby | 3 | 7.032 | 7.003 | 7.085 | 0.041 | 7.040 | 7.124 | 51.2 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.320 | 0.315 | 0.326 | 0.004 | 0.320 | 0.318 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.316 | 0.313 | 0.322 | 0.004 | 0.316 | 0.314 | 188.4 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.318 | 0.315 | 0.322 | 0.003 | 0.318 | 0.317 | 188.0 | 1.00× |
| lazy-ruby | 3 | 6.892 | 6.860 | 6.919 | 0.029 | 6.890 | 6.953 | 157.1 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.481 | 0.479 | 0.486 | 0.003 | 0.482 | 0.481 | 111.9 | 1.00× |
| lazy-ruby | 5 | 0.486 | 0.481 | 0.495 | 0.005 | 0.486 | 0.483 | 112.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.486 | 0.483 | 0.492 | 0.004 | 0.487 | 0.485 | 111.8 | 1.00× |
| lazy-ruby | 3 | 7.089 | 7.064 | 7.112 | 0.024 | 7.088 | 7.147 | 64.8 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- boot / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- fib / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- fib / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- ioread / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- ioread / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- treewalk / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256
- treewalk / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-arm64.tar.gz.sha256

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- boot / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- fib / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- fib / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- ioread / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- ioread / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- treewalk / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256
- treewalk / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-linux-musl/tebako-runtime-0.17.1-ruby-3.3.12-linux-musl-x86_64.sha256

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- boot / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- fib / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- fib / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- ioread / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- ioread / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- treewalk / eager-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256
- treewalk / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-arm64.tar.gz.sha256

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs
- boot / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256
- fib / eager-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs
- fib / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256
- ioread / eager-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs
- ioread / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256
- treewalk / eager-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tfs
- treewalk / lazy-ruby: unavailable — lazy runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.1-ruby3.3-macos/tebako-runtime-0.17.1-ruby-3.3.12-macos-x86_64.tar.gz.sha256

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.229 | 0.261 | 0.013 | 0.241 | 0.234 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.238 | 0.231 | 0.259 | 0.011 | 0.240 | 0.234 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.232 | 0.234 | 0.001 | 0.233 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.803 | 0.798 | 0.814 | 0.008 | 0.805 | 0.844 | 58.5 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.462 | 0.449 | 0.473 | 0.010 | 0.459 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.456 | 0.452 | 0.488 | 0.015 | 0.461 | 0.453 | 68.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.457 | 0.455 | 0.458 | 0.002 | 0.457 | 0.453 | 68.4 | 1.00× |
| lazy-ruby | 3 | 1.027 | 1.026 | 1.030 | 0.002 | 1.028 | 1.062 | 58.8 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.282 | 0.282 | 0.297 | 0.006 | 0.285 | 0.281 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.280 | 0.278 | 0.282 | 0.002 | 0.280 | 0.266 | 109.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.285 | 0.281 | 0.287 | 0.003 | 0.285 | 0.281 | 109.6 | 1.00× |
| lazy-ruby | 3 | 0.851 | 0.845 | 0.851 | 0.003 | 0.849 | 0.875 | 99.9 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.498 | 0.493 | 0.502 | 0.004 | 0.497 | 0.500 | 108.8 | 1.00× |
| lazy-ruby | 5 | 0.494 | 0.490 | 0.503 | 0.005 | 0.495 | 0.500 | 108.8 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.501 | 0.499 | 0.508 | 0.005 | 0.503 | 0.531 | 109.1 | 1.00× |
| lazy-ruby | 3 | 1.064 | 1.054 | 1.076 | 0.011 | 1.064 | 1.109 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

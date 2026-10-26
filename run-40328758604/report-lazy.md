# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.225 | 0.225 | 0.227 | 0.001 | 0.225 | 0.224 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.225 | 0.229 | 0.002 | 0.226 | 0.225 | 108.3 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.225 | 0.227 | 0.001 | 0.225 | 0.223 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.362 | 2.354 | 2.362 | 0.005 | 2.359 | 2.493 | 108.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.426 | 0.441 | 0.006 | 0.432 | 0.429 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.424 | 0.439 | 0.006 | 0.432 | 0.430 | 108.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.430 | 0.426 | 0.432 | 0.003 | 0.430 | 0.429 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.578 | 2.555 | 2.578 | 0.013 | 2.571 | 2.709 | 108.3 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.251 | 0.245 | 0.272 | 0.011 | 0.256 | 0.251 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.249 | 0.247 | 0.262 | 0.006 | 0.253 | 0.248 | 180.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.251 | 0.251 | 0.000 | 0.251 | 0.250 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.378 | 2.376 | 2.391 | 0.008 | 2.382 | 2.511 | 154.8 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.406 | 0.401 | 0.410 | 0.003 | 0.405 | 0.404 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.404 | 0.403 | 0.410 | 0.003 | 0.406 | 0.403 | 108.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.410 | 0.403 | 0.414 | 0.005 | 0.409 | 0.408 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.539 | 2.535 | 2.549 | 0.007 | 2.541 | 2.671 | 108.3 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.213 | 0.213 | 0.219 | 0.003 | 0.215 | 0.212 | 116.0 | 1.00× |
| lazy-ruby | 5 | 0.217 | 0.209 | 0.219 | 0.004 | 0.215 | 0.215 | 116.0 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.215 | 0.211 | 0.217 | 0.003 | 0.214 | 0.214 | 116.0 | 1.00× |
| lazy-ruby | 3 | 5.639 | 5.549 | 5.641 | 0.052 | 5.610 | 5.687 | 116.0 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.406 | 0.386 | 0.406 | 0.009 | 0.400 | 0.404 | 116.0 | 1.00× |
| lazy-ruby | 5 | 0.395 | 0.391 | 0.401 | 0.005 | 0.396 | 0.394 | 116.0 | 1.03× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.395 | 0.391 | 0.395 | 0.002 | 0.394 | 0.393 | 116.0 | 1.00× |
| lazy-ruby | 3 | 5.972 | 5.823 | 6.115 | 0.146 | 5.970 | 6.023 | 116.0 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.258 | 0.254 | 0.264 | 0.005 | 0.259 | 0.258 | 195.6 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.250 | 0.257 | 0.003 | 0.254 | 0.254 | 195.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.260 | 0.259 | 0.260 | 0.001 | 0.260 | 0.260 | 195.4 | 1.00× |
| lazy-ruby | 3 | 5.628 | 5.594 | 5.775 | 0.096 | 5.666 | 5.694 | 166.1 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.396 | 0.388 | 0.407 | 0.007 | 0.398 | 0.396 | 117.1 | 1.00× |
| lazy-ruby | 5 | 0.398 | 0.390 | 0.411 | 0.008 | 0.399 | 0.396 | 117.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.401 | 0.393 | 0.413 | 0.010 | 0.402 | 0.400 | 117.4 | 1.00× |
| lazy-ruby | 3 | 5.960 | 5.720 | 6.129 | 0.206 | 5.936 | 6.026 | 116.0 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.233 | 0.235 | 0.001 | 0.234 | 0.232 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.231 | 0.235 | 0.002 | 0.233 | 0.232 | 74.8 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.231 | 0.233 | 0.001 | 0.232 | 0.232 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.241 | 2.239 | 2.253 | 0.008 | 2.244 | 2.355 | 74.8 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.443 | 0.439 | 0.445 | 0.003 | 0.442 | 0.441 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.441 | 0.439 | 0.443 | 0.002 | 0.441 | 0.440 | 74.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.441 | 0.441 | 0.441 | 0.000 | 0.441 | 0.440 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.443 | 2.433 | 2.447 | 0.007 | 2.441 | 2.558 | 74.8 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.256 | 0.253 | 0.260 | 0.002 | 0.256 | 0.255 | 182.5 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.255 | 0.258 | 0.001 | 0.256 | 0.255 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.256 | 0.256 | 0.258 | 0.001 | 0.256 | 0.255 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.257 | 2.255 | 2.267 | 0.007 | 2.260 | 2.373 | 156.4 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.424 | 0.420 | 0.441 | 0.008 | 0.426 | 0.422 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.422 | 0.432 | 0.004 | 0.425 | 0.423 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.422 | 0.422 | 0.424 | 0.001 | 0.423 | 0.421 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.428 | 2.426 | 2.438 | 0.007 | 2.431 | 2.539 | 74.8 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.291 | 0.290 | 0.294 | 0.002 | 0.292 | 0.290 | 82.5 | 1.00× |
| lazy-ruby | 5 | 0.293 | 0.290 | 0.294 | 0.002 | 0.292 | 0.291 | 82.6 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.291 | 0.289 | 0.293 | 0.002 | 0.291 | 0.290 | 82.5 | 1.00× |
| lazy-ruby | 3 | 7.410 | 7.387 | 7.424 | 0.019 | 7.407 | 7.446 | 80.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.522 | 0.512 | 0.528 | 0.007 | 0.521 | 0.521 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.521 | 0.519 | 0.535 | 0.007 | 0.524 | 0.519 | 82.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.520 | 0.519 | 0.525 | 0.003 | 0.521 | 0.520 | 82.5 | 1.00× |
| lazy-ruby | 3 | 7.624 | 7.620 | 7.630 | 0.005 | 7.625 | 7.697 | 80.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.325 | 0.324 | 0.326 | 0.001 | 0.325 | 0.324 | 192.5 | 1.00× |
| lazy-ruby | 5 | 0.326 | 0.324 | 0.326 | 0.001 | 0.325 | 0.325 | 191.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.326 | 0.326 | 0.329 | 0.002 | 0.327 | 0.324 | 192.2 | 1.00× |
| lazy-ruby | 3 | 7.447 | 7.441 | 7.476 | 0.019 | 7.454 | 7.514 | 168.0 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.541 | 0.539 | 0.543 | 0.002 | 0.541 | 0.540 | 112.5 | 1.00× |
| lazy-ruby | 5 | 0.542 | 0.541 | 0.546 | 0.002 | 0.543 | 0.540 | 112.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.544 | 0.542 | 0.547 | 0.003 | 0.544 | 0.542 | 112.6 | 1.00× |
| lazy-ruby | 3 | 7.687 | 7.665 | 7.694 | 0.015 | 7.682 | 7.739 | 80.6 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.239 | 0.222 | 0.248 | 0.012 | 0.237 | 0.229 | 66.2 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.211 | 0.274 | 0.023 | 0.242 | 0.229 | 65.4 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.282 | 0.255 | 0.310 | 0.028 | 0.283 | 0.272 | 65.7 | 1.00× |
| lazy-ruby | 3 | 1.054 | 1.032 | 1.238 | 0.113 | 1.108 | 1.160 | 51.9 | 0.27× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.441 | 0.428 | 0.468 | 0.019 | 0.448 | 0.420 | 64.5 | 1.00× |
| lazy-ruby | 5 | 0.464 | 0.440 | 0.516 | 0.031 | 0.465 | 0.444 | 65.5 | 0.95× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.459 | 0.420 | 0.462 | 0.024 | 0.447 | 0.443 | 64.0 | 1.00× |
| lazy-ruby | 3 | 1.177 | 1.095 | 1.267 | 0.086 | 1.179 | 1.243 | 54.7 | 0.39× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.219 | 0.204 | 0.230 | 0.010 | 0.218 | 0.202 | 166.2 | 1.00× |
| lazy-ruby | 5 | 0.221 | 0.214 | 0.236 | 0.009 | 0.221 | 0.203 | 165.3 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.235 | 0.221 | 0.238 | 0.009 | 0.231 | 0.208 | 168.3 | 1.00× |
| lazy-ruby | 3 | 0.996 | 0.991 | 1.039 | 0.027 | 1.009 | 1.099 | 150.8 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.431 | 0.395 | 0.451 | 0.022 | 0.428 | 0.413 | 98.2 | 1.00× |
| lazy-ruby | 5 | 0.427 | 0.390 | 0.434 | 0.018 | 0.420 | 0.409 | 94.9 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.427 | 0.395 | 0.430 | 0.019 | 0.418 | 0.406 | 92.7 | 1.00× |
| lazy-ruby | 3 | 1.161 | 1.129 | 1.209 | 0.040 | 1.166 | 1.226 | 67.1 | 0.37× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- boot / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- boot / lazy-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- fib / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- fib / lazy-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- ioread / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- ioread / lazy-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- treewalk / eager-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS
- treewalk / lazy-ruby: unavailable — the fixture image build failed: acquire: download failed for https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS: not found: https://github.com/tamatebako/tebako/releases/latest/download/SHA256SUMS

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.64

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.235 | 0.242 | 0.002 | 0.239 | 0.250 | 33.4 | 1.00× |
| lazy-ruby | 5 | 0.238 | 0.236 | 0.247 | 0.004 | 0.240 | 0.234 | 33.6 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.244 | 0.237 | 0.246 | 0.005 | 0.242 | 0.234 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.823 | 0.821 | 0.826 | 0.002 | 0.823 | 0.844 | 25.7 | 0.30× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.460 | 0.454 | 0.469 | 0.007 | 0.462 | 0.453 | 33.3 | 1.00× |
| lazy-ruby | 5 | 0.468 | 0.458 | 0.470 | 0.006 | 0.465 | 0.453 | 33.3 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.459 | 0.458 | 0.462 | 0.002 | 0.460 | 0.453 | 33.5 | 1.00× |
| lazy-ruby | 3 | 1.033 | 1.027 | 1.049 | 0.011 | 1.036 | 1.094 | 25.6 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.289 | 0.284 | 0.297 | 0.005 | 0.290 | 0.297 | 140.6 | 1.00× |
| lazy-ruby | 5 | 0.289 | 0.285 | 0.298 | 0.005 | 0.291 | 0.281 | 140.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.289 | 0.283 | 0.292 | 0.005 | 0.288 | 0.297 | 141.1 | 1.00× |
| lazy-ruby | 3 | 0.853 | 0.851 | 0.861 | 0.005 | 0.855 | 0.875 | 132.8 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.501 | 0.499 | 0.521 | 0.010 | 0.506 | 0.500 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.503 | 0.501 | 0.506 | 0.002 | 0.503 | 0.516 | 75.4 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.504 | 0.503 | 0.512 | 0.005 | 0.507 | 0.516 | 75.4 | 1.00× |
| lazy-ruby | 3 | 1.066 | 1.061 | 1.073 | 0.006 | 1.067 | 1.109 | 50.8 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

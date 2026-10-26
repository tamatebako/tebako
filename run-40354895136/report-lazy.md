# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.222 | 0.220 | 0.225 | 0.002 | 0.222 | 0.221 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.222 | 0.218 | 0.225 | 0.002 | 0.222 | 0.220 | 108.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.222 | 0.220 | 0.222 | 0.001 | 0.222 | 0.220 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.339 | 2.327 | 2.368 | 0.021 | 2.344 | 2.471 | 108.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.420 | 0.428 | 0.003 | 0.422 | 0.421 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.416 | 0.436 | 0.008 | 0.424 | 0.421 | 108.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.424 | 0.422 | 0.424 | 0.001 | 0.423 | 0.423 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.539 | 2.539 | 2.555 | 0.010 | 2.544 | 2.671 | 108.1 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.243 | 0.241 | 0.243 | 0.001 | 0.243 | 0.241 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.243 | 0.241 | 0.243 | 0.001 | 0.243 | 0.242 | 180.1 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.243 | 0.243 | 0.243 | 0.000 | 0.243 | 0.242 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.353 | 2.349 | 2.355 | 0.003 | 2.353 | 2.487 | 156.6 | 0.10× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.401 | 0.399 | 0.403 | 0.002 | 0.401 | 0.400 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.403 | 0.399 | 0.403 | 0.002 | 0.402 | 0.401 | 108.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.399 | 0.397 | 0.399 | 0.001 | 0.399 | 0.397 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.520 | 2.520 | 2.528 | 0.005 | 2.523 | 2.653 | 108.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.250 | 0.249 | 0.264 | 0.006 | 0.254 | 0.249 | 115.6 | 1.00× |
| lazy-ruby | 5 | 0.249 | 0.248 | 0.251 | 0.001 | 0.249 | 0.248 | 115.6 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.250 | 0.248 | 0.251 | 0.001 | 0.250 | 0.249 | 115.6 | 1.00× |
| lazy-ruby | 3 | 6.798 | 6.791 | 6.812 | 0.011 | 6.800 | 6.861 | 115.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.470 | 0.466 | 0.472 | 0.003 | 0.470 | 0.469 | 115.6 | 1.00× |
| lazy-ruby | 5 | 0.468 | 0.466 | 0.473 | 0.003 | 0.469 | 0.467 | 115.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.468 | 0.468 | 0.472 | 0.002 | 0.469 | 0.467 | 115.6 | 1.00× |
| lazy-ruby | 3 | 7.007 | 6.987 | 7.022 | 0.017 | 7.005 | 7.077 | 115.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.286 | 0.284 | 0.289 | 0.002 | 0.286 | 0.284 | 194.0 | 1.00× |
| lazy-ruby | 5 | 0.286 | 0.284 | 0.287 | 0.001 | 0.285 | 0.285 | 193.7 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.284 | 0.284 | 0.286 | 0.001 | 0.284 | 0.283 | 193.3 | 1.00× |
| lazy-ruby | 3 | 6.823 | 6.788 | 6.828 | 0.022 | 6.813 | 6.884 | 165.9 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.451 | 0.449 | 0.458 | 0.003 | 0.452 | 0.450 | 117.3 | 1.00× |
| lazy-ruby | 5 | 0.450 | 0.449 | 0.453 | 0.002 | 0.451 | 0.450 | 117.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.451 | 0.449 | 0.453 | 0.002 | 0.451 | 0.450 | 117.2 | 1.00× |
| lazy-ruby | 3 | 7.007 | 6.956 | 7.008 | 0.030 | 6.990 | 7.063 | 115.6 | 0.06× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.235 | 0.002 | 0.232 | 0.231 | 74.9 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.231 | 0.235 | 0.002 | 0.232 | 0.231 | 74.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.233 | 0.235 | 0.001 | 0.233 | 0.232 | 74.9 | 1.00× |
| lazy-ruby | 3 | 2.247 | 2.244 | 2.248 | 0.002 | 2.246 | 2.359 | 74.9 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.443 | 0.441 | 0.447 | 0.002 | 0.443 | 0.442 | 74.9 | 1.00× |
| lazy-ruby | 5 | 0.443 | 0.441 | 0.445 | 0.001 | 0.443 | 0.442 | 74.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.445 | 0.445 | 0.447 | 0.001 | 0.445 | 0.443 | 74.9 | 1.00× |
| lazy-ruby | 3 | 2.460 | 2.456 | 2.468 | 0.006 | 2.462 | 2.576 | 74.9 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.258 | 0.258 | 0.262 | 0.002 | 0.259 | 0.257 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.255 | 0.262 | 0.003 | 0.257 | 0.255 | 182.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.256 | 0.255 | 0.257 | 0.001 | 0.256 | 0.255 | 182.5 | 1.00× |
| lazy-ruby | 3 | 2.261 | 2.261 | 2.261 | 0.000 | 2.261 | 2.377 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.424 | 0.418 | 0.428 | 0.004 | 0.423 | 0.423 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.420 | 0.426 | 0.003 | 0.423 | 0.421 | 101.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.424 | 0.424 | 0.424 | 0.000 | 0.424 | 0.423 | 101.6 | 1.00× |
| lazy-ruby | 3 | 2.438 | 2.437 | 2.440 | 0.002 | 2.438 | 2.550 | 74.9 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.225 | 0.228 | 0.001 | 0.227 | 0.225 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.225 | 0.224 | 0.232 | 0.003 | 0.226 | 0.224 | 82.5 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.226 | 0.225 | 0.230 | 0.002 | 0.227 | 0.225 | 82.4 | 1.00× |
| lazy-ruby | 3 | 5.830 | 5.771 | 5.870 | 0.050 | 5.824 | 5.899 | 80.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.389 | 0.387 | 0.406 | 0.008 | 0.392 | 0.387 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.391 | 0.389 | 0.393 | 0.002 | 0.391 | 0.390 | 82.4 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.391 | 0.389 | 0.394 | 0.003 | 0.391 | 0.390 | 82.6 | 1.00× |
| lazy-ruby | 3 | 6.082 | 5.866 | 6.117 | 0.136 | 6.022 | 6.120 | 80.8 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.256 | 0.256 | 0.258 | 0.001 | 0.256 | 0.255 | 192.1 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.254 | 0.258 | 0.002 | 0.257 | 0.256 | 191.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.258 | 0.252 | 0.259 | 0.004 | 0.256 | 0.256 | 192.0 | 1.00× |
| lazy-ruby | 3 | 5.785 | 5.785 | 5.923 | 0.080 | 5.831 | 5.829 | 167.9 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.414 | 0.422 | 0.003 | 0.417 | 0.416 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.418 | 0.413 | 0.433 | 0.008 | 0.421 | 0.417 | 112.5 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.415 | 0.418 | 0.002 | 0.417 | 0.416 | 112.6 | 1.00× |
| lazy-ruby | 3 | 6.027 | 5.974 | 6.052 | 0.040 | 6.018 | 6.090 | 80.8 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.204 | 0.195 | 0.214 | 0.007 | 0.205 | 0.187 | 65.3 | 1.00× |
| lazy-ruby | 5 | 0.203 | 0.195 | 0.218 | 0.010 | 0.205 | 0.189 | 65.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.209 | 0.192 | 0.209 | 0.010 | 0.203 | 0.189 | 66.7 | 1.00× |
| lazy-ruby | 3 | 0.922 | 0.909 | 0.940 | 0.016 | 0.924 | 0.990 | 56.5 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.411 | 0.404 | 0.422 | 0.008 | 0.414 | 0.392 | 65.8 | 1.00× |
| lazy-ruby | 5 | 0.408 | 0.402 | 0.432 | 0.012 | 0.412 | 0.392 | 66.8 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.412 | 0.407 | 0.414 | 0.004 | 0.411 | 0.392 | 64.9 | 1.00× |
| lazy-ruby | 3 | 1.058 | 1.022 | 1.067 | 0.024 | 1.049 | 1.110 | 57.8 | 0.39× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.208 | 0.200 | 0.212 | 0.005 | 0.207 | 0.190 | 167.3 | 1.00× |
| lazy-ruby | 5 | 0.205 | 0.193 | 0.212 | 0.007 | 0.204 | 0.188 | 166.8 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.198 | 0.195 | 0.223 | 0.015 | 0.206 | 0.187 | 166.5 | 1.00× |
| lazy-ruby | 3 | 0.957 | 0.946 | 1.021 | 0.040 | 0.974 | 1.016 | 146.6 | 0.21× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.406 | 0.382 | 0.428 | 0.021 | 0.406 | 0.389 | 93.0 | 1.00× |
| lazy-ruby | 5 | 0.400 | 0.374 | 0.418 | 0.017 | 0.399 | 0.381 | 102.5 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.406 | 0.392 | 0.412 | 0.010 | 0.403 | 0.387 | 100.1 | 1.00× |
| lazy-ruby | 3 | 1.127 | 1.103 | 1.205 | 0.053 | 1.145 | 1.189 | 71.9 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.544 | 0.522 | 0.597 | 0.033 | 0.556 | 0.537 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.566 | 0.537 | 0.618 | 0.034 | 0.575 | 0.554 | 57.3 | 0.96× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.554 | 0.547 | 0.577 | 0.016 | 0.560 | 0.540 | 57.1 | 1.00× |
| lazy-ruby | 3 | 5.050 | 4.942 | 5.442 | 0.263 | 5.145 | 5.244 | 42.7 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.128 | 1.065 | 1.193 | 0.053 | 1.120 | 1.113 | 57.1 | 1.00× |
| lazy-ruby | 5 | 1.105 | 1.042 | 1.161 | 0.048 | 1.113 | 1.093 | 57.1 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.131 | 1.079 | 1.143 | 0.034 | 1.118 | 1.114 | 57.3 | 1.00× |
| lazy-ruby | 3 | 5.732 | 5.488 | 5.849 | 0.184 | 5.689 | 5.857 | 42.9 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.621 | 0.572 | 0.645 | 0.029 | 0.620 | 0.612 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.640 | 0.610 | 0.683 | 0.026 | 0.645 | 0.630 | 141.9 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.794 | 0.707 | 0.843 | 0.069 | 0.781 | 0.736 | 142.1 | 1.00× |
| lazy-ruby | 3 | 4.474 | 3.820 | 4.640 | 0.434 | 4.311 | 4.656 | 128.9 | 0.18× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.930 | 0.853 | 0.979 | 0.053 | 0.913 | 0.915 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.918 | 0.841 | 0.956 | 0.048 | 0.911 | 0.891 | 82.9 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.006 | 0.991 | 1.157 | 0.092 | 1.051 | 0.996 | 82.9 | 1.00× |
| lazy-ruby | 3 | 4.898 | 4.397 | 5.212 | 0.411 | 4.836 | 5.017 | 46.7 | 0.21× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.65

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.232 | 0.233 | 0.000 | 0.233 | 0.250 | 33.3 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.234 | 0.238 | 0.002 | 0.235 | 0.250 | 33.6 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.234 | 0.237 | 0.002 | 0.235 | 0.234 | 33.6 | 1.00× |
| lazy-ruby | 3 | 0.809 | 0.805 | 0.809 | 0.002 | 0.807 | 0.844 | 25.6 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.455 | 0.451 | 0.469 | 0.007 | 0.457 | 0.453 | 33.7 | 1.00× |
| lazy-ruby | 5 | 0.453 | 0.447 | 0.454 | 0.003 | 0.451 | 0.453 | 33.6 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.453 | 0.458 | 0.002 | 0.456 | 0.453 | 33.8 | 1.00× |
| lazy-ruby | 3 | 1.022 | 1.012 | 1.022 | 0.005 | 1.019 | 1.047 | 25.6 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.285 | 0.281 | 0.290 | 0.004 | 0.285 | 0.281 | 140.6 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.282 | 0.293 | 0.004 | 0.288 | 0.281 | 140.6 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.284 | 0.284 | 0.289 | 0.003 | 0.286 | 0.281 | 140.7 | 1.00× |
| lazy-ruby | 3 | 0.859 | 0.848 | 0.864 | 0.008 | 0.857 | 0.891 | 132.9 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.509 | 0.498 | 0.515 | 0.006 | 0.507 | 0.500 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.503 | 0.491 | 0.509 | 0.007 | 0.502 | 0.500 | 75.2 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.498 | 0.497 | 0.508 | 0.006 | 0.501 | 0.500 | 75.3 | 1.00× |
| lazy-ruby | 3 | 1.071 | 1.048 | 1.073 | 0.014 | 1.064 | 1.078 | 50.8 | 0.46× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

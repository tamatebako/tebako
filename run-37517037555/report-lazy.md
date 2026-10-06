# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.220 | 0.231 | 0.004 | 0.227 | 0.226 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.227 | 0.229 | 0.001 | 0.228 | 0.227 | 108.0 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.227 | 0.227 | 0.000 | 0.227 | 0.225 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.346 | 2.334 | 2.354 | 0.010 | 2.344 | 2.478 | 108.0 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.428 | 0.426 | 0.430 | 0.001 | 0.428 | 0.427 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.426 | 0.441 | 0.008 | 0.432 | 0.428 | 108.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.430 | 0.426 | 0.430 | 0.002 | 0.429 | 0.428 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.564 | 2.558 | 2.570 | 0.006 | 2.564 | 2.695 | 108.0 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.251 | 0.251 | 0.251 | 0.000 | 0.251 | 0.250 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.251 | 0.251 | 0.253 | 0.001 | 0.252 | 0.251 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.249 | 0.253 | 0.002 | 0.251 | 0.249 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.376 | 2.374 | 2.382 | 0.004 | 2.378 | 2.509 | 155.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.405 | 0.402 | 0.410 | 0.003 | 0.406 | 0.404 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.406 | 0.405 | 0.412 | 0.003 | 0.407 | 0.405 | 108.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.406 | 0.403 | 0.408 | 0.002 | 0.406 | 0.405 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.529 | 2.521 | 2.533 | 0.006 | 2.527 | 2.660 | 108.0 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.201 | 0.199 | 0.209 | 0.004 | 0.202 | 0.199 | 115.2 | 1.00× |
| lazy-ruby | 5 | 0.201 | 0.198 | 0.203 | 0.002 | 0.201 | 0.200 | 115.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.202 | 0.200 | 0.202 | 0.001 | 0.202 | 0.200 | 115.2 | 1.00× |
| lazy-ruby | 3 | 5.416 | 5.228 | 5.484 | 0.133 | 5.376 | 5.467 | 115.2 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.362 | 0.362 | 0.379 | 0.007 | 0.366 | 0.360 | 115.2 | 1.00× |
| lazy-ruby | 5 | 0.365 | 0.360 | 0.379 | 0.007 | 0.367 | 0.364 | 115.2 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.359 | 0.359 | 0.362 | 0.001 | 0.360 | 0.359 | 115.2 | 1.00× |
| lazy-ruby | 3 | 5.610 | 5.461 | 5.627 | 0.092 | 5.566 | 5.662 | 115.2 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.228 | 0.227 | 0.229 | 0.001 | 0.228 | 0.227 | 193.6 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.229 | 0.232 | 0.001 | 0.230 | 0.228 | 193.7 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.229 | 0.228 | 0.230 | 0.001 | 0.229 | 0.228 | 195.4 | 1.00× |
| lazy-ruby | 3 | 5.436 | 5.312 | 5.495 | 0.094 | 5.414 | 5.488 | 167.1 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.361 | 0.357 | 0.362 | 0.002 | 0.360 | 0.359 | 117.2 | 1.00× |
| lazy-ruby | 5 | 0.359 | 0.357 | 0.361 | 0.002 | 0.359 | 0.358 | 117.2 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.363 | 0.359 | 0.364 | 0.003 | 0.362 | 0.361 | 117.5 | 1.00× |
| lazy-ruby | 3 | 5.451 | 5.416 | 5.643 | 0.122 | 5.503 | 5.511 | 115.2 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.233 | 0.246 | 0.006 | 0.238 | 0.234 | 74.3 | 1.00× |
| lazy-ruby | 5 | 0.239 | 0.236 | 0.242 | 0.002 | 0.239 | 0.237 | 74.3 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.231 | 0.237 | 0.003 | 0.234 | 0.232 | 74.3 | 1.00× |
| lazy-ruby | 3 | 2.267 | 2.249 | 2.428 | 0.099 | 2.315 | 2.366 | 74.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.449 | 0.441 | 0.497 | 0.025 | 0.463 | 0.449 | 74.3 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.444 | 0.464 | 0.008 | 0.449 | 0.445 | 74.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.442 | 0.438 | 0.443 | 0.003 | 0.441 | 0.440 | 74.3 | 1.00× |
| lazy-ruby | 3 | 2.452 | 2.426 | 2.458 | 0.017 | 2.445 | 2.569 | 74.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.247 | 0.243 | 0.256 | 0.005 | 0.248 | 0.246 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.245 | 0.242 | 0.254 | 0.005 | 0.248 | 0.245 | 182.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.257 | 0.264 | 0.003 | 0.261 | 0.260 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.290 | 2.287 | 2.316 | 0.016 | 2.298 | 2.413 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.429 | 0.445 | 0.007 | 0.435 | 0.431 | 101.5 | 1.00× |
| lazy-ruby | 5 | 0.425 | 0.416 | 0.460 | 0.018 | 0.431 | 0.424 | 101.7 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.421 | 0.445 | 0.013 | 0.435 | 0.436 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.458 | 2.450 | 2.462 | 0.006 | 2.457 | 2.563 | 74.3 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.224 | 0.230 | 0.002 | 0.227 | 0.226 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.225 | 0.223 | 0.258 | 0.015 | 0.232 | 0.225 | 82.4 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.223 | 0.226 | 0.001 | 0.225 | 0.223 | 84.3 | 1.00× |
| lazy-ruby | 3 | 5.750 | 5.700 | 5.910 | 0.110 | 5.787 | 5.789 | 80.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.388 | 0.387 | 0.393 | 0.002 | 0.390 | 0.387 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.389 | 0.386 | 0.390 | 0.002 | 0.388 | 0.388 | 82.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.387 | 0.387 | 0.389 | 0.001 | 0.388 | 0.386 | 82.4 | 1.00× |
| lazy-ruby | 3 | 5.886 | 5.853 | 5.994 | 0.073 | 5.911 | 5.926 | 80.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.258 | 0.256 | 0.261 | 0.002 | 0.258 | 0.256 | 191.8 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.254 | 0.258 | 0.002 | 0.255 | 0.253 | 192.1 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.255 | 0.254 | 0.255 | 0.000 | 0.254 | 0.253 | 192.1 | 1.00× |
| lazy-ruby | 3 | 5.804 | 5.679 | 6.010 | 0.167 | 5.831 | 5.845 | 168.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.413 | 0.419 | 0.003 | 0.416 | 0.413 | 112.7 | 1.00× |
| lazy-ruby | 5 | 0.417 | 0.415 | 0.420 | 0.002 | 0.417 | 0.416 | 112.4 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.415 | 0.411 | 0.418 | 0.004 | 0.415 | 0.414 | 114.5 | 1.00× |
| lazy-ruby | 3 | 5.902 | 5.857 | 6.140 | 0.152 | 5.966 | 5.941 | 80.8 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.212 | 0.180 | 0.229 | 0.021 | 0.204 | 0.189 | 63.9 | 1.00× |
| lazy-ruby | 5 | 0.202 | 0.188 | 0.230 | 0.017 | 0.208 | 0.191 | 62.8 | 1.05× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.208 | 0.200 | 0.219 | 0.009 | 0.209 | 0.199 | 62.5 | 1.00× |
| lazy-ruby | 3 | 0.947 | 0.884 | 0.996 | 0.056 | 0.942 | 1.010 | 52.0 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.416 | 0.379 | 0.438 | 0.022 | 0.411 | 0.394 | 62.6 | 1.00× |
| lazy-ruby | 5 | 0.401 | 0.398 | 0.469 | 0.030 | 0.416 | 0.388 | 63.8 | 1.04× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.404 | 0.396 | 0.411 | 0.008 | 0.404 | 0.384 | 63.3 | 1.00× |
| lazy-ruby | 3 | 1.163 | 1.109 | 1.165 | 0.032 | 1.146 | 1.226 | 57.0 | 0.35× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.228 | 0.221 | 0.276 | 0.023 | 0.240 | 0.212 | 163.7 | 1.00× |
| lazy-ruby | 5 | 0.238 | 0.221 | 0.384 | 0.068 | 0.263 | 0.224 | 166.0 | 0.96× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.243 | 0.222 | 0.243 | 0.012 | 0.236 | 0.211 | 165.2 | 1.00× |
| lazy-ruby | 3 | 1.041 | 0.952 | 1.080 | 0.066 | 1.024 | 1.076 | 145.8 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.422 | 0.487 | 0.027 | 0.445 | 0.420 | 98.6 | 1.00× |
| lazy-ruby | 5 | 0.445 | 0.417 | 0.477 | 0.027 | 0.448 | 0.426 | 91.3 | 0.97× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.462 | 0.419 | 0.549 | 0.066 | 0.477 | 0.455 | 95.9 | 1.00× |
| lazy-ruby | 3 | 1.164 | 1.105 | 1.217 | 0.056 | 1.162 | 1.236 | 71.6 | 0.40× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.479 | 0.437 | 0.525 | 0.040 | 0.478 | 0.470 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.552 | 0.499 | 0.751 | 0.098 | 0.585 | 0.516 | 57.3 | 0.87× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.513 | 0.416 | 0.582 | 0.084 | 0.504 | 0.420 | 57.3 | 1.00× |
| lazy-ruby | 3 | 4.320 | 4.139 | 4.623 | 0.244 | 4.361 | 4.460 | 43.0 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.933 | 0.859 | 1.098 | 0.088 | 0.958 | 0.924 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.987 | 0.970 | 1.124 | 0.073 | 1.029 | 0.958 | 57.5 | 0.94× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.981 | 0.885 | 1.011 | 0.066 | 0.959 | 0.968 | 57.3 | 1.00× |
| lazy-ruby | 3 | 4.542 | 4.109 | 5.930 | 0.951 | 4.860 | 4.626 | 42.9 | 0.22× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.545 | 0.489 | 0.877 | 0.160 | 0.623 | 0.529 | 141.9 | 1.00× |
| lazy-ruby | 5 | 0.547 | 0.509 | 0.652 | 0.058 | 0.556 | 0.536 | 142.1 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.516 | 0.480 | 0.534 | 0.028 | 0.510 | 0.504 | 141.8 | 1.00× |
| lazy-ruby | 3 | 5.282 | 4.097 | 5.910 | 0.920 | 5.096 | 5.278 | 128.9 | 0.10× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.856 | 0.801 | 1.072 | 0.132 | 0.924 | 0.848 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.809 | 0.775 | 1.141 | 0.152 | 0.888 | 0.787 | 83.2 | 1.06× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.936 | 0.735 | 1.035 | 0.153 | 0.902 | 0.922 | 83.0 | 1.00× |
| lazy-ruby | 3 | 4.504 | 4.499 | 4.669 | 0.096 | 4.557 | 4.615 | 46.6 | 0.21× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.226 | 0.242 | 0.006 | 0.232 | 0.234 | 33.5 | 1.00× |
| lazy-ruby | 5 | 0.230 | 0.228 | 0.238 | 0.005 | 0.232 | 0.234 | 33.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.232 | 0.230 | 0.235 | 0.003 | 0.232 | 0.219 | 33.6 | 1.00× |
| lazy-ruby | 3 | 0.794 | 0.791 | 0.798 | 0.004 | 0.794 | 0.812 | 25.4 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.444 | 0.455 | 0.005 | 0.448 | 0.453 | 33.6 | 1.00× |
| lazy-ruby | 5 | 0.448 | 0.446 | 0.454 | 0.004 | 0.450 | 0.453 | 33.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.450 | 0.449 | 0.451 | 0.001 | 0.450 | 0.453 | 33.7 | 1.00× |
| lazy-ruby | 3 | 1.016 | 1.010 | 1.019 | 0.005 | 1.015 | 1.047 | 25.5 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.277 | 0.276 | 0.280 | 0.002 | 0.277 | 0.266 | 140.8 | 1.00× |
| lazy-ruby | 5 | 0.276 | 0.274 | 0.280 | 0.002 | 0.276 | 0.266 | 140.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.277 | 0.273 | 0.280 | 0.003 | 0.277 | 0.297 | 141.0 | 1.00× |
| lazy-ruby | 3 | 0.833 | 0.827 | 0.835 | 0.004 | 0.832 | 0.859 | 132.5 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.487 | 0.485 | 0.498 | 0.006 | 0.489 | 0.484 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.488 | 0.487 | 0.495 | 0.003 | 0.489 | 0.484 | 75.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.492 | 0.489 | 0.498 | 0.005 | 0.493 | 0.469 | 75.4 | 1.00× |
| lazy-ruby | 3 | 1.054 | 1.051 | 1.056 | 0.003 | 1.054 | 1.109 | 50.8 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

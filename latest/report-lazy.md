# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.230 | 0.228 | 0.234 | 0.003 | 0.231 | 0.229 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.232 | 0.229 | 0.234 | 0.002 | 0.232 | 0.230 | 70.6 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.230 | 0.229 | 0.230 | 0.000 | 0.230 | 0.228 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.344 | 2.343 | 2.349 | 0.003 | 2.346 | 2.478 | 49.4 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.429 | 0.442 | 0.005 | 0.434 | 0.432 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.433 | 0.428 | 0.434 | 0.003 | 0.432 | 0.431 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.431 | 0.428 | 0.432 | 0.002 | 0.430 | 0.429 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.543 | 2.568 | 0.013 | 2.556 | 2.690 | 48.7 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.257 | 0.255 | 0.263 | 0.003 | 0.257 | 0.255 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.257 | 0.255 | 0.259 | 0.002 | 0.257 | 0.257 | 180.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.257 | 0.257 | 0.257 | 0.000 | 0.257 | 0.256 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.382 | 2.369 | 2.385 | 0.008 | 2.379 | 2.517 | 156.9 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.411 | 0.411 | 0.419 | 0.004 | 0.413 | 0.409 | 102.1 | 1.00× |
| lazy-ruby | 5 | 0.411 | 0.409 | 0.413 | 0.002 | 0.411 | 0.411 | 102.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.409 | 0.409 | 0.411 | 0.001 | 0.410 | 0.408 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.527 | 2.523 | 2.537 | 0.007 | 2.529 | 2.660 | 62.4 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.228 | 0.225 | 0.230 | 0.002 | 0.228 | 0.226 | 79.6 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.223 | 0.232 | 0.003 | 0.229 | 0.228 | 79.9 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.232 | 0.230 | 0.236 | 0.003 | 0.233 | 0.232 | 79.8 | 1.00× |
| lazy-ruby | 3 | 5.517 | 5.481 | 5.566 | 0.043 | 5.521 | 5.567 | 51.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.390 | 0.388 | 0.397 | 0.003 | 0.391 | 0.390 | 80.1 | 1.00× |
| lazy-ruby | 5 | 0.392 | 0.390 | 0.397 | 0.003 | 0.393 | 0.391 | 79.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.397 | 0.396 | 0.398 | 0.001 | 0.397 | 0.397 | 79.6 | 1.00× |
| lazy-ruby | 3 | 5.416 | 5.401 | 5.627 | 0.126 | 5.481 | 5.466 | 51.5 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.263 | 0.269 | 0.003 | 0.266 | 0.264 | 188.7 | 1.00× |
| lazy-ruby | 5 | 0.267 | 0.261 | 0.270 | 0.004 | 0.267 | 0.267 | 188.7 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.261 | 0.265 | 0.002 | 0.264 | 0.264 | 188.8 | 1.00× |
| lazy-ruby | 3 | 5.423 | 5.281 | 5.476 | 0.101 | 5.393 | 5.475 | 157.9 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.390 | 0.386 | 0.394 | 0.003 | 0.391 | 0.389 | 112.2 | 1.00× |
| lazy-ruby | 5 | 0.394 | 0.387 | 0.396 | 0.004 | 0.392 | 0.393 | 112.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.394 | 0.392 | 0.396 | 0.002 | 0.394 | 0.393 | 111.9 | 1.00× |
| lazy-ruby | 3 | 5.566 | 5.512 | 5.607 | 0.048 | 5.562 | 5.616 | 65.0 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.238 | 0.250 | 0.006 | 0.244 | 0.239 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.244 | 0.236 | 0.251 | 0.006 | 0.245 | 0.243 | 72.3 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.233 | 0.238 | 0.002 | 0.235 | 0.233 | 72.1 | 1.00× |
| lazy-ruby | 3 | 2.246 | 2.237 | 2.285 | 0.026 | 2.256 | 2.355 | 47.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.444 | 0.444 | 0.448 | 0.002 | 0.445 | 0.443 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.444 | 0.443 | 0.450 | 0.003 | 0.445 | 0.443 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.448 | 0.448 | 0.455 | 0.004 | 0.450 | 0.447 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.449 | 2.448 | 2.452 | 0.002 | 2.450 | 2.558 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.261 | 0.260 | 0.261 | 0.000 | 0.261 | 0.260 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.260 | 0.263 | 0.001 | 0.261 | 0.261 | 182.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.263 | 0.261 | 0.263 | 0.001 | 0.262 | 0.262 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.274 | 2.270 | 2.283 | 0.007 | 2.276 | 2.388 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.429 | 0.427 | 0.433 | 0.002 | 0.430 | 0.427 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.429 | 0.427 | 0.429 | 0.001 | 0.428 | 0.427 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.429 | 0.427 | 0.429 | 0.001 | 0.428 | 0.427 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.439 | 2.427 | 2.449 | 0.011 | 2.438 | 2.549 | 62.4 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.239 | 0.003 | 0.233 | 0.232 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.237 | 0.231 | 0.248 | 0.006 | 0.238 | 0.236 | 75.0 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.239 | 0.239 | 0.240 | 0.001 | 0.240 | 0.238 | 75.0 | 1.00× |
| lazy-ruby | 3 | 5.499 | 5.412 | 5.515 | 0.055 | 5.475 | 5.500 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.356 | 0.351 | 0.364 | 0.005 | 0.356 | 0.354 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.349 | 0.347 | 0.358 | 0.004 | 0.351 | 0.349 | 75.2 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.356 | 0.345 | 0.356 | 0.006 | 0.352 | 0.354 | 75.2 | 1.00× |
| lazy-ruby | 3 | 5.705 | 5.556 | 5.757 | 0.105 | 5.672 | 5.702 | 49.5 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.264 | 0.256 | 0.324 | 0.028 | 0.275 | 0.264 | 185.4 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.252 | 0.266 | 0.006 | 0.260 | 0.261 | 185.4 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.250 | 0.262 | 0.007 | 0.258 | 0.261 | 185.6 | 1.00× |
| lazy-ruby | 3 | 5.538 | 5.534 | 5.556 | 0.011 | 5.543 | 5.538 | 156.0 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.423 | 0.414 | 0.425 | 0.006 | 0.420 | 0.422 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.418 | 0.426 | 0.003 | 0.423 | 0.423 | 105.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.420 | 0.419 | 0.427 | 0.004 | 0.422 | 0.418 | 105.4 | 1.00× |
| lazy-ruby | 3 | 5.757 | 5.745 | 5.786 | 0.021 | 5.763 | 5.758 | 63.1 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.204 | 0.267 | 0.023 | 0.230 | 0.213 | 62.9 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.219 | 0.238 | 0.009 | 0.228 | 0.210 | 62.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.250 | 0.215 | 0.251 | 0.020 | 0.239 | 0.227 | 62.2 | 1.00× |
| lazy-ruby | 3 | 1.100 | 0.945 | 1.111 | 0.093 | 1.052 | 1.195 | 53.4 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.499 | 0.458 | 0.567 | 0.045 | 0.516 | 0.477 | 64.0 | 1.00× |
| lazy-ruby | 5 | 0.519 | 0.445 | 0.611 | 0.060 | 0.518 | 0.496 | 63.7 | 0.96× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.419 | 0.496 | 0.041 | 0.450 | 0.431 | 61.3 | 1.00× |
| lazy-ruby | 3 | 1.267 | 1.157 | 1.293 | 0.072 | 1.239 | 1.340 | 51.5 | 0.34× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.270 | 0.241 | 0.321 | 0.032 | 0.272 | 0.248 | 164.7 | 1.00× |
| lazy-ruby | 5 | 0.282 | 0.225 | 0.300 | 0.029 | 0.273 | 0.249 | 164.4 | 0.96× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.267 | 0.246 | 0.288 | 0.021 | 0.267 | 0.243 | 165.4 | 1.00× |
| lazy-ruby | 3 | 1.202 | 1.106 | 1.344 | 0.120 | 1.217 | 1.267 | 147.3 | 0.22× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.464 | 0.445 | 0.704 | 0.111 | 0.506 | 0.440 | 92.1 | 1.00× |
| lazy-ruby | 5 | 0.461 | 0.433 | 0.495 | 0.022 | 0.461 | 0.439 | 98.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.442 | 0.430 | 0.502 | 0.038 | 0.458 | 0.437 | 94.6 | 1.00× |
| lazy-ruby | 3 | 1.170 | 1.167 | 1.212 | 0.025 | 1.183 | 1.257 | 68.9 | 0.38× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.453 | 0.401 | 0.542 | 0.051 | 0.462 | 0.441 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.423 | 0.415 | 0.456 | 0.016 | 0.428 | 0.414 | 57.2 | 1.07× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.441 | 0.426 | 0.459 | 0.016 | 0.442 | 0.433 | 57.2 | 1.00× |
| lazy-ruby | 3 | 4.507 | 4.297 | 4.565 | 0.141 | 4.456 | 4.379 | 43.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.931 | 0.894 | 0.973 | 0.032 | 0.929 | 0.905 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.920 | 0.880 | 1.080 | 0.081 | 0.941 | 0.901 | 57.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.936 | 0.905 | 1.038 | 0.070 | 0.960 | 0.920 | 57.2 | 1.00× |
| lazy-ruby | 3 | 4.661 | 4.411 | 4.664 | 0.145 | 4.578 | 4.804 | 43.0 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.593 | 0.517 | 1.381 | 0.365 | 0.735 | 0.582 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.572 | 0.540 | 1.724 | 0.517 | 0.800 | 0.556 | 141.7 | 1.04× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.614 | 0.588 | 0.827 | 0.131 | 0.677 | 0.540 | 142.3 | 1.00× |
| lazy-ruby | 3 | 4.602 | 4.509 | 4.689 | 0.090 | 4.600 | 4.772 | 127.1 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.811 | 0.777 | 0.936 | 0.076 | 0.851 | 0.802 | 83.2 | 1.00× |
| lazy-ruby | 5 | 0.888 | 0.756 | 1.152 | 0.157 | 0.938 | 0.876 | 83.3 | 0.91× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.813 | 0.786 | 0.877 | 0.047 | 0.825 | 0.803 | 82.9 | 1.00× |
| lazy-ruby | 3 | 6.486 | 5.672 | 9.798 | 2.185 | 7.319 | 6.143 | 46.9 | 0.13× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.228 | 0.223 | 0.229 | 0.003 | 0.227 | 0.219 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.226 | 0.222 | 0.230 | 0.003 | 0.226 | 0.219 | 68.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.226 | 0.233 | 0.004 | 0.228 | 0.219 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.733 | 0.727 | 0.734 | 0.004 | 0.731 | 0.719 | 58.6 | 0.31× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.419 | 0.413 | 0.424 | 0.004 | 0.418 | 0.406 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.414 | 0.444 | 0.013 | 0.421 | 0.406 | 68.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.412 | 0.421 | 0.005 | 0.416 | 0.438 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.934 | 0.920 | 0.938 | 0.010 | 0.930 | 0.938 | 58.8 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.284 | 0.283 | 0.292 | 0.004 | 0.286 | 0.281 | 109.4 | 1.00× |
| lazy-ruby | 5 | 0.285 | 0.284 | 0.287 | 0.001 | 0.285 | 0.281 | 109.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.289 | 0.285 | 0.291 | 0.003 | 0.288 | 0.297 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.790 | 0.782 | 0.815 | 0.017 | 0.795 | 0.781 | 100.0 | 0.37× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.467 | 0.462 | 0.474 | 0.005 | 0.468 | 0.453 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.465 | 0.462 | 0.469 | 0.003 | 0.466 | 0.453 | 108.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.475 | 0.469 | 0.480 | 0.006 | 0.475 | 0.484 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.962 | 0.957 | 0.981 | 0.013 | 0.966 | 0.984 | 82.8 | 0.49× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

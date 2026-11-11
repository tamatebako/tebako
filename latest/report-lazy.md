# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.232 | 0.235 | 0.001 | 0.233 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.232 | 0.236 | 0.002 | 0.234 | 0.233 | 70.6 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.234 | 0.234 | 0.000 | 0.234 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.375 | 2.366 | 2.385 | 0.009 | 2.375 | 2.511 | 49.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.436 | 0.439 | 0.001 | 0.437 | 0.435 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.436 | 0.435 | 0.450 | 0.007 | 0.440 | 0.435 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.433 | 0.434 | 0.000 | 0.434 | 0.432 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.577 | 2.560 | 2.627 | 0.035 | 2.588 | 2.713 | 49.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.261 | 0.260 | 0.261 | 0.000 | 0.261 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.261 | 0.263 | 0.001 | 0.262 | 0.260 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.261 | 0.262 | 0.000 | 0.261 | 0.261 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.388 | 2.386 | 2.397 | 0.006 | 2.390 | 2.525 | 155.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.416 | 0.417 | 0.000 | 0.417 | 0.415 | 102.3 | 1.00× |
| lazy-ruby | 5 | 0.417 | 0.417 | 0.419 | 0.001 | 0.417 | 0.416 | 102.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.411 | 0.415 | 0.002 | 0.413 | 0.413 | 102.4 | 1.00× |
| lazy-ruby | 3 | 2.547 | 2.541 | 2.557 | 0.008 | 2.548 | 2.683 | 62.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.288 | 0.295 | 0.003 | 0.291 | 0.289 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.286 | 0.284 | 0.292 | 0.003 | 0.287 | 0.286 | 79.7 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.292 | 0.290 | 0.294 | 0.002 | 0.292 | 0.290 | 79.7 | 1.00× |
| lazy-ruby | 3 | 6.875 | 6.843 | 6.906 | 0.032 | 6.875 | 6.936 | 51.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.509 | 0.504 | 0.516 | 0.005 | 0.510 | 0.508 | 79.6 | 1.00× |
| lazy-ruby | 5 | 0.505 | 0.498 | 0.512 | 0.005 | 0.505 | 0.504 | 79.7 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.507 | 0.506 | 0.513 | 0.004 | 0.509 | 0.506 | 79.8 | 1.00× |
| lazy-ruby | 3 | 7.092 | 7.076 | 7.140 | 0.033 | 7.103 | 7.150 | 51.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.330 | 0.324 | 0.341 | 0.007 | 0.331 | 0.328 | 188.7 | 1.00× |
| lazy-ruby | 5 | 0.326 | 0.318 | 0.329 | 0.005 | 0.325 | 0.325 | 188.6 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.327 | 0.318 | 0.337 | 0.010 | 0.327 | 0.326 | 188.6 | 1.00× |
| lazy-ruby | 3 | 6.875 | 6.873 | 6.903 | 0.017 | 6.884 | 6.936 | 157.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.507 | 0.499 | 0.514 | 0.006 | 0.506 | 0.506 | 111.9 | 1.00× |
| lazy-ruby | 5 | 0.492 | 0.486 | 0.493 | 0.003 | 0.491 | 0.491 | 111.7 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.500 | 0.500 | 0.515 | 0.009 | 0.505 | 0.498 | 111.8 | 1.00× |
| lazy-ruby | 3 | 7.063 | 7.058 | 7.075 | 0.009 | 7.065 | 7.120 | 64.6 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.233 | 0.240 | 0.003 | 0.238 | 0.238 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.240 | 0.240 | 0.000 | 0.240 | 0.238 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.232 | 0.240 | 0.004 | 0.236 | 0.235 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.250 | 2.234 | 2.254 | 0.011 | 2.246 | 2.360 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.448 | 0.445 | 0.450 | 0.002 | 0.448 | 0.447 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.448 | 0.446 | 0.448 | 0.001 | 0.447 | 0.446 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.450 | 0.448 | 0.450 | 0.001 | 0.449 | 0.449 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.469 | 2.449 | 2.472 | 0.012 | 2.463 | 2.579 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.264 | 0.267 | 0.001 | 0.265 | 0.264 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.264 | 0.267 | 0.001 | 0.265 | 0.264 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.265 | 0.265 | 0.000 | 0.265 | 0.264 | 182.5 | 1.00× |
| lazy-ruby | 3 | 2.257 | 2.257 | 2.268 | 0.006 | 2.261 | 2.370 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.431 | 0.435 | 0.002 | 0.433 | 0.431 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.433 | 0.429 | 0.433 | 0.002 | 0.432 | 0.431 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.431 | 0.429 | 0.431 | 0.001 | 0.430 | 0.429 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.468 | 2.445 | 2.486 | 0.020 | 2.466 | 2.572 | 62.7 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.320 | 0.314 | 0.322 | 0.004 | 0.318 | 0.318 | 74.9 | 1.00× |
| lazy-ruby | 5 | 0.317 | 0.314 | 0.322 | 0.003 | 0.318 | 0.316 | 75.1 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.319 | 0.316 | 0.319 | 0.002 | 0.318 | 0.318 | 75.2 | 1.00× |
| lazy-ruby | 3 | 7.464 | 7.442 | 7.495 | 0.027 | 7.467 | 7.463 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.548 | 0.544 | 0.551 | 0.003 | 0.547 | 0.547 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.551 | 0.545 | 0.559 | 0.006 | 0.552 | 0.549 | 75.1 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.545 | 0.545 | 0.548 | 0.002 | 0.546 | 0.545 | 75.4 | 1.00× |
| lazy-ruby | 3 | 7.697 | 7.673 | 7.711 | 0.019 | 7.694 | 7.695 | 49.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.355 | 0.352 | 0.358 | 0.003 | 0.355 | 0.355 | 186.0 | 1.00× |
| lazy-ruby | 5 | 0.354 | 0.352 | 0.360 | 0.004 | 0.355 | 0.352 | 185.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.350 | 0.348 | 0.352 | 0.002 | 0.350 | 0.348 | 185.4 | 1.00× |
| lazy-ruby | 3 | 7.487 | 7.470 | 7.556 | 0.045 | 7.504 | 7.486 | 156.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.566 | 0.566 | 0.574 | 0.003 | 0.568 | 0.565 | 105.1 | 1.00× |
| lazy-ruby | 5 | 0.566 | 0.562 | 0.566 | 0.002 | 0.565 | 0.564 | 105.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.569 | 0.569 | 0.576 | 0.004 | 0.571 | 0.568 | 105.3 | 1.00× |
| lazy-ruby | 3 | 7.728 | 7.705 | 7.763 | 0.029 | 7.732 | 7.724 | 62.8 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.268 | 0.239 | 0.288 | 0.018 | 0.265 | 0.247 | 62.0 | 1.00× |
| lazy-ruby | 5 | 0.267 | 0.246 | 0.284 | 0.018 | 0.266 | 0.258 | 63.8 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.266 | 0.253 | 0.303 | 0.026 | 0.274 | 0.243 | 62.6 | 1.00× |
| lazy-ruby | 3 | 1.185 | 1.148 | 1.270 | 0.062 | 1.201 | 1.235 | 53.3 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.520 | 0.485 | 0.555 | 0.028 | 0.517 | 0.510 | 62.2 | 1.00× |
| lazy-ruby | 5 | 0.524 | 0.491 | 0.732 | 0.097 | 0.562 | 0.504 | 63.7 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.572 | 0.531 | 0.702 | 0.089 | 0.602 | 0.549 | 62.7 | 1.00× |
| lazy-ruby | 3 | 1.535 | 1.431 | 1.576 | 0.075 | 1.514 | 1.609 | 51.2 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.250 | 0.244 | 0.463 | 0.095 | 0.295 | 0.229 | 166.4 | 1.00× |
| lazy-ruby | 5 | 0.272 | 0.246 | 0.417 | 0.071 | 0.293 | 0.259 | 172.8 | 0.92× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.271 | 0.258 | 0.278 | 0.010 | 0.269 | 0.256 | 164.7 | 1.00× |
| lazy-ruby | 3 | 1.168 | 1.163 | 1.191 | 0.015 | 1.174 | 1.264 | 147.2 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.553 | 0.475 | 0.584 | 0.043 | 0.544 | 0.532 | 99.8 | 1.00× |
| lazy-ruby | 5 | 0.506 | 0.452 | 0.597 | 0.055 | 0.522 | 0.497 | 93.7 | 1.09× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.495 | 0.426 | 0.500 | 0.041 | 0.473 | 0.475 | 95.4 | 1.00× |
| lazy-ruby | 3 | 1.305 | 1.264 | 1.428 | 0.085 | 1.332 | 1.388 | 65.3 | 0.38× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.499 | 0.489 | 0.510 | 0.008 | 0.499 | 0.486 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.487 | 0.369 | 0.517 | 0.059 | 0.471 | 0.479 | 57.4 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.511 | 0.481 | 0.513 | 0.018 | 0.502 | 0.497 | 57.4 | 1.00× |
| lazy-ruby | 3 | 4.170 | 3.519 | 4.224 | 0.392 | 3.971 | 4.088 | 42.8 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.947 | 0.941 | 1.075 | 0.057 | 0.973 | 0.933 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.996 | 0.925 | 1.090 | 0.066 | 0.990 | 0.981 | 57.3 | 0.95× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.007 | 0.956 | 1.098 | 0.072 | 1.021 | 0.991 | 57.5 | 1.00× |
| lazy-ruby | 3 | 5.077 | 5.019 | 5.192 | 0.088 | 5.096 | 5.219 | 40.4 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.561 | 0.538 | 0.600 | 0.027 | 0.563 | 0.547 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.581 | 0.553 | 0.589 | 0.015 | 0.573 | 0.568 | 141.9 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.483 | 0.454 | 0.573 | 0.062 | 0.503 | 0.476 | 141.8 | 1.00× |
| lazy-ruby | 3 | 4.565 | 4.415 | 4.580 | 0.091 | 4.520 | 4.699 | 126.9 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.955 | 0.934 | 0.987 | 0.021 | 0.958 | 0.945 | 82.8 | 1.00× |
| lazy-ruby | 5 | 0.953 | 0.906 | 0.993 | 0.032 | 0.950 | 0.938 | 82.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.938 | 0.914 | 0.970 | 0.028 | 0.941 | 0.927 | 83.0 | 1.00× |
| lazy-ruby | 3 | 3.766 | 3.606 | 4.680 | 0.579 | 4.017 | 3.884 | 46.8 | 0.25× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.184 | 0.183 | 0.203 | 0.010 | 0.191 | 0.188 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.183 | 0.182 | 0.187 | 0.002 | 0.184 | 0.188 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.185 | 0.184 | 0.188 | 0.002 | 0.185 | 0.172 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.644 | 0.643 | 0.646 | 0.001 | 0.644 | 0.672 | 58.7 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.338 | 0.336 | 0.347 | 0.004 | 0.340 | 0.344 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.344 | 0.339 | 0.350 | 0.004 | 0.344 | 0.344 | 68.3 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.343 | 0.340 | 0.349 | 0.005 | 0.344 | 0.344 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.808 | 0.801 | 0.808 | 0.004 | 0.806 | 0.828 | 58.8 | 0.42× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.230 | 0.236 | 0.002 | 0.232 | 0.219 | 108.8 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.230 | 0.238 | 0.003 | 0.234 | 0.234 | 109.3 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.233 | 0.233 | 0.000 | 0.233 | 0.234 | 108.9 | 1.00× |
| lazy-ruby | 3 | 0.688 | 0.687 | 0.701 | 0.008 | 0.692 | 0.703 | 100.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.398 | 0.394 | 0.409 | 0.006 | 0.400 | 0.391 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.393 | 0.390 | 0.404 | 0.006 | 0.395 | 0.391 | 109.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.395 | 0.394 | 0.404 | 0.005 | 0.398 | 0.406 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.846 | 0.845 | 0.847 | 0.001 | 0.846 | 0.875 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

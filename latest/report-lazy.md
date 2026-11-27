# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB · image ubuntu24-arm64/20261122.193.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.234 | 0.232 | 0.236 | 0.002 | 0.233 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.232 | 0.232 | 0.234 | 0.001 | 0.232 | 0.231 | 70.6 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.232 | 0.232 | 0.232 | 0.000 | 0.232 | 0.231 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.366 | 2.357 | 2.370 | 0.007 | 2.364 | 2.501 | 49.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.430 | 0.432 | 0.001 | 0.430 | 0.429 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.428 | 0.447 | 0.007 | 0.434 | 0.431 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.430 | 0.429 | 0.432 | 0.002 | 0.430 | 0.430 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.560 | 2.547 | 2.565 | 0.010 | 2.557 | 2.695 | 48.5 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.255 | 0.255 | 0.000 | 0.255 | 0.254 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.255 | 0.257 | 0.001 | 0.255 | 0.255 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.253 | 0.254 | 0.001 | 0.253 | 0.253 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.383 | 2.372 | 2.386 | 0.007 | 2.381 | 2.519 | 158.3 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.413 | 0.409 | 0.414 | 0.002 | 0.413 | 0.412 | 102.2 | 1.00× |
| lazy-ruby | 5 | 0.413 | 0.411 | 0.413 | 0.001 | 0.412 | 0.411 | 102.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.411 | 0.411 | 0.413 | 0.001 | 0.412 | 0.410 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.537 | 2.533 | 2.543 | 0.005 | 2.538 | 2.672 | 62.2 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB · image ubuntu24/20261122.23.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.182 | 0.182 | 0.187 | 0.002 | 0.184 | 0.181 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.183 | 0.182 | 0.185 | 0.001 | 0.183 | 0.181 | 79.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.183 | 0.183 | 0.185 | 0.001 | 0.183 | 0.182 | 79.6 | 1.00× |
| lazy-ruby | 3 | 4.432 | 4.428 | 4.448 | 0.011 | 4.436 | 4.469 | 51.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.337 | 0.333 | 0.340 | 0.003 | 0.336 | 0.335 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.335 | 0.333 | 0.337 | 0.002 | 0.335 | 0.333 | 79.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.334 | 0.333 | 0.335 | 0.001 | 0.334 | 0.332 | 80.0 | 1.00× |
| lazy-ruby | 3 | 4.618 | 4.604 | 4.623 | 0.010 | 4.615 | 4.639 | 51.5 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.214 | 0.213 | 0.215 | 0.001 | 0.214 | 0.213 | 188.7 | 1.00× |
| lazy-ruby | 5 | 0.212 | 0.211 | 0.217 | 0.002 | 0.213 | 0.211 | 188.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.212 | 0.211 | 0.216 | 0.002 | 0.213 | 0.210 | 189.0 | 1.00× |
| lazy-ruby | 3 | 4.473 | 4.469 | 4.494 | 0.013 | 4.479 | 4.496 | 157.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.314 | 0.313 | 0.315 | 0.001 | 0.314 | 0.313 | 112.1 | 1.00× |
| lazy-ruby | 5 | 0.313 | 0.311 | 0.313 | 0.001 | 0.312 | 0.312 | 111.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.312 | 0.310 | 0.313 | 0.001 | 0.312 | 0.311 | 112.1 | 1.00× |
| lazy-ruby | 3 | 4.562 | 4.561 | 4.576 | 0.009 | 4.567 | 4.584 | 65.1 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB · image ubuntu24-arm64/20261122.193.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.234 | 0.242 | 0.003 | 0.239 | 0.238 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.240 | 0.241 | 0.000 | 0.240 | 0.239 | 72.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.236 | 0.236 | 0.237 | 0.000 | 0.236 | 0.235 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.241 | 2.236 | 2.250 | 0.007 | 2.242 | 2.349 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.447 | 0.445 | 0.449 | 0.001 | 0.447 | 0.445 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.447 | 0.450 | 0.001 | 0.448 | 0.446 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.448 | 0.447 | 0.448 | 0.001 | 0.448 | 0.446 | 72.5 | 1.00× |
| lazy-ruby | 3 | 2.463 | 2.457 | 2.470 | 0.006 | 2.463 | 2.573 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.266 | 0.265 | 0.266 | 0.000 | 0.266 | 0.265 | 182.5 | 1.00× |
| lazy-ruby | 5 | 0.266 | 0.266 | 0.270 | 0.002 | 0.267 | 0.266 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.265 | 0.267 | 0.001 | 0.266 | 0.265 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.288 | 2.261 | 2.289 | 0.016 | 2.279 | 2.399 | 156.4 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.430 | 0.435 | 0.002 | 0.432 | 0.431 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.431 | 0.437 | 0.002 | 0.433 | 0.432 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.433 | 0.432 | 0.435 | 0.001 | 0.433 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.446 | 2.441 | 2.446 | 0.003 | 2.445 | 2.557 | 62.5 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB · image ubuntu24/20261122.23.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.310 | 0.309 | 0.312 | 0.001 | 0.310 | 0.308 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.309 | 0.307 | 0.311 | 0.001 | 0.309 | 0.307 | 75.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.309 | 0.309 | 0.310 | 0.001 | 0.309 | 0.308 | 75.0 | 1.00× |
| lazy-ruby | 3 | 7.858 | 7.827 | 7.876 | 0.025 | 7.854 | 7.843 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.456 | 0.454 | 0.462 | 0.003 | 0.457 | 0.454 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.457 | 0.454 | 0.458 | 0.002 | 0.457 | 0.455 | 75.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.455 | 0.453 | 0.457 | 0.002 | 0.455 | 0.454 | 75.3 | 1.00× |
| lazy-ruby | 3 | 7.977 | 7.940 | 8.006 | 0.033 | 7.975 | 7.985 | 49.6 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.336 | 0.336 | 0.340 | 0.002 | 0.337 | 0.336 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.338 | 0.334 | 0.342 | 0.003 | 0.338 | 0.337 | 185.4 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.336 | 0.335 | 0.337 | 0.001 | 0.336 | 0.335 | 185.8 | 1.00× |
| lazy-ruby | 3 | 7.861 | 7.815 | 7.882 | 0.034 | 7.853 | 7.846 | 152.6 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.562 | 0.561 | 0.562 | 0.001 | 0.562 | 0.561 | 105.4 | 1.00× |
| lazy-ruby | 5 | 0.563 | 0.562 | 0.564 | 0.000 | 0.563 | 0.562 | 105.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.563 | 0.562 | 0.564 | 0.001 | 0.563 | 0.562 | 105.3 | 1.00× |
| lazy-ruby | 3 | 8.095 | 8.091 | 8.096 | 0.003 | 8.094 | 8.085 | 62.9 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB · image macos15/20261118.0550.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.208 | 0.198 | 0.218 | 0.007 | 0.208 | 0.194 | 61.5 | 1.00× |
| lazy-ruby | 5 | 0.204 | 0.188 | 0.213 | 0.010 | 0.204 | 0.189 | 61.7 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.185 | 0.204 | 0.009 | 0.195 | 0.179 | 60.9 | 1.00× |
| lazy-ruby | 3 | 0.863 | 0.823 | 0.886 | 0.032 | 0.857 | 0.930 | 51.4 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.384 | 0.375 | 0.423 | 0.019 | 0.391 | 0.371 | 61.2 | 1.00× |
| lazy-ruby | 5 | 0.387 | 0.373 | 0.422 | 0.019 | 0.392 | 0.366 | 63.2 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.400 | 0.399 | 0.416 | 0.010 | 0.405 | 0.376 | 63.7 | 1.00× |
| lazy-ruby | 3 | 1.113 | 1.107 | 1.127 | 0.010 | 1.116 | 1.173 | 51.1 | 0.36× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.209 | 0.200 | 0.218 | 0.008 | 0.208 | 0.190 | 163.5 | 1.00× |
| lazy-ruby | 5 | 0.214 | 0.196 | 0.234 | 0.014 | 0.213 | 0.191 | 164.7 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.233 | 0.234 | 0.000 | 0.234 | 0.211 | 164.0 | 1.00× |
| lazy-ruby | 3 | 0.905 | 0.876 | 0.932 | 0.028 | 0.904 | 0.938 | 143.3 | 0.26× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.402 | 0.462 | 0.025 | 0.431 | 0.410 | 96.5 | 1.00× |
| lazy-ruby | 5 | 0.445 | 0.413 | 0.507 | 0.039 | 0.455 | 0.426 | 95.5 | 0.94× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.390 | 0.377 | 0.403 | 0.013 | 0.390 | 0.379 | 91.2 | 1.00× |
| lazy-ruby | 3 | 1.084 | 1.040 | 1.098 | 0.030 | 1.074 | 1.132 | 68.4 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB · image macos15/20260824.0482.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.386 | 0.374 | 0.472 | 0.041 | 0.400 | 0.372 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.377 | 0.373 | 0.465 | 0.044 | 0.407 | 0.365 | 57.1 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.381 | 0.432 | 0.026 | 0.409 | 0.402 | 57.2 | 1.00× |
| lazy-ruby | 3 | 3.533 | 3.385 | 3.638 | 0.127 | 3.519 | 3.645 | 42.6 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.786 | 0.757 | 0.825 | 0.029 | 0.793 | 0.773 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.801 | 0.750 | 0.947 | 0.076 | 0.822 | 0.790 | 57.2 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.755 | 0.752 | 0.819 | 0.038 | 0.775 | 0.746 | 57.2 | 1.00× |
| lazy-ruby | 3 | 3.930 | 3.903 | 4.144 | 0.132 | 3.993 | 4.044 | 43.1 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.456 | 0.438 | 0.499 | 0.026 | 0.462 | 0.439 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.470 | 0.432 | 0.485 | 0.021 | 0.464 | 0.452 | 141.8 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.447 | 0.441 | 0.460 | 0.010 | 0.449 | 0.431 | 141.6 | 1.00× |
| lazy-ruby | 3 | 3.482 | 3.460 | 3.574 | 0.060 | 3.505 | 3.614 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.712 | 0.704 | 0.793 | 0.038 | 0.726 | 0.698 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.716 | 0.692 | 0.724 | 0.013 | 0.712 | 0.704 | 82.8 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.724 | 0.714 | 0.748 | 0.017 | 0.729 | 0.714 | 82.6 | 1.00× |
| lazy-ruby | 3 | 3.801 | 3.774 | 3.848 | 0.038 | 3.808 | 3.923 | 46.8 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB · image win25-vs2026/20261123.321.1
Versions: tebako 2.8.86

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.238 | 0.237 | 0.242 | 0.002 | 0.239 | 0.234 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.239 | 0.247 | 0.004 | 0.241 | 0.234 | 68.3 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.241 | 0.241 | 0.248 | 0.004 | 0.243 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.819 | 0.817 | 0.823 | 0.003 | 0.820 | 0.828 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.463 | 0.460 | 0.472 | 0.005 | 0.465 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.465 | 0.460 | 0.473 | 0.005 | 0.466 | 0.484 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.464 | 0.464 | 0.469 | 0.003 | 0.465 | 0.484 | 68.2 | 1.00× |
| lazy-ruby | 3 | 1.041 | 1.040 | 1.045 | 0.002 | 1.042 | 1.062 | 58.8 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.285 | 0.294 | 0.004 | 0.289 | 0.297 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.293 | 0.288 | 0.296 | 0.004 | 0.292 | 0.281 | 109.3 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.288 | 0.287 | 0.291 | 0.002 | 0.289 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.869 | 0.853 | 0.878 | 0.013 | 0.867 | 0.875 | 99.9 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.514 | 0.512 | 0.516 | 0.002 | 0.514 | 0.516 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.515 | 0.508 | 0.527 | 0.007 | 0.517 | 0.500 | 108.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.511 | 0.511 | 0.518 | 0.004 | 0.513 | 0.500 | 109.1 | 1.00× |
| lazy-ruby | 3 | 1.094 | 1.084 | 1.100 | 0.008 | 1.093 | 1.125 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

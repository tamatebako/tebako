# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.229 | 0.231 | 0.001 | 0.229 | 0.227 | 107.9 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.229 | 0.231 | 0.001 | 0.229 | 0.228 | 107.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.229 | 0.229 | 0.229 | 0.000 | 0.229 | 0.227 | 107.9 | 1.00× |
| lazy-ruby | 3 | 2.352 | 2.348 | 2.355 | 0.003 | 2.352 | 2.487 | 107.9 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.428 | 0.447 | 0.007 | 0.435 | 0.433 | 107.9 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.430 | 0.434 | 0.002 | 0.432 | 0.431 | 107.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.432 | 0.432 | 0.432 | 0.000 | 0.432 | 0.431 | 107.9 | 1.00× |
| lazy-ruby | 3 | 2.554 | 2.542 | 2.564 | 0.011 | 2.554 | 2.688 | 107.9 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.257 | 0.257 | 0.258 | 0.000 | 0.257 | 0.256 | 180.1 | 1.00× |
| lazy-ruby | 5 | 0.257 | 0.257 | 0.260 | 0.001 | 0.258 | 0.257 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.258 | 0.255 | 0.259 | 0.002 | 0.257 | 0.257 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.371 | 2.367 | 2.376 | 0.005 | 2.371 | 2.508 | 155.1 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.414 | 0.412 | 0.416 | 0.001 | 0.414 | 0.412 | 107.9 | 1.00× |
| lazy-ruby | 5 | 0.414 | 0.412 | 0.420 | 0.003 | 0.415 | 0.412 | 107.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.412 | 0.414 | 0.001 | 0.413 | 0.412 | 107.9 | 1.00× |
| lazy-ruby | 3 | 2.544 | 2.529 | 2.544 | 0.008 | 2.539 | 2.678 | 107.9 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.257 | 0.248 | 0.276 | 0.012 | 0.259 | 0.255 | 114.0 | 1.00× |
| lazy-ruby | 5 | 0.263 | 0.250 | 0.274 | 0.011 | 0.261 | 0.261 | 114.0 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.252 | 0.252 | 0.265 | 0.007 | 0.256 | 0.251 | 114.0 | 1.00× |
| lazy-ruby | 3 | 6.120 | 6.080 | 6.223 | 0.074 | 6.141 | 6.146 | 114.0 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.453 | 0.442 | 0.487 | 0.019 | 0.460 | 0.453 | 114.0 | 1.00× |
| lazy-ruby | 5 | 0.455 | 0.440 | 0.503 | 0.026 | 0.463 | 0.454 | 114.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.451 | 0.450 | 0.459 | 0.005 | 0.453 | 0.449 | 114.0 | 1.00× |
| lazy-ruby | 3 | 6.227 | 6.192 | 6.251 | 0.030 | 6.223 | 6.252 | 114.0 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.285 | 0.285 | 0.298 | 0.006 | 0.289 | 0.285 | 188.8 | 1.00× |
| lazy-ruby | 5 | 0.291 | 0.283 | 0.323 | 0.016 | 0.297 | 0.291 | 188.9 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.289 | 0.289 | 0.293 | 0.002 | 0.291 | 0.288 | 188.6 | 1.00× |
| lazy-ruby | 3 | 6.086 | 6.048 | 6.157 | 0.055 | 6.097 | 6.108 | 157.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.427 | 0.423 | 0.441 | 0.007 | 0.430 | 0.426 | 114.0 | 1.00× |
| lazy-ruby | 5 | 0.423 | 0.423 | 0.478 | 0.024 | 0.435 | 0.423 | 114.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.423 | 0.423 | 0.432 | 0.005 | 0.426 | 0.422 | 114.0 | 1.00× |
| lazy-ruby | 3 | 6.252 | 6.197 | 6.301 | 0.052 | 6.250 | 6.272 | 114.0 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.226 | 0.229 | 0.001 | 0.227 | 0.225 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.227 | 0.229 | 0.001 | 0.227 | 0.226 | 75.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.227 | 0.228 | 0.001 | 0.227 | 0.226 | 75.2 | 1.00× |
| lazy-ruby | 3 | 2.209 | 2.203 | 2.226 | 0.012 | 2.212 | 2.321 | 75.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.436 | 0.438 | 0.001 | 0.437 | 0.435 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.438 | 0.436 | 0.438 | 0.001 | 0.438 | 0.436 | 75.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.436 | 0.436 | 0.000 | 0.436 | 0.434 | 75.2 | 1.00× |
| lazy-ruby | 3 | 2.433 | 2.431 | 2.437 | 0.003 | 2.434 | 2.544 | 75.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.251 | 0.255 | 0.002 | 0.253 | 0.252 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.251 | 0.253 | 0.001 | 0.253 | 0.251 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.253 | 0.253 | 0.000 | 0.253 | 0.252 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.242 | 2.236 | 2.244 | 0.004 | 2.240 | 2.356 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.416 | 0.414 | 0.420 | 0.002 | 0.417 | 0.415 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.414 | 0.420 | 0.002 | 0.416 | 0.414 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.416 | 0.416 | 0.416 | 0.000 | 0.416 | 0.414 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.408 | 2.394 | 2.412 | 0.010 | 2.405 | 2.523 | 75.2 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.317 | 0.315 | 0.325 | 0.004 | 0.318 | 0.316 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.317 | 0.316 | 0.319 | 0.001 | 0.317 | 0.316 | 79.8 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.319 | 0.317 | 0.322 | 0.003 | 0.319 | 0.318 | 79.8 | 1.00× |
| lazy-ruby | 3 | 7.492 | 7.416 | 7.496 | 0.045 | 7.468 | 7.487 | 79.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.550 | 0.548 | 0.557 | 0.004 | 0.551 | 0.548 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.542 | 0.540 | 0.548 | 0.004 | 0.544 | 0.540 | 79.8 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.558 | 0.551 | 0.560 | 0.004 | 0.556 | 0.556 | 79.8 | 1.00× |
| lazy-ruby | 3 | 7.674 | 7.665 | 7.688 | 0.011 | 7.676 | 7.688 | 79.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.354 | 0.352 | 0.356 | 0.002 | 0.353 | 0.352 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.354 | 0.352 | 0.357 | 0.002 | 0.354 | 0.353 | 185.9 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.353 | 0.348 | 0.356 | 0.004 | 0.352 | 0.351 | 185.7 | 1.00× |
| lazy-ruby | 3 | 7.490 | 7.468 | 7.501 | 0.017 | 7.486 | 7.490 | 155.2 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.569 | 0.564 | 0.571 | 0.002 | 0.568 | 0.568 | 105.2 | 1.00× |
| lazy-ruby | 5 | 0.569 | 0.565 | 0.577 | 0.005 | 0.570 | 0.567 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.563 | 0.563 | 0.578 | 0.009 | 0.568 | 0.562 | 105.3 | 1.00× |
| lazy-ruby | 3 | 7.674 | 7.659 | 7.732 | 0.039 | 7.688 | 7.674 | 79.8 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.191 | 0.184 | 0.203 | 0.008 | 0.192 | 0.177 | 63.7 | 1.00× |
| lazy-ruby | 5 | 0.193 | 0.186 | 0.201 | 0.006 | 0.192 | 0.180 | 64.1 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.195 | 0.190 | 0.196 | 0.003 | 0.194 | 0.180 | 65.6 | 1.00× |
| lazy-ruby | 3 | 0.876 | 0.860 | 0.918 | 0.030 | 0.885 | 0.936 | 52.2 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.394 | 0.386 | 0.399 | 0.005 | 0.393 | 0.380 | 62.8 | 1.00× |
| lazy-ruby | 5 | 0.385 | 0.378 | 0.395 | 0.006 | 0.386 | 0.375 | 65.1 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.392 | 0.386 | 0.396 | 0.005 | 0.391 | 0.380 | 64.0 | 1.00× |
| lazy-ruby | 3 | 1.063 | 1.048 | 1.066 | 0.010 | 1.059 | 1.122 | 55.3 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.219 | 0.200 | 0.253 | 0.020 | 0.222 | 0.194 | 166.5 | 1.00× |
| lazy-ruby | 5 | 0.222 | 0.193 | 0.238 | 0.017 | 0.218 | 0.208 | 164.7 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.209 | 0.194 | 0.219 | 0.012 | 0.207 | 0.197 | 167.7 | 1.00× |
| lazy-ruby | 3 | 0.871 | 0.870 | 0.877 | 0.004 | 0.873 | 0.940 | 148.9 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.383 | 0.366 | 0.385 | 0.008 | 0.379 | 0.365 | 93.8 | 1.00× |
| lazy-ruby | 5 | 0.368 | 0.366 | 0.393 | 0.011 | 0.374 | 0.360 | 93.6 | 1.04× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.369 | 0.356 | 0.377 | 0.011 | 0.367 | 0.363 | 93.0 | 1.00× |
| lazy-ruby | 3 | 1.050 | 1.021 | 1.073 | 0.026 | 1.048 | 1.124 | 66.6 | 0.35× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.507 | 0.493 | 0.554 | 0.031 | 0.520 | 0.497 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.526 | 0.506 | 0.550 | 0.018 | 0.528 | 0.514 | 57.3 | 0.96× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.536 | 0.528 | 0.545 | 0.008 | 0.536 | 0.525 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.531 | 4.387 | 4.700 | 0.157 | 4.540 | 4.680 | 42.9 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.951 | 0.936 | 1.009 | 0.030 | 0.959 | 0.940 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.970 | 0.945 | 1.100 | 0.061 | 0.998 | 0.958 | 57.2 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.004 | 0.954 | 1.034 | 0.040 | 0.997 | 0.988 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.308 | 5.038 | 5.463 | 0.215 | 5.270 | 5.370 | 42.8 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.579 | 0.568 | 0.632 | 0.028 | 0.593 | 0.564 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.584 | 0.563 | 0.778 | 0.089 | 0.620 | 0.575 | 141.8 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.569 | 0.539 | 0.577 | 0.020 | 0.562 | 0.553 | 141.9 | 1.00× |
| lazy-ruby | 3 | 4.617 | 4.578 | 4.727 | 0.077 | 4.641 | 4.767 | 126.8 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.964 | 0.907 | 1.114 | 0.083 | 0.994 | 0.954 | 83.2 | 1.00× |
| lazy-ruby | 5 | 0.933 | 0.908 | 0.995 | 0.037 | 0.949 | 0.923 | 83.0 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.954 | 0.927 | 1.031 | 0.054 | 0.971 | 0.931 | 83.1 | 1.00× |
| lazy-ruby | 3 | 5.028 | 4.963 | 5.115 | 0.077 | 5.035 | 5.175 | 46.8 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.68

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.234 | 0.231 | 0.249 | 0.008 | 0.238 | 0.234 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.233 | 0.254 | 0.010 | 0.240 | 0.234 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.233 | 0.240 | 0.004 | 0.237 | 0.234 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.795 | 0.795 | 0.803 | 0.005 | 0.798 | 0.859 | 58.8 | 0.30× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.459 | 0.453 | 0.460 | 0.002 | 0.458 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.457 | 0.451 | 0.461 | 0.004 | 0.456 | 0.453 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.456 | 0.460 | 0.002 | 0.457 | 0.453 | 68.5 | 1.00× |
| lazy-ruby | 3 | 1.026 | 1.011 | 1.031 | 0.010 | 1.022 | 1.031 | 58.7 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.281 | 0.291 | 0.005 | 0.287 | 0.281 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.281 | 0.280 | 0.288 | 0.003 | 0.282 | 0.281 | 109.3 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.282 | 0.281 | 0.283 | 0.001 | 0.282 | 0.281 | 109.6 | 1.00× |
| lazy-ruby | 3 | 0.855 | 0.845 | 0.859 | 0.007 | 0.853 | 0.875 | 100.6 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.494 | 0.493 | 0.498 | 0.002 | 0.495 | 0.484 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.499 | 0.495 | 0.502 | 0.003 | 0.499 | 0.516 | 108.8 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.502 | 0.496 | 0.505 | 0.004 | 0.501 | 0.500 | 109.0 | 1.00× |
| lazy-ruby | 3 | 1.069 | 1.067 | 1.070 | 0.002 | 1.069 | 1.109 | 82.8 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

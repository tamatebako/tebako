# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.232 | 0.231 | 0.234 | 0.001 | 0.232 | 0.231 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.230 | 0.229 | 0.232 | 0.001 | 0.230 | 0.229 | 70.5 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.232 | 0.234 | 0.001 | 0.233 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.384 | 2.363 | 2.384 | 0.012 | 2.377 | 2.518 | 48.4 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.436 | 0.440 | 0.002 | 0.437 | 0.436 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.438 | 0.436 | 0.471 | 0.015 | 0.447 | 0.437 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.432 | 0.437 | 0.003 | 0.434 | 0.432 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.572 | 2.571 | 2.581 | 0.005 | 2.575 | 2.707 | 49.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.261 | 0.265 | 0.001 | 0.263 | 0.262 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.259 | 0.263 | 0.002 | 0.261 | 0.260 | 180.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.258 | 0.263 | 0.002 | 0.261 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.407 | 2.376 | 2.420 | 0.023 | 2.401 | 2.528 | 156.6 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.421 | 0.419 | 0.421 | 0.001 | 0.421 | 0.421 | 102.0 | 1.00× |
| lazy-ruby | 5 | 0.421 | 0.417 | 0.426 | 0.003 | 0.421 | 0.419 | 102.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.417 | 0.417 | 0.419 | 0.001 | 0.418 | 0.416 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.560 | 2.552 | 2.618 | 0.036 | 2.577 | 2.695 | 62.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.202 | 0.200 | 0.204 | 0.002 | 0.202 | 0.200 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.204 | 0.201 | 0.204 | 0.002 | 0.203 | 0.203 | 79.8 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.204 | 0.204 | 0.206 | 0.001 | 0.205 | 0.203 | 79.5 | 1.00× |
| lazy-ruby | 3 | 4.656 | 4.559 | 4.748 | 0.095 | 4.654 | 4.697 | 51.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.302 | 0.293 | 0.309 | 0.006 | 0.302 | 0.301 | 80.0 | 1.00× |
| lazy-ruby | 5 | 0.302 | 0.299 | 0.302 | 0.002 | 0.301 | 0.300 | 79.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.296 | 0.295 | 0.306 | 0.006 | 0.299 | 0.295 | 79.8 | 1.00× |
| lazy-ruby | 3 | 4.748 | 4.684 | 4.768 | 0.044 | 4.734 | 4.790 | 51.4 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.230 | 0.227 | 0.235 | 0.003 | 0.230 | 0.230 | 188.0 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.223 | 0.229 | 0.003 | 0.226 | 0.226 | 188.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.230 | 0.226 | 0.230 | 0.002 | 0.229 | 0.228 | 187.9 | 1.00× |
| lazy-ruby | 3 | 4.681 | 4.652 | 4.701 | 0.025 | 4.678 | 4.731 | 157.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.343 | 0.337 | 0.348 | 0.004 | 0.342 | 0.342 | 111.4 | 1.00× |
| lazy-ruby | 5 | 0.341 | 0.331 | 0.348 | 0.008 | 0.339 | 0.341 | 111.5 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.335 | 0.332 | 0.343 | 0.006 | 0.337 | 0.333 | 111.2 | 1.00× |
| lazy-ruby | 3 | 4.805 | 4.770 | 4.839 | 0.035 | 4.804 | 4.870 | 64.6 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.238 | 0.235 | 0.238 | 0.001 | 0.237 | 0.236 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.236 | 0.235 | 0.237 | 0.001 | 0.236 | 0.235 | 72.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.236 | 0.236 | 0.237 | 0.001 | 0.236 | 0.235 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.259 | 2.252 | 2.259 | 0.004 | 2.257 | 2.370 | 47.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.450 | 0.448 | 0.450 | 0.001 | 0.449 | 0.448 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.449 | 0.448 | 0.450 | 0.001 | 0.449 | 0.447 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.450 | 0.447 | 0.450 | 0.001 | 0.449 | 0.448 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.464 | 2.460 | 2.473 | 0.006 | 2.466 | 2.576 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.265 | 0.271 | 0.003 | 0.266 | 0.265 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.264 | 0.273 | 0.004 | 0.266 | 0.264 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.262 | 0.267 | 0.002 | 0.264 | 0.263 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.286 | 2.274 | 2.289 | 0.008 | 2.283 | 2.397 | 156.2 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.431 | 0.429 | 0.433 | 0.002 | 0.431 | 0.429 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.427 | 0.431 | 0.002 | 0.430 | 0.429 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.433 | 0.431 | 0.433 | 0.001 | 0.432 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.442 | 2.438 | 2.452 | 0.007 | 2.444 | 2.552 | 62.6 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.307 | 0.305 | 0.310 | 0.002 | 0.307 | 0.305 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.307 | 0.305 | 0.309 | 0.001 | 0.307 | 0.305 | 75.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.306 | 0.306 | 0.306 | 0.000 | 0.306 | 0.305 | 75.2 | 1.00× |
| lazy-ruby | 3 | 7.836 | 7.809 | 7.862 | 0.026 | 7.836 | 7.820 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.457 | 0.455 | 0.461 | 0.002 | 0.458 | 0.456 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.457 | 0.454 | 0.461 | 0.003 | 0.457 | 0.456 | 75.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.455 | 0.461 | 0.003 | 0.457 | 0.454 | 75.3 | 1.00× |
| lazy-ruby | 3 | 7.969 | 7.958 | 8.025 | 0.036 | 7.984 | 8.012 | 49.8 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.335 | 0.332 | 0.340 | 0.003 | 0.336 | 0.334 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.336 | 0.330 | 0.339 | 0.004 | 0.336 | 0.335 | 185.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.334 | 0.333 | 0.344 | 0.006 | 0.337 | 0.333 | 185.7 | 1.00× |
| lazy-ruby | 3 | 7.859 | 7.850 | 7.879 | 0.015 | 7.862 | 7.845 | 155.6 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.565 | 0.561 | 0.570 | 0.004 | 0.565 | 0.563 | 105.4 | 1.00× |
| lazy-ruby | 5 | 0.565 | 0.559 | 0.569 | 0.005 | 0.564 | 0.564 | 105.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.563 | 0.563 | 0.567 | 0.002 | 0.564 | 0.562 | 105.5 | 1.00× |
| lazy-ruby | 3 | 8.101 | 8.097 | 8.122 | 0.014 | 8.107 | 8.101 | 63.0 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.207 | 0.179 | 0.219 | 0.015 | 0.201 | 0.182 | 62.0 | 1.00× |
| lazy-ruby | 5 | 0.210 | 0.193 | 0.214 | 0.010 | 0.205 | 0.196 | 62.0 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.197 | 0.183 | 0.198 | 0.009 | 0.193 | 0.178 | 62.1 | 1.00× |
| lazy-ruby | 3 | 0.835 | 0.820 | 0.852 | 0.016 | 0.836 | 0.896 | 54.2 | 0.24× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.380 | 0.367 | 0.425 | 0.027 | 0.394 | 0.364 | 62.1 | 1.00× |
| lazy-ruby | 5 | 0.384 | 0.376 | 0.439 | 0.026 | 0.394 | 0.370 | 61.5 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.385 | 0.379 | 0.395 | 0.008 | 0.387 | 0.369 | 61.3 | 1.00× |
| lazy-ruby | 3 | 1.004 | 1.000 | 1.013 | 0.007 | 1.006 | 1.063 | 52.6 | 0.38× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.198 | 0.186 | 0.218 | 0.014 | 0.199 | 0.181 | 164.0 | 1.00× |
| lazy-ruby | 5 | 0.192 | 0.189 | 0.202 | 0.006 | 0.195 | 0.181 | 164.7 | 1.03× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.207 | 0.203 | 0.208 | 0.003 | 0.206 | 0.181 | 163.9 | 1.00× |
| lazy-ruby | 3 | 0.885 | 0.838 | 0.950 | 0.056 | 0.891 | 0.935 | 150.5 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.362 | 0.347 | 0.379 | 0.013 | 0.363 | 0.344 | 91.3 | 1.00× |
| lazy-ruby | 5 | 0.363 | 0.348 | 0.377 | 0.010 | 0.362 | 0.342 | 90.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.385 | 0.352 | 0.398 | 0.024 | 0.378 | 0.366 | 91.5 | 1.00× |
| lazy-ruby | 3 | 0.997 | 0.995 | 1.002 | 0.004 | 0.998 | 1.047 | 67.5 | 0.39× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.512 | 0.493 | 0.548 | 0.022 | 0.515 | 0.502 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.530 | 0.483 | 0.601 | 0.048 | 0.536 | 0.520 | 57.2 | 0.97× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.536 | 0.534 | 0.537 | 0.002 | 0.536 | 0.523 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.371 | 3.968 | 4.572 | 0.307 | 4.304 | 4.545 | 42.9 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.927 | 0.883 | 0.950 | 0.028 | 0.919 | 0.916 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.890 | 0.887 | 0.954 | 0.030 | 0.908 | 0.882 | 57.2 | 1.04× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.883 | 0.863 | 1.022 | 0.087 | 0.922 | 0.868 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.592 | 4.429 | 4.808 | 0.190 | 4.610 | 4.724 | 42.7 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.618 | 0.591 | 0.643 | 0.024 | 0.618 | 0.600 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.623 | 0.614 | 0.639 | 0.009 | 0.625 | 0.612 | 141.7 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.572 | 0.570 | 0.643 | 0.042 | 0.595 | 0.561 | 141.7 | 1.00× |
| lazy-ruby | 3 | 4.709 | 4.625 | 4.793 | 0.084 | 4.709 | 4.853 | 126.9 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.849 | 0.810 | 1.084 | 0.130 | 0.922 | 0.838 | 83.1 | 1.00× |
| lazy-ruby | 5 | 0.858 | 0.794 | 0.946 | 0.069 | 0.872 | 0.846 | 82.8 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.788 | 0.784 | 0.801 | 0.009 | 0.791 | 0.777 | 83.0 | 1.00× |
| lazy-ruby | 3 | 4.885 | 4.479 | 5.085 | 0.309 | 4.816 | 5.043 | 46.5 | 0.16× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.82

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.239 | 0.234 | 0.244 | 0.004 | 0.239 | 0.234 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.237 | 0.236 | 0.240 | 0.002 | 0.238 | 0.234 | 68.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.236 | 0.239 | 0.001 | 0.237 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.821 | 0.818 | 0.824 | 0.003 | 0.821 | 0.844 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.439 | 0.432 | 0.449 | 0.007 | 0.441 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.441 | 0.438 | 0.456 | 0.008 | 0.445 | 0.453 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.443 | 0.439 | 0.447 | 0.004 | 0.443 | 0.422 | 68.5 | 1.00× |
| lazy-ruby | 3 | 1.026 | 1.025 | 1.033 | 0.004 | 1.028 | 1.062 | 58.8 | 0.43× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.290 | 0.292 | 0.001 | 0.290 | 0.281 | 109.2 | 1.00× |
| lazy-ruby | 5 | 0.295 | 0.292 | 0.296 | 0.002 | 0.295 | 0.312 | 109.0 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.294 | 0.292 | 0.297 | 0.003 | 0.295 | 0.297 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.877 | 0.876 | 0.880 | 0.002 | 0.878 | 0.922 | 100.6 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.504 | 0.502 | 0.511 | 0.004 | 0.505 | 0.500 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.510 | 0.509 | 0.523 | 0.006 | 0.513 | 0.500 | 109.0 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.506 | 0.505 | 0.511 | 0.003 | 0.507 | 0.500 | 109.1 | 1.00× |
| lazy-ruby | 3 | 1.084 | 1.083 | 1.094 | 0.006 | 1.087 | 1.125 | 83.0 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

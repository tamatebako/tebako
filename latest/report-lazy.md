# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.229 | 0.231 | 0.001 | 0.230 | 0.229 | 108.2 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.229 | 0.231 | 0.001 | 0.229 | 0.227 | 108.2 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.231 | 0.233 | 0.001 | 0.231 | 0.230 | 108.2 | 1.00× |
| lazy-ruby | 3 | 2.356 | 2.331 | 2.361 | 0.016 | 2.349 | 2.491 | 108.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.428 | 0.434 | 0.003 | 0.431 | 0.429 | 108.2 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.428 | 0.432 | 0.002 | 0.430 | 0.428 | 108.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.432 | 0.432 | 0.434 | 0.001 | 0.433 | 0.432 | 108.2 | 1.00× |
| lazy-ruby | 3 | 2.552 | 2.536 | 2.557 | 0.011 | 2.549 | 2.685 | 108.2 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.251 | 0.257 | 0.002 | 0.255 | 0.254 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.253 | 0.253 | 0.000 | 0.253 | 0.253 | 180.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.255 | 0.253 | 0.255 | 0.001 | 0.255 | 0.255 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.378 | 2.373 | 2.380 | 0.003 | 2.377 | 2.512 | 156.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.412 | 0.410 | 0.418 | 0.003 | 0.413 | 0.411 | 108.2 | 1.00× |
| lazy-ruby | 5 | 0.412 | 0.410 | 0.414 | 0.002 | 0.412 | 0.410 | 108.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.407 | 0.412 | 0.002 | 0.409 | 0.406 | 108.2 | 1.00× |
| lazy-ruby | 3 | 2.532 | 2.514 | 2.534 | 0.011 | 2.527 | 2.666 | 108.2 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.225 | 0.229 | 0.002 | 0.227 | 0.225 | 115.4 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.227 | 0.231 | 0.001 | 0.229 | 0.228 | 115.4 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.225 | 0.229 | 0.002 | 0.227 | 0.227 | 115.4 | 1.00× |
| lazy-ruby | 3 | 5.554 | 5.352 | 5.589 | 0.128 | 5.499 | 5.603 | 115.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.392 | 0.390 | 0.420 | 0.012 | 0.398 | 0.392 | 115.4 | 1.00× |
| lazy-ruby | 5 | 0.394 | 0.394 | 0.406 | 0.006 | 0.398 | 0.394 | 115.4 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.396 | 0.392 | 0.403 | 0.005 | 0.397 | 0.395 | 115.4 | 1.00× |
| lazy-ruby | 3 | 5.534 | 5.523 | 5.670 | 0.082 | 5.576 | 5.617 | 115.4 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.264 | 0.262 | 0.268 | 0.003 | 0.264 | 0.262 | 188.8 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.260 | 0.268 | 0.003 | 0.263 | 0.261 | 188.6 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.260 | 0.260 | 0.262 | 0.001 | 0.261 | 0.260 | 188.4 | 1.00× |
| lazy-ruby | 3 | 5.355 | 5.355 | 5.404 | 0.028 | 5.371 | 5.407 | 157.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.398 | 0.396 | 0.403 | 0.003 | 0.398 | 0.397 | 115.4 | 1.00× |
| lazy-ruby | 5 | 0.398 | 0.394 | 0.398 | 0.002 | 0.397 | 0.397 | 115.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.396 | 0.392 | 0.400 | 0.004 | 0.396 | 0.395 | 115.4 | 1.00× |
| lazy-ruby | 3 | 5.511 | 5.406 | 5.612 | 0.103 | 5.510 | 5.561 | 115.4 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.235 | 0.243 | 0.003 | 0.238 | 0.236 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.235 | 0.239 | 0.002 | 0.236 | 0.234 | 74.5 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.237 | 0.239 | 0.001 | 0.238 | 0.236 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.254 | 2.246 | 2.310 | 0.035 | 2.270 | 2.363 | 74.5 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.449 | 0.443 | 0.449 | 0.003 | 0.447 | 0.447 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.445 | 0.457 | 0.005 | 0.449 | 0.445 | 74.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.443 | 0.440 | 0.443 | 0.001 | 0.442 | 0.442 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.456 | 2.452 | 2.470 | 0.010 | 2.459 | 2.566 | 74.5 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.258 | 0.255 | 0.262 | 0.003 | 0.259 | 0.257 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.258 | 0.255 | 0.266 | 0.004 | 0.259 | 0.256 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.262 | 0.262 | 0.000 | 0.262 | 0.260 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.266 | 2.261 | 2.269 | 0.004 | 2.265 | 2.378 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.432 | 0.428 | 0.436 | 0.004 | 0.432 | 0.431 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.430 | 0.426 | 0.432 | 0.003 | 0.430 | 0.429 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.426 | 0.426 | 0.434 | 0.005 | 0.429 | 0.426 | 101.6 | 1.00× |
| lazy-ruby | 3 | 2.456 | 2.442 | 2.456 | 0.008 | 2.451 | 2.566 | 74.5 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.319 | 0.317 | 0.323 | 0.002 | 0.320 | 0.318 | 80.4 | 1.00× |
| lazy-ruby | 5 | 0.317 | 0.315 | 0.326 | 0.004 | 0.318 | 0.315 | 80.4 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.319 | 0.317 | 0.319 | 0.001 | 0.318 | 0.317 | 80.4 | 1.00× |
| lazy-ruby | 3 | 7.462 | 7.409 | 7.469 | 0.033 | 7.447 | 7.463 | 80.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.549 | 0.540 | 0.551 | 0.005 | 0.547 | 0.547 | 80.4 | 1.00× |
| lazy-ruby | 5 | 0.544 | 0.538 | 0.548 | 0.004 | 0.544 | 0.543 | 80.4 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.545 | 0.545 | 0.548 | 0.002 | 0.546 | 0.543 | 80.4 | 1.00× |
| lazy-ruby | 3 | 7.681 | 7.675 | 7.721 | 0.025 | 7.693 | 7.680 | 80.4 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.357 | 0.350 | 0.358 | 0.003 | 0.355 | 0.356 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.356 | 0.350 | 0.362 | 0.005 | 0.355 | 0.355 | 185.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.352 | 0.352 | 0.355 | 0.002 | 0.353 | 0.352 | 185.5 | 1.00× |
| lazy-ruby | 3 | 7.483 | 7.466 | 7.508 | 0.021 | 7.486 | 7.485 | 155.4 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.569 | 0.565 | 0.578 | 0.006 | 0.571 | 0.567 | 105.1 | 1.00× |
| lazy-ruby | 5 | 0.569 | 0.565 | 0.571 | 0.003 | 0.568 | 0.568 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.568 | 0.565 | 0.569 | 0.002 | 0.567 | 0.567 | 105.5 | 1.00× |
| lazy-ruby | 3 | 7.701 | 7.669 | 7.717 | 0.025 | 7.696 | 7.716 | 80.4 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.209 | 0.250 | 0.019 | 0.231 | 0.222 | 63.9 | 1.00× |
| lazy-ruby | 5 | 0.228 | 0.217 | 0.249 | 0.016 | 0.232 | 0.220 | 62.9 | 1.05× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.259 | 0.240 | 0.260 | 0.011 | 0.253 | 0.238 | 61.9 | 1.00× |
| lazy-ruby | 3 | 0.990 | 0.985 | 1.033 | 0.026 | 1.003 | 1.060 | 51.5 | 0.26× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.453 | 0.416 | 0.576 | 0.070 | 0.487 | 0.443 | 64.5 | 1.00× |
| lazy-ruby | 5 | 0.468 | 0.377 | 0.566 | 0.077 | 0.484 | 0.449 | 62.9 | 0.97× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.404 | 0.397 | 0.470 | 0.040 | 0.424 | 0.384 | 62.4 | 1.00× |
| lazy-ruby | 3 | 1.107 | 1.056 | 1.173 | 0.059 | 1.112 | 1.188 | 53.8 | 0.36× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.236 | 0.218 | 0.256 | 0.015 | 0.236 | 0.219 | 163.2 | 1.00× |
| lazy-ruby | 5 | 0.217 | 0.201 | 0.234 | 0.013 | 0.217 | 0.206 | 163.6 | 1.09× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.217 | 0.203 | 0.228 | 0.013 | 0.216 | 0.202 | 163.5 | 1.00× |
| lazy-ruby | 3 | 0.854 | 0.829 | 0.876 | 0.024 | 0.853 | 0.916 | 151.0 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.367 | 0.360 | 0.415 | 0.023 | 0.379 | 0.353 | 93.1 | 1.00× |
| lazy-ruby | 5 | 0.377 | 0.349 | 0.387 | 0.016 | 0.373 | 0.355 | 90.8 | 0.97× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.374 | 0.373 | 0.429 | 0.032 | 0.392 | 0.354 | 90.4 | 1.00× |
| lazy-ruby | 3 | 1.102 | 1.040 | 1.197 | 0.079 | 1.113 | 1.144 | 70.0 | 0.34× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.377 | 0.367 | 0.394 | 0.011 | 0.377 | 0.363 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.380 | 0.367 | 0.533 | 0.070 | 0.410 | 0.374 | 57.6 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.533 | 0.429 | 0.533 | 0.060 | 0.498 | 0.516 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.507 | 3.434 | 3.510 | 0.043 | 3.483 | 3.553 | 42.8 | 0.15× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.778 | 0.748 | 0.817 | 0.028 | 0.781 | 0.765 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.810 | 0.777 | 0.840 | 0.028 | 0.807 | 0.786 | 57.5 | 0.96× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.783 | 0.773 | 0.788 | 0.008 | 0.782 | 0.776 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.967 | 3.956 | 4.430 | 0.270 | 4.118 | 4.086 | 42.9 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.433 | 0.489 | 0.023 | 0.454 | 0.432 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.427 | 0.451 | 0.011 | 0.436 | 0.422 | 141.7 | 1.03× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.473 | 0.470 | 0.522 | 0.029 | 0.488 | 0.458 | 142.0 | 1.00× |
| lazy-ruby | 3 | 3.709 | 3.498 | 3.801 | 0.155 | 3.669 | 3.826 | 126.7 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.738 | 0.718 | 0.755 | 0.014 | 0.737 | 0.731 | 83.0 | 1.00× |
| lazy-ruby | 5 | 0.736 | 0.718 | 0.791 | 0.028 | 0.744 | 0.723 | 82.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.717 | 0.714 | 0.725 | 0.006 | 0.719 | 0.708 | 83.1 | 1.00× |
| lazy-ruby | 3 | 4.274 | 3.923 | 6.856 | 1.602 | 5.018 | 4.378 | 47.1 | 0.17× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.73

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.234 | 0.232 | 0.241 | 0.003 | 0.235 | 0.219 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.232 | 0.243 | 0.005 | 0.236 | 0.234 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.235 | 0.235 | 0.238 | 0.002 | 0.236 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.799 | 0.796 | 0.803 | 0.003 | 0.799 | 0.812 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.451 | 0.449 | 0.456 | 0.003 | 0.452 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.460 | 0.456 | 0.465 | 0.004 | 0.461 | 0.453 | 68.3 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.462 | 0.454 | 0.463 | 0.005 | 0.460 | 0.469 | 68.4 | 1.00× |
| lazy-ruby | 3 | 1.018 | 1.015 | 1.029 | 0.008 | 1.020 | 1.062 | 58.8 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.282 | 0.281 | 0.289 | 0.003 | 0.284 | 0.281 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.281 | 0.279 | 0.283 | 0.002 | 0.281 | 0.281 | 109.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.285 | 0.283 | 0.289 | 0.003 | 0.285 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.848 | 0.848 | 0.851 | 0.001 | 0.849 | 0.891 | 100.4 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.494 | 0.493 | 0.504 | 0.005 | 0.497 | 0.500 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.496 | 0.495 | 0.497 | 0.001 | 0.496 | 0.484 | 108.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.498 | 0.497 | 0.503 | 0.003 | 0.499 | 0.484 | 109.2 | 1.00× |
| lazy-ruby | 3 | 1.070 | 1.055 | 1.074 | 0.010 | 1.066 | 1.062 | 82.7 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

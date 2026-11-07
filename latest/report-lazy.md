# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.233 | 0.001 | 0.232 | 0.231 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.231 | 0.235 | 0.002 | 0.232 | 0.232 | 108.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.235 | 0.233 | 0.235 | 0.001 | 0.234 | 0.233 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.367 | 2.367 | 2.378 | 0.006 | 2.371 | 2.503 | 108.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.434 | 0.437 | 0.001 | 0.435 | 0.434 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.432 | 0.442 | 0.004 | 0.435 | 0.433 | 108.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.432 | 0.441 | 0.004 | 0.436 | 0.434 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.574 | 2.570 | 2.575 | 0.003 | 2.573 | 2.709 | 108.1 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.262 | 0.264 | 0.001 | 0.263 | 0.262 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.260 | 0.264 | 0.001 | 0.262 | 0.260 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.262 | 0.264 | 0.001 | 0.262 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.417 | 2.392 | 2.451 | 0.029 | 2.420 | 2.527 | 156.5 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.418 | 0.414 | 0.420 | 0.003 | 0.417 | 0.418 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.418 | 0.422 | 0.002 | 0.420 | 0.418 | 108.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.416 | 0.416 | 0.420 | 0.002 | 0.417 | 0.415 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.554 | 2.564 | 0.005 | 2.558 | 2.692 | 108.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.225 | 0.223 | 0.227 | 0.001 | 0.225 | 0.225 | 114.3 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.225 | 0.229 | 0.002 | 0.227 | 0.226 | 114.3 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.223 | 0.227 | 0.002 | 0.225 | 0.224 | 114.3 | 1.00× |
| lazy-ruby | 3 | 5.336 | 5.297 | 5.443 | 0.076 | 5.359 | 5.417 | 114.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.392 | 0.388 | 0.392 | 0.002 | 0.391 | 0.390 | 114.3 | 1.00× |
| lazy-ruby | 5 | 0.392 | 0.388 | 0.394 | 0.002 | 0.391 | 0.390 | 114.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.392 | 0.392 | 0.396 | 0.002 | 0.394 | 0.392 | 114.3 | 1.00× |
| lazy-ruby | 3 | 5.388 | 5.379 | 5.514 | 0.075 | 5.427 | 5.439 | 114.3 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.260 | 0.258 | 0.264 | 0.002 | 0.261 | 0.259 | 188.7 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.260 | 0.262 | 0.001 | 0.262 | 0.261 | 188.6 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.260 | 0.267 | 0.004 | 0.263 | 0.261 | 188.6 | 1.00× |
| lazy-ruby | 3 | 5.264 | 5.263 | 5.433 | 0.098 | 5.320 | 5.344 | 157.4 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.394 | 0.390 | 0.394 | 0.002 | 0.392 | 0.393 | 114.3 | 1.00× |
| lazy-ruby | 5 | 0.392 | 0.391 | 0.394 | 0.001 | 0.392 | 0.391 | 114.3 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.394 | 0.392 | 0.396 | 0.002 | 0.394 | 0.392 | 114.3 | 1.00× |
| lazy-ruby | 3 | 5.399 | 5.397 | 5.500 | 0.059 | 5.432 | 5.450 | 114.3 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.241 | 0.235 | 0.243 | 0.003 | 0.240 | 0.240 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.239 | 0.243 | 0.001 | 0.241 | 0.240 | 74.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.241 | 0.241 | 0.243 | 0.001 | 0.242 | 0.240 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.259 | 2.244 | 2.264 | 0.010 | 2.256 | 2.368 | 74.5 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.449 | 0.445 | 0.451 | 0.003 | 0.448 | 0.447 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.449 | 0.447 | 0.459 | 0.005 | 0.450 | 0.447 | 74.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.447 | 0.445 | 0.449 | 0.002 | 0.447 | 0.446 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.465 | 2.462 | 2.524 | 0.035 | 2.484 | 2.574 | 74.5 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.268 | 0.264 | 0.270 | 0.003 | 0.267 | 0.267 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.268 | 0.266 | 0.276 | 0.004 | 0.270 | 0.268 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.268 | 0.268 | 0.270 | 0.001 | 0.269 | 0.267 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.283 | 2.279 | 2.296 | 0.009 | 2.286 | 2.393 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.434 | 0.441 | 0.003 | 0.437 | 0.436 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.436 | 0.434 | 0.439 | 0.001 | 0.436 | 0.436 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.434 | 0.441 | 0.004 | 0.437 | 0.434 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.460 | 2.458 | 2.463 | 0.003 | 2.461 | 2.570 | 74.5 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.252 | 0.250 | 0.254 | 0.002 | 0.251 | 0.251 | 80.7 | 1.00× |
| lazy-ruby | 5 | 0.252 | 0.250 | 0.286 | 0.015 | 0.259 | 0.250 | 80.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.250 | 0.250 | 0.252 | 0.001 | 0.250 | 0.248 | 80.7 | 1.00× |
| lazy-ruby | 3 | 5.824 | 5.723 | 5.971 | 0.125 | 5.839 | 5.831 | 80.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.413 | 0.419 | 0.003 | 0.416 | 0.415 | 80.7 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.412 | 0.425 | 0.006 | 0.419 | 0.419 | 80.7 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.419 | 0.419 | 0.421 | 0.001 | 0.420 | 0.417 | 80.7 | 1.00× |
| lazy-ruby | 3 | 6.110 | 5.943 | 6.201 | 0.131 | 6.085 | 6.157 | 80.7 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.285 | 0.283 | 0.290 | 0.003 | 0.286 | 0.284 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.279 | 0.292 | 0.005 | 0.284 | 0.282 | 185.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.283 | 0.279 | 0.287 | 0.004 | 0.283 | 0.282 | 185.5 | 1.00× |
| lazy-ruby | 3 | 5.905 | 5.783 | 6.001 | 0.109 | 5.896 | 5.911 | 155.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.453 | 0.449 | 0.456 | 0.003 | 0.453 | 0.452 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.454 | 0.450 | 0.460 | 0.004 | 0.454 | 0.453 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.454 | 0.451 | 0.456 | 0.003 | 0.454 | 0.453 | 105.3 | 1.00× |
| lazy-ruby | 3 | 6.270 | 6.166 | 6.335 | 0.086 | 6.257 | 6.270 | 80.7 | 0.07× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.418 | 0.410 | 0.428 | 0.007 | 0.418 | 0.406 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.417 | 0.410 | 0.433 | 0.009 | 0.418 | 0.407 | 57.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.404 | 0.401 | 0.409 | 0.004 | 0.405 | 0.394 | 57.3 | 1.00× |
| lazy-ruby | 3 | 4.186 | 3.993 | 4.423 | 0.215 | 4.201 | 4.289 | 43.0 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.868 | 0.846 | 0.890 | 0.018 | 0.872 | 0.861 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.864 | 0.837 | 0.911 | 0.033 | 0.869 | 0.855 | 57.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.839 | 0.830 | 0.879 | 0.026 | 0.849 | 0.831 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.363 | 4.325 | 4.516 | 0.101 | 4.402 | 4.492 | 42.8 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.495 | 0.481 | 0.529 | 0.019 | 0.499 | 0.480 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.494 | 0.489 | 0.519 | 0.012 | 0.498 | 0.479 | 141.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.502 | 0.485 | 0.571 | 0.045 | 0.519 | 0.491 | 142.2 | 1.00× |
| lazy-ruby | 3 | 3.812 | 3.791 | 3.902 | 0.059 | 3.835 | 3.958 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.786 | 0.759 | 0.803 | 0.018 | 0.783 | 0.777 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.775 | 0.749 | 0.784 | 0.015 | 0.769 | 0.764 | 82.8 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.779 | 0.764 | 0.786 | 0.011 | 0.776 | 0.766 | 83.1 | 1.00× |
| lazy-ruby | 3 | 4.300 | 4.254 | 4.509 | 0.136 | 4.355 | 4.410 | 47.2 | 0.18× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.71

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.156 | 0.155 | 0.158 | 0.002 | 0.157 | 0.156 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.157 | 0.155 | 0.160 | 0.002 | 0.157 | 0.156 | 68.3 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.155 | 0.155 | 0.160 | 0.003 | 0.157 | 0.141 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.538 | 0.531 | 0.542 | 0.005 | 0.537 | 0.547 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.266 | 0.264 | 0.277 | 0.006 | 0.269 | 0.266 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.275 | 0.270 | 0.305 | 0.014 | 0.281 | 0.297 | 68.3 | 0.97× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.270 | 0.269 | 0.272 | 0.002 | 0.270 | 0.266 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.659 | 0.654 | 0.689 | 0.019 | 0.667 | 0.688 | 58.8 | 0.41× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.195 | 0.193 | 0.200 | 0.003 | 0.196 | 0.188 | 109.5 | 1.00× |
| lazy-ruby | 5 | 0.198 | 0.196 | 0.208 | 0.005 | 0.200 | 0.203 | 109.4 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.195 | 0.194 | 0.196 | 0.001 | 0.195 | 0.203 | 109.0 | 1.00× |
| lazy-ruby | 3 | 0.579 | 0.570 | 0.579 | 0.005 | 0.576 | 0.594 | 100.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.337 | 0.334 | 0.340 | 0.002 | 0.337 | 0.328 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.336 | 0.334 | 0.344 | 0.004 | 0.338 | 0.344 | 108.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.332 | 0.331 | 0.339 | 0.004 | 0.334 | 0.328 | 108.9 | 1.00× |
| lazy-ruby | 3 | 0.709 | 0.703 | 0.713 | 0.005 | 0.708 | 0.719 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

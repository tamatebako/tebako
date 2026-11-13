# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.231 | 0.236 | 0.002 | 0.232 | 0.231 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.223 | 0.240 | 0.007 | 0.231 | 0.232 | 70.5 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.232 | 0.231 | 0.232 | 0.000 | 0.231 | 0.230 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.359 | 2.355 | 2.362 | 0.004 | 2.359 | 2.493 | 48.9 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.429 | 0.454 | 0.010 | 0.436 | 0.431 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.433 | 0.429 | 0.450 | 0.008 | 0.435 | 0.431 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.435 | 0.433 | 0.435 | 0.001 | 0.435 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.554 | 2.562 | 0.004 | 2.557 | 2.689 | 49.5 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.259 | 0.258 | 0.263 | 0.002 | 0.260 | 0.259 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.259 | 0.261 | 0.001 | 0.260 | 0.259 | 180.3 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.261 | 0.261 | 0.000 | 0.261 | 0.259 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.395 | 2.385 | 2.397 | 0.007 | 2.392 | 2.530 | 158.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.414 | 0.419 | 0.002 | 0.416 | 0.414 | 102.4 | 1.00× |
| lazy-ruby | 5 | 0.413 | 0.410 | 0.415 | 0.002 | 0.413 | 0.413 | 102.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.416 | 0.415 | 0.417 | 0.001 | 0.416 | 0.416 | 102.1 | 1.00× |
| lazy-ruby | 3 | 2.536 | 2.535 | 2.555 | 0.012 | 2.542 | 2.671 | 62.3 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.234 | 0.234 | 0.237 | 0.002 | 0.235 | 0.233 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.234 | 0.240 | 0.003 | 0.236 | 0.234 | 79.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.234 | 0.234 | 0.000 | 0.234 | 0.233 | 79.8 | 1.00× |
| lazy-ruby | 3 | 5.594 | 5.566 | 5.642 | 0.038 | 5.601 | 5.624 | 51.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.419 | 0.417 | 0.426 | 0.004 | 0.420 | 0.418 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.417 | 0.427 | 0.004 | 0.421 | 0.418 | 80.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.419 | 0.417 | 0.420 | 0.002 | 0.419 | 0.417 | 79.8 | 1.00× |
| lazy-ruby | 3 | 5.761 | 5.749 | 5.831 | 0.044 | 5.781 | 5.820 | 51.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.274 | 0.267 | 0.274 | 0.003 | 0.272 | 0.272 | 188.9 | 1.00× |
| lazy-ruby | 5 | 0.274 | 0.269 | 0.284 | 0.006 | 0.275 | 0.272 | 188.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.274 | 0.271 | 0.274 | 0.002 | 0.273 | 0.272 | 188.5 | 1.00× |
| lazy-ruby | 3 | 5.606 | 5.579 | 5.630 | 0.025 | 5.605 | 5.637 | 157.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.411 | 0.409 | 0.413 | 0.001 | 0.411 | 0.410 | 112.0 | 1.00× |
| lazy-ruby | 5 | 0.411 | 0.409 | 0.414 | 0.002 | 0.411 | 0.411 | 111.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.415 | 0.411 | 0.417 | 0.003 | 0.415 | 0.414 | 112.5 | 1.00× |
| lazy-ruby | 3 | 5.761 | 5.747 | 5.797 | 0.026 | 5.769 | 5.791 | 64.8 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.239 | 0.236 | 0.240 | 0.002 | 0.239 | 0.238 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.233 | 0.240 | 0.003 | 0.237 | 0.237 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.238 | 0.240 | 0.001 | 0.239 | 0.237 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.245 | 2.241 | 2.247 | 0.003 | 2.244 | 2.356 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.448 | 0.448 | 0.450 | 0.001 | 0.449 | 0.447 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.448 | 0.446 | 0.450 | 0.002 | 0.447 | 0.446 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.448 | 0.446 | 0.450 | 0.002 | 0.448 | 0.446 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.463 | 2.446 | 2.477 | 0.016 | 2.462 | 2.574 | 47.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.263 | 0.267 | 0.002 | 0.265 | 0.264 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.263 | 0.267 | 0.002 | 0.265 | 0.264 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.265 | 0.265 | 0.000 | 0.265 | 0.264 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.271 | 2.270 | 2.273 | 0.002 | 2.271 | 2.382 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.432 | 0.436 | 0.002 | 0.434 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.431 | 0.436 | 0.002 | 0.433 | 0.433 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.433 | 0.432 | 0.434 | 0.001 | 0.433 | 0.432 | 101.9 | 1.00× |
| lazy-ruby | 3 | 2.440 | 2.439 | 2.450 | 0.006 | 2.443 | 2.550 | 62.7 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.254 | 0.252 | 0.267 | 0.007 | 0.258 | 0.254 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.252 | 0.244 | 0.274 | 0.011 | 0.255 | 0.250 | 75.0 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.250 | 0.248 | 0.258 | 0.006 | 0.252 | 0.249 | 75.2 | 1.00× |
| lazy-ruby | 3 | 5.957 | 5.862 | 6.071 | 0.104 | 5.963 | 5.954 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.384 | 0.379 | 0.391 | 0.005 | 0.384 | 0.381 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.380 | 0.375 | 0.384 | 0.004 | 0.379 | 0.378 | 75.0 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.385 | 0.377 | 0.388 | 0.006 | 0.383 | 0.384 | 75.3 | 1.00× |
| lazy-ruby | 3 | 6.019 | 6.014 | 6.184 | 0.097 | 6.072 | 6.020 | 49.7 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.286 | 0.273 | 0.307 | 0.012 | 0.287 | 0.284 | 185.8 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.279 | 0.290 | 0.004 | 0.283 | 0.282 | 186.0 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.292 | 0.286 | 0.296 | 0.005 | 0.291 | 0.291 | 185.7 | 1.00× |
| lazy-ruby | 3 | 5.948 | 5.919 | 5.952 | 0.018 | 5.939 | 5.967 | 156.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.456 | 0.447 | 0.471 | 0.009 | 0.456 | 0.454 | 105.2 | 1.00× |
| lazy-ruby | 5 | 0.456 | 0.453 | 0.477 | 0.012 | 0.463 | 0.455 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.460 | 0.456 | 0.468 | 0.006 | 0.462 | 0.459 | 105.5 | 1.00× |
| lazy-ruby | 3 | 5.940 | 5.926 | 6.053 | 0.070 | 5.973 | 5.940 | 63.1 | 0.08× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.309 | 0.285 | 0.342 | 0.027 | 0.313 | 0.301 | 62.3 | 1.00× |
| lazy-ruby | 5 | 0.324 | 0.283 | 0.397 | 0.044 | 0.333 | 0.314 | 63.6 | 0.95× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.238 | 0.287 | 0.025 | 0.259 | 0.232 | 62.1 | 1.00× |
| lazy-ruby | 3 | 1.142 | 0.992 | 1.394 | 0.204 | 1.176 | 1.237 | 57.8 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.463 | 0.407 | 0.980 | 0.239 | 0.560 | 0.404 | 64.2 | 1.00× |
| lazy-ruby | 5 | 0.464 | 0.400 | 0.486 | 0.036 | 0.447 | 0.428 | 62.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.398 | 0.419 | 0.011 | 0.411 | 0.399 | 61.8 | 1.00× |
| lazy-ruby | 3 | 1.179 | 1.139 | 1.222 | 0.041 | 1.180 | 1.252 | 49.9 | 0.35× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.259 | 0.243 | 0.282 | 0.016 | 0.262 | 0.238 | 164.0 | 1.00× |
| lazy-ruby | 5 | 0.276 | 0.246 | 0.295 | 0.020 | 0.270 | 0.253 | 166.1 | 0.94× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.234 | 0.292 | 0.029 | 0.262 | 0.244 | 163.9 | 1.00× |
| lazy-ruby | 3 | 1.056 | 1.035 | 1.093 | 0.029 | 1.061 | 1.129 | 152.9 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.567 | 0.468 | 0.593 | 0.058 | 0.541 | 0.545 | 92.3 | 1.00× |
| lazy-ruby | 5 | 0.612 | 0.505 | 0.627 | 0.055 | 0.581 | 0.565 | 95.7 | 0.93× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.628 | 0.619 | 0.649 | 0.015 | 0.632 | 0.610 | 97.6 | 1.00× |
| lazy-ruby | 3 | 1.518 | 1.462 | 1.679 | 0.112 | 1.553 | 1.608 | 70.6 | 0.41× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.341 | 0.659 | 0.121 | 0.466 | 0.412 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.426 | 0.352 | 0.687 | 0.131 | 0.474 | 0.413 | 57.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.453 | 0.335 | 0.499 | 0.085 | 0.429 | 0.430 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.288 | 2.950 | 4.392 | 0.755 | 3.543 | 3.437 | 42.9 | 0.14× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.697 | 0.659 | 0.974 | 0.158 | 0.787 | 0.682 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.878 | 0.667 | 0.951 | 0.132 | 0.815 | 0.842 | 57.4 | 0.79× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.895 | 0.887 | 1.650 | 0.438 | 1.144 | 0.880 | 57.5 | 1.00× |
| lazy-ruby | 3 | 4.467 | 4.208 | 4.741 | 0.267 | 4.472 | 4.589 | 42.8 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.557 | 0.433 | 0.684 | 0.092 | 0.551 | 0.550 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.557 | 0.466 | 0.759 | 0.114 | 0.575 | 0.522 | 141.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.460 | 0.446 | 0.485 | 0.020 | 0.464 | 0.449 | 141.9 | 1.00× |
| lazy-ruby | 3 | 3.359 | 3.341 | 3.897 | 0.316 | 3.532 | 3.497 | 126.9 | 0.14× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.941 | 0.739 | 1.426 | 0.334 | 1.045 | 0.920 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.846 | 0.681 | 1.057 | 0.156 | 0.863 | 0.821 | 83.1 | 1.11× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.905 | 0.852 | 0.977 | 0.063 | 0.911 | 0.890 | 83.0 | 1.00× |
| lazy-ruby | 3 | 4.025 | 3.650 | 4.221 | 0.290 | 3.966 | 4.144 | 47.0 | 0.22× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.79

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.232 | 0.240 | 0.003 | 0.236 | 0.234 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.232 | 0.237 | 0.002 | 0.235 | 0.234 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.236 | 0.242 | 0.004 | 0.238 | 0.219 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.806 | 0.803 | 0.808 | 0.003 | 0.806 | 0.828 | 58.7 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.453 | 0.450 | 0.456 | 0.002 | 0.454 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.454 | 0.451 | 0.468 | 0.007 | 0.456 | 0.453 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.461 | 0.454 | 0.466 | 0.006 | 0.460 | 0.453 | 68.5 | 1.00× |
| lazy-ruby | 3 | 1.034 | 1.019 | 1.058 | 0.020 | 1.037 | 1.062 | 58.8 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.303 | 0.292 | 0.315 | 0.010 | 0.304 | 0.312 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.296 | 0.287 | 0.318 | 0.012 | 0.298 | 0.297 | 109.3 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.290 | 0.289 | 0.293 | 0.002 | 0.291 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.857 | 0.852 | 0.863 | 0.005 | 0.858 | 0.906 | 100.1 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.503 | 0.498 | 0.511 | 0.005 | 0.504 | 0.500 | 108.7 | 1.00× |
| lazy-ruby | 5 | 0.504 | 0.498 | 0.508 | 0.004 | 0.503 | 0.500 | 108.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.504 | 0.503 | 0.508 | 0.002 | 0.505 | 0.500 | 109.2 | 1.00× |
| lazy-ruby | 3 | 1.077 | 1.076 | 1.079 | 0.002 | 1.078 | 1.094 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

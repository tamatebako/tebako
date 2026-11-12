# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.234 | 0.233 | 0.234 | 0.000 | 0.234 | 0.233 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.234 | 0.236 | 0.001 | 0.234 | 0.232 | 70.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.234 | 0.233 | 0.234 | 0.000 | 0.234 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.363 | 2.347 | 2.371 | 0.013 | 2.360 | 2.497 | 49.0 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.431 | 0.441 | 0.004 | 0.436 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.431 | 0.436 | 0.002 | 0.433 | 0.431 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.434 | 0.434 | 0.000 | 0.434 | 0.433 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.575 | 2.567 | 2.580 | 0.007 | 2.574 | 2.709 | 48.5 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.261 | 0.257 | 0.261 | 0.002 | 0.260 | 0.259 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.259 | 0.259 | 0.259 | 0.000 | 0.259 | 0.258 | 180.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.259 | 0.256 | 0.259 | 0.001 | 0.258 | 0.257 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.389 | 2.377 | 2.390 | 0.007 | 2.385 | 2.526 | 157.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.415 | 0.417 | 0.001 | 0.416 | 0.415 | 102.0 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.415 | 0.417 | 0.001 | 0.415 | 0.414 | 102.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.417 | 0.415 | 0.417 | 0.001 | 0.416 | 0.415 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.537 | 2.537 | 2.565 | 0.016 | 2.546 | 2.671 | 62.2 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.286 | 0.284 | 0.292 | 0.003 | 0.287 | 0.284 | 79.9 | 1.00× |
| lazy-ruby | 5 | 0.284 | 0.282 | 0.291 | 0.004 | 0.285 | 0.282 | 79.5 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.288 | 0.283 | 0.291 | 0.004 | 0.287 | 0.286 | 80.1 | 1.00× |
| lazy-ruby | 3 | 6.853 | 6.836 | 6.883 | 0.024 | 6.857 | 6.915 | 51.5 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.507 | 0.501 | 0.510 | 0.004 | 0.506 | 0.505 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.502 | 0.499 | 0.512 | 0.005 | 0.504 | 0.502 | 79.8 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.500 | 0.496 | 0.506 | 0.005 | 0.500 | 0.499 | 79.5 | 1.00× |
| lazy-ruby | 3 | 7.106 | 7.053 | 7.136 | 0.042 | 7.098 | 7.164 | 51.2 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.324 | 0.318 | 0.327 | 0.003 | 0.324 | 0.324 | 188.4 | 1.00× |
| lazy-ruby | 5 | 0.324 | 0.318 | 0.329 | 0.004 | 0.324 | 0.324 | 188.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.330 | 0.324 | 0.342 | 0.009 | 0.332 | 0.328 | 188.4 | 1.00× |
| lazy-ruby | 3 | 6.880 | 6.875 | 6.905 | 0.016 | 6.887 | 6.952 | 156.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.490 | 0.488 | 0.494 | 0.002 | 0.491 | 0.489 | 111.8 | 1.00× |
| lazy-ruby | 5 | 0.487 | 0.483 | 0.500 | 0.007 | 0.490 | 0.485 | 111.9 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.491 | 0.489 | 0.494 | 0.003 | 0.491 | 0.489 | 111.9 | 1.00× |
| lazy-ruby | 3 | 7.047 | 6.998 | 7.083 | 0.042 | 7.043 | 7.106 | 64.5 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.234 | 0.246 | 0.004 | 0.240 | 0.239 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.238 | 0.244 | 0.002 | 0.240 | 0.239 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.242 | 0.242 | 0.244 | 0.001 | 0.243 | 0.242 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.259 | 2.249 | 2.259 | 0.006 | 2.256 | 2.367 | 47.2 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.446 | 0.448 | 0.001 | 0.446 | 0.445 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.444 | 0.448 | 0.002 | 0.445 | 0.444 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.446 | 0.444 | 0.446 | 0.001 | 0.445 | 0.445 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.454 | 2.445 | 2.462 | 0.009 | 2.454 | 2.565 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.261 | 0.259 | 0.263 | 0.002 | 0.261 | 0.259 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.257 | 0.261 | 0.002 | 0.259 | 0.259 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.260 | 0.263 | 0.001 | 0.261 | 0.260 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.255 | 2.250 | 2.273 | 0.012 | 2.259 | 2.367 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.425 | 0.423 | 0.427 | 0.001 | 0.425 | 0.425 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.425 | 0.425 | 0.433 | 0.003 | 0.427 | 0.424 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.427 | 0.425 | 0.433 | 0.004 | 0.428 | 0.425 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.415 | 2.413 | 2.428 | 0.008 | 2.419 | 2.526 | 62.6 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.318 | 0.318 | 0.324 | 0.002 | 0.319 | 0.317 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.318 | 0.318 | 0.320 | 0.001 | 0.318 | 0.317 | 75.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.320 | 0.320 | 0.329 | 0.005 | 0.323 | 0.319 | 75.3 | 1.00× |
| lazy-ruby | 3 | 7.487 | 7.454 | 7.521 | 0.034 | 7.488 | 7.487 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.552 | 0.548 | 0.560 | 0.005 | 0.553 | 0.550 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.549 | 0.545 | 0.585 | 0.016 | 0.556 | 0.548 | 75.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.557 | 0.551 | 0.560 | 0.005 | 0.556 | 0.556 | 75.3 | 1.00× |
| lazy-ruby | 3 | 7.777 | 7.683 | 7.806 | 0.064 | 7.755 | 7.776 | 49.4 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.361 | 0.358 | 0.364 | 0.003 | 0.361 | 0.359 | 185.3 | 1.00× |
| lazy-ruby | 5 | 0.358 | 0.352 | 0.377 | 0.010 | 0.362 | 0.357 | 185.8 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.354 | 0.352 | 0.358 | 0.003 | 0.354 | 0.353 | 185.5 | 1.00× |
| lazy-ruby | 3 | 7.561 | 7.528 | 7.569 | 0.022 | 7.553 | 7.563 | 156.0 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.584 | 0.579 | 0.591 | 0.005 | 0.583 | 0.582 | 105.5 | 1.00× |
| lazy-ruby | 5 | 0.584 | 0.580 | 0.589 | 0.004 | 0.585 | 0.582 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.587 | 0.581 | 0.591 | 0.005 | 0.586 | 0.586 | 105.0 | 1.00× |
| lazy-ruby | 3 | 7.806 | 7.751 | 7.808 | 0.032 | 7.788 | 7.807 | 62.8 | 0.08× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.201 | 0.301 | 0.043 | 0.239 | 0.224 | 63.6 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.206 | 0.247 | 0.018 | 0.227 | 0.213 | 63.8 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.229 | 0.227 | 0.233 | 0.003 | 0.230 | 0.211 | 62.3 | 1.00× |
| lazy-ruby | 3 | 0.897 | 0.866 | 0.939 | 0.036 | 0.901 | 0.954 | 54.4 | 0.26× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.419 | 0.463 | 0.019 | 0.430 | 0.414 | 61.4 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.409 | 0.471 | 0.025 | 0.431 | 0.410 | 62.4 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.459 | 0.422 | 0.463 | 0.022 | 0.448 | 0.438 | 64.1 | 1.00× |
| lazy-ruby | 3 | 1.135 | 1.127 | 1.136 | 0.005 | 1.132 | 1.202 | 52.5 | 0.40× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.217 | 0.210 | 0.249 | 0.016 | 0.225 | 0.199 | 164.0 | 1.00× |
| lazy-ruby | 5 | 0.218 | 0.203 | 0.220 | 0.007 | 0.214 | 0.202 | 163.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.212 | 0.206 | 0.224 | 0.009 | 0.214 | 0.201 | 163.3 | 1.00× |
| lazy-ruby | 3 | 0.950 | 0.903 | 0.999 | 0.048 | 0.951 | 1.006 | 139.3 | 0.22× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.394 | 0.381 | 0.431 | 0.020 | 0.398 | 0.373 | 99.3 | 1.00× |
| lazy-ruby | 5 | 0.386 | 0.362 | 0.419 | 0.022 | 0.391 | 0.380 | 97.5 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.430 | 0.399 | 0.438 | 0.020 | 0.422 | 0.401 | 96.5 | 1.00× |
| lazy-ruby | 3 | 1.263 | 1.235 | 1.358 | 0.065 | 1.285 | 1.327 | 65.3 | 0.34× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.393 | 0.353 | 0.423 | 0.026 | 0.389 | 0.380 | 56.9 | 1.00× |
| lazy-ruby | 5 | 0.395 | 0.373 | 0.418 | 0.019 | 0.393 | 0.384 | 57.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.388 | 0.380 | 0.396 | 0.008 | 0.388 | 0.380 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.461 | 3.317 | 3.761 | 0.226 | 3.513 | 3.556 | 42.7 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.752 | 0.696 | 0.826 | 0.058 | 0.757 | 0.745 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.750 | 0.720 | 0.777 | 0.022 | 0.745 | 0.736 | 56.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.809 | 0.760 | 1.033 | 0.146 | 0.867 | 0.799 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.718 | 3.680 | 3.960 | 0.152 | 3.786 | 3.817 | 42.7 | 0.22× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.457 | 0.416 | 0.471 | 0.022 | 0.451 | 0.443 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.440 | 0.397 | 0.473 | 0.028 | 0.435 | 0.426 | 141.9 | 1.04× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.397 | 0.443 | 0.025 | 0.426 | 0.428 | 141.8 | 1.00× |
| lazy-ruby | 3 | 3.269 | 3.137 | 3.441 | 0.153 | 3.282 | 3.371 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.749 | 0.679 | 0.822 | 0.059 | 0.752 | 0.735 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.780 | 0.684 | 0.917 | 0.087 | 0.779 | 0.769 | 83.1 | 0.96× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.780 | 0.733 | 0.856 | 0.062 | 0.790 | 0.771 | 83.4 | 1.00× |
| lazy-ruby | 3 | 3.725 | 3.722 | 3.948 | 0.130 | 3.798 | 3.856 | 47.1 | 0.21× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.77

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.221 | 0.254 | 0.014 | 0.236 | 0.234 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.237 | 0.229 | 0.258 | 0.013 | 0.242 | 0.234 | 68.3 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.227 | 0.240 | 0.007 | 0.235 | 0.219 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.760 | 0.759 | 0.783 | 0.013 | 0.767 | 0.766 | 58.8 | 0.31× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.451 | 0.409 | 0.466 | 0.024 | 0.440 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.438 | 0.430 | 0.466 | 0.015 | 0.444 | 0.438 | 68.3 | 1.03× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.432 | 0.455 | 0.012 | 0.442 | 0.453 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.997 | 0.995 | 1.006 | 0.006 | 0.999 | 1.016 | 58.8 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.299 | 0.286 | 0.324 | 0.015 | 0.300 | 0.297 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.316 | 0.295 | 0.343 | 0.020 | 0.318 | 0.312 | 109.2 | 0.94× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.308 | 0.298 | 0.317 | 0.009 | 0.308 | 0.312 | 109.0 | 1.00× |
| lazy-ruby | 3 | 0.827 | 0.814 | 0.845 | 0.015 | 0.828 | 0.844 | 100.5 | 0.37× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.503 | 0.489 | 0.512 | 0.009 | 0.503 | 0.500 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.507 | 0.462 | 0.524 | 0.023 | 0.500 | 0.500 | 109.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.463 | 0.461 | 0.465 | 0.002 | 0.463 | 0.453 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.970 | 0.961 | 0.979 | 0.009 | 0.970 | 0.984 | 82.9 | 0.48× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

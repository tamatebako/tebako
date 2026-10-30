# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.226 | 0.231 | 0.002 | 0.229 | 0.228 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.226 | 0.229 | 0.001 | 0.228 | 0.228 | 108.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.229 | 0.229 | 0.229 | 0.000 | 0.229 | 0.228 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.347 | 2.346 | 2.363 | 0.010 | 2.352 | 2.480 | 108.4 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.428 | 0.432 | 0.002 | 0.431 | 0.429 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.430 | 0.428 | 0.432 | 0.002 | 0.431 | 0.429 | 108.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.430 | 0.447 | 0.009 | 0.437 | 0.433 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.571 | 2.552 | 2.584 | 0.016 | 2.569 | 2.706 | 108.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.253 | 0.258 | 0.002 | 0.255 | 0.254 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.253 | 0.257 | 0.002 | 0.255 | 0.255 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.255 | 0.253 | 0.255 | 0.001 | 0.255 | 0.253 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.381 | 2.357 | 2.386 | 0.016 | 2.375 | 2.517 | 158.3 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.407 | 0.405 | 0.414 | 0.003 | 0.408 | 0.406 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.408 | 0.405 | 0.410 | 0.001 | 0.407 | 0.406 | 108.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.405 | 0.408 | 0.001 | 0.407 | 0.405 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.527 | 2.523 | 2.530 | 0.003 | 2.527 | 2.662 | 108.4 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.286 | 0.293 | 0.003 | 0.289 | 0.288 | 114.4 | 1.00× |
| lazy-ruby | 5 | 0.285 | 0.282 | 0.292 | 0.005 | 0.286 | 0.284 | 114.4 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.288 | 0.284 | 0.290 | 0.003 | 0.287 | 0.287 | 114.4 | 1.00× |
| lazy-ruby | 3 | 6.856 | 6.831 | 6.928 | 0.050 | 6.872 | 6.915 | 114.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.508 | 0.505 | 0.539 | 0.014 | 0.515 | 0.506 | 114.4 | 1.00× |
| lazy-ruby | 5 | 0.508 | 0.502 | 0.522 | 0.009 | 0.511 | 0.508 | 114.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.506 | 0.505 | 0.508 | 0.002 | 0.506 | 0.504 | 114.4 | 1.00× |
| lazy-ruby | 3 | 7.094 | 7.093 | 7.096 | 0.002 | 7.094 | 7.152 | 114.4 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.324 | 0.321 | 0.329 | 0.003 | 0.325 | 0.323 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.324 | 0.320 | 0.329 | 0.004 | 0.325 | 0.323 | 188.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.328 | 0.324 | 0.336 | 0.006 | 0.329 | 0.327 | 188.7 | 1.00× |
| lazy-ruby | 3 | 6.912 | 6.864 | 6.920 | 0.031 | 6.899 | 6.980 | 158.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.495 | 0.489 | 0.499 | 0.004 | 0.495 | 0.494 | 114.4 | 1.00× |
| lazy-ruby | 5 | 0.493 | 0.491 | 0.502 | 0.004 | 0.495 | 0.493 | 114.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.497 | 0.489 | 0.498 | 0.005 | 0.495 | 0.497 | 114.4 | 1.00× |
| lazy-ruby | 3 | 7.013 | 7.012 | 7.047 | 0.020 | 7.024 | 7.104 | 114.4 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.241 | 0.239 | 0.241 | 0.001 | 0.240 | 0.239 | 74.7 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.239 | 0.241 | 0.001 | 0.240 | 0.239 | 74.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.241 | 0.239 | 0.241 | 0.001 | 0.240 | 0.240 | 74.7 | 1.00× |
| lazy-ruby | 3 | 2.240 | 2.230 | 2.252 | 0.011 | 2.240 | 2.350 | 74.7 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.447 | 0.447 | 0.449 | 0.001 | 0.448 | 0.446 | 74.7 | 1.00× |
| lazy-ruby | 5 | 0.449 | 0.447 | 0.449 | 0.001 | 0.448 | 0.447 | 74.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.449 | 0.447 | 0.449 | 0.001 | 0.448 | 0.447 | 74.7 | 1.00× |
| lazy-ruby | 3 | 2.469 | 2.466 | 2.489 | 0.013 | 2.475 | 2.577 | 74.7 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.264 | 0.264 | 0.266 | 0.001 | 0.264 | 0.263 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.264 | 0.264 | 0.268 | 0.002 | 0.264 | 0.263 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.262 | 0.264 | 0.001 | 0.262 | 0.261 | 182.5 | 1.00× |
| lazy-ruby | 3 | 2.268 | 2.262 | 2.283 | 0.011 | 2.271 | 2.380 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.432 | 0.430 | 0.434 | 0.002 | 0.433 | 0.432 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.428 | 0.434 | 0.003 | 0.433 | 0.432 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.432 | 0.432 | 0.434 | 0.001 | 0.433 | 0.431 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.452 | 2.434 | 2.452 | 0.010 | 2.446 | 2.561 | 74.7 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.319 | 0.317 | 0.320 | 0.001 | 0.318 | 0.317 | 80.1 | 1.00× |
| lazy-ruby | 5 | 0.319 | 0.315 | 0.321 | 0.002 | 0.318 | 0.318 | 80.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.319 | 0.319 | 0.319 | 0.000 | 0.319 | 0.317 | 80.1 | 1.00× |
| lazy-ruby | 3 | 7.481 | 7.474 | 7.490 | 0.008 | 7.481 | 7.490 | 80.1 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.549 | 0.548 | 0.551 | 0.001 | 0.549 | 0.547 | 80.1 | 1.00× |
| lazy-ruby | 5 | 0.547 | 0.546 | 0.549 | 0.001 | 0.547 | 0.546 | 80.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.552 | 0.552 | 0.557 | 0.003 | 0.554 | 0.551 | 80.1 | 1.00× |
| lazy-ruby | 3 | 7.692 | 7.676 | 7.744 | 0.036 | 7.704 | 7.693 | 80.1 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.364 | 0.357 | 0.371 | 0.005 | 0.364 | 0.363 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.361 | 0.355 | 0.363 | 0.003 | 0.359 | 0.360 | 185.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.364 | 0.361 | 0.367 | 0.003 | 0.364 | 0.360 | 185.9 | 1.00× |
| lazy-ruby | 3 | 7.547 | 7.425 | 7.557 | 0.073 | 7.509 | 7.539 | 155.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.569 | 0.563 | 0.573 | 0.004 | 0.568 | 0.567 | 105.1 | 1.00× |
| lazy-ruby | 5 | 0.569 | 0.567 | 0.573 | 0.003 | 0.569 | 0.567 | 105.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.568 | 0.565 | 0.575 | 0.005 | 0.569 | 0.567 | 104.8 | 1.00× |
| lazy-ruby | 3 | 7.710 | 7.692 | 7.726 | 0.017 | 7.709 | 7.709 | 80.1 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.225 | 0.211 | 0.258 | 0.020 | 0.233 | 0.204 | 65.0 | 1.00× |
| lazy-ruby | 5 | 0.221 | 0.219 | 0.252 | 0.014 | 0.230 | 0.210 | 65.0 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.209 | 0.268 | 0.030 | 0.242 | 0.228 | 65.2 | 1.00× |
| lazy-ruby | 3 | 1.081 | 1.037 | 1.172 | 0.069 | 1.097 | 1.119 | 56.4 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.443 | 0.427 | 0.502 | 0.029 | 0.451 | 0.426 | 64.6 | 1.00× |
| lazy-ruby | 5 | 0.436 | 0.425 | 0.467 | 0.018 | 0.444 | 0.429 | 64.0 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.457 | 0.436 | 0.465 | 0.015 | 0.453 | 0.427 | 65.1 | 1.00× |
| lazy-ruby | 3 | 1.200 | 1.184 | 1.246 | 0.032 | 1.210 | 1.255 | 56.1 | 0.38× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.254 | 0.239 | 0.351 | 0.051 | 0.285 | 0.243 | 166.1 | 1.00× |
| lazy-ruby | 5 | 0.248 | 0.241 | 0.256 | 0.006 | 0.249 | 0.238 | 165.4 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.235 | 0.228 | 0.236 | 0.004 | 0.233 | 0.223 | 167.4 | 1.00× |
| lazy-ruby | 3 | 0.950 | 0.935 | 0.951 | 0.009 | 0.945 | 1.006 | 151.3 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.414 | 0.495 | 0.032 | 0.442 | 0.425 | 92.9 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.407 | 0.482 | 0.030 | 0.441 | 0.409 | 103.1 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.450 | 0.450 | 0.456 | 0.004 | 0.452 | 0.427 | 98.7 | 1.00× |
| lazy-ruby | 3 | 1.149 | 1.111 | 1.170 | 0.030 | 1.143 | 1.201 | 74.6 | 0.39× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.448 | 0.437 | 0.454 | 0.006 | 0.447 | 0.436 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.452 | 0.443 | 0.486 | 0.018 | 0.462 | 0.443 | 57.1 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.490 | 0.426 | 0.507 | 0.043 | 0.475 | 0.480 | 57.0 | 1.00× |
| lazy-ruby | 3 | 4.927 | 4.137 | 5.282 | 0.586 | 4.782 | 5.086 | 42.7 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.989 | 0.913 | 1.171 | 0.106 | 1.009 | 0.975 | 57.1 | 1.00× |
| lazy-ruby | 5 | 1.073 | 0.908 | 1.138 | 0.096 | 1.044 | 0.971 | 57.3 | 0.92× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.026 | 1.017 | 1.037 | 0.010 | 1.027 | 1.013 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.006 | 5.002 | 5.291 | 0.166 | 5.099 | 5.166 | 42.9 | 0.21× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.590 | 0.555 | 0.651 | 0.035 | 0.594 | 0.577 | 142.0 | 1.00× |
| lazy-ruby | 5 | 0.589 | 0.562 | 0.637 | 0.028 | 0.597 | 0.573 | 141.9 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.599 | 0.570 | 0.600 | 0.017 | 0.590 | 0.582 | 141.7 | 1.00× |
| lazy-ruby | 3 | 4.531 | 4.389 | 4.540 | 0.085 | 4.487 | 4.691 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.936 | 0.899 | 0.997 | 0.035 | 0.942 | 0.922 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.942 | 0.935 | 1.030 | 0.040 | 0.964 | 0.925 | 83.0 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.936 | 0.920 | 1.144 | 0.125 | 1.000 | 0.919 | 83.2 | 1.00× |
| lazy-ruby | 3 | 5.026 | 4.857 | 6.095 | 0.671 | 5.326 | 5.166 | 46.9 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.231 | 0.242 | 0.004 | 0.235 | 0.234 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.235 | 0.239 | 0.002 | 0.236 | 0.219 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.235 | 0.240 | 0.003 | 0.237 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.803 | 0.803 | 0.803 | 0.000 | 0.803 | 0.828 | 58.8 | 0.30× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.456 | 0.455 | 0.459 | 0.002 | 0.457 | 0.453 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.459 | 0.453 | 0.460 | 0.003 | 0.457 | 0.438 | 68.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.457 | 0.456 | 0.465 | 0.005 | 0.459 | 0.469 | 68.5 | 1.00× |
| lazy-ruby | 3 | 1.025 | 1.023 | 1.036 | 0.007 | 1.028 | 1.047 | 58.7 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.284 | 0.282 | 0.292 | 0.004 | 0.285 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.287 | 0.292 | 0.002 | 0.288 | 0.281 | 108.9 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.289 | 0.286 | 0.293 | 0.004 | 0.289 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.859 | 0.858 | 0.862 | 0.002 | 0.860 | 0.891 | 100.6 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.507 | 0.503 | 0.510 | 0.003 | 0.507 | 0.500 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.509 | 0.501 | 0.514 | 0.005 | 0.508 | 0.500 | 108.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.507 | 0.502 | 0.510 | 0.004 | 0.507 | 0.500 | 109.2 | 1.00× |
| lazy-ruby | 3 | 1.074 | 1.069 | 1.075 | 0.004 | 1.073 | 1.109 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

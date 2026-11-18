# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.236 | 0.233 | 0.238 | 0.002 | 0.236 | 0.234 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.233 | 0.236 | 0.001 | 0.235 | 0.234 | 70.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.229 | 0.233 | 0.002 | 0.231 | 0.230 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.365 | 2.364 | 2.366 | 0.001 | 2.365 | 2.501 | 49.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.431 | 0.438 | 0.003 | 0.435 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.437 | 0.435 | 0.437 | 0.001 | 0.436 | 0.436 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.435 | 0.433 | 0.436 | 0.001 | 0.435 | 0.435 | 70.7 | 1.00× |
| lazy-ruby | 3 | 2.572 | 2.559 | 2.579 | 0.010 | 2.570 | 2.707 | 48.8 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.261 | 0.269 | 0.003 | 0.265 | 0.263 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.260 | 0.269 | 0.003 | 0.264 | 0.265 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.269 | 0.265 | 0.269 | 0.002 | 0.267 | 0.267 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.397 | 2.390 | 2.411 | 0.011 | 2.399 | 2.535 | 156.8 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.415 | 0.427 | 0.005 | 0.419 | 0.416 | 102.3 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.415 | 0.425 | 0.004 | 0.420 | 0.419 | 102.2 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.417 | 0.416 | 0.419 | 0.001 | 0.417 | 0.415 | 102.4 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.554 | 2.589 | 0.020 | 2.567 | 2.690 | 62.2 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.184 | 0.183 | 0.185 | 0.001 | 0.184 | 0.182 | 79.6 | 1.00× |
| lazy-ruby | 5 | 0.183 | 0.182 | 0.185 | 0.001 | 0.184 | 0.183 | 80.0 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.183 | 0.181 | 0.184 | 0.002 | 0.182 | 0.181 | 79.7 | 1.00× |
| lazy-ruby | 3 | 4.445 | 4.422 | 4.466 | 0.022 | 4.445 | 4.482 | 51.5 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.335 | 0.334 | 0.346 | 0.005 | 0.337 | 0.334 | 79.9 | 1.00× |
| lazy-ruby | 5 | 0.336 | 0.335 | 0.337 | 0.001 | 0.336 | 0.335 | 79.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.335 | 0.334 | 0.336 | 0.001 | 0.335 | 0.334 | 79.9 | 1.00× |
| lazy-ruby | 3 | 4.616 | 4.609 | 4.633 | 0.013 | 4.619 | 4.636 | 51.5 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.213 | 0.210 | 0.219 | 0.003 | 0.213 | 0.212 | 188.6 | 1.00× |
| lazy-ruby | 5 | 0.212 | 0.211 | 0.214 | 0.001 | 0.213 | 0.212 | 188.6 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.211 | 0.211 | 0.212 | 0.000 | 0.212 | 0.210 | 188.6 | 1.00× |
| lazy-ruby | 3 | 4.469 | 4.467 | 4.475 | 0.004 | 4.470 | 4.491 | 157.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.312 | 0.311 | 0.313 | 0.001 | 0.312 | 0.311 | 112.2 | 1.00× |
| lazy-ruby | 5 | 0.312 | 0.310 | 0.312 | 0.001 | 0.312 | 0.311 | 111.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.310 | 0.309 | 0.310 | 0.000 | 0.310 | 0.309 | 111.9 | 1.00× |
| lazy-ruby | 3 | 4.571 | 4.568 | 4.580 | 0.006 | 4.573 | 4.592 | 64.9 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.240 | 0.240 | 0.242 | 0.001 | 0.240 | 0.239 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.240 | 0.242 | 0.001 | 0.241 | 0.239 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.240 | 0.238 | 0.242 | 0.002 | 0.240 | 0.239 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.253 | 2.248 | 2.508 | 0.149 | 2.336 | 2.364 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.450 | 0.448 | 0.462 | 0.006 | 0.451 | 0.448 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.450 | 0.445 | 0.452 | 0.003 | 0.449 | 0.448 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.450 | 0.448 | 0.454 | 0.003 | 0.451 | 0.448 | 72.2 | 1.00× |
| lazy-ruby | 3 | 2.459 | 2.458 | 2.470 | 0.007 | 2.462 | 2.569 | 47.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.263 | 0.269 | 0.003 | 0.265 | 0.263 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.263 | 0.267 | 0.002 | 0.264 | 0.263 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.263 | 0.263 | 0.265 | 0.001 | 0.263 | 0.262 | 182.5 | 1.00× |
| lazy-ruby | 3 | 2.283 | 2.271 | 2.286 | 0.008 | 2.280 | 2.395 | 156.5 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.429 | 0.429 | 0.431 | 0.001 | 0.430 | 0.429 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.429 | 0.435 | 0.002 | 0.432 | 0.429 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.433 | 0.437 | 0.002 | 0.435 | 0.432 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.449 | 2.449 | 2.455 | 0.003 | 2.451 | 2.560 | 62.4 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.318 | 0.316 | 0.321 | 0.002 | 0.318 | 0.315 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.318 | 0.316 | 0.320 | 0.002 | 0.318 | 0.317 | 75.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.318 | 0.318 | 0.321 | 0.002 | 0.319 | 0.317 | 75.3 | 1.00× |
| lazy-ruby | 3 | 7.470 | 7.464 | 7.528 | 0.036 | 7.487 | 7.465 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.547 | 0.545 | 0.552 | 0.003 | 0.548 | 0.547 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.545 | 0.543 | 0.568 | 0.010 | 0.549 | 0.544 | 75.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.547 | 0.540 | 0.547 | 0.004 | 0.545 | 0.545 | 75.1 | 1.00× |
| lazy-ruby | 3 | 7.667 | 7.631 | 7.743 | 0.057 | 7.680 | 7.720 | 49.7 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.354 | 0.351 | 0.356 | 0.002 | 0.354 | 0.352 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.354 | 0.349 | 0.354 | 0.002 | 0.353 | 0.352 | 185.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.354 | 0.354 | 0.356 | 0.001 | 0.354 | 0.353 | 185.4 | 1.00× |
| lazy-ruby | 3 | 7.491 | 7.479 | 7.535 | 0.030 | 7.502 | 7.493 | 155.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.568 | 0.564 | 0.571 | 0.003 | 0.568 | 0.566 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.566 | 0.565 | 0.568 | 0.001 | 0.566 | 0.565 | 105.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.570 | 0.562 | 0.573 | 0.006 | 0.568 | 0.570 | 105.6 | 1.00× |
| lazy-ruby | 3 | 7.721 | 7.691 | 7.735 | 0.022 | 7.716 | 7.721 | 62.7 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.216 | 0.213 | 0.251 | 0.018 | 0.228 | 0.209 | 62.0 | 1.00× |
| lazy-ruby | 5 | 0.250 | 0.206 | 0.252 | 0.024 | 0.234 | 0.222 | 63.2 | 0.86× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.216 | 0.202 | 0.231 | 0.014 | 0.216 | 0.194 | 62.9 | 1.00× |
| lazy-ruby | 3 | 0.948 | 0.916 | 0.958 | 0.022 | 0.941 | 1.008 | 49.3 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.416 | 0.430 | 0.006 | 0.422 | 0.402 | 62.0 | 1.00× |
| lazy-ruby | 5 | 0.418 | 0.406 | 0.432 | 0.009 | 0.418 | 0.399 | 61.6 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.431 | 0.428 | 0.476 | 0.027 | 0.445 | 0.422 | 62.5 | 1.00× |
| lazy-ruby | 3 | 1.161 | 1.152 | 1.170 | 0.009 | 1.161 | 1.223 | 52.2 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.242 | 0.232 | 0.263 | 0.011 | 0.244 | 0.219 | 163.6 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.225 | 0.239 | 0.006 | 0.234 | 0.217 | 165.1 | 1.03× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.225 | 0.235 | 0.005 | 0.230 | 0.213 | 163.0 | 1.00× |
| lazy-ruby | 3 | 0.965 | 0.960 | 0.996 | 0.019 | 0.974 | 1.018 | 142.0 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.424 | 0.395 | 0.450 | 0.022 | 0.425 | 0.402 | 96.5 | 1.00× |
| lazy-ruby | 5 | 0.411 | 0.403 | 0.423 | 0.009 | 0.412 | 0.387 | 90.5 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.397 | 0.394 | 0.398 | 0.002 | 0.396 | 0.380 | 92.7 | 1.00× |
| lazy-ruby | 3 | 1.135 | 1.099 | 1.196 | 0.049 | 1.144 | 1.208 | 64.3 | 0.35× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.388 | 0.377 | 0.390 | 0.006 | 0.385 | 0.374 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.391 | 0.371 | 0.418 | 0.019 | 0.392 | 0.379 | 57.2 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.369 | 0.366 | 0.369 | 0.002 | 0.368 | 0.356 | 57.2 | 1.00× |
| lazy-ruby | 3 | 3.811 | 3.499 | 4.039 | 0.271 | 3.783 | 3.822 | 42.9 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.797 | 0.716 | 0.885 | 0.070 | 0.797 | 0.789 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.841 | 0.728 | 0.863 | 0.068 | 0.802 | 0.831 | 57.1 | 0.95× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.851 | 0.834 | 0.878 | 0.022 | 0.854 | 0.843 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.938 | 3.892 | 4.347 | 0.250 | 4.059 | 4.041 | 42.7 | 0.22× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.452 | 0.411 | 0.498 | 0.032 | 0.453 | 0.443 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.459 | 0.412 | 0.587 | 0.068 | 0.472 | 0.438 | 141.8 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.411 | 0.423 | 0.006 | 0.417 | 0.407 | 141.8 | 1.00× |
| lazy-ruby | 3 | 3.968 | 3.523 | 4.333 | 0.406 | 3.941 | 4.131 | 127.4 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.850 | 0.801 | 1.059 | 0.104 | 0.885 | 0.840 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.823 | 0.771 | 0.891 | 0.048 | 0.821 | 0.810 | 82.8 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.797 | 0.796 | 0.823 | 0.015 | 0.805 | 0.786 | 83.3 | 1.00× |
| lazy-ruby | 3 | 4.011 | 3.925 | 4.123 | 0.099 | 4.020 | 4.143 | 47.0 | 0.20× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.83

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.184 | 0.183 | 0.188 | 0.002 | 0.185 | 0.188 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.185 | 0.184 | 0.186 | 0.001 | 0.185 | 0.188 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.185 | 0.185 | 0.191 | 0.003 | 0.187 | 0.172 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.654 | 0.644 | 0.656 | 0.006 | 0.652 | 0.688 | 58.8 | 0.28× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.340 | 0.338 | 0.344 | 0.003 | 0.340 | 0.328 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.344 | 0.339 | 0.346 | 0.003 | 0.344 | 0.344 | 68.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.345 | 0.343 | 0.347 | 0.002 | 0.345 | 0.344 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.806 | 0.804 | 0.817 | 0.007 | 0.809 | 0.828 | 58.8 | 0.43× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.231 | 0.239 | 0.004 | 0.233 | 0.234 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.232 | 0.230 | 0.234 | 0.001 | 0.232 | 0.234 | 109.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.232 | 0.231 | 0.232 | 0.001 | 0.232 | 0.219 | 109.6 | 1.00× |
| lazy-ruby | 3 | 0.701 | 0.686 | 0.707 | 0.010 | 0.698 | 0.719 | 100.6 | 0.33× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.399 | 0.396 | 0.400 | 0.002 | 0.398 | 0.391 | 108.8 | 1.00× |
| lazy-ruby | 5 | 0.398 | 0.394 | 0.400 | 0.002 | 0.398 | 0.406 | 108.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.402 | 0.398 | 0.408 | 0.005 | 0.402 | 0.406 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.861 | 0.859 | 0.862 | 0.002 | 0.861 | 0.906 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

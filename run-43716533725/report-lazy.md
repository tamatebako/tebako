# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.233 | 0.235 | 0.001 | 0.234 | 0.233 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.233 | 0.236 | 0.001 | 0.235 | 0.234 | 70.5 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.231 | 0.234 | 0.001 | 0.233 | 0.232 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.366 | 2.362 | 2.369 | 0.003 | 2.366 | 2.501 | 48.5 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.433 | 0.436 | 0.001 | 0.434 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.435 | 0.433 | 0.456 | 0.010 | 0.438 | 0.434 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.435 | 0.433 | 0.435 | 0.001 | 0.434 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.566 | 2.560 | 2.572 | 0.006 | 2.566 | 2.702 | 49.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.260 | 0.263 | 0.001 | 0.262 | 0.261 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.259 | 0.263 | 0.002 | 0.262 | 0.261 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.259 | 0.261 | 0.001 | 0.260 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.398 | 2.391 | 2.413 | 0.011 | 2.401 | 2.527 | 157.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.416 | 0.412 | 0.417 | 0.002 | 0.415 | 0.414 | 102.0 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.414 | 0.417 | 0.001 | 0.416 | 0.414 | 102.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.412 | 0.414 | 0.001 | 0.414 | 0.414 | 102.2 | 1.00× |
| lazy-ruby | 3 | 2.553 | 2.535 | 2.555 | 0.011 | 2.548 | 2.687 | 62.6 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.182 | 0.180 | 0.186 | 0.003 | 0.183 | 0.181 | 79.5 | 1.00× |
| lazy-ruby | 5 | 0.181 | 0.180 | 0.182 | 0.001 | 0.181 | 0.180 | 79.9 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.181 | 0.180 | 0.182 | 0.001 | 0.181 | 0.180 | 79.6 | 1.00× |
| lazy-ruby | 3 | 4.431 | 4.422 | 4.453 | 0.016 | 4.436 | 4.460 | 51.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.337 | 0.335 | 0.354 | 0.008 | 0.340 | 0.335 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.335 | 0.333 | 0.337 | 0.001 | 0.335 | 0.333 | 79.7 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.334 | 0.334 | 0.334 | 0.000 | 0.334 | 0.333 | 80.1 | 1.00× |
| lazy-ruby | 3 | 4.579 | 4.577 | 4.618 | 0.023 | 4.591 | 4.601 | 51.3 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.210 | 0.206 | 0.211 | 0.002 | 0.210 | 0.209 | 188.9 | 1.00× |
| lazy-ruby | 5 | 0.209 | 0.209 | 0.211 | 0.001 | 0.210 | 0.208 | 188.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.210 | 0.209 | 0.213 | 0.002 | 0.211 | 0.209 | 189.2 | 1.00× |
| lazy-ruby | 3 | 4.461 | 4.458 | 4.474 | 0.009 | 4.464 | 4.483 | 157.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.310 | 0.308 | 0.312 | 0.002 | 0.310 | 0.308 | 111.6 | 1.00× |
| lazy-ruby | 5 | 0.311 | 0.310 | 0.311 | 0.001 | 0.311 | 0.309 | 112.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.308 | 0.308 | 0.310 | 0.001 | 0.309 | 0.307 | 112.4 | 1.00× |
| lazy-ruby | 3 | 4.558 | 4.554 | 4.559 | 0.003 | 4.557 | 4.580 | 64.9 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.233 | 0.238 | 0.002 | 0.236 | 0.235 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.236 | 0.235 | 0.237 | 0.001 | 0.236 | 0.235 | 72.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.236 | 0.236 | 0.238 | 0.001 | 0.236 | 0.235 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.266 | 2.265 | 2.327 | 0.035 | 2.286 | 2.375 | 47.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.444 | 0.439 | 0.457 | 0.007 | 0.446 | 0.444 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.444 | 0.441 | 0.458 | 0.007 | 0.446 | 0.442 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.444 | 0.442 | 0.446 | 0.002 | 0.444 | 0.442 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.458 | 2.455 | 2.506 | 0.029 | 2.473 | 2.566 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.262 | 0.264 | 0.001 | 0.263 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.263 | 0.262 | 0.268 | 0.003 | 0.264 | 0.263 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.264 | 0.263 | 0.264 | 0.001 | 0.264 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.284 | 2.256 | 2.309 | 0.026 | 2.283 | 2.387 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.429 | 0.425 | 0.433 | 0.003 | 0.429 | 0.428 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.423 | 0.436 | 0.005 | 0.431 | 0.430 | 101.8 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.426 | 0.423 | 0.427 | 0.002 | 0.425 | 0.424 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.454 | 2.425 | 2.490 | 0.032 | 2.456 | 2.567 | 62.8 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.321 | 0.316 | 0.326 | 0.004 | 0.321 | 0.320 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.320 | 0.318 | 0.327 | 0.004 | 0.321 | 0.318 | 75.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.322 | 0.320 | 0.324 | 0.002 | 0.322 | 0.321 | 75.0 | 1.00× |
| lazy-ruby | 3 | 7.523 | 7.502 | 7.533 | 0.016 | 7.519 | 7.519 | 49.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.551 | 0.548 | 0.556 | 0.004 | 0.551 | 0.549 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.549 | 0.545 | 0.553 | 0.003 | 0.550 | 0.547 | 75.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.553 | 0.551 | 0.558 | 0.004 | 0.554 | 0.552 | 75.0 | 1.00× |
| lazy-ruby | 3 | 7.763 | 7.711 | 7.769 | 0.032 | 7.748 | 7.758 | 49.7 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.358 | 0.356 | 0.362 | 0.002 | 0.358 | 0.356 | 185.7 | 1.00× |
| lazy-ruby | 5 | 0.364 | 0.354 | 0.387 | 0.012 | 0.366 | 0.364 | 185.3 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.356 | 0.354 | 0.358 | 0.002 | 0.356 | 0.355 | 185.5 | 1.00× |
| lazy-ruby | 3 | 7.556 | 7.542 | 7.588 | 0.023 | 7.562 | 7.559 | 155.7 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.575 | 0.569 | 0.585 | 0.007 | 0.575 | 0.574 | 105.2 | 1.00× |
| lazy-ruby | 5 | 0.577 | 0.575 | 0.583 | 0.003 | 0.578 | 0.575 | 105.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.570 | 0.568 | 0.572 | 0.002 | 0.570 | 0.568 | 105.1 | 1.00× |
| lazy-ruby | 3 | 7.753 | 7.734 | 7.761 | 0.014 | 7.749 | 7.753 | 62.8 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.206 | 0.187 | 0.226 | 0.014 | 0.205 | 0.197 | 62.4 | 1.00× |
| lazy-ruby | 5 | 0.203 | 0.196 | 0.205 | 0.004 | 0.201 | 0.184 | 61.9 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.194 | 0.200 | 0.003 | 0.197 | 0.180 | 61.4 | 1.00× |
| lazy-ruby | 3 | 0.816 | 0.811 | 0.978 | 0.095 | 0.868 | 0.890 | 54.6 | 0.24× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.406 | 0.395 | 0.438 | 0.018 | 0.414 | 0.399 | 61.8 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.389 | 0.435 | 0.017 | 0.415 | 0.396 | 62.1 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.389 | 0.379 | 0.394 | 0.008 | 0.387 | 0.380 | 61.0 | 1.00× |
| lazy-ruby | 3 | 1.043 | 1.035 | 1.122 | 0.048 | 1.067 | 1.079 | 52.8 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.203 | 0.196 | 0.209 | 0.005 | 0.204 | 0.186 | 163.9 | 1.00× |
| lazy-ruby | 5 | 0.203 | 0.195 | 0.216 | 0.008 | 0.204 | 0.184 | 163.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.208 | 0.196 | 0.219 | 0.011 | 0.207 | 0.188 | 163.5 | 1.00× |
| lazy-ruby | 3 | 0.815 | 0.811 | 0.843 | 0.017 | 0.823 | 0.878 | 150.0 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.359 | 0.345 | 0.373 | 0.012 | 0.357 | 0.339 | 94.5 | 1.00× |
| lazy-ruby | 5 | 0.357 | 0.341 | 0.387 | 0.017 | 0.359 | 0.338 | 90.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.360 | 0.352 | 0.363 | 0.006 | 0.358 | 0.339 | 92.6 | 1.00× |
| lazy-ruby | 3 | 1.006 | 0.983 | 1.021 | 0.019 | 1.004 | 1.066 | 66.6 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.330 | 0.318 | 0.367 | 0.019 | 0.335 | 0.317 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.327 | 0.312 | 0.347 | 0.014 | 0.330 | 0.321 | 57.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.329 | 0.327 | 0.381 | 0.031 | 0.346 | 0.323 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.156 | 3.119 | 3.252 | 0.069 | 3.176 | 3.231 | 43.4 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.668 | 0.660 | 0.678 | 0.006 | 0.668 | 0.658 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.681 | 0.650 | 0.701 | 0.019 | 0.680 | 0.667 | 57.5 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.681 | 0.676 | 0.751 | 0.042 | 0.703 | 0.674 | 57.0 | 1.00× |
| lazy-ruby | 3 | 4.630 | 4.397 | 4.958 | 0.282 | 4.661 | 4.791 | 42.9 | 0.15× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.410 | 0.452 | 0.017 | 0.431 | 0.425 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.468 | 0.420 | 0.480 | 0.028 | 0.455 | 0.461 | 141.7 | 0.93× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.551 | 0.492 | 0.575 | 0.042 | 0.539 | 0.530 | 141.8 | 1.00× |
| lazy-ruby | 3 | 3.807 | 3.174 | 4.312 | 0.571 | 3.764 | 3.901 | 126.9 | 0.14× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.651 | 0.640 | 0.730 | 0.036 | 0.668 | 0.641 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.657 | 0.629 | 0.661 | 0.013 | 0.652 | 0.650 | 83.2 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.653 | 0.642 | 0.657 | 0.008 | 0.651 | 0.644 | 82.7 | 1.00× |
| lazy-ruby | 3 | 3.524 | 3.342 | 3.542 | 0.111 | 3.470 | 3.612 | 46.8 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.81

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.285 | 0.265 | 0.341 | 0.029 | 0.291 | 0.281 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.269 | 0.261 | 0.295 | 0.013 | 0.274 | 0.297 | 68.3 | 1.06× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.259 | 0.276 | 0.009 | 0.266 | 0.266 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.905 | 0.902 | 0.942 | 0.023 | 0.916 | 0.953 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.514 | 0.498 | 0.522 | 0.012 | 0.511 | 0.516 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.498 | 0.483 | 0.521 | 0.015 | 0.500 | 0.484 | 68.3 | 1.03× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.487 | 0.472 | 0.490 | 0.010 | 0.483 | 0.469 | 68.4 | 1.00× |
| lazy-ruby | 3 | 1.129 | 1.127 | 1.184 | 0.032 | 1.147 | 1.172 | 58.8 | 0.43× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.322 | 0.309 | 0.348 | 0.016 | 0.324 | 0.328 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.314 | 0.308 | 0.334 | 0.011 | 0.320 | 0.312 | 109.3 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.326 | 0.319 | 0.330 | 0.006 | 0.325 | 0.312 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.955 | 0.925 | 0.971 | 0.023 | 0.950 | 1.000 | 100.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.565 | 0.553 | 0.578 | 0.010 | 0.565 | 0.562 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.549 | 0.540 | 0.569 | 0.012 | 0.553 | 0.547 | 108.9 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.555 | 0.552 | 0.559 | 0.003 | 0.555 | 0.547 | 109.3 | 1.00× |
| lazy-ruby | 3 | 1.161 | 1.158 | 1.182 | 0.013 | 1.167 | 1.188 | 82.9 | 0.48× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

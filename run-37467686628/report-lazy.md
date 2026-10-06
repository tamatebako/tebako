# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.220 | 0.220 | 0.221 | 0.000 | 0.220 | 0.219 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.220 | 0.218 | 0.221 | 0.001 | 0.220 | 0.219 | 108.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.220 | 0.218 | 0.220 | 0.001 | 0.220 | 0.218 | 108.9 | 1.00× |
| lazy-ruby | 3 | 2.337 | 2.325 | 2.362 | 0.019 | 2.341 | 2.469 | 108.9 | 0.09× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.420 | 0.422 | 0.001 | 0.422 | 0.420 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.422 | 0.449 | 0.012 | 0.428 | 0.422 | 108.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.424 | 0.424 | 0.432 | 0.005 | 0.427 | 0.422 | 108.9 | 1.00× |
| lazy-ruby | 3 | 2.541 | 2.536 | 2.547 | 0.006 | 2.541 | 2.672 | 108.9 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.247 | 0.247 | 0.249 | 0.001 | 0.248 | 0.246 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.247 | 0.245 | 0.249 | 0.001 | 0.247 | 0.246 | 180.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.245 | 0.245 | 0.247 | 0.001 | 0.246 | 0.245 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.347 | 2.345 | 2.391 | 0.026 | 2.361 | 2.481 | 156.9 | 0.10× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.401 | 0.395 | 0.402 | 0.003 | 0.399 | 0.400 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.399 | 0.395 | 0.401 | 0.002 | 0.399 | 0.397 | 108.9 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.401 | 0.399 | 0.401 | 0.001 | 0.401 | 0.399 | 108.9 | 1.00× |
| lazy-ruby | 3 | 2.531 | 2.508 | 2.539 | 0.016 | 2.526 | 2.639 | 108.9 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.259 | 0.256 | 0.261 | 0.002 | 0.259 | 0.257 | 115.3 | 1.00× |
| lazy-ruby | 5 | 0.258 | 0.255 | 0.261 | 0.002 | 0.258 | 0.256 | 115.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.259 | 0.258 | 0.261 | 0.001 | 0.260 | 0.258 | 115.3 | 1.00× |
| lazy-ruby | 3 | 6.835 | 6.829 | 6.922 | 0.052 | 6.862 | 6.902 | 115.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.472 | 0.468 | 0.475 | 0.003 | 0.472 | 0.471 | 115.3 | 1.00× |
| lazy-ruby | 5 | 0.470 | 0.463 | 0.474 | 0.004 | 0.470 | 0.469 | 115.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.478 | 0.473 | 0.513 | 0.022 | 0.488 | 0.476 | 115.3 | 1.00× |
| lazy-ruby | 3 | 7.102 | 6.940 | 7.310 | 0.185 | 7.117 | 7.201 | 115.3 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.299 | 0.297 | 0.304 | 0.003 | 0.300 | 0.297 | 195.2 | 1.00× |
| lazy-ruby | 5 | 0.300 | 0.296 | 0.312 | 0.007 | 0.301 | 0.298 | 193.9 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.299 | 0.296 | 0.302 | 0.003 | 0.299 | 0.298 | 193.5 | 1.00× |
| lazy-ruby | 3 | 6.966 | 6.753 | 7.183 | 0.215 | 6.967 | 7.034 | 165.7 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.470 | 0.467 | 0.477 | 0.004 | 0.471 | 0.468 | 117.4 | 1.00× |
| lazy-ruby | 5 | 0.471 | 0.469 | 0.473 | 0.002 | 0.471 | 0.470 | 118.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.474 | 0.470 | 0.476 | 0.003 | 0.474 | 0.472 | 117.2 | 1.00× |
| lazy-ruby | 3 | 7.001 | 6.909 | 7.187 | 0.142 | 7.032 | 7.080 | 115.3 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.224 | 0.222 | 0.225 | 0.001 | 0.224 | 0.223 | 74.2 | 1.00× |
| lazy-ruby | 5 | 0.225 | 0.222 | 0.227 | 0.002 | 0.225 | 0.224 | 74.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.224 | 0.227 | 0.001 | 0.225 | 0.224 | 74.2 | 1.00× |
| lazy-ruby | 3 | 2.209 | 2.203 | 2.215 | 0.006 | 2.209 | 2.326 | 74.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.432 | 0.451 | 0.007 | 0.438 | 0.434 | 74.2 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.430 | 0.438 | 0.003 | 0.434 | 0.431 | 74.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.434 | 0.436 | 0.001 | 0.436 | 0.435 | 74.2 | 1.00× |
| lazy-ruby | 3 | 2.413 | 2.413 | 2.425 | 0.007 | 2.417 | 2.529 | 74.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.245 | 0.249 | 0.002 | 0.248 | 0.248 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.249 | 0.247 | 0.251 | 0.001 | 0.249 | 0.249 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.247 | 0.247 | 0.249 | 0.001 | 0.248 | 0.246 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.236 | 2.223 | 2.238 | 0.008 | 2.232 | 2.353 | 155.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.412 | 0.410 | 0.414 | 0.002 | 0.412 | 0.411 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.412 | 0.410 | 0.416 | 0.002 | 0.412 | 0.411 | 101.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.414 | 0.416 | 0.001 | 0.414 | 0.413 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.406 | 2.394 | 2.409 | 0.008 | 2.403 | 2.521 | 74.2 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.292 | 0.290 | 0.293 | 0.002 | 0.291 | 0.291 | 82.5 | 1.00× |
| lazy-ruby | 5 | 0.296 | 0.294 | 0.298 | 0.002 | 0.296 | 0.294 | 82.2 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.290 | 0.290 | 0.293 | 0.002 | 0.291 | 0.289 | 82.5 | 1.00× |
| lazy-ruby | 3 | 7.395 | 7.392 | 7.438 | 0.026 | 7.408 | 7.451 | 80.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.526 | 0.521 | 0.529 | 0.003 | 0.525 | 0.523 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.534 | 0.528 | 0.539 | 0.005 | 0.533 | 0.532 | 82.3 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.530 | 0.526 | 0.534 | 0.004 | 0.530 | 0.528 | 82.3 | 1.00× |
| lazy-ruby | 3 | 7.677 | 7.672 | 7.714 | 0.023 | 7.688 | 7.754 | 80.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.329 | 0.328 | 0.335 | 0.003 | 0.331 | 0.329 | 192.0 | 1.00× |
| lazy-ruby | 5 | 0.337 | 0.333 | 0.340 | 0.003 | 0.336 | 0.336 | 192.2 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.331 | 0.325 | 0.331 | 0.004 | 0.329 | 0.329 | 192.5 | 1.00× |
| lazy-ruby | 3 | 7.457 | 7.437 | 7.462 | 0.013 | 7.452 | 7.518 | 167.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.542 | 0.539 | 0.546 | 0.003 | 0.542 | 0.541 | 112.5 | 1.00× |
| lazy-ruby | 5 | 0.554 | 0.546 | 0.561 | 0.005 | 0.554 | 0.554 | 112.4 | 0.98× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.548 | 0.546 | 0.550 | 0.002 | 0.548 | 0.547 | 112.3 | 1.00× |
| lazy-ruby | 3 | 7.725 | 7.717 | 7.733 | 0.008 | 7.725 | 7.789 | 80.6 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.209 | 0.196 | 0.234 | 0.015 | 0.213 | 0.200 | 63.7 | 1.00× |
| lazy-ruby | 5 | 0.196 | 0.176 | 0.237 | 0.028 | 0.205 | 0.177 | 65.6 | 1.07× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.195 | 0.195 | 0.198 | 0.002 | 0.196 | 0.175 | 64.0 | 1.00× |
| lazy-ruby | 3 | 0.872 | 0.849 | 0.875 | 0.014 | 0.866 | 0.927 | 54.8 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.384 | 0.373 | 0.435 | 0.024 | 0.394 | 0.375 | 63.5 | 1.00× |
| lazy-ruby | 5 | 0.395 | 0.370 | 0.411 | 0.016 | 0.395 | 0.388 | 64.1 | 0.97× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.391 | 0.388 | 0.413 | 0.014 | 0.397 | 0.386 | 63.5 | 1.00× |
| lazy-ruby | 3 | 1.062 | 1.048 | 1.070 | 0.012 | 1.060 | 1.121 | 53.3 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.214 | 0.202 | 0.283 | 0.032 | 0.228 | 0.206 | 167.1 | 1.00× |
| lazy-ruby | 5 | 0.226 | 0.210 | 0.240 | 0.013 | 0.225 | 0.203 | 166.3 | 0.95× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.304 | 0.229 | 0.314 | 0.046 | 0.282 | 0.293 | 166.7 | 1.00× |
| lazy-ruby | 3 | 1.013 | 1.010 | 1.014 | 0.002 | 1.012 | 1.082 | 156.6 | 0.30× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.412 | 0.394 | 0.423 | 0.012 | 0.411 | 0.397 | 92.5 | 1.00× |
| lazy-ruby | 5 | 0.404 | 0.390 | 0.467 | 0.032 | 0.415 | 0.383 | 92.8 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.410 | 0.481 | 0.036 | 0.442 | 0.408 | 91.6 | 1.00× |
| lazy-ruby | 3 | 1.123 | 1.116 | 1.137 | 0.010 | 1.126 | 1.201 | 65.3 | 0.39× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.581 | 0.453 | 0.656 | 0.083 | 0.583 | 0.557 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.557 | 0.458 | 0.650 | 0.079 | 0.558 | 0.547 | 57.1 | 1.04× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.469 | 0.442 | 0.479 | 0.019 | 0.463 | 0.452 | 57.6 | 1.00× |
| lazy-ruby | 3 | 4.665 | 4.183 | 4.702 | 0.290 | 4.517 | 4.818 | 43.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.935 | 0.867 | 0.984 | 0.051 | 0.920 | 0.925 | 57.0 | 1.00× |
| lazy-ruby | 5 | 0.919 | 0.836 | 0.995 | 0.058 | 0.923 | 0.910 | 57.1 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.940 | 0.927 | 0.946 | 0.010 | 0.938 | 0.930 | 57.0 | 1.00× |
| lazy-ruby | 3 | 6.023 | 4.429 | 6.209 | 0.978 | 5.554 | 6.066 | 42.7 | 0.16× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.505 | 0.494 | 0.543 | 0.020 | 0.512 | 0.495 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.539 | 0.483 | 0.547 | 0.031 | 0.519 | 0.526 | 141.7 | 0.94× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.561 | 0.544 | 0.565 | 0.011 | 0.557 | 0.531 | 142.0 | 1.00× |
| lazy-ruby | 3 | 4.072 | 3.878 | 4.585 | 0.366 | 4.178 | 4.240 | 126.9 | 0.14× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.970 | 0.760 | 1.253 | 0.214 | 0.996 | 0.960 | 82.8 | 1.00× |
| lazy-ruby | 5 | 0.953 | 0.815 | 1.202 | 0.156 | 0.954 | 0.939 | 82.8 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.013 | 0.996 | 1.021 | 0.013 | 1.010 | 0.996 | 82.8 | 1.00× |
| lazy-ruby | 3 | 4.276 | 4.211 | 5.134 | 0.515 | 4.540 | 4.401 | 46.5 | 0.24× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.26

Failed runs:

- boot / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- boot / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- fib / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- fib / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- ioread / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- ioread / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- treewalk / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- treewalk / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

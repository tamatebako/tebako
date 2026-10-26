# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.227 | 0.237 | 0.004 | 0.234 | 0.234 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.231 | 0.239 | 0.003 | 0.236 | 0.234 | 108.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.239 | 0.237 | 0.239 | 0.001 | 0.238 | 0.237 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.386 | 2.371 | 2.396 | 0.012 | 2.384 | 2.518 | 108.3 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.437 | 0.434 | 0.437 | 0.001 | 0.436 | 0.434 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.438 | 0.437 | 0.439 | 0.001 | 0.438 | 0.436 | 108.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.439 | 0.439 | 0.439 | 0.000 | 0.439 | 0.438 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.582 | 2.581 | 2.589 | 0.004 | 2.584 | 2.715 | 108.3 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.260 | 0.264 | 0.002 | 0.262 | 0.261 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.264 | 0.262 | 0.264 | 0.001 | 0.263 | 0.262 | 180.2 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.260 | 0.260 | 0.260 | 0.000 | 0.260 | 0.259 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.402 | 2.398 | 2.404 | 0.003 | 2.401 | 2.536 | 156.9 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.412 | 0.412 | 0.414 | 0.001 | 0.413 | 0.411 | 108.3 | 1.00× |
| lazy-ruby | 5 | 0.414 | 0.412 | 0.416 | 0.002 | 0.414 | 0.412 | 108.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.412 | 0.408 | 0.412 | 0.002 | 0.410 | 0.410 | 108.3 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.535 | 2.585 | 0.025 | 2.559 | 2.671 | 108.3 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.186 | 0.184 | 0.192 | 0.003 | 0.187 | 0.184 | 113.7 | 1.00× |
| lazy-ruby | 5 | 0.186 | 0.184 | 0.192 | 0.003 | 0.187 | 0.185 | 113.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.184 | 0.181 | 0.206 | 0.013 | 0.190 | 0.183 | 113.7 | 1.00× |
| lazy-ruby | 3 | 4.805 | 4.787 | 4.819 | 0.016 | 4.804 | 4.861 | 113.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.309 | 0.305 | 0.323 | 0.007 | 0.312 | 0.308 | 113.7 | 1.00× |
| lazy-ruby | 5 | 0.291 | 0.285 | 0.305 | 0.009 | 0.292 | 0.289 | 113.7 | 1.06× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.309 | 0.307 | 0.309 | 0.001 | 0.309 | 0.308 | 113.7 | 1.00× |
| lazy-ruby | 3 | 4.862 | 4.837 | 4.940 | 0.054 | 4.880 | 4.922 | 113.7 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.204 | 0.200 | 0.204 | 0.002 | 0.203 | 0.202 | 193.7 | 1.00× |
| lazy-ruby | 5 | 0.204 | 0.200 | 0.213 | 0.005 | 0.206 | 0.203 | 195.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.202 | 0.200 | 0.202 | 0.001 | 0.201 | 0.201 | 194.1 | 1.00× |
| lazy-ruby | 3 | 4.795 | 4.782 | 4.816 | 0.017 | 4.798 | 4.858 | 166.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.319 | 0.317 | 0.326 | 0.003 | 0.320 | 0.318 | 117.7 | 1.00× |
| lazy-ruby | 5 | 0.318 | 0.317 | 0.326 | 0.004 | 0.320 | 0.317 | 117.5 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.320 | 0.318 | 0.320 | 0.001 | 0.319 | 0.318 | 117.7 | 1.00× |
| lazy-ruby | 3 | 4.879 | 4.853 | 4.928 | 0.038 | 4.887 | 4.940 | 113.7 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.225 | 0.231 | 0.003 | 0.230 | 0.229 | 74.4 | 1.00× |
| lazy-ruby | 5 | 0.231 | 0.229 | 0.233 | 0.001 | 0.231 | 0.230 | 74.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.229 | 0.231 | 0.001 | 0.230 | 0.229 | 74.4 | 1.00× |
| lazy-ruby | 3 | 2.224 | 2.222 | 2.241 | 0.010 | 2.229 | 2.337 | 74.4 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.441 | 0.440 | 0.447 | 0.003 | 0.442 | 0.440 | 74.4 | 1.00× |
| lazy-ruby | 5 | 0.439 | 0.437 | 0.441 | 0.001 | 0.439 | 0.437 | 74.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.443 | 0.441 | 0.443 | 0.001 | 0.442 | 0.440 | 74.4 | 1.00× |
| lazy-ruby | 3 | 2.436 | 2.423 | 2.444 | 0.011 | 2.434 | 2.550 | 74.4 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.255 | 0.257 | 0.001 | 0.256 | 0.254 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.255 | 0.259 | 0.002 | 0.256 | 0.254 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.255 | 0.253 | 0.257 | 0.002 | 0.255 | 0.254 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.256 | 2.250 | 2.265 | 0.007 | 2.257 | 2.371 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.420 | 0.426 | 0.003 | 0.425 | 0.425 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.426 | 0.424 | 0.426 | 0.001 | 0.425 | 0.424 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.428 | 0.426 | 0.430 | 0.002 | 0.428 | 0.426 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.421 | 2.419 | 2.423 | 0.002 | 2.421 | 2.535 | 74.4 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.288 | 0.291 | 0.002 | 0.289 | 0.287 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.290 | 0.286 | 0.295 | 0.003 | 0.290 | 0.289 | 82.5 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.287 | 0.285 | 0.290 | 0.003 | 0.288 | 0.285 | 82.4 | 1.00× |
| lazy-ruby | 3 | 7.479 | 7.340 | 7.583 | 0.122 | 7.467 | 7.531 | 80.1 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.502 | 0.497 | 0.505 | 0.003 | 0.502 | 0.501 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.500 | 0.498 | 0.504 | 0.003 | 0.501 | 0.499 | 82.4 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.502 | 0.501 | 0.503 | 0.002 | 0.502 | 0.500 | 84.2 | 1.00× |
| lazy-ruby | 3 | 7.546 | 7.494 | 7.558 | 0.034 | 7.533 | 7.600 | 80.1 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.327 | 0.326 | 0.329 | 0.001 | 0.328 | 0.326 | 191.7 | 1.00× |
| lazy-ruby | 5 | 0.326 | 0.325 | 0.329 | 0.001 | 0.327 | 0.324 | 192.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.327 | 0.323 | 0.327 | 0.003 | 0.326 | 0.326 | 191.5 | 1.00× |
| lazy-ruby | 3 | 7.489 | 7.473 | 7.606 | 0.073 | 7.523 | 7.541 | 166.8 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.537 | 0.535 | 0.538 | 0.001 | 0.537 | 0.535 | 112.2 | 1.00× |
| lazy-ruby | 5 | 0.532 | 0.530 | 0.534 | 0.001 | 0.532 | 0.531 | 112.8 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.533 | 0.530 | 0.535 | 0.002 | 0.533 | 0.531 | 112.6 | 1.00× |
| lazy-ruby | 3 | 7.876 | 7.771 | 7.902 | 0.069 | 7.850 | 7.964 | 80.1 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.180 | 0.168 | 0.183 | 0.006 | 0.177 | 0.163 | 65.1 | 1.00× |
| lazy-ruby | 5 | 0.182 | 0.167 | 0.185 | 0.007 | 0.179 | 0.163 | 64.9 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.179 | 0.175 | 0.184 | 0.005 | 0.179 | 0.164 | 66.2 | 1.00× |
| lazy-ruby | 3 | 0.793 | 0.791 | 0.799 | 0.004 | 0.794 | 0.856 | 53.7 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.363 | 0.354 | 0.381 | 0.011 | 0.365 | 0.353 | 65.8 | 1.00× |
| lazy-ruby | 5 | 0.372 | 0.357 | 0.375 | 0.007 | 0.370 | 0.353 | 65.4 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.369 | 0.356 | 0.372 | 0.008 | 0.366 | 0.351 | 66.4 | 1.00× |
| lazy-ruby | 3 | 0.992 | 0.992 | 0.995 | 0.002 | 0.993 | 1.044 | 54.5 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.198 | 0.193 | 0.205 | 0.005 | 0.198 | 0.179 | 167.7 | 1.00× |
| lazy-ruby | 5 | 0.200 | 0.193 | 0.203 | 0.004 | 0.199 | 0.182 | 166.5 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.198 | 0.197 | 0.201 | 0.002 | 0.199 | 0.177 | 167.0 | 1.00× |
| lazy-ruby | 3 | 0.828 | 0.823 | 0.856 | 0.018 | 0.836 | 0.899 | 153.4 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.351 | 0.339 | 0.355 | 0.007 | 0.348 | 0.333 | 94.8 | 1.00× |
| lazy-ruby | 5 | 0.352 | 0.345 | 0.360 | 0.006 | 0.351 | 0.333 | 95.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.354 | 0.349 | 0.357 | 0.004 | 0.353 | 0.334 | 94.5 | 1.00× |
| lazy-ruby | 3 | 1.001 | 0.967 | 1.013 | 0.024 | 0.994 | 1.047 | 67.0 | 0.35× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.451 | 0.402 | 0.582 | 0.070 | 0.475 | 0.443 | 57.0 | 1.00× |
| lazy-ruby | 5 | 0.448 | 0.419 | 0.472 | 0.021 | 0.449 | 0.443 | 57.2 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.476 | 0.466 | 1.026 | 0.321 | 0.656 | 0.464 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.453 | 3.189 | 3.658 | 0.235 | 3.433 | 3.558 | 42.8 | 0.14× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.738 | 0.678 | 0.852 | 0.066 | 0.751 | 0.726 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.769 | 0.704 | 0.882 | 0.071 | 0.768 | 0.748 | 57.5 | 0.96× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.878 | 0.867 | 0.908 | 0.021 | 0.884 | 0.867 | 57.2 | 1.00× |
| lazy-ruby | 3 | 4.426 | 4.195 | 5.048 | 0.441 | 4.556 | 4.573 | 43.0 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.400 | 0.396 | 0.412 | 0.006 | 0.402 | 0.391 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.398 | 0.396 | 0.411 | 0.006 | 0.402 | 0.388 | 141.7 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.394 | 0.394 | 0.406 | 0.007 | 0.398 | 0.386 | 141.6 | 1.00× |
| lazy-ruby | 3 | 3.305 | 3.229 | 3.360 | 0.066 | 3.298 | 3.410 | 126.7 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.676 | 0.641 | 0.736 | 0.038 | 0.688 | 0.660 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.700 | 0.633 | 0.736 | 0.038 | 0.692 | 0.688 | 83.3 | 0.97× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.666 | 0.660 | 0.694 | 0.018 | 0.673 | 0.649 | 82.6 | 1.00× |
| lazy-ruby | 3 | 4.128 | 3.563 | 5.115 | 0.785 | 4.269 | 4.239 | 46.6 | 0.16× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.63

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.233 | 0.247 | 0.007 | 0.239 | 0.234 | 33.3 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.233 | 0.241 | 0.003 | 0.235 | 0.234 | 33.4 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.241 | 0.234 | 0.243 | 0.005 | 0.239 | 0.234 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.806 | 0.803 | 0.809 | 0.003 | 0.806 | 0.828 | 25.6 | 0.30× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.463 | 0.461 | 0.469 | 0.003 | 0.464 | 0.453 | 33.3 | 1.00× |
| lazy-ruby | 5 | 0.459 | 0.458 | 0.462 | 0.002 | 0.459 | 0.453 | 33.6 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.462 | 0.459 | 0.472 | 0.007 | 0.464 | 0.453 | 33.9 | 1.00× |
| lazy-ruby | 3 | 1.033 | 1.027 | 1.041 | 0.007 | 1.034 | 1.047 | 25.5 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.284 | 0.293 | 0.004 | 0.288 | 0.281 | 140.9 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.282 | 0.292 | 0.004 | 0.286 | 0.297 | 140.6 | 1.03× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.286 | 0.284 | 0.290 | 0.003 | 0.286 | 0.281 | 141.0 | 1.00× |
| lazy-ruby | 3 | 0.852 | 0.843 | 0.867 | 0.012 | 0.854 | 0.891 | 132.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.508 | 0.494 | 0.518 | 0.009 | 0.507 | 0.500 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.501 | 0.494 | 0.510 | 0.006 | 0.502 | 0.500 | 75.2 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.504 | 0.503 | 0.504 | 0.001 | 0.504 | 0.500 | 75.4 | 1.00× |
| lazy-ruby | 3 | 1.070 | 1.065 | 1.071 | 0.003 | 1.069 | 1.109 | 50.6 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

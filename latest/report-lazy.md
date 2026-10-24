# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.220 | 0.218 | 0.223 | 0.002 | 0.220 | 0.220 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.222 | 0.222 | 0.222 | 0.000 | 0.222 | 0.222 | 108.4 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.222 | 0.222 | 0.224 | 0.001 | 0.223 | 0.222 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.353 | 2.343 | 2.355 | 0.006 | 2.350 | 2.484 | 108.4 | 0.09× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.422 | 0.426 | 0.002 | 0.425 | 0.425 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.424 | 0.432 | 0.004 | 0.426 | 0.423 | 108.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.426 | 0.424 | 0.428 | 0.002 | 0.426 | 0.424 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.560 | 2.549 | 2.568 | 0.010 | 2.559 | 2.691 | 108.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.251 | 0.249 | 0.257 | 0.003 | 0.253 | 0.251 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.251 | 0.251 | 0.252 | 0.000 | 0.251 | 0.250 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.251 | 0.252 | 0.000 | 0.251 | 0.250 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.387 | 2.375 | 2.391 | 0.008 | 2.384 | 2.519 | 157.5 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.410 | 0.405 | 0.410 | 0.002 | 0.408 | 0.408 | 108.4 | 1.00× |
| lazy-ruby | 5 | 0.408 | 0.408 | 0.410 | 0.001 | 0.408 | 0.407 | 108.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.410 | 0.410 | 0.412 | 0.001 | 0.410 | 0.408 | 108.4 | 1.00× |
| lazy-ruby | 3 | 2.538 | 2.528 | 2.540 | 0.007 | 2.535 | 2.671 | 108.4 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.253 | 0.286 | 0.015 | 0.266 | 0.261 | 115.6 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.255 | 0.284 | 0.013 | 0.264 | 0.254 | 115.6 | 1.03× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.259 | 0.257 | 0.262 | 0.003 | 0.259 | 0.258 | 115.6 | 1.00× |
| lazy-ruby | 3 | 6.832 | 6.799 | 6.850 | 0.026 | 6.827 | 6.906 | 115.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.482 | 0.472 | 0.489 | 0.008 | 0.481 | 0.481 | 115.6 | 1.00× |
| lazy-ruby | 5 | 0.478 | 0.472 | 0.488 | 0.007 | 0.479 | 0.477 | 115.6 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.490 | 0.486 | 0.490 | 0.002 | 0.489 | 0.488 | 115.6 | 1.00× |
| lazy-ruby | 3 | 7.041 | 7.002 | 7.050 | 0.026 | 7.031 | 7.097 | 115.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.286 | 0.291 | 0.002 | 0.288 | 0.287 | 194.1 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.284 | 0.290 | 0.002 | 0.287 | 0.287 | 193.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.291 | 0.290 | 0.294 | 0.002 | 0.291 | 0.290 | 195.2 | 1.00× |
| lazy-ruby | 3 | 6.854 | 6.843 | 6.898 | 0.029 | 6.865 | 6.927 | 166.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.459 | 0.453 | 0.468 | 0.006 | 0.460 | 0.458 | 117.2 | 1.00× |
| lazy-ruby | 5 | 0.461 | 0.457 | 0.463 | 0.002 | 0.461 | 0.460 | 117.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.460 | 0.457 | 0.461 | 0.002 | 0.459 | 0.459 | 117.2 | 1.00× |
| lazy-ruby | 3 | 7.016 | 7.012 | 7.439 | 0.245 | 7.156 | 7.087 | 115.6 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.229 | 0.231 | 0.001 | 0.230 | 0.229 | 74.2 | 1.00× |
| lazy-ruby | 5 | 0.231 | 0.229 | 0.231 | 0.001 | 0.230 | 0.229 | 74.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.229 | 0.231 | 0.001 | 0.230 | 0.229 | 74.2 | 1.00× |
| lazy-ruby | 3 | 2.239 | 2.236 | 2.245 | 0.004 | 2.240 | 2.353 | 74.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.434 | 0.439 | 0.002 | 0.436 | 0.434 | 74.2 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.432 | 0.439 | 0.003 | 0.436 | 0.433 | 74.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.434 | 0.437 | 0.001 | 0.436 | 0.435 | 74.2 | 1.00× |
| lazy-ruby | 3 | 2.434 | 2.426 | 2.437 | 0.006 | 2.432 | 2.552 | 74.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.249 | 0.251 | 0.001 | 0.250 | 0.249 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.251 | 0.249 | 0.251 | 0.001 | 0.251 | 0.250 | 182.4 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.249 | 0.253 | 0.002 | 0.251 | 0.249 | 182.2 | 1.00× |
| lazy-ruby | 3 | 2.240 | 2.239 | 2.282 | 0.024 | 2.254 | 2.357 | 155.6 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.420 | 0.422 | 0.001 | 0.421 | 0.420 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.420 | 0.422 | 0.001 | 0.421 | 0.419 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.420 | 0.420 | 0.420 | 0.000 | 0.420 | 0.418 | 101.6 | 1.00× |
| lazy-ruby | 3 | 2.430 | 2.406 | 2.432 | 0.015 | 2.422 | 2.546 | 74.2 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.225 | 0.223 | 0.227 | 0.002 | 0.225 | 0.223 | 82.5 | 1.00× |
| lazy-ruby | 5 | 0.225 | 0.225 | 0.225 | 0.000 | 0.225 | 0.223 | 82.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.223 | 0.225 | 0.001 | 0.225 | 0.223 | 82.5 | 1.00× |
| lazy-ruby | 3 | 5.780 | 5.710 | 5.791 | 0.044 | 5.760 | 5.827 | 80.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.388 | 0.387 | 0.395 | 0.003 | 0.389 | 0.387 | 82.5 | 1.00× |
| lazy-ruby | 5 | 0.392 | 0.387 | 0.397 | 0.005 | 0.392 | 0.391 | 82.4 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.391 | 0.390 | 0.392 | 0.001 | 0.391 | 0.389 | 82.4 | 1.00× |
| lazy-ruby | 3 | 5.824 | 5.791 | 6.048 | 0.140 | 5.888 | 5.863 | 80.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.254 | 0.251 | 0.254 | 0.002 | 0.253 | 0.252 | 192.3 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.254 | 0.256 | 0.001 | 0.255 | 0.254 | 192.1 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.254 | 0.252 | 0.254 | 0.001 | 0.253 | 0.252 | 192.4 | 1.00× |
| lazy-ruby | 3 | 5.747 | 5.713 | 5.865 | 0.080 | 5.775 | 5.817 | 167.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.411 | 0.417 | 0.003 | 0.415 | 0.414 | 112.8 | 1.00× |
| lazy-ruby | 5 | 0.414 | 0.413 | 0.421 | 0.003 | 0.415 | 0.413 | 112.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.413 | 0.411 | 0.414 | 0.002 | 0.412 | 0.412 | 112.4 | 1.00× |
| lazy-ruby | 3 | 5.880 | 5.863 | 5.950 | 0.046 | 5.898 | 5.931 | 80.8 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.205 | 0.199 | 0.235 | 0.016 | 0.213 | 0.190 | 64.1 | 1.00× |
| lazy-ruby | 5 | 0.205 | 0.192 | 0.238 | 0.017 | 0.209 | 0.184 | 63.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.195 | 0.200 | 0.003 | 0.197 | 0.183 | 64.1 | 1.00× |
| lazy-ruby | 3 | 0.888 | 0.879 | 0.893 | 0.007 | 0.887 | 0.949 | 54.2 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.428 | 0.401 | 0.458 | 0.027 | 0.428 | 0.406 | 64.9 | 1.00× |
| lazy-ruby | 5 | 0.409 | 0.399 | 0.420 | 0.008 | 0.409 | 0.384 | 63.4 | 1.05× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.390 | 0.384 | 0.405 | 0.010 | 0.393 | 0.384 | 64.0 | 1.00× |
| lazy-ruby | 3 | 1.135 | 1.086 | 1.135 | 0.028 | 1.119 | 1.179 | 54.3 | 0.34× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.232 | 0.202 | 0.239 | 0.014 | 0.227 | 0.216 | 166.5 | 1.00× |
| lazy-ruby | 5 | 0.231 | 0.203 | 0.246 | 0.019 | 0.226 | 0.209 | 165.9 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.220 | 0.210 | 0.228 | 0.009 | 0.219 | 0.202 | 166.4 | 1.00× |
| lazy-ruby | 3 | 0.913 | 0.893 | 0.973 | 0.042 | 0.926 | 0.972 | 145.5 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.370 | 0.364 | 0.386 | 0.010 | 0.375 | 0.363 | 91.0 | 1.00× |
| lazy-ruby | 5 | 0.388 | 0.382 | 0.392 | 0.004 | 0.388 | 0.367 | 94.5 | 0.95× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.403 | 0.397 | 0.413 | 0.008 | 0.404 | 0.393 | 91.8 | 1.00× |
| lazy-ruby | 3 | 1.146 | 1.080 | 1.148 | 0.039 | 1.125 | 1.205 | 68.3 | 0.35× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.350 | 0.324 | 0.367 | 0.016 | 0.347 | 0.337 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.349 | 0.342 | 0.545 | 0.089 | 0.387 | 0.338 | 57.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.346 | 0.341 | 0.350 | 0.005 | 0.346 | 0.335 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.198 | 3.188 | 3.234 | 0.024 | 3.207 | 3.300 | 42.9 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.706 | 0.680 | 0.720 | 0.015 | 0.704 | 0.696 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.704 | 0.693 | 0.705 | 0.006 | 0.701 | 0.691 | 57.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.701 | 0.686 | 0.705 | 0.010 | 0.697 | 0.695 | 57.4 | 1.00× |
| lazy-ruby | 3 | 3.682 | 3.606 | 4.340 | 0.403 | 3.876 | 3.789 | 42.7 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.408 | 0.403 | 0.538 | 0.059 | 0.434 | 0.400 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.417 | 0.404 | 0.582 | 0.076 | 0.447 | 0.408 | 141.7 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.491 | 0.439 | 1.157 | 0.400 | 0.696 | 0.474 | 141.9 | 1.00× |
| lazy-ruby | 3 | 4.233 | 4.051 | 4.639 | 0.301 | 4.308 | 4.357 | 126.6 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.968 | 0.863 | 1.427 | 0.244 | 1.069 | 0.957 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.913 | 0.807 | 1.058 | 0.091 | 0.930 | 0.900 | 83.2 | 1.06× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.833 | 0.746 | 2.143 | 0.783 | 1.241 | 0.814 | 82.7 | 1.00× |
| lazy-ruby | 3 | 5.667 | 4.389 | 6.210 | 0.935 | 5.422 | 5.787 | 45.1 | 0.15× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.61

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.239 | 0.234 | 0.257 | 0.009 | 0.242 | 0.250 | 33.4 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.234 | 0.247 | 0.006 | 0.241 | 0.234 | 33.4 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.236 | 0.235 | 0.242 | 0.004 | 0.238 | 0.250 | 33.6 | 1.00× |
| lazy-ruby | 3 | 0.817 | 0.813 | 0.848 | 0.019 | 0.826 | 0.844 | 25.5 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.459 | 0.455 | 0.466 | 0.004 | 0.459 | 0.453 | 33.6 | 1.00× |
| lazy-ruby | 5 | 0.458 | 0.456 | 0.464 | 0.003 | 0.459 | 0.453 | 33.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.463 | 0.461 | 0.464 | 0.002 | 0.463 | 0.453 | 33.6 | 1.00× |
| lazy-ruby | 3 | 1.036 | 1.031 | 1.044 | 0.007 | 1.037 | 1.047 | 25.4 | 0.45× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.287 | 0.284 | 0.291 | 0.003 | 0.287 | 0.281 | 140.8 | 1.00× |
| lazy-ruby | 5 | 0.291 | 0.285 | 0.294 | 0.003 | 0.290 | 0.312 | 140.8 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.297 | 0.291 | 0.298 | 0.004 | 0.295 | 0.297 | 141.1 | 1.00× |
| lazy-ruby | 3 | 0.877 | 0.861 | 0.887 | 0.013 | 0.875 | 0.906 | 132.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.508 | 0.507 | 0.519 | 0.005 | 0.511 | 0.500 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.510 | 0.503 | 0.516 | 0.005 | 0.510 | 0.500 | 75.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.520 | 0.510 | 0.521 | 0.006 | 0.517 | 0.516 | 75.4 | 1.00× |
| lazy-ruby | 3 | 1.084 | 1.077 | 1.143 | 0.036 | 1.101 | 1.125 | 50.5 | 0.48× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

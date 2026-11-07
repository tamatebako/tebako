# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.233 | 0.235 | 0.001 | 0.233 | 0.232 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.231 | 0.235 | 0.002 | 0.233 | 0.231 | 108.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.235 | 0.237 | 0.001 | 0.236 | 0.235 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.379 | 2.365 | 2.386 | 0.011 | 2.377 | 2.515 | 108.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.430 | 0.434 | 0.002 | 0.433 | 0.432 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.432 | 0.432 | 0.438 | 0.003 | 0.434 | 0.431 | 108.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.434 | 0.434 | 0.442 | 0.005 | 0.437 | 0.433 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.578 | 2.573 | 2.637 | 0.036 | 2.596 | 2.712 | 108.1 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.260 | 0.268 | 0.003 | 0.263 | 0.261 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.264 | 0.262 | 0.264 | 0.001 | 0.263 | 0.261 | 180.3 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.262 | 0.262 | 0.000 | 0.262 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.382 | 2.376 | 2.396 | 0.010 | 2.385 | 2.518 | 156.6 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.418 | 0.422 | 0.001 | 0.420 | 0.420 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.420 | 0.422 | 0.001 | 0.420 | 0.419 | 108.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.418 | 0.418 | 0.000 | 0.418 | 0.417 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.550 | 2.545 | 2.551 | 0.003 | 2.548 | 2.685 | 108.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.283 | 0.281 | 0.290 | 0.003 | 0.285 | 0.283 | 114.7 | 1.00× |
| lazy-ruby | 5 | 0.284 | 0.284 | 0.286 | 0.001 | 0.284 | 0.283 | 114.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.281 | 0.281 | 0.284 | 0.001 | 0.282 | 0.281 | 114.7 | 1.00× |
| lazy-ruby | 3 | 6.818 | 6.797 | 6.866 | 0.035 | 6.827 | 6.910 | 114.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.503 | 0.501 | 0.513 | 0.005 | 0.505 | 0.502 | 114.7 | 1.00× |
| lazy-ruby | 5 | 0.505 | 0.501 | 0.509 | 0.003 | 0.505 | 0.504 | 114.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.503 | 0.501 | 0.503 | 0.001 | 0.502 | 0.501 | 114.7 | 1.00× |
| lazy-ruby | 3 | 7.058 | 7.048 | 7.061 | 0.007 | 7.056 | 7.116 | 114.7 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.319 | 0.317 | 0.321 | 0.002 | 0.319 | 0.318 | 188.6 | 1.00× |
| lazy-ruby | 5 | 0.321 | 0.319 | 0.323 | 0.002 | 0.320 | 0.320 | 188.6 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.323 | 0.321 | 0.323 | 0.001 | 0.322 | 0.322 | 188.4 | 1.00× |
| lazy-ruby | 3 | 6.844 | 6.839 | 6.847 | 0.004 | 6.843 | 6.907 | 157.9 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.492 | 0.486 | 0.501 | 0.006 | 0.492 | 0.491 | 114.7 | 1.00× |
| lazy-ruby | 5 | 0.490 | 0.484 | 0.494 | 0.004 | 0.490 | 0.489 | 114.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.487 | 0.486 | 0.489 | 0.002 | 0.487 | 0.486 | 114.7 | 1.00× |
| lazy-ruby | 3 | 7.031 | 6.979 | 7.071 | 0.046 | 7.027 | 7.090 | 114.7 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.233 | 0.233 | 0.000 | 0.233 | 0.232 | 74.9 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.233 | 0.235 | 0.001 | 0.234 | 0.232 | 74.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.233 | 0.235 | 0.001 | 0.233 | 0.232 | 74.9 | 1.00× |
| lazy-ruby | 3 | 2.237 | 2.229 | 2.246 | 0.008 | 2.237 | 2.348 | 74.9 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.440 | 0.438 | 0.440 | 0.001 | 0.440 | 0.439 | 74.9 | 1.00× |
| lazy-ruby | 5 | 0.440 | 0.438 | 0.442 | 0.001 | 0.440 | 0.439 | 74.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.440 | 0.438 | 0.441 | 0.001 | 0.440 | 0.439 | 74.9 | 1.00× |
| lazy-ruby | 3 | 2.445 | 2.437 | 2.449 | 0.006 | 2.444 | 2.557 | 74.9 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.253 | 0.259 | 0.002 | 0.256 | 0.254 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.255 | 0.253 | 0.259 | 0.002 | 0.256 | 0.254 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.257 | 0.255 | 0.257 | 0.001 | 0.257 | 0.256 | 182.5 | 1.00× |
| lazy-ruby | 3 | 2.246 | 2.246 | 2.256 | 0.006 | 2.249 | 2.361 | 156.3 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.420 | 0.422 | 0.001 | 0.421 | 0.421 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.420 | 0.422 | 0.001 | 0.421 | 0.420 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.420 | 0.418 | 0.422 | 0.002 | 0.420 | 0.419 | 101.6 | 1.00× |
| lazy-ruby | 3 | 2.424 | 2.424 | 2.427 | 0.001 | 2.425 | 2.537 | 74.9 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.248 | 0.248 | 0.250 | 0.001 | 0.249 | 0.247 | 80.2 | 1.00× |
| lazy-ruby | 5 | 0.250 | 0.250 | 0.252 | 0.001 | 0.250 | 0.249 | 80.2 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.250 | 0.248 | 0.250 | 0.001 | 0.249 | 0.248 | 80.2 | 1.00× |
| lazy-ruby | 3 | 5.716 | 5.691 | 5.856 | 0.089 | 5.754 | 5.766 | 80.2 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.417 | 0.413 | 0.419 | 0.002 | 0.416 | 0.415 | 80.2 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.413 | 0.417 | 0.002 | 0.415 | 0.414 | 80.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.415 | 0.421 | 0.003 | 0.418 | 0.417 | 80.2 | 1.00× |
| lazy-ruby | 3 | 6.050 | 5.880 | 6.175 | 0.148 | 6.035 | 6.051 | 80.2 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.279 | 0.277 | 0.285 | 0.004 | 0.280 | 0.278 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.279 | 0.290 | 0.005 | 0.283 | 0.281 | 185.6 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.281 | 0.281 | 0.285 | 0.002 | 0.282 | 0.280 | 185.8 | 1.00× |
| lazy-ruby | 3 | 5.804 | 5.763 | 5.857 | 0.047 | 5.808 | 5.808 | 156.2 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.442 | 0.439 | 0.457 | 0.007 | 0.444 | 0.441 | 105.4 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.439 | 0.458 | 0.007 | 0.446 | 0.444 | 104.9 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.446 | 0.439 | 0.447 | 0.004 | 0.444 | 0.444 | 105.4 | 1.00× |
| lazy-ruby | 3 | 6.127 | 6.087 | 6.187 | 0.050 | 6.134 | 6.136 | 80.2 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.188 | 0.173 | 0.189 | 0.007 | 0.183 | 0.169 | 63.3 | 1.00× |
| lazy-ruby | 5 | 0.186 | 0.176 | 0.189 | 0.005 | 0.184 | 0.169 | 62.7 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.189 | 0.178 | 0.189 | 0.006 | 0.186 | 0.169 | 64.0 | 1.00× |
| lazy-ruby | 3 | 0.799 | 0.798 | 0.813 | 0.008 | 0.803 | 0.865 | 49.6 | 0.24× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.372 | 0.366 | 0.377 | 0.004 | 0.372 | 0.356 | 62.9 | 1.00× |
| lazy-ruby | 5 | 0.376 | 0.363 | 0.385 | 0.008 | 0.374 | 0.356 | 62.7 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.379 | 0.371 | 0.405 | 0.018 | 0.385 | 0.358 | 62.9 | 1.00× |
| lazy-ruby | 3 | 0.991 | 0.988 | 1.001 | 0.007 | 0.994 | 1.053 | 52.1 | 0.38× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.198 | 0.186 | 0.407 | 0.095 | 0.237 | 0.180 | 164.5 | 1.00× |
| lazy-ruby | 5 | 0.205 | 0.199 | 0.388 | 0.083 | 0.239 | 0.179 | 164.2 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.199 | 0.193 | 0.200 | 0.004 | 0.197 | 0.179 | 163.3 | 1.00× |
| lazy-ruby | 3 | 0.817 | 0.817 | 0.817 | 0.000 | 0.817 | 0.877 | 145.4 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.358 | 0.350 | 0.389 | 0.017 | 0.365 | 0.341 | 92.6 | 1.00× |
| lazy-ruby | 5 | 0.361 | 0.353 | 0.407 | 0.023 | 0.372 | 0.341 | 98.2 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.359 | 0.358 | 0.360 | 0.001 | 0.359 | 0.340 | 91.0 | 1.00× |
| lazy-ruby | 3 | 0.985 | 0.983 | 0.988 | 0.002 | 0.985 | 1.038 | 69.1 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.465 | 0.452 | 1.762 | 0.579 | 0.726 | 0.455 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.466 | 0.458 | 0.921 | 0.202 | 0.561 | 0.451 | 57.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.489 | 0.449 | 0.753 | 0.165 | 0.564 | 0.458 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.340 | 3.576 | 4.537 | 0.507 | 4.151 | 4.443 | 42.7 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.813 | 0.770 | 1.059 | 0.127 | 0.876 | 0.806 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.805 | 0.774 | 0.934 | 0.064 | 0.830 | 0.793 | 57.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.760 | 0.748 | 0.796 | 0.025 | 0.768 | 0.747 | 57.3 | 1.00× |
| lazy-ruby | 3 | 4.844 | 4.596 | 5.272 | 0.342 | 4.904 | 4.990 | 42.8 | 0.16× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.529 | 0.490 | 0.567 | 0.034 | 0.530 | 0.515 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.544 | 0.502 | 0.555 | 0.021 | 0.537 | 0.528 | 141.8 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.533 | 0.510 | 0.537 | 0.015 | 0.527 | 0.522 | 141.7 | 1.00× |
| lazy-ruby | 3 | 4.392 | 4.266 | 4.442 | 0.091 | 4.367 | 4.542 | 127.4 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.969 | 0.860 | 1.002 | 0.054 | 0.948 | 0.958 | 82.9 | 1.00× |
| lazy-ruby | 5 | 1.001 | 0.940 | 1.030 | 0.041 | 0.989 | 0.985 | 83.2 | 0.97× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.986 | 0.956 | 1.005 | 0.025 | 0.982 | 0.969 | 82.9 | 1.00× |
| lazy-ruby | 3 | 4.790 | 4.761 | 4.791 | 0.017 | 4.780 | 4.939 | 46.6 | 0.21× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.72

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.228 | 0.224 | 0.231 | 0.003 | 0.227 | 0.219 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.224 | 0.222 | 0.238 | 0.007 | 0.227 | 0.219 | 68.3 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.228 | 0.227 | 0.233 | 0.003 | 0.229 | 0.219 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.730 | 0.726 | 0.759 | 0.018 | 0.738 | 0.734 | 58.8 | 0.31× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.411 | 0.407 | 0.419 | 0.005 | 0.412 | 0.406 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.409 | 0.407 | 0.418 | 0.005 | 0.412 | 0.422 | 68.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.409 | 0.408 | 0.411 | 0.002 | 0.409 | 0.406 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.919 | 0.919 | 0.925 | 0.003 | 0.921 | 0.938 | 58.7 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.284 | 0.291 | 0.003 | 0.288 | 0.281 | 109.4 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.285 | 0.300 | 0.006 | 0.290 | 0.297 | 109.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.291 | 0.284 | 0.308 | 0.012 | 0.294 | 0.281 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.791 | 0.783 | 0.794 | 0.006 | 0.789 | 0.812 | 100.6 | 0.37× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.467 | 0.464 | 0.473 | 0.004 | 0.468 | 0.469 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.469 | 0.463 | 0.483 | 0.008 | 0.471 | 0.453 | 108.7 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.469 | 0.465 | 0.479 | 0.007 | 0.471 | 0.469 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.963 | 0.961 | 0.966 | 0.002 | 0.963 | 0.953 | 82.9 | 0.49× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

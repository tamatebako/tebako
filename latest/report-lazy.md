# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.227 | 0.229 | 0.001 | 0.228 | 0.226 | 108.6 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.219 | 0.229 | 0.004 | 0.226 | 0.226 | 108.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.227 | 0.227 | 0.000 | 0.227 | 0.225 | 108.6 | 1.00× |
| lazy-ruby | 3 | 2.352 | 2.342 | 2.358 | 0.008 | 2.350 | 2.482 | 108.6 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.418 | 0.428 | 0.004 | 0.425 | 0.425 | 108.6 | 1.00× |
| lazy-ruby | 5 | 0.426 | 0.425 | 0.428 | 0.002 | 0.427 | 0.425 | 108.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.428 | 0.424 | 0.431 | 0.003 | 0.428 | 0.427 | 108.6 | 1.00× |
| lazy-ruby | 3 | 2.568 | 2.564 | 2.571 | 0.003 | 2.567 | 2.699 | 108.6 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.247 | 0.253 | 0.003 | 0.250 | 0.249 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.247 | 0.247 | 0.249 | 0.001 | 0.248 | 0.247 | 180.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.249 | 0.245 | 0.251 | 0.003 | 0.249 | 0.248 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.368 | 2.364 | 2.381 | 0.009 | 2.371 | 2.501 | 156.7 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.402 | 0.401 | 0.410 | 0.003 | 0.404 | 0.401 | 108.6 | 1.00× |
| lazy-ruby | 5 | 0.404 | 0.402 | 0.408 | 0.003 | 0.404 | 0.403 | 108.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.403 | 0.403 | 0.403 | 0.000 | 0.403 | 0.402 | 108.6 | 1.00× |
| lazy-ruby | 3 | 2.531 | 2.529 | 2.537 | 0.004 | 2.532 | 2.663 | 108.6 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.251 | 0.255 | 0.002 | 0.253 | 0.251 | 114.0 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.251 | 0.255 | 0.002 | 0.252 | 0.250 | 114.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.250 | 0.255 | 0.002 | 0.253 | 0.251 | 114.0 | 1.00× |
| lazy-ruby | 3 | 6.814 | 6.797 | 6.835 | 0.019 | 6.815 | 6.873 | 114.0 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.470 | 0.467 | 0.472 | 0.002 | 0.469 | 0.468 | 114.0 | 1.00× |
| lazy-ruby | 5 | 0.469 | 0.463 | 0.492 | 0.011 | 0.473 | 0.468 | 114.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.470 | 0.470 | 0.489 | 0.011 | 0.476 | 0.469 | 114.0 | 1.00× |
| lazy-ruby | 3 | 7.027 | 6.980 | 7.028 | 0.027 | 7.012 | 7.082 | 114.0 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.286 | 0.288 | 0.001 | 0.287 | 0.286 | 193.9 | 1.00× |
| lazy-ruby | 5 | 0.286 | 0.282 | 0.287 | 0.002 | 0.285 | 0.285 | 195.8 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.286 | 0.286 | 0.289 | 0.002 | 0.287 | 0.286 | 193.9 | 1.00× |
| lazy-ruby | 3 | 6.848 | 6.843 | 6.850 | 0.004 | 6.847 | 6.907 | 166.1 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.457 | 0.451 | 0.462 | 0.004 | 0.457 | 0.456 | 118.9 | 1.00× |
| lazy-ruby | 5 | 0.451 | 0.447 | 0.476 | 0.012 | 0.455 | 0.450 | 117.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.453 | 0.459 | 0.003 | 0.456 | 0.454 | 117.1 | 1.00× |
| lazy-ruby | 3 | 6.993 | 6.990 | 7.009 | 0.010 | 6.998 | 7.055 | 114.0 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.225 | 0.231 | 0.003 | 0.229 | 0.228 | 74.6 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.229 | 0.231 | 0.001 | 0.229 | 0.228 | 74.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.231 | 0.233 | 0.001 | 0.231 | 0.230 | 74.6 | 1.00× |
| lazy-ruby | 3 | 2.238 | 2.230 | 2.244 | 0.007 | 2.237 | 2.352 | 74.6 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.442 | 0.438 | 0.449 | 0.004 | 0.443 | 0.440 | 74.6 | 1.00× |
| lazy-ruby | 5 | 0.441 | 0.440 | 0.445 | 0.002 | 0.442 | 0.439 | 74.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.441 | 0.440 | 0.441 | 0.000 | 0.441 | 0.439 | 74.6 | 1.00× |
| lazy-ruby | 3 | 2.438 | 2.429 | 2.446 | 0.008 | 2.438 | 2.555 | 74.6 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.251 | 0.257 | 0.003 | 0.254 | 0.252 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.251 | 0.258 | 0.003 | 0.253 | 0.251 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.253 | 0.255 | 0.001 | 0.254 | 0.253 | 182.3 | 1.00× |
| lazy-ruby | 3 | 2.244 | 2.242 | 2.253 | 0.005 | 2.246 | 2.361 | 156.3 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.418 | 0.430 | 0.005 | 0.422 | 0.419 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.420 | 0.424 | 0.002 | 0.422 | 0.420 | 101.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.416 | 0.420 | 0.002 | 0.418 | 0.418 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.421 | 2.417 | 2.425 | 0.004 | 2.421 | 2.536 | 74.6 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.227 | 0.233 | 0.002 | 0.230 | 0.228 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.229 | 0.239 | 0.004 | 0.234 | 0.232 | 82.7 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.235 | 0.221 | 0.288 | 0.035 | 0.248 | 0.235 | 82.4 | 1.00× |
| lazy-ruby | 3 | 5.949 | 5.899 | 6.001 | 0.051 | 5.950 | 6.004 | 81.6 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.365 | 0.353 | 0.374 | 0.008 | 0.364 | 0.364 | 84.3 | 1.00× |
| lazy-ruby | 5 | 0.359 | 0.357 | 0.372 | 0.006 | 0.361 | 0.358 | 82.4 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.363 | 0.361 | 0.365 | 0.002 | 0.363 | 0.362 | 82.6 | 1.00× |
| lazy-ruby | 3 | 5.922 | 5.856 | 5.933 | 0.042 | 5.904 | 5.974 | 81.6 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.254 | 0.250 | 0.256 | 0.002 | 0.253 | 0.252 | 192.8 | 1.00× |
| lazy-ruby | 5 | 0.254 | 0.252 | 0.256 | 0.002 | 0.254 | 0.253 | 192.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.256 | 0.254 | 0.256 | 0.001 | 0.255 | 0.255 | 191.7 | 1.00× |
| lazy-ruby | 3 | 5.885 | 5.851 | 5.928 | 0.039 | 5.888 | 5.939 | 168.2 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.427 | 0.426 | 0.435 | 0.004 | 0.429 | 0.427 | 112.5 | 1.00× |
| lazy-ruby | 5 | 0.429 | 0.417 | 0.442 | 0.010 | 0.429 | 0.428 | 112.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.419 | 0.415 | 0.431 | 0.008 | 0.422 | 0.418 | 112.5 | 1.00× |
| lazy-ruby | 3 | 6.028 | 6.019 | 6.037 | 0.009 | 6.028 | 6.080 | 81.6 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.210 | 0.200 | 0.219 | 0.007 | 0.210 | 0.192 | 64.5 | 1.00× |
| lazy-ruby | 5 | 0.202 | 0.198 | 0.214 | 0.007 | 0.205 | 0.189 | 63.1 | 1.04× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.213 | 0.209 | 0.215 | 0.003 | 0.212 | 0.200 | 62.1 | 1.00× |
| lazy-ruby | 3 | 0.921 | 0.919 | 0.963 | 0.025 | 0.934 | 0.994 | 55.5 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.423 | 0.417 | 0.436 | 0.008 | 0.424 | 0.404 | 63.3 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.405 | 0.438 | 0.013 | 0.423 | 0.408 | 62.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.445 | 0.420 | 0.454 | 0.018 | 0.440 | 0.432 | 63.5 | 1.00× |
| lazy-ruby | 3 | 1.101 | 1.057 | 1.151 | 0.047 | 1.103 | 1.170 | 49.4 | 0.40× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.214 | 0.211 | 0.235 | 0.011 | 0.219 | 0.206 | 164.9 | 1.00× |
| lazy-ruby | 5 | 0.218 | 0.202 | 0.255 | 0.021 | 0.220 | 0.205 | 165.0 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.211 | 0.208 | 0.214 | 0.003 | 0.211 | 0.198 | 164.3 | 1.00× |
| lazy-ruby | 3 | 1.021 | 0.997 | 1.084 | 0.045 | 1.034 | 1.103 | 152.8 | 0.21× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.445 | 0.429 | 0.645 | 0.090 | 0.491 | 0.424 | 91.9 | 1.00× |
| lazy-ruby | 5 | 0.423 | 0.415 | 0.490 | 0.032 | 0.442 | 0.408 | 92.7 | 1.05× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.411 | 0.401 | 0.412 | 0.006 | 0.408 | 0.390 | 95.9 | 1.00× |
| lazy-ruby | 3 | 1.202 | 1.169 | 1.256 | 0.044 | 1.209 | 1.272 | 64.8 | 0.34× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.350 | 0.339 | 0.358 | 0.008 | 0.350 | 0.345 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.351 | 0.338 | 0.356 | 0.007 | 0.349 | 0.341 | 57.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.354 | 0.352 | 0.358 | 0.003 | 0.354 | 0.348 | 57.2 | 1.00× |
| lazy-ruby | 3 | 3.281 | 3.241 | 3.313 | 0.036 | 3.278 | 3.373 | 42.9 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.717 | 0.708 | 0.739 | 0.014 | 0.723 | 0.709 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.733 | 0.687 | 0.755 | 0.027 | 0.726 | 0.727 | 57.4 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.731 | 0.712 | 0.746 | 0.017 | 0.729 | 0.725 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.708 | 3.700 | 3.721 | 0.010 | 3.710 | 3.808 | 43.2 | 0.20× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.414 | 0.405 | 0.496 | 0.038 | 0.429 | 0.400 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.414 | 0.400 | 0.414 | 0.007 | 0.410 | 0.401 | 142.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.406 | 0.401 | 0.408 | 0.003 | 0.405 | 0.394 | 141.6 | 1.00× |
| lazy-ruby | 3 | 3.370 | 3.254 | 3.476 | 0.111 | 3.366 | 3.474 | 128.7 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.659 | 0.656 | 0.697 | 0.018 | 0.666 | 0.652 | 82.8 | 1.00× |
| lazy-ruby | 5 | 0.655 | 0.646 | 0.708 | 0.025 | 0.664 | 0.644 | 82.8 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.655 | 0.653 | 0.696 | 0.024 | 0.668 | 0.649 | 83.4 | 1.00× |
| lazy-ruby | 3 | 3.553 | 3.540 | 3.703 | 0.091 | 3.599 | 3.660 | 46.8 | 0.18× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.62

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.162 | 0.158 | 0.193 | 0.014 | 0.167 | 0.156 | 33.6 | 1.00× |
| lazy-ruby | 5 | 0.165 | 0.161 | 0.169 | 0.004 | 0.165 | 0.172 | 33.6 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.161 | 0.159 | 0.162 | 0.002 | 0.161 | 0.156 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.550 | 0.539 | 0.588 | 0.026 | 0.559 | 0.562 | 25.6 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.264 | 0.259 | 0.304 | 0.019 | 0.273 | 0.266 | 33.4 | 1.00× |
| lazy-ruby | 5 | 0.268 | 0.261 | 0.280 | 0.007 | 0.269 | 0.266 | 33.4 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.256 | 0.267 | 0.006 | 0.262 | 0.250 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.649 | 0.631 | 0.651 | 0.011 | 0.644 | 0.688 | 25.5 | 0.40× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.206 | 0.198 | 0.211 | 0.005 | 0.206 | 0.203 | 140.6 | 1.00× |
| lazy-ruby | 5 | 0.207 | 0.199 | 0.207 | 0.004 | 0.205 | 0.203 | 140.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.204 | 0.198 | 0.209 | 0.005 | 0.204 | 0.203 | 141.0 | 1.00× |
| lazy-ruby | 3 | 0.588 | 0.586 | 0.627 | 0.023 | 0.600 | 0.625 | 132.9 | 0.35× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.342 | 0.337 | 0.348 | 0.005 | 0.343 | 0.344 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.341 | 0.334 | 0.381 | 0.020 | 0.352 | 0.344 | 75.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.346 | 0.345 | 0.353 | 0.004 | 0.348 | 0.359 | 75.5 | 1.00× |
| lazy-ruby | 3 | 0.728 | 0.727 | 0.730 | 0.002 | 0.728 | 0.734 | 50.8 | 0.48× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

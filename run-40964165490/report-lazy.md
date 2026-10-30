# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.227 | 0.233 | 0.003 | 0.231 | 0.230 | 108.5 | 1.00× |
| lazy-ruby | 5 | 0.231 | 0.229 | 0.233 | 0.002 | 0.231 | 0.231 | 108.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.231 | 0.233 | 0.001 | 0.231 | 0.230 | 108.5 | 1.00× |
| lazy-ruby | 3 | 2.369 | 2.356 | 2.370 | 0.008 | 2.365 | 2.504 | 108.5 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.438 | 0.434 | 0.453 | 0.007 | 0.440 | 0.436 | 108.5 | 1.00× |
| lazy-ruby | 5 | 0.435 | 0.432 | 0.436 | 0.002 | 0.435 | 0.433 | 108.5 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.434 | 0.438 | 0.002 | 0.436 | 0.434 | 108.5 | 1.00× |
| lazy-ruby | 3 | 2.567 | 2.561 | 2.569 | 0.004 | 2.565 | 2.701 | 108.5 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.257 | 0.262 | 0.002 | 0.260 | 0.260 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.260 | 0.259 | 0.264 | 0.002 | 0.261 | 0.259 | 180.3 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.264 | 0.261 | 0.264 | 0.001 | 0.263 | 0.262 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.416 | 2.392 | 2.442 | 0.025 | 2.416 | 2.554 | 155.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.416 | 0.414 | 0.422 | 0.003 | 0.416 | 0.415 | 108.5 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.414 | 0.420 | 0.002 | 0.417 | 0.416 | 108.5 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.414 | 0.414 | 0.418 | 0.002 | 0.415 | 0.413 | 108.5 | 1.00× |
| lazy-ruby | 3 | 2.540 | 2.533 | 2.542 | 0.005 | 2.538 | 2.676 | 108.5 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.196 | 0.194 | 0.204 | 0.004 | 0.198 | 0.195 | 114.5 | 1.00× |
| lazy-ruby | 5 | 0.196 | 0.194 | 0.203 | 0.003 | 0.197 | 0.196 | 114.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.194 | 0.200 | 0.003 | 0.197 | 0.195 | 114.5 | 1.00× |
| lazy-ruby | 3 | 4.748 | 4.632 | 4.801 | 0.087 | 4.727 | 4.757 | 114.5 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.324 | 0.315 | 0.332 | 0.007 | 0.324 | 0.322 | 114.5 | 1.00× |
| lazy-ruby | 5 | 0.307 | 0.297 | 0.320 | 0.010 | 0.308 | 0.305 | 114.5 | 1.05× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.326 | 0.326 | 0.328 | 0.001 | 0.326 | 0.324 | 114.5 | 1.00× |
| lazy-ruby | 3 | 4.865 | 4.864 | 4.949 | 0.049 | 4.893 | 4.915 | 114.5 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.229 | 0.273 | 0.018 | 0.243 | 0.233 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.227 | 0.235 | 0.004 | 0.233 | 0.233 | 188.9 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.231 | 0.235 | 0.002 | 0.233 | 0.233 | 188.6 | 1.00× |
| lazy-ruby | 3 | 4.855 | 4.785 | 5.006 | 0.113 | 4.882 | 4.915 | 157.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.347 | 0.341 | 0.378 | 0.015 | 0.352 | 0.346 | 114.5 | 1.00× |
| lazy-ruby | 5 | 0.344 | 0.343 | 0.377 | 0.015 | 0.353 | 0.343 | 114.5 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.357 | 0.350 | 0.359 | 0.005 | 0.356 | 0.356 | 114.5 | 1.00× |
| lazy-ruby | 3 | 4.937 | 4.925 | 4.965 | 0.021 | 4.943 | 4.978 | 114.5 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.233 | 0.237 | 0.002 | 0.236 | 0.235 | 74.7 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.233 | 0.237 | 0.002 | 0.235 | 0.234 | 74.7 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.237 | 0.237 | 0.000 | 0.237 | 0.235 | 74.7 | 1.00× |
| lazy-ruby | 3 | 2.248 | 2.242 | 2.250 | 0.004 | 2.247 | 2.360 | 74.7 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.444 | 0.443 | 0.447 | 0.002 | 0.445 | 0.443 | 74.7 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.443 | 0.451 | 0.003 | 0.446 | 0.445 | 74.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.445 | 0.442 | 0.445 | 0.001 | 0.444 | 0.443 | 74.7 | 1.00× |
| lazy-ruby | 3 | 2.452 | 2.450 | 2.457 | 0.004 | 2.453 | 2.562 | 74.7 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.258 | 0.255 | 0.301 | 0.020 | 0.266 | 0.257 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.257 | 0.255 | 0.260 | 0.001 | 0.257 | 0.256 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.258 | 0.255 | 0.264 | 0.004 | 0.259 | 0.257 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.262 | 2.261 | 2.264 | 0.002 | 2.262 | 2.375 | 156.2 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.426 | 0.432 | 0.003 | 0.429 | 0.429 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.426 | 0.432 | 0.002 | 0.429 | 0.427 | 101.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.426 | 0.424 | 0.430 | 0.003 | 0.427 | 0.425 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.450 | 2.445 | 2.454 | 0.004 | 2.450 | 2.556 | 74.7 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.250 | 0.248 | 0.252 | 0.002 | 0.249 | 0.248 | 80.0 | 1.00× |
| lazy-ruby | 5 | 0.250 | 0.250 | 0.278 | 0.012 | 0.256 | 0.249 | 80.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.252 | 0.250 | 0.252 | 0.001 | 0.251 | 0.250 | 80.0 | 1.00× |
| lazy-ruby | 3 | 5.989 | 5.767 | 6.033 | 0.142 | 5.930 | 5.989 | 80.0 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.413 | 0.417 | 0.002 | 0.415 | 0.414 | 80.0 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.415 | 0.415 | 0.000 | 0.415 | 0.414 | 80.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.413 | 0.413 | 0.417 | 0.002 | 0.414 | 0.411 | 80.0 | 1.00× |
| lazy-ruby | 3 | 6.034 | 5.957 | 6.109 | 0.076 | 6.033 | 6.037 | 80.0 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.283 | 0.277 | 0.285 | 0.004 | 0.281 | 0.282 | 185.3 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.281 | 0.285 | 0.002 | 0.283 | 0.281 | 185.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.283 | 0.281 | 0.287 | 0.003 | 0.283 | 0.282 | 185.4 | 1.00× |
| lazy-ruby | 3 | 5.864 | 5.778 | 5.967 | 0.095 | 5.870 | 5.866 | 156.0 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.444 | 0.441 | 0.448 | 0.003 | 0.445 | 0.443 | 105.1 | 1.00× |
| lazy-ruby | 5 | 0.443 | 0.441 | 0.446 | 0.002 | 0.444 | 0.443 | 105.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.443 | 0.441 | 0.446 | 0.003 | 0.444 | 0.442 | 105.3 | 1.00× |
| lazy-ruby | 3 | 5.933 | 5.877 | 5.945 | 0.036 | 5.918 | 5.936 | 80.0 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.207 | 0.190 | 0.211 | 0.009 | 0.205 | 0.191 | 64.0 | 1.00× |
| lazy-ruby | 5 | 0.204 | 0.198 | 0.214 | 0.006 | 0.205 | 0.192 | 63.4 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.206 | 0.191 | 0.215 | 0.012 | 0.204 | 0.186 | 63.5 | 1.00× |
| lazy-ruby | 3 | 0.988 | 0.945 | 1.003 | 0.030 | 0.979 | 1.062 | 57.0 | 0.21× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.412 | 0.504 | 0.038 | 0.438 | 0.402 | 63.8 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.419 | 0.472 | 0.021 | 0.438 | 0.418 | 65.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.391 | 0.411 | 0.011 | 0.403 | 0.389 | 63.5 | 1.00× |
| lazy-ruby | 3 | 1.117 | 1.115 | 1.168 | 0.030 | 1.133 | 1.177 | 51.8 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.220 | 0.209 | 0.229 | 0.010 | 0.219 | 0.202 | 164.5 | 1.00× |
| lazy-ruby | 5 | 0.217 | 0.207 | 0.224 | 0.007 | 0.216 | 0.200 | 165.1 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.217 | 0.210 | 0.227 | 0.009 | 0.218 | 0.201 | 165.8 | 1.00× |
| lazy-ruby | 3 | 0.928 | 0.898 | 0.937 | 0.021 | 0.921 | 0.998 | 146.1 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.412 | 0.387 | 0.446 | 0.024 | 0.413 | 0.390 | 99.4 | 1.00× |
| lazy-ruby | 5 | 0.401 | 0.396 | 0.437 | 0.019 | 0.412 | 0.383 | 91.8 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.399 | 0.375 | 0.404 | 0.015 | 0.392 | 0.381 | 92.7 | 1.00× |
| lazy-ruby | 3 | 1.135 | 1.124 | 1.145 | 0.010 | 1.134 | 1.204 | 68.6 | 0.35× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.485 | 0.463 | 0.530 | 0.028 | 0.492 | 0.476 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.451 | 0.426 | 0.501 | 0.029 | 0.460 | 0.435 | 57.3 | 1.08× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.454 | 0.478 | 0.013 | 0.463 | 0.444 | 57.4 | 1.00× |
| lazy-ruby | 3 | 4.287 | 3.450 | 4.348 | 0.501 | 4.028 | 4.196 | 42.8 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.778 | 0.775 | 0.780 | 0.002 | 0.778 | 0.767 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.784 | 0.774 | 0.885 | 0.046 | 0.805 | 0.770 | 57.1 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.791 | 0.778 | 0.808 | 0.015 | 0.792 | 0.784 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.060 | 4.491 | 5.169 | 0.364 | 4.907 | 5.204 | 42.9 | 0.16× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.601 | 0.582 | 0.665 | 0.034 | 0.608 | 0.586 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.594 | 0.580 | 0.632 | 0.020 | 0.599 | 0.577 | 141.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.572 | 0.549 | 0.621 | 0.037 | 0.580 | 0.566 | 142.0 | 1.00× |
| lazy-ruby | 3 | 4.797 | 4.455 | 4.963 | 0.259 | 4.738 | 4.949 | 126.9 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.925 | 0.851 | 1.012 | 0.061 | 0.932 | 0.913 | 82.8 | 1.00× |
| lazy-ruby | 5 | 0.954 | 0.865 | 1.119 | 0.101 | 0.964 | 0.938 | 83.0 | 0.97× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.927 | 0.915 | 0.948 | 0.016 | 0.930 | 0.912 | 82.8 | 1.00× |
| lazy-ruby | 3 | 4.671 | 4.560 | 4.890 | 0.168 | 4.707 | 4.844 | 46.7 | 0.20× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.67

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.187 | 0.185 | 0.188 | 0.001 | 0.187 | 0.188 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.187 | 0.187 | 0.191 | 0.002 | 0.188 | 0.188 | 68.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.186 | 0.185 | 0.194 | 0.005 | 0.188 | 0.188 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.650 | 0.645 | 0.656 | 0.006 | 0.651 | 0.672 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.343 | 0.339 | 0.359 | 0.008 | 0.347 | 0.359 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.345 | 0.342 | 0.352 | 0.005 | 0.347 | 0.344 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.345 | 0.344 | 0.349 | 0.003 | 0.346 | 0.344 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.855 | 0.813 | 0.958 | 0.074 | 0.875 | 0.875 | 58.7 | 0.40× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.238 | 0.237 | 0.247 | 0.004 | 0.240 | 0.234 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.237 | 0.247 | 0.004 | 0.241 | 0.234 | 109.4 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.242 | 0.238 | 0.243 | 0.002 | 0.241 | 0.234 | 109.6 | 1.00× |
| lazy-ruby | 3 | 0.704 | 0.695 | 0.706 | 0.006 | 0.702 | 0.719 | 100.6 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.399 | 0.395 | 0.400 | 0.003 | 0.398 | 0.406 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.398 | 0.395 | 0.402 | 0.002 | 0.398 | 0.391 | 109.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.398 | 0.396 | 0.405 | 0.004 | 0.399 | 0.406 | 109.2 | 1.00× |
| lazy-ruby | 3 | 0.858 | 0.856 | 0.887 | 0.018 | 0.867 | 0.906 | 83.0 | 0.46× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

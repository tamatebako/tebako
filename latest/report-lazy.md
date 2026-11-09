# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.230 | 0.229 | 0.232 | 0.001 | 0.230 | 0.229 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.229 | 0.229 | 0.229 | 0.000 | 0.229 | 0.228 | 70.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.229 | 0.229 | 0.229 | 0.000 | 0.229 | 0.228 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.368 | 2.366 | 2.385 | 0.010 | 2.373 | 2.503 | 49.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.435 | 0.433 | 0.438 | 0.002 | 0.435 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.437 | 0.431 | 0.438 | 0.003 | 0.436 | 0.435 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.434 | 0.437 | 0.002 | 0.435 | 0.434 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.568 | 2.557 | 2.576 | 0.010 | 2.567 | 2.703 | 48.7 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.258 | 0.265 | 0.003 | 0.262 | 0.261 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.261 | 0.258 | 0.267 | 0.003 | 0.262 | 0.260 | 180.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.257 | 0.257 | 0.259 | 0.001 | 0.257 | 0.257 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.392 | 2.384 | 2.411 | 0.014 | 2.396 | 2.527 | 158.3 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.411 | 0.417 | 0.002 | 0.415 | 0.413 | 102.0 | 1.00× |
| lazy-ruby | 5 | 0.412 | 0.410 | 0.419 | 0.004 | 0.414 | 0.412 | 102.3 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.416 | 0.414 | 0.417 | 0.001 | 0.416 | 0.415 | 102.1 | 1.00× |
| lazy-ruby | 3 | 2.545 | 2.543 | 2.554 | 0.006 | 2.547 | 2.681 | 62.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.282 | 0.280 | 0.282 | 0.001 | 0.281 | 0.280 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.282 | 0.280 | 0.291 | 0.004 | 0.284 | 0.281 | 79.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.282 | 0.282 | 0.285 | 0.002 | 0.283 | 0.281 | 80.4 | 1.00× |
| lazy-ruby | 3 | 6.849 | 6.839 | 6.869 | 0.015 | 6.852 | 6.927 | 51.4 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.500 | 0.500 | 0.510 | 0.005 | 0.502 | 0.499 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.500 | 0.498 | 0.511 | 0.005 | 0.503 | 0.500 | 79.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.504 | 0.500 | 0.506 | 0.003 | 0.503 | 0.502 | 79.8 | 1.00× |
| lazy-ruby | 3 | 7.073 | 7.052 | 7.073 | 0.012 | 7.066 | 7.132 | 51.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.319 | 0.317 | 0.324 | 0.003 | 0.320 | 0.318 | 188.6 | 1.00× |
| lazy-ruby | 5 | 0.320 | 0.316 | 0.322 | 0.002 | 0.319 | 0.319 | 188.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.317 | 0.316 | 0.318 | 0.001 | 0.317 | 0.316 | 184.0 | 1.00× |
| lazy-ruby | 3 | 6.878 | 6.834 | 6.885 | 0.028 | 6.866 | 6.939 | 157.3 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.491 | 0.483 | 0.492 | 0.004 | 0.488 | 0.488 | 111.9 | 1.00× |
| lazy-ruby | 5 | 0.486 | 0.484 | 0.488 | 0.002 | 0.486 | 0.484 | 111.7 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.492 | 0.483 | 0.496 | 0.006 | 0.490 | 0.491 | 112.1 | 1.00× |
| lazy-ruby | 3 | 7.055 | 7.001 | 7.081 | 0.041 | 7.046 | 7.125 | 64.7 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.238 | 0.232 | 0.240 | 0.003 | 0.237 | 0.237 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.238 | 0.235 | 0.240 | 0.002 | 0.238 | 0.236 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.238 | 0.240 | 0.001 | 0.238 | 0.237 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.264 | 2.257 | 2.267 | 0.005 | 2.262 | 2.376 | 47.2 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.444 | 0.446 | 0.001 | 0.445 | 0.444 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.442 | 0.448 | 0.002 | 0.445 | 0.444 | 72.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.446 | 0.446 | 0.446 | 0.000 | 0.446 | 0.444 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.467 | 2.465 | 2.468 | 0.001 | 2.467 | 2.577 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.263 | 0.262 | 0.269 | 0.003 | 0.265 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.263 | 0.263 | 0.263 | 0.000 | 0.263 | 0.262 | 182.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.263 | 0.261 | 0.267 | 0.003 | 0.264 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.280 | 2.270 | 2.282 | 0.006 | 2.277 | 2.392 | 156.1 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.429 | 0.427 | 0.433 | 0.002 | 0.430 | 0.428 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.429 | 0.427 | 0.430 | 0.001 | 0.429 | 0.428 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.427 | 0.427 | 0.429 | 0.001 | 0.428 | 0.427 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.454 | 2.452 | 2.460 | 0.004 | 2.455 | 2.564 | 62.7 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.251 | 0.251 | 0.253 | 0.001 | 0.251 | 0.250 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.247 | 0.244 | 0.252 | 0.003 | 0.248 | 0.246 | 75.2 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.252 | 0.257 | 0.003 | 0.254 | 0.251 | 75.2 | 1.00× |
| lazy-ruby | 3 | 5.791 | 5.780 | 5.859 | 0.043 | 5.810 | 5.792 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.418 | 0.424 | 0.003 | 0.421 | 0.418 | 75.4 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.413 | 0.416 | 0.001 | 0.415 | 0.414 | 75.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.418 | 0.422 | 0.002 | 0.419 | 0.417 | 75.3 | 1.00× |
| lazy-ruby | 3 | 5.907 | 5.867 | 6.127 | 0.140 | 5.967 | 5.911 | 49.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.284 | 0.284 | 0.286 | 0.001 | 0.285 | 0.283 | 185.8 | 1.00× |
| lazy-ruby | 5 | 0.282 | 0.278 | 0.286 | 0.003 | 0.282 | 0.281 | 185.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.284 | 0.284 | 0.286 | 0.001 | 0.285 | 0.284 | 185.8 | 1.00× |
| lazy-ruby | 3 | 5.890 | 5.890 | 6.075 | 0.107 | 5.952 | 5.896 | 156.2 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.447 | 0.445 | 0.452 | 0.003 | 0.447 | 0.446 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.441 | 0.440 | 0.443 | 0.001 | 0.442 | 0.440 | 105.4 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.449 | 0.447 | 0.449 | 0.001 | 0.448 | 0.447 | 105.6 | 1.00× |
| lazy-ruby | 3 | 5.936 | 5.931 | 5.994 | 0.035 | 5.954 | 5.938 | 63.0 | 0.08× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.243 | 0.224 | 0.250 | 0.010 | 0.241 | 0.225 | 63.7 | 1.00× |
| lazy-ruby | 5 | 0.250 | 0.220 | 0.255 | 0.016 | 0.240 | 0.228 | 61.9 | 0.97× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.256 | 0.245 | 0.262 | 0.009 | 0.255 | 0.245 | 64.4 | 1.00× |
| lazy-ruby | 3 | 1.079 | 1.042 | 1.186 | 0.075 | 1.102 | 1.130 | 49.8 | 0.24× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.497 | 0.427 | 0.505 | 0.033 | 0.478 | 0.473 | 61.7 | 1.00× |
| lazy-ruby | 5 | 0.481 | 0.457 | 0.494 | 0.017 | 0.475 | 0.455 | 61.3 | 1.03× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.463 | 0.438 | 0.475 | 0.019 | 0.458 | 0.441 | 61.5 | 1.00× |
| lazy-ruby | 3 | 1.199 | 1.127 | 1.334 | 0.105 | 1.220 | 1.237 | 51.9 | 0.39× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.259 | 0.248 | 0.273 | 0.010 | 0.258 | 0.236 | 163.7 | 1.00× |
| lazy-ruby | 5 | 0.260 | 0.233 | 0.269 | 0.015 | 0.254 | 0.236 | 163.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.218 | 0.236 | 0.010 | 0.229 | 0.211 | 163.3 | 1.00× |
| lazy-ruby | 3 | 0.912 | 0.907 | 0.951 | 0.024 | 0.923 | 0.964 | 150.0 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.405 | 0.376 | 0.479 | 0.040 | 0.421 | 0.394 | 93.1 | 1.00× |
| lazy-ruby | 5 | 0.399 | 0.371 | 0.419 | 0.019 | 0.393 | 0.378 | 91.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.376 | 0.370 | 0.399 | 0.015 | 0.382 | 0.371 | 97.2 | 1.00× |
| lazy-ruby | 3 | 1.183 | 1.132 | 1.289 | 0.080 | 1.201 | 1.268 | 67.7 | 0.32× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.471 | 0.440 | 0.595 | 0.063 | 0.498 | 0.456 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.478 | 0.437 | 0.515 | 0.031 | 0.472 | 0.469 | 57.1 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.619 | 0.519 | 0.701 | 0.091 | 0.613 | 0.604 | 57.2 | 1.00× |
| lazy-ruby | 3 | 4.480 | 4.136 | 4.794 | 0.329 | 4.470 | 4.589 | 42.7 | 0.14× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.985 | 0.887 | 1.420 | 0.214 | 1.049 | 0.966 | 57.3 | 1.00× |
| lazy-ruby | 5 | 0.969 | 0.910 | 1.256 | 0.145 | 1.023 | 0.952 | 57.2 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.045 | 0.874 | 1.159 | 0.143 | 1.026 | 1.020 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.081 | 4.737 | 5.166 | 0.227 | 4.995 | 5.109 | 42.9 | 0.21× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.535 | 0.521 | 0.611 | 0.037 | 0.548 | 0.527 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.532 | 0.508 | 0.591 | 0.031 | 0.539 | 0.516 | 141.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.653 | 0.584 | 0.909 | 0.171 | 0.716 | 0.644 | 142.3 | 1.00× |
| lazy-ruby | 3 | 4.408 | 4.291 | 4.551 | 0.130 | 4.417 | 4.564 | 127.0 | 0.15× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.777 | 0.744 | 0.860 | 0.046 | 0.786 | 0.765 | 83.0 | 1.00× |
| lazy-ruby | 5 | 1.006 | 0.731 | 1.123 | 0.171 | 0.946 | 0.996 | 82.8 | 0.77× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.758 | 0.739 | 0.891 | 0.083 | 0.796 | 0.746 | 82.9 | 1.00× |
| lazy-ruby | 3 | 4.259 | 4.046 | 4.742 | 0.357 | 4.349 | 4.357 | 46.7 | 0.18× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.74

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.187 | 0.187 | 0.192 | 0.003 | 0.189 | 0.188 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.188 | 0.185 | 0.190 | 0.002 | 0.187 | 0.188 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.189 | 0.186 | 0.190 | 0.002 | 0.188 | 0.188 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.653 | 0.650 | 0.662 | 0.006 | 0.655 | 0.672 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.340 | 0.339 | 0.343 | 0.002 | 0.341 | 0.328 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.342 | 0.341 | 0.344 | 0.001 | 0.342 | 0.328 | 68.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.344 | 0.342 | 0.358 | 0.009 | 0.348 | 0.359 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.806 | 0.805 | 0.808 | 0.001 | 0.806 | 0.844 | 58.5 | 0.43× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.236 | 0.242 | 0.003 | 0.238 | 0.234 | 108.7 | 1.00× |
| lazy-ruby | 5 | 0.236 | 0.235 | 0.238 | 0.001 | 0.236 | 0.234 | 108.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.242 | 0.240 | 0.243 | 0.001 | 0.242 | 0.250 | 109.4 | 1.00× |
| lazy-ruby | 3 | 0.695 | 0.695 | 0.696 | 0.001 | 0.695 | 0.734 | 100.5 | 0.35× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.393 | 0.392 | 0.409 | 0.007 | 0.397 | 0.391 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.396 | 0.395 | 0.402 | 0.003 | 0.398 | 0.391 | 108.7 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.396 | 0.395 | 0.398 | 0.001 | 0.396 | 0.391 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.854 | 0.848 | 0.856 | 0.004 | 0.853 | 0.875 | 82.9 | 0.46× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

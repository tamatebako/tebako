# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.244 | 0.233 | 0.246 | 0.005 | 0.242 | 0.243 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.244 | 0.242 | 0.246 | 0.001 | 0.244 | 0.243 | 70.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.244 | 0.236 | 0.246 | 0.005 | 0.242 | 0.244 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.401 | 2.391 | 2.403 | 0.006 | 2.398 | 2.535 | 49.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.448 | 0.446 | 0.448 | 0.001 | 0.447 | 0.446 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.444 | 0.446 | 0.001 | 0.445 | 0.444 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.444 | 0.442 | 0.450 | 0.004 | 0.445 | 0.443 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.601 | 2.589 | 2.616 | 0.013 | 2.602 | 2.736 | 49.5 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.282 | 0.279 | 0.282 | 0.001 | 0.281 | 0.281 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.282 | 0.280 | 0.284 | 0.001 | 0.282 | 0.281 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.284 | 0.280 | 0.284 | 0.003 | 0.283 | 0.282 | 180.4 | 1.00× |
| lazy-ruby | 3 | 2.439 | 2.430 | 2.448 | 0.009 | 2.439 | 2.577 | 157.8 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.437 | 0.436 | 0.440 | 0.002 | 0.437 | 0.436 | 102.0 | 1.00× |
| lazy-ruby | 5 | 0.439 | 0.437 | 0.440 | 0.001 | 0.438 | 0.437 | 102.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.437 | 0.438 | 0.000 | 0.438 | 0.437 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.588 | 2.579 | 2.603 | 0.012 | 2.590 | 2.722 | 62.3 | 0.17× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.232 | 0.230 | 0.234 | 0.002 | 0.232 | 0.230 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.230 | 0.228 | 0.232 | 0.001 | 0.230 | 0.229 | 79.8 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.230 | 0.229 | 0.230 | 0.001 | 0.229 | 0.228 | 79.7 | 1.00× |
| lazy-ruby | 3 | 5.365 | 5.341 | 5.504 | 0.088 | 5.403 | 5.413 | 51.5 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.397 | 0.392 | 0.401 | 0.004 | 0.396 | 0.395 | 80.4 | 1.00× |
| lazy-ruby | 5 | 0.396 | 0.393 | 0.400 | 0.003 | 0.397 | 0.395 | 79.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.395 | 0.394 | 0.396 | 0.001 | 0.395 | 0.394 | 80.1 | 1.00× |
| lazy-ruby | 3 | 5.440 | 5.428 | 5.545 | 0.064 | 5.471 | 5.491 | 51.5 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.267 | 0.263 | 0.268 | 0.002 | 0.266 | 0.266 | 188.6 | 1.00× |
| lazy-ruby | 5 | 0.267 | 0.263 | 0.274 | 0.004 | 0.267 | 0.265 | 188.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.269 | 0.267 | 0.272 | 0.002 | 0.269 | 0.269 | 188.5 | 1.00× |
| lazy-ruby | 3 | 5.365 | 5.297 | 5.536 | 0.123 | 5.399 | 5.415 | 157.3 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.396 | 0.392 | 0.401 | 0.003 | 0.396 | 0.394 | 112.1 | 1.00× |
| lazy-ruby | 5 | 0.394 | 0.390 | 0.399 | 0.004 | 0.394 | 0.393 | 111.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.402 | 0.398 | 0.404 | 0.003 | 0.401 | 0.400 | 111.9 | 1.00× |
| lazy-ruby | 3 | 5.693 | 5.520 | 5.722 | 0.109 | 5.645 | 5.743 | 64.7 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.244 | 0.244 | 0.244 | 0.000 | 0.244 | 0.242 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.244 | 0.238 | 0.244 | 0.003 | 0.242 | 0.242 | 72.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.238 | 0.238 | 0.000 | 0.238 | 0.237 | 72.2 | 1.00× |
| lazy-ruby | 3 | 2.244 | 2.240 | 2.253 | 0.006 | 2.246 | 2.351 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.450 | 0.447 | 0.456 | 0.004 | 0.451 | 0.448 | 72.3 | 1.00× |
| lazy-ruby | 5 | 0.446 | 0.446 | 0.449 | 0.002 | 0.447 | 0.445 | 72.3 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.446 | 0.446 | 0.448 | 0.001 | 0.446 | 0.445 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.459 | 2.455 | 2.489 | 0.018 | 2.468 | 2.566 | 47.2 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.261 | 0.269 | 0.003 | 0.265 | 0.263 | 182.5 | 1.00× |
| lazy-ruby | 5 | 0.263 | 0.261 | 0.267 | 0.002 | 0.264 | 0.261 | 182.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.267 | 0.263 | 0.267 | 0.002 | 0.265 | 0.265 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.279 | 2.268 | 2.283 | 0.007 | 2.277 | 2.388 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.431 | 0.431 | 0.433 | 0.001 | 0.431 | 0.430 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.433 | 0.431 | 0.435 | 0.001 | 0.433 | 0.431 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.429 | 0.429 | 0.431 | 0.001 | 0.430 | 0.428 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.445 | 2.432 | 2.466 | 0.017 | 2.447 | 2.554 | 62.7 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.326 | 0.321 | 0.334 | 0.005 | 0.326 | 0.324 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.330 | 0.323 | 0.343 | 0.008 | 0.331 | 0.328 | 74.9 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.323 | 0.323 | 0.330 | 0.004 | 0.325 | 0.322 | 75.2 | 1.00× |
| lazy-ruby | 3 | 7.579 | 7.502 | 7.712 | 0.107 | 7.598 | 7.641 | 49.7 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.539 | 0.533 | 0.682 | 0.065 | 0.566 | 0.538 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.537 | 0.535 | 0.544 | 0.004 | 0.539 | 0.535 | 75.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.540 | 0.537 | 0.541 | 0.002 | 0.539 | 0.538 | 75.1 | 1.00× |
| lazy-ruby | 3 | 7.925 | 7.742 | 7.980 | 0.125 | 7.882 | 7.931 | 49.6 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.368 | 0.361 | 0.368 | 0.003 | 0.366 | 0.366 | 185.3 | 1.00× |
| lazy-ruby | 5 | 0.364 | 0.363 | 0.372 | 0.004 | 0.366 | 0.364 | 185.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.365 | 0.365 | 0.365 | 0.000 | 0.365 | 0.364 | 186.0 | 1.00× |
| lazy-ruby | 3 | 7.561 | 7.482 | 7.591 | 0.056 | 7.545 | 7.570 | 155.3 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.575 | 0.573 | 0.582 | 0.004 | 0.576 | 0.574 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.577 | 0.573 | 0.581 | 0.004 | 0.577 | 0.574 | 104.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.580 | 0.579 | 0.585 | 0.003 | 0.581 | 0.578 | 105.4 | 1.00× |
| lazy-ruby | 3 | 7.759 | 7.718 | 7.964 | 0.132 | 7.813 | 7.762 | 63.0 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.224 | 0.213 | 0.229 | 0.006 | 0.223 | 0.204 | 63.2 | 1.00× |
| lazy-ruby | 5 | 0.221 | 0.213 | 0.228 | 0.006 | 0.220 | 0.203 | 61.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.225 | 0.221 | 0.247 | 0.014 | 0.231 | 0.218 | 63.6 | 1.00× |
| lazy-ruby | 3 | 1.083 | 0.964 | 1.112 | 0.079 | 1.053 | 1.145 | 57.8 | 0.21× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.485 | 0.428 | 0.508 | 0.030 | 0.479 | 0.460 | 63.5 | 1.00× |
| lazy-ruby | 5 | 0.482 | 0.472 | 0.529 | 0.024 | 0.492 | 0.469 | 61.8 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.540 | 0.460 | 0.543 | 0.047 | 0.515 | 0.515 | 62.3 | 1.00× |
| lazy-ruby | 3 | 1.223 | 1.149 | 1.361 | 0.108 | 1.244 | 1.273 | 49.5 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.234 | 0.274 | 0.015 | 0.254 | 0.228 | 164.2 | 1.00× |
| lazy-ruby | 5 | 0.264 | 0.225 | 0.272 | 0.022 | 0.252 | 0.235 | 163.7 | 0.96× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.261 | 0.254 | 0.263 | 0.005 | 0.259 | 0.229 | 162.3 | 1.00× |
| lazy-ruby | 3 | 0.967 | 0.953 | 0.993 | 0.020 | 0.971 | 1.030 | 153.9 | 0.27× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.443 | 0.413 | 0.612 | 0.083 | 0.472 | 0.426 | 95.9 | 1.00× |
| lazy-ruby | 5 | 0.442 | 0.393 | 0.589 | 0.075 | 0.472 | 0.433 | 93.5 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.435 | 0.451 | 0.009 | 0.441 | 0.426 | 91.3 | 1.00× |
| lazy-ruby | 3 | 1.308 | 1.180 | 1.339 | 0.084 | 1.276 | 1.356 | 68.0 | 0.33× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.529 | 0.417 | 0.606 | 0.071 | 0.512 | 0.514 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.484 | 0.430 | 0.545 | 0.048 | 0.483 | 0.468 | 57.4 | 1.09× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.584 | 0.556 | 0.591 | 0.018 | 0.577 | 0.574 | 57.0 | 1.00× |
| lazy-ruby | 3 | 3.858 | 3.668 | 5.324 | 0.906 | 4.283 | 4.019 | 43.0 | 0.15× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.096 | 1.033 | 1.111 | 0.035 | 1.080 | 1.085 | 57.2 | 1.00× |
| lazy-ruby | 5 | 1.081 | 1.054 | 1.316 | 0.109 | 1.122 | 1.064 | 57.1 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.073 | 1.057 | 1.132 | 0.039 | 1.088 | 1.061 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.755 | 5.597 | 5.940 | 0.172 | 5.764 | 5.933 | 42.8 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.659 | 0.583 | 0.843 | 0.103 | 0.670 | 0.653 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.628 | 0.618 | 0.654 | 0.017 | 0.634 | 0.620 | 141.7 | 1.05× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.599 | 0.568 | 0.619 | 0.026 | 0.595 | 0.587 | 141.9 | 1.00× |
| lazy-ruby | 3 | 4.677 | 4.543 | 4.802 | 0.129 | 4.674 | 4.854 | 126.7 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.967 | 0.928 | 1.204 | 0.111 | 1.011 | 0.956 | 82.9 | 1.00× |
| lazy-ruby | 5 | 1.105 | 0.910 | 1.234 | 0.128 | 1.064 | 1.091 | 82.8 | 0.88× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.996 | 0.994 | 1.006 | 0.007 | 0.998 | 0.981 | 82.9 | 1.00× |
| lazy-ruby | 3 | 5.202 | 5.097 | 5.218 | 0.066 | 5.172 | 5.373 | 46.8 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.75

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.244 | 0.237 | 0.246 | 0.004 | 0.243 | 0.250 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.238 | 0.242 | 0.002 | 0.240 | 0.234 | 68.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.238 | 0.240 | 0.001 | 0.239 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.815 | 0.811 | 0.820 | 0.005 | 0.815 | 0.828 | 58.7 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.459 | 0.457 | 0.472 | 0.006 | 0.462 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.463 | 0.461 | 0.470 | 0.004 | 0.465 | 0.453 | 68.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.463 | 0.461 | 0.466 | 0.003 | 0.463 | 0.453 | 68.5 | 1.00× |
| lazy-ruby | 3 | 1.044 | 1.029 | 1.052 | 0.012 | 1.042 | 1.062 | 58.8 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.293 | 0.291 | 0.297 | 0.002 | 0.293 | 0.281 | 109.2 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.287 | 0.289 | 0.001 | 0.288 | 0.297 | 109.3 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.293 | 0.292 | 0.293 | 0.001 | 0.293 | 0.312 | 109.5 | 1.00× |
| lazy-ruby | 3 | 0.871 | 0.857 | 0.873 | 0.009 | 0.867 | 0.891 | 99.8 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.506 | 0.504 | 0.512 | 0.003 | 0.507 | 0.500 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.509 | 0.506 | 0.513 | 0.003 | 0.509 | 0.516 | 109.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.511 | 0.505 | 0.518 | 0.006 | 0.511 | 0.500 | 109.1 | 1.00× |
| lazy-ruby | 3 | 1.066 | 1.063 | 1.077 | 0.007 | 1.069 | 1.094 | 82.9 | 0.48× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

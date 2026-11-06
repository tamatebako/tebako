# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.233 | 0.001 | 0.232 | 0.232 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.231 | 0.229 | 0.233 | 0.001 | 0.231 | 0.230 | 109.0 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.233 | 0.233 | 0.235 | 0.001 | 0.234 | 0.231 | 109.0 | 1.00× |
| lazy-ruby | 3 | 2.378 | 2.377 | 2.385 | 0.004 | 2.380 | 2.512 | 109.0 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.433 | 0.457 | 0.011 | 0.438 | 0.433 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.435 | 0.434 | 0.436 | 0.001 | 0.435 | 0.434 | 109.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.437 | 0.435 | 0.439 | 0.002 | 0.437 | 0.436 | 109.0 | 1.00× |
| lazy-ruby | 3 | 2.578 | 2.572 | 2.590 | 0.009 | 2.580 | 2.706 | 109.0 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.259 | 0.264 | 0.001 | 0.262 | 0.260 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.259 | 0.262 | 0.001 | 0.261 | 0.260 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.260 | 0.259 | 0.262 | 0.001 | 0.260 | 0.260 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.394 | 2.376 | 2.399 | 0.012 | 2.390 | 2.529 | 156.5 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.418 | 0.416 | 0.420 | 0.001 | 0.418 | 0.417 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.416 | 0.424 | 0.003 | 0.420 | 0.418 | 109.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.417 | 0.420 | 0.002 | 0.418 | 0.416 | 109.0 | 1.00× |
| lazy-ruby | 3 | 2.552 | 2.543 | 2.580 | 0.019 | 2.558 | 2.687 | 109.0 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.286 | 0.294 | 0.003 | 0.290 | 0.290 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.290 | 0.288 | 0.292 | 0.002 | 0.290 | 0.289 | 114.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.290 | 0.290 | 0.292 | 0.001 | 0.290 | 0.289 | 114.2 | 1.00× |
| lazy-ruby | 3 | 6.898 | 6.875 | 6.902 | 0.015 | 6.892 | 6.959 | 114.2 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.513 | 0.505 | 0.514 | 0.004 | 0.511 | 0.512 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.506 | 0.503 | 0.519 | 0.007 | 0.508 | 0.505 | 114.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.514 | 0.513 | 0.516 | 0.002 | 0.514 | 0.513 | 114.2 | 1.00× |
| lazy-ruby | 3 | 7.077 | 7.077 | 7.112 | 0.020 | 7.089 | 7.135 | 114.2 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.327 | 0.323 | 0.331 | 0.003 | 0.327 | 0.327 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.327 | 0.325 | 0.329 | 0.001 | 0.327 | 0.326 | 188.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.327 | 0.326 | 0.327 | 0.001 | 0.327 | 0.325 | 188.5 | 1.00× |
| lazy-ruby | 3 | 6.866 | 6.865 | 6.891 | 0.015 | 6.874 | 6.951 | 157.0 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.492 | 0.488 | 0.497 | 0.003 | 0.493 | 0.492 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.492 | 0.490 | 0.503 | 0.006 | 0.496 | 0.492 | 114.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.497 | 0.496 | 0.501 | 0.003 | 0.498 | 0.496 | 114.2 | 1.00× |
| lazy-ruby | 3 | 7.024 | 7.006 | 7.060 | 0.027 | 7.030 | 7.097 | 114.2 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.235 | 0.237 | 0.001 | 0.236 | 0.235 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.235 | 0.233 | 0.237 | 0.002 | 0.235 | 0.233 | 74.8 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.235 | 0.237 | 0.001 | 0.236 | 0.235 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.248 | 2.245 | 2.293 | 0.027 | 2.262 | 2.359 | 74.8 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.447 | 0.445 | 0.447 | 0.001 | 0.446 | 0.446 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.445 | 0.441 | 0.447 | 0.003 | 0.444 | 0.443 | 74.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.449 | 0.447 | 0.453 | 0.003 | 0.449 | 0.447 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.451 | 2.449 | 2.455 | 0.003 | 2.452 | 2.563 | 74.8 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.259 | 0.262 | 0.001 | 0.261 | 0.261 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.262 | 0.262 | 0.264 | 0.001 | 0.262 | 0.261 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.264 | 0.262 | 0.264 | 0.001 | 0.263 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.273 | 2.270 | 2.280 | 0.005 | 2.274 | 2.386 | 156.0 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.430 | 0.428 | 0.430 | 0.001 | 0.430 | 0.429 | 101.6 | 1.00× |
| lazy-ruby | 5 | 0.430 | 0.428 | 0.430 | 0.001 | 0.429 | 0.428 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.430 | 0.430 | 0.436 | 0.004 | 0.432 | 0.429 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.445 | 2.443 | 2.449 | 0.003 | 2.446 | 2.559 | 74.8 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.323 | 0.321 | 0.329 | 0.004 | 0.325 | 0.323 | 80.0 | 1.00× |
| lazy-ruby | 5 | 0.323 | 0.321 | 0.323 | 0.001 | 0.322 | 0.321 | 80.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.321 | 0.321 | 0.325 | 0.002 | 0.322 | 0.320 | 80.0 | 1.00× |
| lazy-ruby | 3 | 7.526 | 7.495 | 7.532 | 0.020 | 7.517 | 7.533 | 80.0 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.549 | 0.546 | 0.556 | 0.004 | 0.551 | 0.548 | 80.0 | 1.00× |
| lazy-ruby | 5 | 0.548 | 0.547 | 0.567 | 0.008 | 0.552 | 0.548 | 80.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.559 | 0.554 | 0.565 | 0.005 | 0.559 | 0.557 | 80.0 | 1.00× |
| lazy-ruby | 3 | 7.736 | 7.714 | 7.751 | 0.019 | 7.734 | 7.730 | 80.0 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.362 | 0.352 | 0.366 | 0.006 | 0.361 | 0.361 | 185.6 | 1.00× |
| lazy-ruby | 5 | 0.358 | 0.356 | 0.364 | 0.003 | 0.359 | 0.356 | 185.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.364 | 0.363 | 0.365 | 0.001 | 0.364 | 0.362 | 185.8 | 1.00× |
| lazy-ruby | 3 | 7.545 | 7.543 | 7.592 | 0.028 | 7.560 | 7.545 | 155.5 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.577 | 0.575 | 0.588 | 0.005 | 0.578 | 0.575 | 105.0 | 1.00× |
| lazy-ruby | 5 | 0.577 | 0.575 | 0.577 | 0.001 | 0.576 | 0.575 | 105.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.581 | 0.579 | 0.583 | 0.002 | 0.581 | 0.580 | 104.9 | 1.00× |
| lazy-ruby | 3 | 7.782 | 7.731 | 7.792 | 0.033 | 7.768 | 7.789 | 80.0 | 0.07× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.331 | 0.317 | 0.332 | 0.006 | 0.328 | 0.325 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.327 | 0.325 | 0.527 | 0.087 | 0.376 | 0.320 | 56.9 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.322 | 0.317 | 0.328 | 0.006 | 0.322 | 0.315 | 57.1 | 1.00× |
| lazy-ruby | 3 | 2.984 | 2.939 | 3.145 | 0.108 | 3.022 | 3.099 | 42.9 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.824 | 0.713 | 0.971 | 0.102 | 0.813 | 0.806 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.807 | 0.705 | 0.902 | 0.075 | 0.793 | 0.780 | 57.2 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.772 | 0.722 | 0.835 | 0.057 | 0.777 | 0.764 | 57.2 | 1.00× |
| lazy-ruby | 3 | 4.149 | 4.111 | 4.623 | 0.285 | 4.294 | 4.266 | 42.9 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.497 | 0.464 | 0.542 | 0.028 | 0.500 | 0.487 | 141.9 | 1.00× |
| lazy-ruby | 5 | 0.488 | 0.461 | 0.507 | 0.018 | 0.487 | 0.473 | 141.8 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.497 | 0.482 | 0.510 | 0.014 | 0.496 | 0.484 | 141.6 | 1.00× |
| lazy-ruby | 3 | 3.736 | 3.626 | 3.878 | 0.126 | 3.747 | 3.861 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.716 | 0.695 | 0.811 | 0.052 | 0.741 | 0.704 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.757 | 0.669 | 0.823 | 0.056 | 0.748 | 0.748 | 83.0 | 0.95× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.747 | 0.731 | 0.748 | 0.009 | 0.742 | 0.733 | 83.1 | 1.00× |
| lazy-ruby | 3 | 3.726 | 3.596 | 3.836 | 0.120 | 3.719 | 3.843 | 46.6 | 0.20× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.69

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.239 | 0.238 | 0.242 | 0.001 | 0.240 | 0.234 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.240 | 0.237 | 0.240 | 0.001 | 0.239 | 0.234 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.236 | 0.241 | 0.003 | 0.238 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.813 | 0.812 | 0.824 | 0.007 | 0.816 | 0.844 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.460 | 0.454 | 0.470 | 0.006 | 0.462 | 0.453 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.463 | 0.456 | 0.464 | 0.003 | 0.462 | 0.453 | 68.3 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.461 | 0.457 | 0.473 | 0.008 | 0.464 | 0.453 | 68.4 | 1.00× |
| lazy-ruby | 3 | 1.047 | 1.033 | 1.056 | 0.012 | 1.046 | 1.062 | 58.8 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.288 | 0.300 | 0.006 | 0.293 | 0.297 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.292 | 0.289 | 0.300 | 0.005 | 0.294 | 0.297 | 108.8 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.293 | 0.290 | 0.299 | 0.004 | 0.294 | 0.281 | 109.0 | 1.00× |
| lazy-ruby | 3 | 0.874 | 0.862 | 0.877 | 0.008 | 0.871 | 0.891 | 100.0 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.512 | 0.504 | 0.524 | 0.008 | 0.513 | 0.500 | 109.0 | 1.00× |
| lazy-ruby | 5 | 0.511 | 0.509 | 0.518 | 0.003 | 0.512 | 0.500 | 108.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.508 | 0.506 | 0.516 | 0.006 | 0.510 | 0.500 | 109.0 | 1.00× |
| lazy-ruby | 3 | 1.087 | 1.075 | 1.087 | 0.007 | 1.083 | 1.094 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

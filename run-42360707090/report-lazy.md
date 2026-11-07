# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.227 | 0.231 | 0.002 | 0.228 | 0.226 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.225 | 0.229 | 0.001 | 0.227 | 0.225 | 108.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.225 | 0.227 | 0.001 | 0.226 | 0.225 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.341 | 2.334 | 2.357 | 0.012 | 2.344 | 2.475 | 108.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.428 | 0.426 | 0.430 | 0.002 | 0.429 | 0.427 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.430 | 0.428 | 0.430 | 0.001 | 0.429 | 0.428 | 108.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.432 | 0.428 | 0.434 | 0.003 | 0.432 | 0.431 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.546 | 2.545 | 2.548 | 0.002 | 2.546 | 2.681 | 108.1 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.253 | 0.255 | 0.001 | 0.254 | 0.253 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.253 | 0.253 | 0.000 | 0.253 | 0.252 | 180.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.251 | 0.253 | 0.001 | 0.252 | 0.251 | 180.4 | 1.00× |
| lazy-ruby | 3 | 2.363 | 2.361 | 2.370 | 0.005 | 2.365 | 2.498 | 156.5 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.410 | 0.407 | 0.412 | 0.001 | 0.410 | 0.408 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.410 | 0.405 | 0.414 | 0.003 | 0.410 | 0.409 | 108.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.408 | 0.410 | 0.001 | 0.408 | 0.407 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.534 | 2.525 | 2.538 | 0.006 | 2.532 | 2.668 | 108.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.283 | 0.281 | 0.284 | 0.001 | 0.283 | 0.283 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.283 | 0.286 | 0.001 | 0.284 | 0.283 | 114.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.286 | 0.283 | 0.292 | 0.004 | 0.287 | 0.284 | 114.2 | 1.00× |
| lazy-ruby | 3 | 6.855 | 6.851 | 6.915 | 0.036 | 6.874 | 6.928 | 114.2 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.509 | 0.499 | 0.515 | 0.006 | 0.508 | 0.507 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.507 | 0.500 | 0.513 | 0.005 | 0.507 | 0.506 | 114.2 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.509 | 0.507 | 0.511 | 0.002 | 0.509 | 0.508 | 114.2 | 1.00× |
| lazy-ruby | 3 | 7.064 | 7.060 | 7.070 | 0.005 | 7.065 | 7.131 | 114.2 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.323 | 0.321 | 0.327 | 0.003 | 0.323 | 0.322 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.323 | 0.319 | 0.323 | 0.002 | 0.322 | 0.321 | 188.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.321 | 0.319 | 0.321 | 0.001 | 0.320 | 0.320 | 188.3 | 1.00× |
| lazy-ruby | 3 | 6.893 | 6.872 | 6.949 | 0.040 | 6.904 | 6.965 | 157.3 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.490 | 0.488 | 0.492 | 0.002 | 0.490 | 0.488 | 114.2 | 1.00× |
| lazy-ruby | 5 | 0.494 | 0.488 | 0.494 | 0.003 | 0.492 | 0.492 | 114.2 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.493 | 0.490 | 0.494 | 0.002 | 0.492 | 0.492 | 114.2 | 1.00× |
| lazy-ruby | 3 | 7.053 | 7.045 | 7.053 | 0.004 | 7.050 | 7.112 | 114.2 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.245 | 0.241 | 0.247 | 0.002 | 0.244 | 0.243 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.241 | 0.237 | 0.245 | 0.003 | 0.242 | 0.240 | 74.8 | 1.02× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.243 | 0.243 | 0.245 | 0.001 | 0.244 | 0.242 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.270 | 2.242 | 2.275 | 0.017 | 2.262 | 2.379 | 74.8 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.447 | 0.445 | 0.451 | 0.002 | 0.447 | 0.446 | 74.8 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.445 | 0.449 | 0.002 | 0.446 | 0.446 | 74.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.447 | 0.445 | 0.447 | 0.001 | 0.446 | 0.445 | 74.8 | 1.00× |
| lazy-ruby | 3 | 2.493 | 2.477 | 2.506 | 0.014 | 2.492 | 2.586 | 74.8 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.270 | 0.268 | 0.276 | 0.004 | 0.271 | 0.270 | 182.3 | 1.00× |
| lazy-ruby | 5 | 0.270 | 0.268 | 0.272 | 0.002 | 0.270 | 0.269 | 182.4 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.270 | 0.266 | 0.270 | 0.002 | 0.269 | 0.268 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.287 | 2.267 | 2.287 | 0.012 | 2.281 | 2.399 | 156.4 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.426 | 0.438 | 0.006 | 0.430 | 0.425 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.426 | 0.439 | 0.006 | 0.432 | 0.428 | 101.6 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.439 | 0.432 | 0.439 | 0.004 | 0.437 | 0.436 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.451 | 2.447 | 2.471 | 0.013 | 2.456 | 2.561 | 74.8 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.388 | 0.386 | 0.390 | 0.002 | 0.388 | 0.387 | 79.9 | 1.00× |
| lazy-ruby | 5 | 0.388 | 0.388 | 0.390 | 0.001 | 0.389 | 0.387 | 79.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.388 | 0.388 | 0.390 | 0.001 | 0.388 | 0.387 | 79.9 | 1.00× |
| lazy-ruby | 3 | 9.908 | 9.892 | 9.910 | 0.010 | 9.903 | 9.890 | 79.9 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.578 | 0.575 | 0.583 | 0.003 | 0.578 | 0.576 | 79.9 | 1.00× |
| lazy-ruby | 5 | 0.577 | 0.573 | 0.579 | 0.002 | 0.577 | 0.577 | 79.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.579 | 0.577 | 0.579 | 0.001 | 0.579 | 0.577 | 79.9 | 1.00× |
| lazy-ruby | 3 | 10.130 | 10.083 | 10.133 | 0.028 | 10.115 | 10.109 | 79.9 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.423 | 0.417 | 0.425 | 0.003 | 0.422 | 0.421 | 185.9 | 1.00× |
| lazy-ruby | 5 | 0.421 | 0.414 | 0.426 | 0.004 | 0.421 | 0.420 | 185.9 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.423 | 0.421 | 0.423 | 0.001 | 0.422 | 0.421 | 185.5 | 1.00× |
| lazy-ruby | 3 | 9.970 | 9.956 | 9.986 | 0.015 | 9.971 | 9.956 | 155.4 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.715 | 0.715 | 0.722 | 0.003 | 0.717 | 0.714 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.716 | 0.711 | 0.723 | 0.004 | 0.717 | 0.715 | 105.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.713 | 0.713 | 0.715 | 0.001 | 0.714 | 0.713 | 105.2 | 1.00× |
| lazy-ruby | 3 | 10.234 | 10.216 | 10.267 | 0.026 | 10.239 | 10.218 | 79.9 | 0.07× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.573 | 0.557 | 0.623 | 0.025 | 0.580 | 0.559 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.569 | 0.541 | 0.644 | 0.039 | 0.582 | 0.559 | 57.2 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.568 | 0.566 | 0.627 | 0.034 | 0.587 | 0.557 | 57.3 | 1.00× |
| lazy-ruby | 3 | 5.404 | 5.317 | 5.425 | 0.057 | 5.382 | 5.534 | 43.0 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.257 | 1.038 | 1.423 | 0.145 | 1.222 | 1.224 | 57.4 | 1.00× |
| lazy-ruby | 5 | 1.134 | 1.041 | 1.557 | 0.211 | 1.219 | 1.112 | 57.2 | 1.11× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.069 | 1.028 | 1.069 | 0.024 | 1.055 | 1.054 | 57.1 | 1.00× |
| lazy-ruby | 3 | 5.926 | 5.826 | 5.930 | 0.059 | 5.894 | 6.026 | 42.9 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.625 | 0.611 | 0.675 | 0.026 | 0.635 | 0.612 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.647 | 0.617 | 0.671 | 0.021 | 0.644 | 0.628 | 141.8 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.682 | 0.647 | 0.692 | 0.023 | 0.674 | 0.640 | 141.7 | 1.00× |
| lazy-ruby | 3 | 5.194 | 4.994 | 5.211 | 0.121 | 5.133 | 5.361 | 126.9 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.076 | 1.030 | 1.116 | 0.035 | 1.079 | 1.064 | 82.7 | 1.00× |
| lazy-ruby | 5 | 1.077 | 1.035 | 1.128 | 0.034 | 1.081 | 1.065 | 83.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.146 | 1.076 | 1.725 | 0.356 | 1.315 | 1.128 | 82.8 | 1.00× |
| lazy-ruby | 3 | 5.645 | 5.464 | 5.942 | 0.242 | 5.684 | 5.798 | 47.4 | 0.20× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.70

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.187 | 0.185 | 0.197 | 0.005 | 0.188 | 0.188 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.189 | 0.187 | 0.193 | 0.002 | 0.189 | 0.188 | 68.2 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.189 | 0.187 | 0.191 | 0.002 | 0.189 | 0.188 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.660 | 0.654 | 0.666 | 0.006 | 0.660 | 0.688 | 58.8 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.343 | 0.342 | 0.348 | 0.002 | 0.344 | 0.344 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.345 | 0.340 | 0.350 | 0.004 | 0.345 | 0.344 | 68.2 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.344 | 0.340 | 0.354 | 0.007 | 0.346 | 0.344 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.808 | 0.807 | 0.816 | 0.005 | 0.810 | 0.797 | 58.5 | 0.43× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.232 | 0.243 | 0.004 | 0.237 | 0.250 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.234 | 0.232 | 0.239 | 0.003 | 0.235 | 0.234 | 109.4 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.234 | 0.243 | 0.004 | 0.238 | 0.234 | 109.4 | 1.00× |
| lazy-ruby | 3 | 0.705 | 0.700 | 0.710 | 0.005 | 0.705 | 0.734 | 100.6 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.404 | 0.399 | 0.409 | 0.004 | 0.403 | 0.406 | 108.8 | 1.00× |
| lazy-ruby | 5 | 0.401 | 0.399 | 0.405 | 0.003 | 0.401 | 0.391 | 109.0 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.403 | 0.398 | 0.404 | 0.003 | 0.402 | 0.391 | 109.1 | 1.00× |
| lazy-ruby | 3 | 0.857 | 0.852 | 0.861 | 0.004 | 0.857 | 0.891 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

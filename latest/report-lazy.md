# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.238 | 0.234 | 0.240 | 0.002 | 0.238 | 0.237 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.238 | 0.228 | 0.242 | 0.006 | 0.237 | 0.237 | 70.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.237 | 0.238 | 0.000 | 0.238 | 0.237 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.365 | 2.361 | 2.465 | 0.059 | 2.397 | 2.501 | 49.2 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.437 | 0.433 | 0.448 | 0.005 | 0.438 | 0.435 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.433 | 0.431 | 0.442 | 0.004 | 0.435 | 0.432 | 70.6 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.438 | 0.437 | 0.440 | 0.001 | 0.438 | 0.437 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.581 | 2.578 | 2.581 | 0.002 | 2.580 | 2.716 | 49.4 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.267 | 0.265 | 0.269 | 0.002 | 0.267 | 0.267 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.267 | 0.267 | 0.269 | 0.001 | 0.268 | 0.267 | 180.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.267 | 0.267 | 0.269 | 0.001 | 0.268 | 0.267 | 180.3 | 1.00× |
| lazy-ruby | 3 | 2.397 | 2.389 | 2.399 | 0.005 | 2.395 | 2.532 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.419 | 0.417 | 0.419 | 0.001 | 0.418 | 0.417 | 102.1 | 1.00× |
| lazy-ruby | 5 | 0.419 | 0.412 | 0.423 | 0.004 | 0.418 | 0.417 | 102.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.423 | 0.419 | 0.425 | 0.003 | 0.422 | 0.421 | 102.4 | 1.00× |
| lazy-ruby | 3 | 2.556 | 2.550 | 2.559 | 0.005 | 2.555 | 2.690 | 62.1 | 0.17× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.289 | 0.282 | 0.297 | 0.005 | 0.289 | 0.288 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.285 | 0.289 | 0.002 | 0.288 | 0.288 | 80.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.289 | 0.288 | 0.290 | 0.001 | 0.289 | 0.288 | 79.8 | 1.00× |
| lazy-ruby | 3 | 6.917 | 6.883 | 6.981 | 0.049 | 6.927 | 6.988 | 51.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.509 | 0.507 | 0.517 | 0.005 | 0.511 | 0.508 | 79.8 | 1.00× |
| lazy-ruby | 5 | 0.507 | 0.503 | 0.515 | 0.005 | 0.507 | 0.504 | 79.9 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.511 | 0.502 | 0.513 | 0.006 | 0.509 | 0.509 | 79.8 | 1.00× |
| lazy-ruby | 3 | 7.107 | 7.086 | 7.118 | 0.016 | 7.103 | 7.166 | 51.4 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.326 | 0.319 | 0.328 | 0.004 | 0.324 | 0.325 | 188.8 | 1.00× |
| lazy-ruby | 5 | 0.322 | 0.322 | 0.338 | 0.007 | 0.325 | 0.320 | 188.5 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.320 | 0.317 | 0.323 | 0.003 | 0.320 | 0.319 | 188.5 | 1.00× |
| lazy-ruby | 3 | 6.924 | 6.863 | 6.953 | 0.046 | 6.913 | 7.014 | 157.6 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.490 | 0.488 | 0.499 | 0.005 | 0.491 | 0.488 | 112.1 | 1.00× |
| lazy-ruby | 5 | 0.490 | 0.488 | 0.511 | 0.010 | 0.494 | 0.489 | 112.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.489 | 0.486 | 0.494 | 0.004 | 0.490 | 0.488 | 111.8 | 1.00× |
| lazy-ruby | 3 | 7.036 | 7.016 | 7.124 | 0.057 | 7.059 | 7.108 | 64.6 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.244 | 0.242 | 0.244 | 0.001 | 0.244 | 0.242 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.242 | 0.238 | 0.244 | 0.002 | 0.241 | 0.241 | 72.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.240 | 0.238 | 0.240 | 0.001 | 0.239 | 0.238 | 72.2 | 1.00× |
| lazy-ruby | 3 | 2.259 | 2.244 | 2.271 | 0.013 | 2.258 | 2.371 | 47.1 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.452 | 0.448 | 0.458 | 0.004 | 0.452 | 0.451 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.452 | 0.447 | 0.458 | 0.005 | 0.452 | 0.451 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.456 | 0.450 | 0.463 | 0.007 | 0.456 | 0.455 | 72.4 | 1.00× |
| lazy-ruby | 3 | 2.460 | 2.453 | 2.505 | 0.028 | 2.473 | 2.569 | 47.2 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.265 | 0.263 | 0.267 | 0.002 | 0.265 | 0.264 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.263 | 0.276 | 0.005 | 0.266 | 0.264 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.265 | 0.267 | 0.001 | 0.265 | 0.263 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.283 | 2.267 | 2.289 | 0.011 | 2.280 | 2.396 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.433 | 0.433 | 0.436 | 0.001 | 0.434 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.433 | 0.435 | 0.001 | 0.434 | 0.433 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.435 | 0.433 | 0.437 | 0.002 | 0.435 | 0.434 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.452 | 2.446 | 2.452 | 0.003 | 2.450 | 2.561 | 62.6 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.250 | 0.249 | 0.253 | 0.002 | 0.250 | 0.248 | 75.1 | 1.00× |
| lazy-ruby | 5 | 0.250 | 0.247 | 0.253 | 0.002 | 0.250 | 0.248 | 75.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.247 | 0.244 | 0.247 | 0.001 | 0.246 | 0.245 | 75.2 | 1.00× |
| lazy-ruby | 3 | 5.849 | 5.849 | 6.015 | 0.096 | 5.904 | 5.852 | 49.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.419 | 0.414 | 0.445 | 0.013 | 0.423 | 0.417 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.414 | 0.421 | 0.003 | 0.417 | 0.415 | 75.1 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.416 | 0.416 | 0.417 | 0.001 | 0.416 | 0.415 | 75.2 | 1.00× |
| lazy-ruby | 3 | 6.142 | 5.988 | 6.149 | 0.091 | 6.093 | 6.146 | 49.8 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.282 | 0.280 | 0.285 | 0.002 | 0.282 | 0.281 | 185.8 | 1.00× |
| lazy-ruby | 5 | 0.280 | 0.278 | 0.284 | 0.003 | 0.280 | 0.278 | 185.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.282 | 0.281 | 0.282 | 0.001 | 0.282 | 0.281 | 185.7 | 1.00× |
| lazy-ruby | 3 | 5.793 | 5.760 | 5.832 | 0.036 | 5.795 | 5.798 | 156.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.443 | 0.436 | 0.447 | 0.004 | 0.442 | 0.441 | 105.3 | 1.00× |
| lazy-ruby | 5 | 0.442 | 0.438 | 0.447 | 0.004 | 0.443 | 0.442 | 105.5 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.443 | 0.440 | 0.443 | 0.001 | 0.442 | 0.442 | 103.5 | 1.00× |
| lazy-ruby | 3 | 6.098 | 6.022 | 6.182 | 0.080 | 6.101 | 6.101 | 62.9 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.188 | 0.176 | 0.202 | 0.009 | 0.189 | 0.170 | 62.0 | 1.00× |
| lazy-ruby | 5 | 0.188 | 0.181 | 0.205 | 0.009 | 0.190 | 0.171 | 62.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.188 | 0.188 | 0.190 | 0.001 | 0.189 | 0.169 | 62.7 | 1.00× |
| lazy-ruby | 3 | 0.814 | 0.808 | 0.827 | 0.009 | 0.817 | 0.871 | 51.3 | 0.23× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.380 | 0.366 | 0.405 | 0.019 | 0.385 | 0.357 | 62.6 | 1.00× |
| lazy-ruby | 5 | 0.375 | 0.360 | 0.404 | 0.019 | 0.378 | 0.355 | 62.0 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.412 | 0.410 | 0.420 | 0.006 | 0.414 | 0.398 | 62.1 | 1.00× |
| lazy-ruby | 3 | 1.188 | 1.112 | 1.205 | 0.049 | 1.168 | 1.234 | 52.5 | 0.35× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.198 | 0.278 | 0.034 | 0.245 | 0.222 | 164.8 | 1.00× |
| lazy-ruby | 5 | 0.224 | 0.221 | 0.279 | 0.026 | 0.240 | 0.216 | 164.1 | 1.11× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.248 | 0.240 | 0.250 | 0.005 | 0.246 | 0.228 | 163.7 | 1.00× |
| lazy-ruby | 3 | 1.001 | 0.934 | 1.047 | 0.056 | 0.994 | 1.055 | 146.9 | 0.25× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.503 | 0.475 | 0.538 | 0.026 | 0.506 | 0.486 | 97.6 | 1.00× |
| lazy-ruby | 5 | 0.499 | 0.430 | 0.586 | 0.060 | 0.503 | 0.484 | 97.2 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.446 | 0.432 | 0.461 | 0.014 | 0.447 | 0.431 | 92.3 | 1.00× |
| lazy-ruby | 3 | 1.245 | 1.126 | 1.311 | 0.094 | 1.227 | 1.264 | 66.6 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.358 | 0.335 | 0.495 | 0.071 | 0.399 | 0.346 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.378 | 0.358 | 0.530 | 0.070 | 0.407 | 0.368 | 57.2 | 0.95× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.583 | 0.427 | 0.691 | 0.133 | 0.567 | 0.575 | 57.0 | 1.00× |
| lazy-ruby | 3 | 4.573 | 3.263 | 5.011 | 0.910 | 4.282 | 4.586 | 43.1 | 0.13× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.789 | 0.676 | 0.927 | 0.091 | 0.792 | 0.778 | 57.0 | 1.00× |
| lazy-ruby | 5 | 0.735 | 0.713 | 0.939 | 0.096 | 0.770 | 0.724 | 57.1 | 1.07× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.697 | 0.674 | 0.735 | 0.031 | 0.702 | 0.682 | 57.1 | 1.00× |
| lazy-ruby | 3 | 3.690 | 3.588 | 3.759 | 0.086 | 3.679 | 3.790 | 43.3 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.404 | 0.480 | 0.032 | 0.435 | 0.410 | 141.8 | 1.00× |
| lazy-ruby | 5 | 0.417 | 0.403 | 0.440 | 0.014 | 0.419 | 0.406 | 141.7 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.405 | 0.389 | 0.409 | 0.011 | 0.401 | 0.396 | 141.8 | 1.00× |
| lazy-ruby | 3 | 3.110 | 3.048 | 3.133 | 0.044 | 3.097 | 3.222 | 126.7 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.696 | 0.649 | 0.847 | 0.075 | 0.718 | 0.687 | 83.2 | 1.00× |
| lazy-ruby | 5 | 0.802 | 0.653 | 0.861 | 0.093 | 0.757 | 0.785 | 82.9 | 0.87× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.635 | 0.624 | 0.704 | 0.043 | 0.654 | 0.621 | 82.9 | 1.00× |
| lazy-ruby | 3 | 3.266 | 3.197 | 3.296 | 0.051 | 3.253 | 3.327 | 46.4 | 0.19× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.76

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.248 | 0.239 | 0.273 | 0.013 | 0.252 | 0.266 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.266 | 0.235 | 0.274 | 0.020 | 0.257 | 0.250 | 68.3 | 0.93× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.240 | 0.238 | 0.242 | 0.002 | 0.240 | 0.234 | 68.2 | 1.00× |
| lazy-ruby | 3 | 0.825 | 0.813 | 0.834 | 0.010 | 0.824 | 0.844 | 58.7 | 0.29× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.461 | 0.456 | 0.467 | 0.004 | 0.461 | 0.453 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.463 | 0.456 | 0.473 | 0.007 | 0.463 | 0.469 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.462 | 0.460 | 0.470 | 0.005 | 0.464 | 0.469 | 68.2 | 1.00× |
| lazy-ruby | 3 | 1.039 | 1.036 | 1.041 | 0.002 | 1.038 | 1.047 | 58.7 | 0.44× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.292 | 0.289 | 0.295 | 0.002 | 0.292 | 0.312 | 109.4 | 1.00× |
| lazy-ruby | 5 | 0.293 | 0.290 | 0.301 | 0.005 | 0.295 | 0.297 | 108.8 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.294 | 0.290 | 0.296 | 0.003 | 0.293 | 0.281 | 109.6 | 1.00× |
| lazy-ruby | 3 | 0.871 | 0.858 | 0.876 | 0.009 | 0.868 | 0.891 | 100.5 | 0.34× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.519 | 0.508 | 0.522 | 0.005 | 0.517 | 0.516 | 108.8 | 1.00× |
| lazy-ruby | 5 | 0.513 | 0.511 | 0.522 | 0.004 | 0.515 | 0.516 | 108.9 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.513 | 0.511 | 0.515 | 0.002 | 0.513 | 0.516 | 109.0 | 1.00× |
| lazy-ruby | 3 | 1.087 | 1.080 | 1.154 | 0.041 | 1.107 | 1.109 | 83.0 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

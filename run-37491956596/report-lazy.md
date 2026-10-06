# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.218 | 0.216 | 0.222 | 0.002 | 0.218 | 0.217 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.222 | 0.218 | 0.222 | 0.002 | 0.221 | 0.221 | 108.1 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.222 | 0.220 | 0.222 | 0.001 | 0.222 | 0.221 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.344 | 2.339 | 2.378 | 0.021 | 2.354 | 2.477 | 108.1 | 0.09× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.426 | 0.424 | 0.440 | 0.007 | 0.428 | 0.424 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.424 | 0.420 | 0.426 | 0.003 | 0.424 | 0.423 | 108.1 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.422 | 0.420 | 0.424 | 0.002 | 0.422 | 0.421 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.545 | 2.542 | 2.559 | 0.009 | 2.549 | 2.675 | 108.1 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.245 | 0.241 | 0.247 | 0.002 | 0.244 | 0.245 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.245 | 0.245 | 0.247 | 0.001 | 0.246 | 0.244 | 180.1 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.245 | 0.245 | 0.247 | 0.001 | 0.246 | 0.245 | 180.4 | 1.00× |
| lazy-ruby | 3 | 2.370 | 2.362 | 2.464 | 0.056 | 2.399 | 2.503 | 157.9 | 0.10× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.399 | 0.397 | 0.405 | 0.003 | 0.401 | 0.399 | 108.1 | 1.00× |
| lazy-ruby | 5 | 0.399 | 0.397 | 0.402 | 0.002 | 0.399 | 0.397 | 108.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.399 | 0.397 | 0.402 | 0.002 | 0.399 | 0.399 | 108.1 | 1.00× |
| lazy-ruby | 3 | 2.512 | 2.510 | 2.522 | 0.006 | 2.515 | 2.645 | 108.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.184 | 0.184 | 0.188 | 0.002 | 0.185 | 0.183 | 115.2 | 1.00× |
| lazy-ruby | 5 | 0.184 | 0.182 | 0.186 | 0.001 | 0.184 | 0.182 | 115.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.184 | 0.182 | 0.186 | 0.002 | 0.184 | 0.181 | 115.2 | 1.00× |
| lazy-ruby | 3 | 4.941 | 4.932 | 4.960 | 0.014 | 4.944 | 5.001 | 115.2 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.316 | 0.314 | 0.318 | 0.002 | 0.316 | 0.315 | 115.2 | 1.00× |
| lazy-ruby | 5 | 0.293 | 0.291 | 0.299 | 0.003 | 0.294 | 0.292 | 115.2 | 1.08× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.326 | 0.324 | 0.327 | 0.001 | 0.326 | 0.324 | 115.2 | 1.00× |
| lazy-ruby | 3 | 5.041 | 4.971 | 5.053 | 0.044 | 5.022 | 5.094 | 115.2 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.208 | 0.206 | 0.215 | 0.004 | 0.210 | 0.207 | 194.1 | 1.00× |
| lazy-ruby | 5 | 0.208 | 0.206 | 0.217 | 0.004 | 0.210 | 0.207 | 195.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.210 | 0.208 | 0.213 | 0.002 | 0.211 | 0.208 | 194.0 | 1.00× |
| lazy-ruby | 3 | 4.892 | 4.852 | 4.959 | 0.054 | 4.901 | 4.956 | 166.9 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.330 | 0.326 | 0.345 | 0.008 | 0.332 | 0.329 | 117.4 | 1.00× |
| lazy-ruby | 5 | 0.333 | 0.328 | 0.351 | 0.010 | 0.335 | 0.332 | 117.1 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.326 | 0.326 | 0.328 | 0.001 | 0.326 | 0.325 | 117.6 | 1.00× |
| lazy-ruby | 3 | 4.986 | 4.980 | 5.077 | 0.054 | 5.015 | 5.050 | 115.2 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.233 | 0.001 | 0.232 | 0.232 | 74.0 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.233 | 0.235 | 0.001 | 0.234 | 0.232 | 74.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.231 | 0.233 | 0.001 | 0.231 | 0.230 | 74.0 | 1.00× |
| lazy-ruby | 3 | 2.216 | 2.214 | 2.231 | 0.009 | 2.220 | 2.331 | 74.0 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.439 | 0.435 | 0.441 | 0.003 | 0.438 | 0.438 | 74.0 | 1.00× |
| lazy-ruby | 5 | 0.441 | 0.437 | 0.441 | 0.002 | 0.439 | 0.439 | 74.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.437 | 0.437 | 0.441 | 0.002 | 0.438 | 0.436 | 74.0 | 1.00× |
| lazy-ruby | 3 | 2.438 | 2.433 | 2.448 | 0.008 | 2.440 | 2.554 | 74.0 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.253 | 0.257 | 0.002 | 0.255 | 0.253 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.253 | 0.253 | 0.255 | 0.001 | 0.254 | 0.253 | 182.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.255 | 0.254 | 0.256 | 0.001 | 0.255 | 0.253 | 182.2 | 1.00× |
| lazy-ruby | 3 | 2.247 | 2.245 | 2.251 | 0.003 | 2.247 | 2.362 | 156.1 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.422 | 0.420 | 0.424 | 0.001 | 0.422 | 0.420 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.420 | 0.424 | 0.002 | 0.422 | 0.420 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.422 | 0.422 | 0.422 | 0.000 | 0.422 | 0.421 | 101.6 | 1.00× |
| lazy-ruby | 3 | 2.424 | 2.407 | 2.432 | 0.012 | 2.421 | 2.537 | 74.0 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.290 | 0.292 | 0.001 | 0.291 | 0.290 | 82.3 | 1.00× |
| lazy-ruby | 5 | 0.290 | 0.289 | 0.290 | 0.001 | 0.290 | 0.289 | 82.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.288 | 0.288 | 0.290 | 0.001 | 0.289 | 0.287 | 82.3 | 1.00× |
| lazy-ruby | 3 | 7.407 | 7.389 | 7.414 | 0.013 | 7.403 | 7.451 | 81.1 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.516 | 0.514 | 0.524 | 0.004 | 0.517 | 0.514 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.520 | 0.512 | 0.526 | 0.005 | 0.519 | 0.518 | 82.4 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.522 | 0.520 | 0.526 | 0.003 | 0.523 | 0.520 | 82.4 | 1.00× |
| lazy-ruby | 3 | 7.639 | 7.589 | 7.662 | 0.038 | 7.630 | 7.694 | 81.1 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.327 | 0.323 | 0.329 | 0.002 | 0.326 | 0.325 | 191.9 | 1.00× |
| lazy-ruby | 5 | 0.324 | 0.323 | 0.328 | 0.002 | 0.324 | 0.322 | 192.9 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.328 | 0.322 | 0.329 | 0.004 | 0.326 | 0.327 | 194.2 | 1.00× |
| lazy-ruby | 3 | 7.439 | 7.415 | 7.467 | 0.026 | 7.440 | 7.483 | 167.0 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.538 | 0.532 | 0.543 | 0.005 | 0.538 | 0.537 | 112.6 | 1.00× |
| lazy-ruby | 5 | 0.535 | 0.535 | 0.540 | 0.002 | 0.537 | 0.534 | 112.3 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.541 | 0.536 | 0.543 | 0.004 | 0.540 | 0.540 | 112.4 | 1.00× |
| lazy-ruby | 3 | 7.637 | 7.614 | 7.643 | 0.016 | 7.631 | 7.679 | 81.1 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.250 | 0.219 | 0.266 | 0.020 | 0.247 | 0.229 | 64.8 | 1.00× |
| lazy-ruby | 5 | 0.224 | 0.222 | 0.263 | 0.018 | 0.234 | 0.219 | 66.1 | 1.12× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.265 | 0.243 | 0.277 | 0.017 | 0.262 | 0.244 | 65.6 | 1.00× |
| lazy-ruby | 3 | 1.208 | 1.099 | 1.221 | 0.067 | 1.176 | 1.307 | 59.2 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.506 | 0.466 | 0.728 | 0.130 | 0.585 | 0.485 | 64.5 | 1.00× |
| lazy-ruby | 5 | 0.504 | 0.490 | 0.812 | 0.139 | 0.564 | 0.492 | 64.9 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.493 | 0.480 | 0.528 | 0.025 | 0.500 | 0.472 | 65.6 | 1.00× |
| lazy-ruby | 3 | 1.342 | 1.304 | 1.419 | 0.059 | 1.355 | 1.448 | 60.4 | 0.37× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.279 | 0.251 | 0.311 | 0.024 | 0.283 | 0.267 | 166.9 | 1.00× |
| lazy-ruby | 5 | 0.293 | 0.271 | 0.323 | 0.019 | 0.294 | 0.277 | 166.4 | 0.95× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.276 | 0.264 | 0.283 | 0.010 | 0.275 | 0.258 | 166.7 | 1.00× |
| lazy-ruby | 3 | 1.134 | 1.102 | 1.230 | 0.067 | 1.155 | 1.205 | 152.6 | 0.24× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.525 | 0.423 | 0.570 | 0.055 | 0.505 | 0.505 | 103.5 | 1.00× |
| lazy-ruby | 5 | 0.474 | 0.422 | 0.541 | 0.044 | 0.480 | 0.465 | 94.2 | 1.11× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.477 | 0.467 | 0.477 | 0.006 | 0.474 | 0.454 | 93.5 | 1.00× |
| lazy-ruby | 3 | 1.312 | 1.237 | 1.454 | 0.110 | 1.334 | 1.429 | 70.6 | 0.36× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.362 | 0.337 | 0.524 | 0.077 | 0.389 | 0.351 | 57.1 | 1.00× |
| lazy-ruby | 5 | 0.375 | 0.349 | 0.516 | 0.068 | 0.405 | 0.354 | 57.6 | 0.97× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.453 | 0.380 | 0.485 | 0.054 | 0.439 | 0.365 | 57.3 | 1.00× |
| lazy-ruby | 3 | 3.382 | 3.239 | 4.202 | 0.520 | 3.608 | 3.462 | 43.0 | 0.13× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.781 | 0.678 | 0.965 | 0.122 | 0.801 | 0.759 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.820 | 0.659 | 0.950 | 0.120 | 0.813 | 0.809 | 57.3 | 0.95× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.929 | 0.901 | 0.951 | 0.025 | 0.927 | 0.918 | 57.3 | 1.00× |
| lazy-ruby | 3 | 4.526 | 4.319 | 4.950 | 0.321 | 4.598 | 4.650 | 42.7 | 0.21× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.594 | 0.531 | 0.606 | 0.030 | 0.582 | 0.579 | 141.9 | 1.00× |
| lazy-ruby | 5 | 0.560 | 0.529 | 0.638 | 0.042 | 0.576 | 0.546 | 141.8 | 1.06× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.484 | 0.465 | 0.560 | 0.050 | 0.503 | 0.472 | 141.9 | 1.00× |
| lazy-ruby | 3 | 4.417 | 4.069 | 4.461 | 0.215 | 4.316 | 4.416 | 126.9 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.101 | 0.896 | 1.919 | 0.406 | 1.223 | 0.973 | 83.2 | 1.00× |
| lazy-ruby | 5 | 1.620 | 0.887 | 2.539 | 0.667 | 1.536 | 1.300 | 83.1 | 0.68× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.322 | 0.992 | 1.437 | 0.231 | 1.250 | 1.288 | 83.1 | 1.00× |
| lazy-ruby | 3 | 5.571 | 4.649 | 5.867 | 0.635 | 5.362 | 5.470 | 46.7 | 0.24× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.26

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.235 | 0.233 | 0.254 | 0.010 | 0.241 | 0.234 | 33.6 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.236 | 0.246 | 0.005 | 0.240 | 0.234 | 33.8 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.459 | 0.456 | 0.467 | 0.004 | 0.460 | 0.453 | 33.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.461 | 0.455 | 0.462 | 0.004 | 0.459 | 0.453 | 33.6 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.503 | 0.495 | 0.520 | 0.011 | 0.506 | 0.531 | 75.1 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.502 | 0.497 | 0.509 | 0.006 | 0.503 | 0.500 | 75.7 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- fib / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- ioread / eager-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- ioread / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- treewalk / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.218 | 0.216 | 0.218 | 0.001 | 0.218 | 0.217 | 70.7 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.218 | 0.216 | 0.218 | 0.001 | 0.218 | 0.217 | 70.8 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.420 | 0.420 | 0.424 | 0.002 | 0.421 | 0.419 | 70.8 | 1.00× |
| lazy-ruby | 5 | 0.420 | 0.418 | 0.422 | 0.001 | 0.420 | 0.418 | 70.7 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.420 | 0.420 | 0.420 | 0.000 | 0.420 | 0.419 | 70.7 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.241 | 0.239 | 0.249 | 0.004 | 0.242 | 0.241 | 181.2 | 1.00× |
| lazy-ruby | 5 | 0.239 | 0.237 | 0.243 | 0.002 | 0.239 | 0.238 | 181.2 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.239 | 0.237 | 0.243 | 0.003 | 0.240 | 0.238 | 181.2 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.395 | 0.393 | 0.399 | 0.003 | 0.396 | 0.394 | 102.1 | 1.00× |
| lazy-ruby | 5 | 0.393 | 0.391 | 0.393 | 0.001 | 0.392 | 0.391 | 102.3 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.395 | 0.395 | 0.397 | 0.001 | 0.396 | 0.395 | 102.4 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.255 | 0.253 | 0.255 | 0.001 | 0.254 | 0.253 | 85.4 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.253 | 0.251 | 0.258 | 0.004 | 0.254 | 0.252 | 89.2 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.476 | 0.472 | 0.480 | 0.003 | 0.476 | 0.474 | 85.2 | 1.00× |
| lazy-ruby | 5 | 0.473 | 0.470 | 0.476 | 0.002 | 0.473 | 0.472 | 85.4 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.472 | 0.471 | 0.472 | 0.001 | 0.472 | 0.470 | 85.4 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.290 | 0.288 | 0.296 | 0.003 | 0.291 | 0.290 | 193.9 | 1.00× |
| lazy-ruby | 5 | 0.288 | 0.288 | 0.291 | 0.001 | 0.289 | 0.286 | 195.4 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.292 | 0.290 | 0.293 | 0.002 | 0.292 | 0.291 | 193.7 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.457 | 0.453 | 0.462 | 0.003 | 0.457 | 0.455 | 116.9 | 1.00× |
| lazy-ruby | 5 | 0.455 | 0.451 | 0.457 | 0.003 | 0.455 | 0.454 | 117.1 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.459 | 0.455 | 0.461 | 0.003 | 0.459 | 0.459 | 118.9 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.175 | 0.171 | 0.175 | 0.002 | 0.174 | 0.173 | 71.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.175 | 0.171 | 0.175 | 0.002 | 0.174 | 0.173 | 71.5 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.354 | 0.352 | 0.354 | 0.001 | 0.354 | 0.352 | 71.5 | 1.00× |
| lazy-ruby | 5 | 0.352 | 0.350 | 0.352 | 0.001 | 0.351 | 0.350 | 71.4 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.356 | 0.354 | 0.357 | 0.001 | 0.355 | 0.355 | 71.5 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.191 | 0.191 | 0.264 | 0.032 | 0.206 | 0.191 | 181.3 | 1.00× |
| lazy-ruby | 5 | 0.192 | 0.189 | 0.210 | 0.008 | 0.195 | 0.191 | 181.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.194 | 0.194 | 0.194 | 0.000 | 0.194 | 0.193 | 181.2 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.321 | 0.319 | 0.325 | 0.003 | 0.321 | 0.320 | 101.4 | 1.00× |
| lazy-ruby | 5 | 0.319 | 0.315 | 0.319 | 0.002 | 0.318 | 0.317 | 101.3 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.321 | 0.321 | 0.323 | 0.001 | 0.322 | 0.320 | 101.5 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.294 | 0.294 | 0.298 | 0.002 | 0.295 | 0.292 | 82.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.295 | 0.294 | 0.296 | 0.001 | 0.295 | 0.294 | 84.3 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.526 | 0.515 | 0.541 | 0.009 | 0.526 | 0.524 | 82.5 | 1.00× |
| lazy-ruby | 5 | 0.519 | 0.519 | 0.524 | 0.002 | 0.521 | 0.518 | 84.0 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.521 | 0.517 | 0.522 | 0.002 | 0.520 | 0.520 | 82.6 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.325 | 0.321 | 0.327 | 0.003 | 0.325 | 0.324 | 192.3 | 1.00× |
| lazy-ruby | 5 | 0.323 | 0.323 | 0.331 | 0.004 | 0.325 | 0.323 | 191.9 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.332 | 0.327 | 0.339 | 0.006 | 0.333 | 0.332 | 191.7 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.542 | 0.540 | 0.546 | 0.002 | 0.543 | 0.540 | 112.8 | 1.00× |
| lazy-ruby | 5 | 0.545 | 0.540 | 0.546 | 0.002 | 0.544 | 0.544 | 112.5 | 0.99× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.542 | 0.540 | 0.546 | 0.003 | 0.543 | 0.540 | 112.5 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.216 | 0.208 | 0.223 | 0.006 | 0.216 | 0.197 | 65.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.238 | 0.225 | 0.252 | 0.013 | 0.238 | 0.220 | 65.4 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.452 | 0.433 | 0.490 | 0.021 | 0.457 | 0.443 | 65.9 | 1.00× |
| lazy-ruby | 5 | 0.463 | 0.430 | 0.484 | 0.024 | 0.460 | 0.446 | 64.8 | 0.98× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.440 | 0.415 | 0.466 | 0.025 | 0.440 | 0.416 | 64.4 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.222 | 0.219 | 0.237 | 0.008 | 0.226 | 0.206 | 165.8 | 1.00× |
| lazy-ruby | 5 | 0.218 | 0.212 | 0.227 | 0.007 | 0.219 | 0.202 | 165.7 | 1.02× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.220 | 0.210 | 0.231 | 0.011 | 0.221 | 0.198 | 165.2 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.429 | 0.396 | 0.446 | 0.020 | 0.422 | 0.413 | 92.4 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.402 | 0.457 | 0.026 | 0.427 | 0.399 | 98.5 | 1.03× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.418 | 0.416 | 0.421 | 0.003 | 0.418 | 0.394 | 100.0 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.25

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.498 | 0.432 | 0.647 | 0.091 | 0.521 | 0.483 | 57.9 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.488 | 0.477 | 0.526 | 0.026 | 0.497 | 0.476 | 57.9 | 1.00× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.965 | 0.853 | 1.583 | 0.299 | 1.080 | 0.937 | 57.6 | 1.00× |
| lazy-ruby | 5 | 0.809 | 0.800 | 1.011 | 0.089 | 0.855 | 0.800 | 57.6 | 1.19× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.864 | 0.829 | 0.898 | 0.034 | 0.864 | 0.853 | 57.5 | 1.00× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.576 | 0.471 | 1.241 | 0.314 | 0.692 | 0.547 | 141.0 | 1.00× |
| lazy-ruby | 5 | 0.594 | 0.458 | 0.775 | 0.126 | 0.595 | 0.583 | 141.0 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.485 | 0.480 | 0.504 | 0.013 | 0.490 | 0.470 | 141.1 | 1.00× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.765 | 0.740 | 0.828 | 0.038 | 0.777 | 0.754 | 84.8 | 1.00× |
| lazy-ruby | 5 | 0.755 | 0.749 | 0.806 | 0.030 | 0.773 | 0.743 | 84.7 | 1.01× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.020 | 0.888 | 1.174 | 0.143 | 1.027 | 0.865 | 85.0 | 1.00× |

Failed runs:

- boot / lazy-ruby [warm] #0 (exit 255): failed — warmup: exit 255 (expected 0)
- fib / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- fib / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- ioread / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #1 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #2 (exit 255): failed — exit 255 (expected 0)
- treewalk / lazy-ruby [cold] #3 (exit 255): failed — exit 255 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.25

Failed runs:

- boot / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- boot / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- fib / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- fib / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- ioread / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- ioread / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- treewalk / eager-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)
- treewalk / lazy-ruby [warm] #0 (exit -1073741511): failed — warmup: exit -1073741511 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

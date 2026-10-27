# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.223 | 0.223 | 0.225 | 0.001 | 0.224 | 0.222 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.223 | 0.221 | 0.223 | 0.001 | 0.222 | 0.221 | 108.0 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.223 | 0.223 | 0.227 | 0.002 | 0.224 | 0.222 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.350 | 2.340 | 2.397 | 0.031 | 2.362 | 2.483 | 108.0 | 0.09× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.428 | 0.427 | 0.429 | 0.001 | 0.428 | 0.427 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.428 | 0.427 | 0.429 | 0.001 | 0.428 | 0.427 | 108.0 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.428 | 0.428 | 0.432 | 0.002 | 0.430 | 0.427 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.559 | 2.554 | 2.564 | 0.005 | 2.559 | 2.693 | 108.0 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.249 | 0.250 | 0.000 | 0.250 | 0.249 | 180.2 | 1.00× |
| lazy-ruby | 5 | 0.249 | 0.249 | 0.252 | 0.001 | 0.250 | 0.249 | 180.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.251 | 0.250 | 0.254 | 0.002 | 0.252 | 0.249 | 180.1 | 1.00× |
| lazy-ruby | 3 | 2.387 | 2.386 | 2.392 | 0.003 | 2.388 | 2.521 | 157.1 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.408 | 0.408 | 0.412 | 0.002 | 0.410 | 0.408 | 108.0 | 1.00× |
| lazy-ruby | 5 | 0.410 | 0.406 | 0.410 | 0.002 | 0.409 | 0.408 | 108.0 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.408 | 0.408 | 0.000 | 0.408 | 0.406 | 108.0 | 1.00× |
| lazy-ruby | 3 | 2.543 | 2.526 | 2.550 | 0.012 | 2.540 | 2.675 | 108.0 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.199 | 0.198 | 0.211 | 0.006 | 0.202 | 0.198 | 116.3 | 1.00× |
| lazy-ruby | 5 | 0.205 | 0.197 | 0.207 | 0.005 | 0.202 | 0.203 | 116.3 | 0.97× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.196 | 0.196 | 0.000 | 0.196 | 0.195 | 116.3 | 1.00× |
| lazy-ruby | 3 | 5.393 | 5.324 | 5.496 | 0.086 | 5.404 | 5.444 | 116.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.363 | 0.359 | 0.371 | 0.005 | 0.365 | 0.362 | 116.3 | 1.00× |
| lazy-ruby | 5 | 0.363 | 0.361 | 0.364 | 0.001 | 0.363 | 0.361 | 116.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.363 | 0.362 | 0.364 | 0.001 | 0.363 | 0.361 | 116.3 | 1.00× |
| lazy-ruby | 3 | 5.611 | 5.403 | 5.633 | 0.127 | 5.549 | 5.662 | 116.3 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.231 | 0.229 | 0.233 | 0.002 | 0.232 | 0.230 | 195.6 | 1.00× |
| lazy-ruby | 5 | 0.230 | 0.229 | 0.233 | 0.002 | 0.230 | 0.229 | 193.8 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.231 | 0.231 | 0.000 | 0.231 | 0.230 | 193.4 | 1.00× |
| lazy-ruby | 3 | 5.312 | 5.257 | 5.355 | 0.049 | 5.308 | 5.365 | 165.8 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.361 | 0.359 | 0.363 | 0.002 | 0.361 | 0.359 | 117.5 | 1.00× |
| lazy-ruby | 5 | 0.361 | 0.360 | 0.365 | 0.002 | 0.362 | 0.360 | 117.4 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.363 | 0.363 | 0.364 | 0.000 | 0.363 | 0.362 | 117.1 | 1.00× |
| lazy-ruby | 3 | 5.469 | 5.340 | 5.598 | 0.129 | 5.469 | 5.531 | 116.3 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.233 | 0.231 | 0.233 | 0.001 | 0.232 | 0.231 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.233 | 0.227 | 0.235 | 0.003 | 0.232 | 0.231 | 74.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.231 | 0.224 | 0.231 | 0.004 | 0.229 | 0.229 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.214 | 2.214 | 2.220 | 0.003 | 2.216 | 2.327 | 74.5 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.432 | 0.438 | 0.003 | 0.435 | 0.432 | 74.5 | 1.00× |
| lazy-ruby | 5 | 0.434 | 0.432 | 0.438 | 0.002 | 0.435 | 0.432 | 74.5 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.436 | 0.434 | 0.451 | 0.009 | 0.441 | 0.436 | 74.5 | 1.00× |
| lazy-ruby | 3 | 2.426 | 2.415 | 2.431 | 0.009 | 2.424 | 2.542 | 74.5 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.249 | 0.245 | 0.253 | 0.003 | 0.249 | 0.248 | 182.2 | 1.00× |
| lazy-ruby | 5 | 0.249 | 0.249 | 0.249 | 0.000 | 0.249 | 0.248 | 182.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.249 | 0.249 | 0.249 | 0.000 | 0.249 | 0.248 | 182.4 | 1.00× |
| lazy-ruby | 3 | 2.250 | 2.236 | 2.252 | 0.009 | 2.246 | 2.367 | 156.0 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.416 | 0.410 | 0.418 | 0.004 | 0.414 | 0.414 | 101.7 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.410 | 0.418 | 0.003 | 0.415 | 0.414 | 101.7 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.420 | 0.418 | 0.420 | 0.001 | 0.419 | 0.417 | 101.7 | 1.00× |
| lazy-ruby | 3 | 2.421 | 2.415 | 2.431 | 0.008 | 2.423 | 2.536 | 74.5 | 0.17× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.223 | 0.221 | 0.226 | 0.002 | 0.223 | 0.222 | 82.4 | 1.00× |
| lazy-ruby | 5 | 0.222 | 0.221 | 0.224 | 0.001 | 0.222 | 0.221 | 82.3 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.226 | 0.225 | 0.227 | 0.001 | 0.226 | 0.224 | 82.6 | 1.00× |
| lazy-ruby | 3 | 5.850 | 5.726 | 5.884 | 0.083 | 5.820 | 5.889 | 81.3 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.391 | 0.386 | 0.407 | 0.008 | 0.393 | 0.390 | 82.6 | 1.00× |
| lazy-ruby | 5 | 0.389 | 0.386 | 0.391 | 0.002 | 0.388 | 0.388 | 82.4 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.389 | 0.389 | 0.395 | 0.004 | 0.391 | 0.388 | 82.6 | 1.00× |
| lazy-ruby | 3 | 5.934 | 5.857 | 6.075 | 0.111 | 5.955 | 5.972 | 81.3 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.256 | 0.254 | 0.258 | 0.002 | 0.256 | 0.254 | 191.4 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.254 | 0.260 | 0.002 | 0.256 | 0.254 | 191.6 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.256 | 0.252 | 0.256 | 0.002 | 0.255 | 0.254 | 193.8 | 1.00× |
| lazy-ruby | 3 | 5.752 | 5.661 | 5.996 | 0.173 | 5.803 | 5.791 | 168.6 | 0.04× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.415 | 0.413 | 0.417 | 0.002 | 0.415 | 0.413 | 112.3 | 1.00× |
| lazy-ruby | 5 | 0.415 | 0.412 | 0.417 | 0.002 | 0.415 | 0.414 | 112.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.415 | 0.413 | 0.417 | 0.002 | 0.415 | 0.414 | 112.4 | 1.00× |
| lazy-ruby | 3 | 6.059 | 5.828 | 6.109 | 0.150 | 5.999 | 6.098 | 81.3 | 0.07× |

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.259 | 0.228 | 0.285 | 0.025 | 0.258 | 0.238 | 62.6 | 1.00× |
| lazy-ruby | 5 | 0.245 | 0.219 | 0.260 | 0.016 | 0.245 | 0.228 | 63.5 | 1.06× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.239 | 0.232 | 0.243 | 0.005 | 0.238 | 0.228 | 63.1 | 1.00× |
| lazy-ruby | 3 | 1.086 | 1.042 | 1.182 | 0.072 | 1.103 | 1.159 | 57.6 | 0.22× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.492 | 0.429 | 0.504 | 0.031 | 0.482 | 0.471 | 62.9 | 1.00× |
| lazy-ruby | 5 | 0.474 | 0.453 | 0.567 | 0.046 | 0.486 | 0.452 | 63.0 | 1.04× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.458 | 0.443 | 0.473 | 0.015 | 0.458 | 0.445 | 64.8 | 1.00× |
| lazy-ruby | 3 | 1.294 | 1.262 | 1.356 | 0.048 | 1.304 | 1.341 | 54.9 | 0.35× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.293 | 0.247 | 0.394 | 0.056 | 0.303 | 0.271 | 165.3 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.244 | 0.499 | 0.104 | 0.317 | 0.265 | 165.4 | 1.03× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.264 | 0.251 | 0.297 | 0.024 | 0.271 | 0.251 | 163.9 | 1.00× |
| lazy-ruby | 3 | 1.126 | 1.092 | 1.143 | 0.026 | 1.120 | 1.203 | 155.2 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.525 | 0.491 | 0.544 | 0.022 | 0.522 | 0.505 | 92.8 | 1.00× |
| lazy-ruby | 5 | 0.516 | 0.462 | 0.573 | 0.044 | 0.516 | 0.492 | 89.9 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.580 | 0.539 | 0.585 | 0.025 | 0.568 | 0.562 | 97.4 | 1.00× |
| lazy-ruby | 3 | 1.397 | 1.384 | 1.469 | 0.046 | 1.417 | 1.474 | 72.2 | 0.42× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.320 | 0.315 | 0.327 | 0.004 | 0.321 | 0.311 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.317 | 0.311 | 0.326 | 0.006 | 0.319 | 0.311 | 57.1 | 1.01× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.337 | 0.313 | 0.584 | 0.150 | 0.411 | 0.329 | 57.4 | 1.00× |
| lazy-ruby | 3 | 2.901 | 2.882 | 2.969 | 0.046 | 2.917 | 3.002 | 43.0 | 0.12× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.654 | 0.645 | 0.671 | 0.011 | 0.654 | 0.648 | 57.2 | 1.00× |
| lazy-ruby | 5 | 0.642 | 0.637 | 0.770 | 0.056 | 0.674 | 0.634 | 57.1 | 1.02× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.643 | 0.640 | 0.647 | 0.004 | 0.643 | 0.636 | 57.0 | 1.00× |
| lazy-ruby | 3 | 3.305 | 3.218 | 3.357 | 0.070 | 3.293 | 3.405 | 43.2 | 0.19× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.370 | 0.369 | 0.387 | 0.009 | 0.376 | 0.362 | 141.7 | 1.00× |
| lazy-ruby | 5 | 0.379 | 0.371 | 0.394 | 0.010 | 0.380 | 0.364 | 141.7 | 0.98× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.388 | 0.373 | 0.418 | 0.023 | 0.393 | 0.368 | 141.7 | 1.00× |
| lazy-ruby | 3 | 2.899 | 2.889 | 2.941 | 0.028 | 2.910 | 2.996 | 127.3 | 0.13× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.597 | 0.589 | 0.600 | 0.005 | 0.596 | 0.584 | 83.0 | 1.00× |
| lazy-ruby | 5 | 0.594 | 0.588 | 0.610 | 0.009 | 0.597 | 0.586 | 82.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.591 | 0.590 | 0.609 | 0.010 | 0.597 | 0.584 | 82.9 | 1.00× |
| lazy-ruby | 3 | 3.223 | 3.207 | 3.235 | 0.014 | 3.221 | 3.311 | 45.8 | 0.18× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.66

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.214 | 0.212 | 0.221 | 0.003 | 0.215 | 0.219 | 33.4 | 1.00× |
| lazy-ruby | 5 | 0.218 | 0.215 | 0.225 | 0.004 | 0.218 | 0.219 | 33.6 | 0.98× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.216 | 0.215 | 0.220 | 0.003 | 0.217 | 0.219 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.724 | 0.722 | 0.731 | 0.004 | 0.726 | 0.734 | 25.4 | 0.30× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.369 | 0.364 | 0.377 | 0.005 | 0.369 | 0.375 | 33.3 | 1.00× |
| lazy-ruby | 5 | 0.369 | 0.365 | 0.380 | 0.006 | 0.371 | 0.375 | 33.4 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.368 | 0.368 | 0.379 | 0.006 | 0.372 | 0.375 | 33.5 | 1.00× |
| lazy-ruby | 3 | 0.878 | 0.872 | 0.887 | 0.007 | 0.879 | 0.891 | 25.5 | 0.42× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.280 | 0.279 | 0.282 | 0.001 | 0.281 | 0.281 | 140.5 | 1.00× |
| lazy-ruby | 5 | 0.289 | 0.285 | 0.293 | 0.003 | 0.288 | 0.281 | 140.6 | 0.97× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.284 | 0.281 | 0.286 | 0.003 | 0.284 | 0.281 | 141.0 | 1.00× |
| lazy-ruby | 3 | 0.796 | 0.791 | 0.796 | 0.003 | 0.794 | 0.812 | 132.9 | 0.36× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.444 | 0.451 | 0.002 | 0.447 | 0.438 | 75.3 | 1.00× |
| lazy-ruby | 5 | 0.448 | 0.442 | 0.474 | 0.012 | 0.453 | 0.453 | 75.2 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.449 | 0.447 | 0.502 | 0.031 | 0.466 | 0.453 | 75.5 | 1.00× |
| lazy-ruby | 3 | 0.948 | 0.945 | 0.956 | 0.006 | 0.950 | 0.938 | 50.5 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

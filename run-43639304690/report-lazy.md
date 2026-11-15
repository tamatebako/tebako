# tebako benchmark report — runtime-lazy-vs-eager

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.227 | 0.227 | 0.229 | 0.001 | 0.228 | 0.226 | 70.5 | 1.00× |
| lazy-ruby | 5 | 0.227 | 0.227 | 0.227 | 0.000 | 0.227 | 0.225 | 70.5 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.227 | 0.227 | 0.227 | 0.000 | 0.227 | 0.226 | 70.5 | 1.00× |
| lazy-ruby | 3 | 2.346 | 2.345 | 2.348 | 0.001 | 2.346 | 2.480 | 49.1 | 0.10× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.431 | 0.429 | 0.433 | 0.002 | 0.431 | 0.430 | 70.6 | 1.00× |
| lazy-ruby | 5 | 0.431 | 0.429 | 0.433 | 0.002 | 0.431 | 0.429 | 70.6 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.431 | 0.431 | 0.431 | 0.000 | 0.431 | 0.429 | 70.6 | 1.00× |
| lazy-ruby | 3 | 2.540 | 2.538 | 2.563 | 0.014 | 2.547 | 2.675 | 49.0 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.254 | 0.254 | 0.256 | 0.001 | 0.255 | 0.253 | 180.3 | 1.00× |
| lazy-ruby | 5 | 0.254 | 0.254 | 0.256 | 0.001 | 0.255 | 0.254 | 180.2 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.254 | 0.254 | 0.256 | 0.001 | 0.255 | 0.254 | 180.2 | 1.00× |
| lazy-ruby | 3 | 2.374 | 2.363 | 2.381 | 0.009 | 2.372 | 2.509 | 156.1 | 0.11× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.410 | 0.408 | 0.412 | 0.002 | 0.410 | 0.410 | 102.2 | 1.00× |
| lazy-ruby | 5 | 0.410 | 0.406 | 0.412 | 0.003 | 0.410 | 0.409 | 102.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.408 | 0.406 | 0.410 | 0.002 | 0.408 | 0.406 | 102.0 | 1.00× |
| lazy-ruby | 3 | 2.525 | 2.517 | 2.536 | 0.009 | 2.526 | 2.659 | 62.1 | 0.16× |

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.198 | 0.192 | 0.198 | 0.003 | 0.197 | 0.196 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.198 | 0.194 | 0.202 | 0.004 | 0.198 | 0.197 | 80.1 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.200 | 0.194 | 0.202 | 0.004 | 0.199 | 0.199 | 79.7 | 1.00× |
| lazy-ruby | 3 | 4.638 | 4.608 | 4.686 | 0.039 | 4.644 | 4.679 | 51.5 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.308 | 0.308 | 0.312 | 0.002 | 0.309 | 0.307 | 79.7 | 1.00× |
| lazy-ruby | 5 | 0.312 | 0.308 | 0.317 | 0.003 | 0.312 | 0.311 | 79.9 | 0.99× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.311 | 0.308 | 0.312 | 0.002 | 0.310 | 0.310 | 79.8 | 1.00× |
| lazy-ruby | 3 | 4.830 | 4.821 | 4.844 | 0.012 | 4.832 | 4.879 | 51.5 | 0.06× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.229 | 0.225 | 0.232 | 0.003 | 0.229 | 0.229 | 188.5 | 1.00× |
| lazy-ruby | 5 | 0.228 | 0.221 | 0.232 | 0.004 | 0.227 | 0.228 | 188.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.230 | 0.229 | 0.231 | 0.001 | 0.230 | 0.228 | 188.7 | 1.00× |
| lazy-ruby | 3 | 4.711 | 4.706 | 4.747 | 0.022 | 4.722 | 4.754 | 157.8 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.345 | 0.341 | 0.359 | 0.007 | 0.347 | 0.343 | 112.2 | 1.00× |
| lazy-ruby | 5 | 0.339 | 0.337 | 0.350 | 0.006 | 0.342 | 0.338 | 112.2 | 1.02× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.339 | 0.335 | 0.341 | 0.003 | 0.338 | 0.339 | 112.0 | 1.00× |
| lazy-ruby | 3 | 4.785 | 4.761 | 5.012 | 0.139 | 4.853 | 4.855 | 64.8 | 0.07× |

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.237 | 0.237 | 0.239 | 0.001 | 0.238 | 0.237 | 72.2 | 1.00× |
| lazy-ruby | 5 | 0.237 | 0.231 | 0.237 | 0.003 | 0.236 | 0.236 | 72.2 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.237 | 0.237 | 0.237 | 0.000 | 0.237 | 0.235 | 72.2 | 1.00× |
| lazy-ruby | 3 | 2.245 | 2.243 | 2.247 | 0.002 | 2.245 | 2.358 | 47.3 | 0.11× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.449 | 0.447 | 0.456 | 0.003 | 0.450 | 0.447 | 72.4 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.446 | 0.450 | 0.002 | 0.448 | 0.446 | 72.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.445 | 0.443 | 0.447 | 0.002 | 0.445 | 0.444 | 72.3 | 1.00× |
| lazy-ruby | 3 | 2.470 | 2.463 | 2.474 | 0.006 | 2.469 | 2.581 | 47.3 | 0.18× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.262 | 0.261 | 0.265 | 0.001 | 0.262 | 0.262 | 182.4 | 1.00× |
| lazy-ruby | 5 | 0.265 | 0.262 | 0.267 | 0.002 | 0.264 | 0.263 | 182.4 | 0.99× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.262 | 0.262 | 0.263 | 0.000 | 0.262 | 0.261 | 182.6 | 1.00× |
| lazy-ruby | 3 | 2.266 | 2.265 | 2.280 | 0.009 | 2.270 | 2.379 | 156.3 | 0.12× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.434 | 0.428 | 0.437 | 0.003 | 0.433 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 5 | 0.435 | 0.429 | 0.437 | 0.003 | 0.434 | 0.433 | 101.8 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.433 | 0.430 | 0.435 | 0.002 | 0.433 | 0.432 | 101.8 | 1.00× |
| lazy-ruby | 3 | 2.452 | 2.449 | 2.465 | 0.009 | 2.455 | 2.562 | 62.7 | 0.18× |

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.253 | 0.251 | 0.265 | 0.005 | 0.255 | 0.252 | 75.0 | 1.00× |
| lazy-ruby | 5 | 0.256 | 0.251 | 0.260 | 0.003 | 0.256 | 0.256 | 75.3 | 0.99× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.249 | 0.247 | 0.249 | 0.001 | 0.248 | 0.247 | 75.0 | 1.00× |
| lazy-ruby | 3 | 5.957 | 5.756 | 5.977 | 0.122 | 5.897 | 5.959 | 49.8 | 0.04× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.418 | 0.416 | 0.448 | 0.014 | 0.424 | 0.417 | 75.2 | 1.00× |
| lazy-ruby | 5 | 0.416 | 0.414 | 0.422 | 0.003 | 0.417 | 0.415 | 75.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.424 | 0.412 | 0.426 | 0.008 | 0.421 | 0.422 | 74.9 | 1.00× |
| lazy-ruby | 3 | 6.146 | 6.011 | 6.183 | 0.090 | 6.113 | 6.148 | 49.7 | 0.07× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.288 | 0.284 | 0.304 | 0.008 | 0.291 | 0.286 | 185.7 | 1.00× |
| lazy-ruby | 5 | 0.284 | 0.282 | 0.303 | 0.009 | 0.288 | 0.284 | 185.9 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.286 | 0.284 | 0.286 | 0.001 | 0.286 | 0.286 | 185.5 | 1.00× |
| lazy-ruby | 3 | 5.996 | 5.941 | 6.033 | 0.047 | 5.990 | 6.000 | 156.1 | 0.05× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.446 | 0.443 | 0.451 | 0.004 | 0.447 | 0.444 | 105.2 | 1.00× |
| lazy-ruby | 5 | 0.447 | 0.447 | 0.449 | 0.001 | 0.448 | 0.447 | 105.3 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.447 | 0.444 | 0.448 | 0.002 | 0.446 | 0.446 | 105.2 | 1.00× |
| lazy-ruby | 3 | 5.992 | 5.957 | 6.216 | 0.140 | 6.055 | 5.993 | 62.9 | 0.07× |

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.213 | 0.184 | 0.221 | 0.014 | 0.210 | 0.201 | 62.6 | 1.00× |
| lazy-ruby | 5 | 0.199 | 0.194 | 0.227 | 0.013 | 0.204 | 0.190 | 61.4 | 1.07× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.196 | 0.188 | 0.208 | 0.010 | 0.197 | 0.188 | 61.3 | 1.00× |
| lazy-ruby | 3 | 1.014 | 0.953 | 1.057 | 0.053 | 1.008 | 1.067 | 52.0 | 0.19× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.423 | 0.422 | 0.433 | 0.005 | 0.425 | 0.408 | 63.5 | 1.00× |
| lazy-ruby | 5 | 0.422 | 0.409 | 0.431 | 0.008 | 0.421 | 0.402 | 61.8 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.400 | 0.399 | 0.420 | 0.012 | 0.406 | 0.384 | 62.0 | 1.00× |
| lazy-ruby | 3 | 1.159 | 1.138 | 1.239 | 0.053 | 1.179 | 1.214 | 53.1 | 0.34× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.224 | 0.213 | 0.234 | 0.009 | 0.222 | 0.202 | 163.9 | 1.00× |
| lazy-ruby | 5 | 0.225 | 0.206 | 0.241 | 0.016 | 0.224 | 0.209 | 163.5 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.219 | 0.218 | 0.229 | 0.006 | 0.222 | 0.209 | 163.9 | 1.00× |
| lazy-ruby | 3 | 0.941 | 0.935 | 0.942 | 0.004 | 0.939 | 0.985 | 146.5 | 0.23× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.393 | 0.369 | 0.485 | 0.045 | 0.409 | 0.387 | 97.6 | 1.00× |
| lazy-ruby | 5 | 0.410 | 0.399 | 0.463 | 0.026 | 0.419 | 0.397 | 91.0 | 0.96× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.437 | 0.423 | 0.449 | 0.013 | 0.436 | 0.426 | 91.1 | 1.00× |
| lazy-ruby | 3 | 1.058 | 1.047 | 1.078 | 0.016 | 1.061 | 1.111 | 64.4 | 0.41× |

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.539 | 0.467 | 0.579 | 0.049 | 0.525 | 0.469 | 57.4 | 1.00× |
| lazy-ruby | 5 | 0.477 | 0.472 | 0.618 | 0.063 | 0.506 | 0.469 | 57.1 | 1.13× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.460 | 0.459 | 0.464 | 0.003 | 0.461 | 0.452 | 57.1 | 1.00× |
| lazy-ruby | 3 | 4.882 | 4.268 | 4.908 | 0.362 | 4.686 | 5.006 | 42.7 | 0.09× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.036 | 1.030 | 1.308 | 0.122 | 1.091 | 1.029 | 57.2 | 1.00× |
| lazy-ruby | 5 | 1.025 | 1.008 | 1.414 | 0.174 | 1.114 | 1.009 | 57.2 | 1.01× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.997 | 0.983 | 1.117 | 0.074 | 1.032 | 0.987 | 57.2 | 1.00× |
| lazy-ruby | 3 | 5.699 | 5.197 | 6.364 | 0.585 | 5.753 | 5.871 | 42.8 | 0.17× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.606 | 0.566 | 0.610 | 0.021 | 0.593 | 0.597 | 141.6 | 1.00× |
| lazy-ruby | 5 | 0.601 | 0.572 | 0.611 | 0.016 | 0.598 | 0.589 | 141.8 | 1.01× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.619 | 0.612 | 0.809 | 0.111 | 0.680 | 0.614 | 141.7 | 1.00× |
| lazy-ruby | 3 | 4.566 | 4.497 | 4.833 | 0.177 | 4.632 | 4.762 | 126.9 | 0.14× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 1.056 | 0.847 | 1.193 | 0.124 | 1.034 | 1.038 | 82.9 | 1.00× |
| lazy-ruby | 5 | 0.972 | 0.878 | 1.041 | 0.059 | 0.961 | 0.958 | 83.1 | 1.09× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 1.014 | 0.951 | 1.066 | 0.057 | 1.010 | 0.999 | 82.9 | 1.00× |
| lazy-ruby | 3 | 4.832 | 4.712 | 5.038 | 0.165 | 4.861 | 5.001 | 47.0 | 0.21× |

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.80

### boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.212 | 0.208 | 0.218 | 0.004 | 0.213 | 0.203 | 68.3 | 1.00× |
| lazy-ruby | 5 | 0.213 | 0.209 | 0.223 | 0.005 | 0.214 | 0.203 | 68.3 | 1.00× |

### boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.212 | 0.209 | 0.214 | 0.002 | 0.212 | 0.234 | 68.4 | 1.00× |
| lazy-ruby | 3 | 0.748 | 0.710 | 0.763 | 0.027 | 0.740 | 0.734 | 58.8 | 0.28× |

### fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.357 | 0.357 | 0.360 | 0.001 | 0.358 | 0.359 | 68.2 | 1.00× |
| lazy-ruby | 5 | 0.358 | 0.356 | 0.367 | 0.006 | 0.361 | 0.359 | 68.3 | 1.00× |

### fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.364 | 0.362 | 0.366 | 0.002 | 0.364 | 0.359 | 68.5 | 1.00× |
| lazy-ruby | 3 | 0.857 | 0.854 | 0.858 | 0.002 | 0.856 | 0.859 | 58.8 | 0.42× |

### ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.284 | 0.280 | 0.289 | 0.003 | 0.284 | 0.297 | 109.3 | 1.00× |
| lazy-ruby | 5 | 0.283 | 0.280 | 0.285 | 0.002 | 0.283 | 0.281 | 109.3 | 1.00× |

### ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.281 | 0.281 | 0.326 | 0.026 | 0.296 | 0.281 | 108.9 | 1.00× |
| lazy-ruby | 3 | 0.782 | 0.770 | 0.790 | 0.010 | 0.780 | 0.797 | 100.6 | 0.36× |

### treewalk — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 5 | 0.436 | 0.435 | 0.442 | 0.003 | 0.438 | 0.422 | 108.9 | 1.00× |
| lazy-ruby | 5 | 0.435 | 0.432 | 0.446 | 0.005 | 0.437 | 0.422 | 108.9 | 1.00× |

### treewalk — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs eager-ruby |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| eager-ruby | 3 | 0.437 | 0.436 | 0.441 | 0.003 | 0.438 | 0.438 | 108.9 | 1.00× |
| lazy-ruby | 3 | 0.936 | 0.925 | 0.945 | 0.010 | 0.935 | 0.922 | 82.9 | 0.47× |

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

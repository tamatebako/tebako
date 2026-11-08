# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.73

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 137.6 | 1.00× |
| tebako-python | 5 | 0.338 | 0.332 | 0.340 | 0.003 | 0.336 | 0.337 | 181.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.333 | 0.332 | 0.334 | 0.001 | 0.333 | 0.332 | 181.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.272 | 0.270 | 0.272 | 0.001 | 0.271 | 0.270 | 137.6 | 1.00× |
| tebako-python | 5 | 0.601 | 0.593 | 0.605 | 0.005 | 0.600 | 0.599 | 181.6 | 0.45× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.600 | 0.595 | 0.605 | 0.007 | 0.600 | 0.598 | 181.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.019 | 0.019 | 0.000 | 0.019 | 0.017 | 137.6 | 1.00× |
| tebako-python | 5 | 0.350 | 0.346 | 0.356 | 0.004 | 0.350 | 0.348 | 184.4 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.349 | 0.346 | 0.352 | 0.004 | 0.349 | 0.349 | 184.3 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.052 | 0.052 | 0.052 | 0.000 | 0.052 | 0.050 | 137.6 | 1.00× |
| tebako-python | 5 | 0.430 | 0.428 | 0.430 | 0.001 | 0.430 | 0.429 | 181.7 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.429 | 0.429 | 0.430 | 0.001 | 0.429 | 0.428 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.027 | 0.031 | 0.002 | 0.029 | 0.027 | 137.6 | 1.00× |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-arm64 under either era's spelling
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-gnu-arm64 under either era's spelling

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.73

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.060 | 0.033 | 0.118 | 0.042 | 0.073 | 0.034 | 143.5 | 1.00× |
| tebako-java | 5 | 0.584 | 0.513 | 0.681 | 0.073 | 0.590 | 0.474 | 189.6 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.503 | 0.483 | 0.522 | 0.027 | 0.503 | 0.482 | 189.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.044 | 0.038 | 0.133 | 0.041 | 0.061 | 0.048 | 143.5 | 1.00× |
| tebako-java | 5 | 0.474 | 0.472 | 0.557 | 0.037 | 0.492 | 0.479 | 189.6 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.502 | 0.492 | 0.512 | 0.014 | 0.502 | 0.493 | 189.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.045 | 0.043 | 0.046 | 0.001 | 0.045 | 0.050 | 143.5 | 1.00× |
| tebako-java | 5 | 0.480 | 0.475 | 0.487 | 0.005 | 0.481 | 0.485 | 191.8 | 0.09× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.488 | 0.487 | 0.488 | 0.001 | 0.488 | 0.493 | 191.2 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.073 | 0.068 | 0.074 | 0.003 | 0.072 | 0.096 | 143.5 | 1.00× |
| tebako-java | 5 | 0.503 | 0.499 | 0.528 | 0.012 | 0.508 | 0.526 | 189.6 | 0.14× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.525 | 0.525 | 0.526 | 0.001 | 0.525 | 0.551 | 189.5 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.178 | 0.174 | 0.193 | 0.008 | 0.182 | 0.408 | 143.5 | 1.00× |
| tebako-java | 5 | 0.626 | 0.622 | 0.650 | 0.013 | 0.632 | 0.850 | 189.6 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.633 | 0.631 | 0.635 | 0.003 | 0.633 | 0.864 | 189.7 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 143.5 | 1.00× |
| tebako-python | 5 | 0.392 | 0.386 | 0.400 | 0.005 | 0.392 | 0.390 | 190.9 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.391 | 0.386 | 0.397 | 0.008 | 0.391 | 0.390 | 190.3 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.266 | 0.262 | 0.274 | 0.006 | 0.267 | 0.266 | 143.5 | 1.00× |
| tebako-python | 5 | 0.612 | 0.602 | 0.613 | 0.005 | 0.609 | 0.611 | 191.0 | 0.43× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.607 | 0.606 | 0.608 | 0.002 | 0.607 | 0.606 | 190.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.019 | 0.019 | 0.000 | 0.019 | 0.017 | 143.5 | 1.00× |
| tebako-python | 5 | 0.402 | 0.396 | 0.447 | 0.021 | 0.411 | 0.401 | 192.9 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.404 | 0.401 | 0.407 | 0.004 | 0.404 | 0.403 | 192.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.066 | 0.066 | 0.068 | 0.001 | 0.067 | 0.066 | 143.5 | 1.00× |
| tebako-python | 5 | 0.466 | 0.462 | 0.470 | 0.003 | 0.467 | 0.465 | 190.6 | 0.14× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.465 | 0.460 | 0.470 | 0.007 | 0.465 | 0.464 | 190.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.029 | 0.031 | 0.001 | 0.030 | 0.029 | 143.5 | 1.00× |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-gnu-x86_64 under either era's spelling
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-arm64 under either era's spelling
- python-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-arm64 under either era's spelling

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-linux-musl-x86_64 under either era's spelling
- python-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-linux-musl-x86_64 under either era's spelling

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.73

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.046 | 0.037 | 0.053 | 0.006 | 0.045 | 0.037 | 34.0 | 1.00× |
| tebako-java | 5 | 0.673 | 0.662 | 0.750 | 0.036 | 0.686 | 0.345 | 40.1 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.524 | 0.502 | 0.547 | 0.032 | 0.524 | 0.295 | 40.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.055 | 0.052 | 0.074 | 0.009 | 0.058 | 0.041 | 34.9 | 1.00× |
| tebako-java | 5 | 0.517 | 0.470 | 0.672 | 0.091 | 0.558 | 0.296 | 41.0 | 0.11× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.503 | 0.499 | 0.508 | 0.006 | 0.503 | 0.288 | 40.8 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.054 | 0.046 | 0.154 | 0.045 | 0.074 | 0.048 | 38.2 | 1.00× |
| tebako-java | 5 | 0.505 | 0.483 | 0.625 | 0.057 | 0.527 | 0.283 | 108.2 | 0.11× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.546 | 0.511 | 0.580 | 0.049 | 0.546 | 0.305 | 108.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.080 | 0.073 | 0.169 | 0.040 | 0.098 | 0.099 | 38.4 | 1.00× |
| tebako-java | 5 | 0.641 | 0.579 | 0.754 | 0.066 | 0.651 | 0.378 | 44.1 | 0.13× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.664 | 0.653 | 0.675 | 0.016 | 0.664 | 0.426 | 44.9 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.167 | 0.127 | 0.177 | 0.020 | 0.160 | 0.238 | 73.9 | 1.00× |
| tebako-java | 5 | 0.615 | 0.503 | 0.632 | 0.062 | 0.576 | 0.459 | 79.3 | 0.27× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.661 | 0.519 | 0.803 | 0.201 | 0.661 | 0.465 | 80.1 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.033 | 0.030 | 0.038 | 0.003 | 0.034 | 0.018 | 11.4 | 1.00× |
| tebako-python | 5 | 0.281 | 0.265 | 0.326 | 0.024 | 0.289 | 0.265 | 127.5 | 0.12× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.292 | 0.272 | 0.312 | 0.028 | 0.292 | 0.275 | 127.1 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.252 | 0.236 | 0.277 | 0.015 | 0.255 | 0.228 | 11.4 | 1.00× |
| tebako-python | 5 | 0.586 | 0.563 | 0.664 | 0.040 | 0.596 | 0.558 | 128.1 | 0.43× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.566 | 0.562 | 0.569 | 0.005 | 0.566 | 0.545 | 128.3 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.033 | 0.029 | 0.041 | 0.004 | 0.034 | 0.025 | 13.5 | 1.00× |
| tebako-python | 5 | 0.291 | 0.269 | 0.356 | 0.038 | 0.310 | 0.276 | 197.1 | 0.11× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.306 | 0.295 | 0.318 | 0.016 | 0.306 | 0.288 | 195.2 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.088 | 0.075 | 0.093 | 0.007 | 0.086 | 0.074 | 19.9 | 1.00× |
| tebako-python | 5 | 0.454 | 0.448 | 0.678 | 0.099 | 0.502 | 0.446 | 139.3 | 0.19× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.849 | 0.784 | 0.914 | 0.092 | 0.849 | 0.753 | 138.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.053 | 0.047 | 0.069 | 0.008 | 0.055 | 0.049 | 18.1 | 1.00× |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-arm64 under either era's spelling
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.73

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.043 | 0.039 | 0.059 | 0.008 | 0.045 | 0.037 | 8.7 | 1.00× |
| tebako-python | 5 | 0.558 | 0.543 | 0.802 | 0.109 | 0.614 | 0.544 | 127.7 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.561 | 0.560 | 0.562 | 0.001 | 0.561 | 0.534 | 127.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.633 | 0.590 | 1.056 | 0.197 | 0.708 | 0.631 | 8.8 | 1.00× |
| tebako-python | 5 | 1.699 | 1.570 | 1.760 | 0.077 | 1.670 | 1.675 | 127.7 | 0.37× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.581 | 1.571 | 1.591 | 0.014 | 1.581 | 1.565 | 127.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.051 | 0.043 | 0.056 | 0.005 | 0.049 | 0.041 | 10.9 | 1.00× |
| tebako-python | 5 | 0.550 | 0.534 | 0.564 | 0.011 | 0.549 | 0.532 | 194.9 | 0.09× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.545 | 0.544 | 0.547 | 0.002 | 0.545 | 0.525 | 195.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.137 | 0.135 | 0.146 | 0.005 | 0.138 | 0.135 | 17.3 | 1.00× |
| tebako-python | 5 | 0.790 | 0.783 | 0.800 | 0.006 | 0.791 | 0.759 | 134.7 | 0.17× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.764 | 0.763 | 0.765 | 0.002 | 0.764 | 0.744 | 134.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.071 | 0.066 | 0.073 | 0.003 | 0.070 | 0.067 | 13.9 | 1.00× |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-macos-x86_64 under either era's spelling
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0 serves no interpreter exe sidecar for 2.7.0-21.0.12-macos-x86_64 under either era's spelling

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.73

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.061 | 0.060 | 0.062 | 0.001 | 0.061 | 0.094 | 331.2 | 1.00× |
| tebako-java | 5 | 0.598 | 0.579 | 0.603 | 0.010 | 0.594 | 0.594 | 469.5 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.587 | 0.586 | 0.588 | 0.002 | 0.587 | 0.594 | 469.5 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.074 | 0.072 | 0.080 | 0.003 | 0.075 | 0.094 | 331.1 | 1.00× |
| tebako-java | 5 | 0.612 | 0.599 | 0.649 | 0.020 | 0.616 | 0.625 | 469.9 | 0.12× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.620 | 0.615 | 0.626 | 0.008 | 0.620 | 0.609 | 469.9 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.088 | 0.086 | 0.094 | 0.003 | 0.089 | 0.125 | 332.7 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.156 | 0.151 | 0.162 | 0.004 | 0.156 | 0.266 | 333.6 | 1.00× |
| tebako-java | 5 | 0.655 | 0.644 | 0.661 | 0.007 | 0.652 | 0.688 | 471.9 | 0.24× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.678 | 0.671 | 0.684 | 0.009 | 0.678 | 0.727 | 472.1 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.261 | 0.256 | 0.265 | 0.004 | 0.261 | 0.594 | 337.8 | 1.00× |
| tebako-java | 5 | 0.784 | 0.774 | 0.791 | 0.007 | 0.784 | 1.109 | 476.9 | 0.33× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.793 | 0.785 | 0.802 | 0.012 | 0.793 | 1.070 | 476.9 | — |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.32 serves no interpreter exe sidecar for 0.16.32-3.3.12-windows-ucrt64 under either era's spelling
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-boot / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-fib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

Failed runs:

- java-ioread / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

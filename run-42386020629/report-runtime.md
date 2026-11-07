# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.71

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 137.7 | 1.00× |
| tebako-python | 5 | 0.342 | 0.340 | 0.358 | 0.008 | 0.344 | 0.340 | 181.6 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.341 | 0.340 | 0.342 | 0.001 | 0.341 | 0.339 | 181.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.270 | 0.270 | 0.272 | 0.001 | 0.271 | 0.270 | 137.7 | 1.00× |
| tebako-python | 5 | 0.601 | 0.599 | 0.623 | 0.010 | 0.605 | 0.601 | 181.6 | 0.45× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.606 | 0.601 | 0.611 | 0.007 | 0.606 | 0.606 | 181.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.019 | 0.019 | 0.000 | 0.019 | 0.018 | 137.7 | 1.00× |
| tebako-python | 5 | 0.355 | 0.353 | 0.362 | 0.004 | 0.356 | 0.355 | 184.4 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.368 | 0.366 | 0.370 | 0.003 | 0.368 | 0.367 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.054 | 0.054 | 0.056 | 0.001 | 0.054 | 0.053 | 137.7 | 1.00× |
| tebako-python | 5 | 0.453 | 0.449 | 0.456 | 0.003 | 0.453 | 0.451 | 181.7 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.457 | 0.456 | 0.458 | 0.001 | 0.457 | 0.456 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.031 | 0.031 | 0.031 | 0.000 | 0.031 | 0.030 | 137.7 | 1.00× |

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
Versions: tebako 2.8.71

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.031 | 0.027 | 0.137 | 0.047 | 0.057 | 0.033 | 142.8 | 1.00× |
| tebako-java | 5 | 0.461 | 0.457 | 0.606 | 0.064 | 0.493 | 0.464 | 189.7 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.522 | 0.476 | 0.568 | 0.065 | 0.522 | 0.468 | 189.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.042 | 0.040 | 0.044 | 0.002 | 0.042 | 0.047 | 142.8 | 1.00× |
| tebako-java | 5 | 0.479 | 0.472 | 0.484 | 0.005 | 0.478 | 0.484 | 189.7 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.475 | 0.469 | 0.480 | 0.008 | 0.475 | 0.479 | 189.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.047 | 0.044 | 0.049 | 0.002 | 0.047 | 0.051 | 142.8 | 1.00× |
| tebako-java | 5 | 0.476 | 0.472 | 0.484 | 0.005 | 0.476 | 0.480 | 191.7 | 0.10× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.479 | 0.477 | 0.481 | 0.003 | 0.479 | 0.483 | 191.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.076 | 0.065 | 0.084 | 0.007 | 0.075 | 0.097 | 142.8 | 1.00× |
| tebako-java | 5 | 0.505 | 0.488 | 0.511 | 0.009 | 0.503 | 0.525 | 189.7 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.509 | 0.505 | 0.512 | 0.005 | 0.509 | 0.532 | 189.5 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.173 | 0.168 | 0.177 | 0.004 | 0.173 | 0.405 | 142.8 | 1.00× |
| tebako-java | 5 | 0.607 | 0.589 | 0.615 | 0.010 | 0.604 | 0.833 | 189.6 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.608 | 0.599 | 0.617 | 0.013 | 0.608 | 0.834 | 189.7 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 142.8 | 1.00× |
| tebako-python | 5 | 0.385 | 0.377 | 0.388 | 0.004 | 0.384 | 0.383 | 190.5 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.394 | 0.392 | 0.396 | 0.003 | 0.394 | 0.393 | 191.0 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.256 | 0.252 | 0.260 | 0.003 | 0.257 | 0.255 | 142.8 | 1.00× |
| tebako-python | 5 | 0.598 | 0.594 | 0.608 | 0.006 | 0.600 | 0.598 | 190.8 | 0.43× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.604 | 0.602 | 0.606 | 0.003 | 0.604 | 0.603 | 190.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.017 | 0.017 | 0.017 | 0.000 | 0.017 | 0.016 | 142.8 | 1.00× |
| tebako-python | 5 | 0.394 | 0.390 | 0.398 | 0.004 | 0.394 | 0.392 | 193.0 | 0.04× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.394 | 0.394 | 0.394 | 0.000 | 0.394 | 0.393 | 193.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.064 | 0.064 | 0.064 | 0.000 | 0.064 | 0.063 | 142.8 | 1.00× |
| tebako-python | 5 | 0.459 | 0.454 | 0.464 | 0.004 | 0.459 | 0.458 | 190.9 | 0.14× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.455 | 0.452 | 0.458 | 0.004 | 0.455 | 0.454 | 191.1 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.029 | 0.029 | 0.000 | 0.029 | 0.028 | 142.8 | 1.00× |

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

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB

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

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.71

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.042 | 0.038 | 0.052 | 0.006 | 0.044 | 0.039 | 8.7 | 1.00× |
| tebako-python | 5 | 0.565 | 0.547 | 0.921 | 0.162 | 0.631 | 0.545 | 127.7 | 0.07× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.548 | 0.546 | 0.551 | 0.003 | 0.548 | 0.532 | 127.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.670 | 0.643 | 0.717 | 0.027 | 0.675 | 0.665 | 8.8 | 1.00× |
| tebako-python | 5 | 1.836 | 1.770 | 1.855 | 0.033 | 1.823 | 1.809 | 127.7 | 0.37× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.777 | 1.762 | 1.793 | 0.022 | 1.777 | 1.761 | 127.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.050 | 0.047 | 0.061 | 0.006 | 0.052 | 0.046 | 10.9 | 1.00× |
| tebako-python | 5 | 0.621 | 0.606 | 0.636 | 0.014 | 0.621 | 0.596 | 194.9 | 0.08× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.599 | 0.594 | 0.604 | 0.008 | 0.599 | 0.577 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.155 | 0.147 | 0.161 | 0.006 | 0.153 | 0.146 | 17.2 | 1.00× |
| tebako-python | 5 | 0.850 | 0.844 | 0.877 | 0.013 | 0.854 | 0.826 | 134.7 | 0.18× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.851 | 0.850 | 0.852 | 0.001 | 0.851 | 0.838 | 135.4 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.090 | 0.078 | 0.101 | 0.008 | 0.090 | 0.074 | 14.0 | 1.00× |

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
Versions: tebako 2.8.71

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.043 | 0.042 | 0.054 | 0.005 | 0.045 | 0.062 | 331.3 | 1.00× |
| tebako-java | 5 | 0.441 | 0.419 | 0.705 | 0.136 | 0.525 | 0.406 | 469.6 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.430 | 0.420 | 0.440 | 0.014 | 0.430 | 0.398 | 469.8 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.049 | 0.047 | 0.057 | 0.004 | 0.050 | 0.078 | 331.4 | 1.00× |
| tebako-java | 5 | 0.451 | 0.428 | 0.991 | 0.248 | 0.610 | 0.406 | 469.8 | 0.11× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.441 | 0.439 | 0.444 | 0.004 | 0.441 | 0.398 | 469.9 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.060 | 0.060 | 0.062 | 0.001 | 0.061 | 0.062 | 333.1 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.087 | 0.084 | 0.096 | 0.005 | 0.088 | 0.156 | 333.7 | 1.00× |
| tebako-java | 5 | 0.466 | 0.461 | 0.479 | 0.007 | 0.468 | 0.469 | 471.5 | 0.19× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.473 | 0.465 | 0.482 | 0.012 | 0.473 | 0.477 | 471.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.162 | 0.158 | 0.175 | 0.008 | 0.165 | 0.344 | 338.5 | 1.00× |
| tebako-java | 5 | 0.551 | 0.542 | 0.580 | 0.016 | 0.556 | 0.703 | 476.9 | 0.29× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.571 | 0.546 | 0.597 | 0.036 | 0.571 | 0.742 | 477.9 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.70

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 137.7 | 1.00× |
| tebako-python | 5 | 0.335 | 0.333 | 0.336 | 0.001 | 0.335 | 0.334 | 181.6 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.335 | 0.333 | 0.336 | 0.002 | 0.335 | 0.333 | 181.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.270 | 0.270 | 0.278 | 0.004 | 0.272 | 0.270 | 137.7 | 1.00× |
| tebako-python | 5 | 0.605 | 0.599 | 0.607 | 0.003 | 0.603 | 0.604 | 181.6 | 0.45× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.608 | 0.603 | 0.613 | 0.007 | 0.608 | 0.607 | 181.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.017 | 0.019 | 0.001 | 0.018 | 0.017 | 137.7 | 1.00× |
| tebako-python | 5 | 0.348 | 0.344 | 0.356 | 0.005 | 0.348 | 0.347 | 184.4 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.357 | 0.356 | 0.358 | 0.001 | 0.357 | 0.357 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.052 | 0.052 | 0.052 | 0.000 | 0.052 | 0.050 | 137.7 | 1.00× |
| tebako-python | 5 | 0.426 | 0.426 | 0.440 | 0.007 | 0.431 | 0.426 | 181.7 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.438 | 0.432 | 0.443 | 0.007 | 0.438 | 0.436 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.029 | 0.029 | 0.000 | 0.029 | 0.028 | 137.7 | 1.00× |

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
Versions: tebako 2.8.70

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.033 | 0.040 | 0.003 | 0.036 | 0.039 | 143.1 | 1.00× |
| tebako-java | 5 | 0.563 | 0.544 | 0.751 | 0.087 | 0.595 | 0.561 | 189.5 | 0.06× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.549 | 0.538 | 0.560 | 0.015 | 0.549 | 0.553 | 189.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.051 | 0.044 | 0.057 | 0.005 | 0.051 | 0.056 | 143.1 | 1.00× |
| tebako-java | 5 | 0.574 | 0.562 | 0.594 | 0.012 | 0.578 | 0.581 | 189.4 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.571 | 0.565 | 0.577 | 0.009 | 0.571 | 0.576 | 189.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.077 | 0.075 | 0.085 | 0.005 | 0.080 | 0.084 | 143.1 | 1.00× |
| tebako-java | 5 | 0.577 | 0.569 | 0.584 | 0.006 | 0.578 | 0.580 | 191.6 | 0.13× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.580 | 0.580 | 0.581 | 0.001 | 0.580 | 0.584 | 191.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.096 | 0.089 | 0.098 | 0.004 | 0.094 | 0.136 | 143.1 | 1.00× |
| tebako-java | 5 | 0.614 | 0.611 | 0.633 | 0.009 | 0.618 | 0.652 | 189.5 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.609 | 0.609 | 0.609 | 0.000 | 0.609 | 0.646 | 189.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.211 | 0.203 | 0.225 | 0.008 | 0.213 | 0.410 | 143.1 | 1.00× |
| tebako-java | 5 | 0.745 | 0.739 | 0.894 | 0.068 | 0.773 | 0.958 | 189.6 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.752 | 0.729 | 0.776 | 0.033 | 0.752 | 1.056 | 189.4 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.015 | 0.015 | 0.000 | 0.015 | 0.013 | 143.1 | 1.00× |
| tebako-python | 5 | 0.474 | 0.471 | 0.500 | 0.013 | 0.480 | 0.473 | 190.7 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.480 | 0.472 | 0.488 | 0.012 | 0.480 | 0.479 | 190.2 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.306 | 0.306 | 0.319 | 0.005 | 0.309 | 0.305 | 143.1 | 1.00× |
| tebako-python | 5 | 0.780 | 0.740 | 0.820 | 0.028 | 0.780 | 0.778 | 190.2 | 0.39× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.743 | 0.743 | 0.744 | 0.001 | 0.743 | 0.742 | 190.4 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.021 | 0.021 | 0.021 | 0.000 | 0.021 | 0.019 | 143.1 | 1.00× |
| tebako-python | 5 | 0.480 | 0.479 | 0.493 | 0.006 | 0.483 | 0.479 | 192.7 | 0.04× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.492 | 0.478 | 0.506 | 0.020 | 0.492 | 0.491 | 192.7 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.075 | 0.075 | 0.075 | 0.000 | 0.075 | 0.074 | 143.1 | 1.00× |
| tebako-python | 5 | 0.565 | 0.554 | 0.568 | 0.006 | 0.562 | 0.564 | 190.4 | 0.13× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.565 | 0.565 | 0.565 | 0.000 | 0.565 | 0.563 | 190.8 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.036 | 0.035 | 0.036 | 0.000 | 0.036 | 0.034 | 143.1 | 1.00× |

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB

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
Versions: tebako 2.8.70

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.065 | 0.050 | 0.068 | 0.008 | 0.060 | 0.050 | 8.7 | 1.00× |
| tebako-python | 5 | 0.737 | 0.678 | 0.915 | 0.092 | 0.757 | 0.713 | 127.7 | 0.09× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.719 | 0.712 | 0.726 | 0.009 | 0.719 | 0.693 | 127.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.799 | 0.792 | 0.885 | 0.039 | 0.819 | 0.796 | 8.8 | 1.00× |
| tebako-python | 5 | 2.160 | 2.142 | 2.208 | 0.026 | 2.169 | 2.143 | 127.6 | 0.37× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.168 | 2.165 | 2.170 | 0.004 | 2.168 | 2.147 | 127.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.070 | 0.062 | 0.074 | 0.005 | 0.068 | 0.058 | 10.9 | 1.00× |
| tebako-python | 5 | 0.757 | 0.750 | 0.801 | 0.021 | 0.765 | 0.741 | 194.9 | 0.09× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.759 | 0.752 | 0.767 | 0.011 | 0.759 | 0.740 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.214 | 0.208 | 0.252 | 0.019 | 0.223 | 0.210 | 17.3 | 1.00× |
| tebako-python | 5 | 1.120 | 1.078 | 1.296 | 0.089 | 1.147 | 1.099 | 134.7 | 0.19× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.136 | 1.110 | 1.162 | 0.037 | 1.136 | 1.112 | 134.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.106 | 0.102 | 0.161 | 0.025 | 0.121 | 0.102 | 14.0 | 1.00× |

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
Versions: tebako 2.8.70

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.055 | 0.051 | 0.057 | 0.002 | 0.054 | 0.062 | 331.2 | 1.00× |
| tebako-java | 5 | 0.676 | 0.561 | 0.958 | 0.157 | 0.707 | 0.547 | 469.5 | 0.08× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.700 | 0.541 | 0.858 | 0.224 | 0.700 | 0.492 | 469.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.063 | 0.062 | 0.067 | 0.002 | 0.064 | 0.078 | 331.4 | 1.00× |
| tebako-java | 5 | 0.584 | 0.560 | 0.679 | 0.046 | 0.598 | 0.562 | 469.7 | 0.11× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.589 | 0.586 | 0.592 | 0.004 | 0.589 | 0.531 | 469.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.074 | 0.072 | 0.077 | 0.002 | 0.074 | 0.078 | 332.7 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.114 | 0.111 | 0.219 | 0.048 | 0.135 | 0.188 | 333.7 | 1.00× |
| tebako-java | 5 | 0.590 | 0.581 | 0.622 | 0.016 | 0.595 | 0.562 | 472.0 | 0.19× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.595 | 0.587 | 0.604 | 0.012 | 0.595 | 0.641 | 472.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.205 | 0.194 | 0.223 | 0.013 | 0.209 | 0.453 | 337.4 | 1.00× |
| tebako-java | 5 | 0.722 | 0.717 | 0.752 | 0.014 | 0.728 | 0.938 | 476.2 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.718 | 0.713 | 0.723 | 0.007 | 0.718 | 0.922 | 476.5 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

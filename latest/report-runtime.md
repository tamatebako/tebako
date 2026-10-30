# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.68

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 137.7 | 1.00× |
| tebako-python | 5 | 0.335 | 0.333 | 0.350 | 0.007 | 0.338 | 0.334 | 181.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.333 | 0.333 | 0.333 | 0.000 | 0.333 | 0.332 | 181.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.272 | 0.270 | 0.276 | 0.002 | 0.273 | 0.271 | 137.7 | 1.00× |
| tebako-python | 5 | 0.599 | 0.599 | 0.607 | 0.004 | 0.600 | 0.598 | 181.7 | 0.45× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.610 | 0.609 | 0.611 | 0.002 | 0.610 | 0.608 | 181.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.019 | 0.019 | 0.000 | 0.019 | 0.017 | 137.7 | 1.00× |
| tebako-python | 5 | 0.354 | 0.344 | 0.358 | 0.006 | 0.352 | 0.354 | 184.4 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.357 | 0.354 | 0.360 | 0.004 | 0.357 | 0.356 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.052 | 0.052 | 0.052 | 0.000 | 0.052 | 0.051 | 137.7 | 1.00× |
| tebako-python | 5 | 0.436 | 0.428 | 0.440 | 0.005 | 0.436 | 0.436 | 181.6 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.440 | 0.439 | 0.442 | 0.003 | 0.440 | 0.439 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.031 | 0.029 | 0.031 | 0.001 | 0.030 | 0.029 | 137.7 | 1.00× |

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15982.9 GiB
Versions: tebako 2.8.68

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.045 | 0.041 | 0.140 | 0.043 | 0.063 | 0.047 | 142.6 | 1.00× |
| tebako-java | 5 | 0.565 | 0.518 | 0.701 | 0.078 | 0.599 | 0.569 | 189.8 | 0.08× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.615 | 0.610 | 0.619 | 0.006 | 0.615 | 0.622 | 189.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.063 | 0.057 | 0.100 | 0.017 | 0.070 | 0.071 | 142.6 | 1.00× |
| tebako-java | 5 | 0.612 | 0.558 | 0.758 | 0.079 | 0.623 | 0.580 | 189.7 | 0.10× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.570 | 0.557 | 0.583 | 0.019 | 0.570 | 0.577 | 189.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.070 | 0.061 | 0.085 | 0.009 | 0.071 | 0.080 | 142.6 | 1.00× |
| tebako-java | 5 | 0.580 | 0.570 | 0.647 | 0.032 | 0.591 | 0.591 | 191.7 | 0.12× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.630 | 0.599 | 0.662 | 0.044 | 0.630 | 0.642 | 191.6 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.088 | 0.080 | 0.092 | 0.005 | 0.087 | 0.124 | 142.6 | 1.00× |
| tebako-java | 5 | 0.609 | 0.568 | 0.650 | 0.029 | 0.609 | 0.649 | 189.5 | 0.14× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.620 | 0.605 | 0.634 | 0.020 | 0.620 | 0.649 | 189.5 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.229 | 0.220 | 0.259 | 0.017 | 0.235 | 0.528 | 142.6 | 1.00× |
| tebako-java | 5 | 0.795 | 0.706 | 0.829 | 0.047 | 0.776 | 1.046 | 189.7 | 0.29× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.739 | 0.721 | 0.756 | 0.025 | 0.739 | 1.078 | 188.8 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.017 | 0.015 | 0.020 | 0.002 | 0.017 | 0.016 | 142.6 | 1.00× |
| tebako-python | 5 | 0.467 | 0.444 | 0.485 | 0.016 | 0.464 | 0.466 | 190.9 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.448 | 0.429 | 0.468 | 0.027 | 0.448 | 0.447 | 190.9 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.257 | 0.241 | 0.270 | 0.011 | 0.257 | 0.255 | 142.6 | 1.00× |
| tebako-python | 5 | 0.695 | 0.640 | 0.698 | 0.024 | 0.683 | 0.693 | 190.6 | 0.37× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.634 | 0.630 | 0.637 | 0.005 | 0.634 | 0.633 | 191.1 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.023 | 0.023 | 0.029 | 0.003 | 0.025 | 0.023 | 142.6 | 1.00× |
| tebako-python | 5 | 0.432 | 0.425 | 0.465 | 0.016 | 0.439 | 0.432 | 193.1 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.450 | 0.432 | 0.468 | 0.026 | 0.450 | 0.449 | 192.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.058 | 0.056 | 0.060 | 0.002 | 0.059 | 0.057 | 142.6 | 1.00× |
| tebako-python | 5 | 0.499 | 0.487 | 0.544 | 0.023 | 0.506 | 0.498 | 190.5 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.515 | 0.500 | 0.530 | 0.021 | 0.515 | 0.513 | 190.4 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.042 | 0.042 | 0.043 | 0.000 | 0.042 | 0.041 | 142.6 | 1.00× |

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

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.68

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.036 | 0.046 | 0.005 | 0.038 | 0.028 | 36.0 | 1.00× |
| tebako-java | 5 | 0.637 | 0.594 | 0.699 | 0.043 | 0.648 | 0.329 | 43.9 | 0.06× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.549 | 0.546 | 0.552 | 0.005 | 0.549 | 0.287 | 44.0 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.049 | 0.035 | 0.072 | 0.015 | 0.049 | 0.035 | 36.8 | 1.00× |
| tebako-java | 5 | 0.533 | 0.503 | 0.604 | 0.040 | 0.537 | 0.275 | 44.8 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.443 | 0.437 | 0.450 | 0.009 | 0.443 | 0.246 | 44.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.054 | 0.048 | 0.066 | 0.008 | 0.057 | 0.042 | 40.1 | 1.00× |
| tebako-java | 5 | 0.513 | 0.503 | 0.649 | 0.061 | 0.543 | 0.274 | 112.0 | 0.10× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.474 | 0.473 | 0.474 | 0.000 | 0.474 | 0.263 | 112.1 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.079 | 0.067 | 0.112 | 0.018 | 0.082 | 0.067 | 40.8 | 1.00× |
| tebako-java | 5 | 0.560 | 0.475 | 0.581 | 0.051 | 0.534 | 0.299 | 47.9 | 0.14× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.526 | 0.518 | 0.534 | 0.012 | 0.526 | 0.297 | 48.9 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.174 | 0.144 | 0.223 | 0.032 | 0.179 | 0.248 | 75.7 | 1.00× |
| tebako-java | 5 | 0.671 | 0.595 | 0.682 | 0.036 | 0.654 | 0.492 | 83.3 | 0.26× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.622 | 0.590 | 0.655 | 0.046 | 0.622 | 0.473 | 83.5 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.017 | 0.030 | 0.005 | 0.021 | 0.013 | 11.6 | 1.00× |
| tebako-python | 5 | 0.248 | 0.224 | 0.259 | 0.015 | 0.242 | 0.228 | 129.8 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.245 | 0.236 | 0.253 | 0.011 | 0.245 | 0.227 | 129.4 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.225 | 0.198 | 0.231 | 0.014 | 0.219 | 0.202 | 11.6 | 1.00× |
| tebako-python | 5 | 0.462 | 0.444 | 0.507 | 0.024 | 0.468 | 0.441 | 129.8 | 0.49× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.448 | 0.439 | 0.457 | 0.012 | 0.448 | 0.435 | 130.1 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.032 | 0.018 | 0.042 | 0.009 | 0.031 | 0.019 | 13.5 | 1.00× |
| tebako-python | 5 | 0.256 | 0.240 | 0.263 | 0.009 | 0.253 | 0.233 | 197.9 | 0.13× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.240 | 0.240 | 0.240 | 0.000 | 0.240 | 0.232 | 198.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.057 | 0.054 | 0.066 | 0.005 | 0.059 | 0.051 | 20.0 | 1.00× |
| tebako-python | 5 | 0.372 | 0.366 | 0.383 | 0.008 | 0.375 | 0.362 | 138.9 | 0.15× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.359 | 0.354 | 0.364 | 0.007 | 0.359 | 0.350 | 138.2 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.033 | 0.030 | 0.045 | 0.006 | 0.036 | 0.027 | 17.8 | 1.00× |

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
Versions: tebako 2.8.68

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.055 | 0.051 | 0.067 | 0.007 | 0.058 | 0.048 | 8.7 | 1.00× |
| tebako-python | 5 | 0.671 | 0.651 | 0.752 | 0.041 | 0.684 | 0.651 | 127.7 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.709 | 0.680 | 0.737 | 0.040 | 0.709 | 0.645 | 127.9 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.715 | 0.689 | 0.747 | 0.023 | 0.716 | 0.712 | 8.8 | 1.00× |
| tebako-python | 5 | 1.948 | 1.917 | 2.026 | 0.043 | 1.953 | 1.929 | 127.7 | 0.37× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.018 | 1.991 | 2.046 | 0.039 | 2.018 | 2.004 | 127.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.061 | 0.058 | 0.070 | 0.005 | 0.063 | 0.056 | 10.8 | 1.00× |
| tebako-python | 5 | 0.731 | 0.711 | 0.739 | 0.012 | 0.726 | 0.713 | 194.9 | 0.08× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.705 | 0.683 | 0.728 | 0.032 | 0.705 | 0.682 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.194 | 0.177 | 0.210 | 0.012 | 0.193 | 0.184 | 17.3 | 1.00× |
| tebako-python | 5 | 1.031 | 0.977 | 1.048 | 0.030 | 1.020 | 1.011 | 134.7 | 0.19× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.024 | 1.010 | 1.039 | 0.020 | 1.024 | 1.003 | 134.9 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.107 | 0.096 | 0.117 | 0.009 | 0.108 | 0.100 | 14.0 | 1.00× |

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
Versions: tebako 2.8.68

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.062 | 0.060 | 0.073 | 0.005 | 0.064 | 0.078 | 331.0 | 1.00× |
| tebako-java | 5 | 0.591 | 0.585 | 0.618 | 0.013 | 0.595 | 0.594 | 469.4 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.629 | 0.595 | 0.664 | 0.049 | 0.629 | 0.633 | 469.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.078 | 0.072 | 0.081 | 0.004 | 0.077 | 0.109 | 331.1 | 1.00× |
| tebako-java | 5 | 0.611 | 0.604 | 0.624 | 0.008 | 0.612 | 0.594 | 469.8 | 0.13× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.625 | 0.622 | 0.627 | 0.004 | 0.625 | 0.594 | 469.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.089 | 0.086 | 0.093 | 0.002 | 0.089 | 0.109 | 332.6 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.152 | 0.148 | 0.154 | 0.002 | 0.151 | 0.234 | 333.6 | 1.00× |
| tebako-java | 5 | 0.654 | 0.650 | 0.670 | 0.008 | 0.657 | 0.719 | 471.9 | 0.23× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.676 | 0.672 | 0.681 | 0.006 | 0.676 | 0.758 | 471.8 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.249 | 0.246 | 0.264 | 0.007 | 0.253 | 0.641 | 338.0 | 1.00× |
| tebako-java | 5 | 0.786 | 0.771 | 0.795 | 0.010 | 0.785 | 1.078 | 477.2 | 0.32× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.779 | 0.772 | 0.786 | 0.010 | 0.779 | 1.078 | 476.5 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15982.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

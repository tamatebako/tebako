# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.61

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 137.8 | 1.00× |
| tebako-python | 5 | 0.313 | 0.311 | 0.319 | 0.003 | 0.314 | 0.311 | 181.6 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.312 | 0.311 | 0.313 | 0.001 | 0.312 | 0.310 | 181.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.272 | 0.270 | 0.276 | 0.002 | 0.272 | 0.271 | 137.8 | 1.00× |
| tebako-python | 5 | 0.583 | 0.576 | 0.586 | 0.004 | 0.582 | 0.582 | 181.6 | 0.47× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.583 | 0.581 | 0.585 | 0.003 | 0.583 | 0.581 | 181.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.019 | 0.019 | 0.000 | 0.019 | 0.017 | 137.8 | 1.00× |
| tebako-python | 5 | 0.321 | 0.317 | 0.331 | 0.006 | 0.324 | 0.320 | 184.4 | 0.06× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.327 | 0.327 | 0.327 | 0.000 | 0.327 | 0.326 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.054 | 0.052 | 0.054 | 0.001 | 0.053 | 0.052 | 137.8 | 1.00× |
| tebako-python | 5 | 0.412 | 0.404 | 0.418 | 0.005 | 0.411 | 0.410 | 181.7 | 0.13× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.415 | 0.414 | 0.416 | 0.002 | 0.415 | 0.414 | 181.6 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.031 | 0.031 | 0.031 | 0.000 | 0.031 | 0.030 | 137.8 | 1.00× |

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.61

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.037 | 0.034 | 0.041 | 0.003 | 0.037 | 0.039 | 143.4 | 1.00× |
| tebako-java | 5 | 0.531 | 0.519 | 0.803 | 0.123 | 0.583 | 0.530 | 188.6 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.511 | 0.507 | 0.514 | 0.004 | 0.511 | 0.515 | 188.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.046 | 0.044 | 0.059 | 0.006 | 0.048 | 0.052 | 143.4 | 1.00× |
| tebako-java | 5 | 0.529 | 0.521 | 0.564 | 0.020 | 0.539 | 0.530 | 188.5 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.491 | 0.478 | 0.503 | 0.018 | 0.491 | 0.496 | 188.4 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.079 | 0.075 | 0.084 | 0.004 | 0.079 | 0.085 | 143.4 | 1.00× |
| tebako-java | 5 | 0.555 | 0.552 | 0.566 | 0.006 | 0.557 | 0.561 | 190.5 | 0.14× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.495 | 0.488 | 0.501 | 0.010 | 0.495 | 0.500 | 190.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.088 | 0.083 | 0.093 | 0.004 | 0.088 | 0.122 | 143.4 | 1.00× |
| tebako-java | 5 | 0.594 | 0.528 | 0.597 | 0.031 | 0.574 | 0.622 | 188.5 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.548 | 0.521 | 0.575 | 0.038 | 0.548 | 0.589 | 188.5 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.207 | 0.196 | 0.226 | 0.012 | 0.210 | 0.472 | 143.4 | 1.00× |
| tebako-java | 5 | 0.719 | 0.649 | 0.751 | 0.040 | 0.716 | 1.014 | 188.5 | 0.29× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.699 | 0.673 | 0.725 | 0.037 | 0.699 | 1.018 | 188.4 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.015 | 0.015 | 0.000 | 0.015 | 0.013 | 143.4 | 1.00× |
| tebako-python | 5 | 0.463 | 0.449 | 0.470 | 0.009 | 0.461 | 0.462 | 195.0 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.475 | 0.461 | 0.489 | 0.020 | 0.475 | 0.474 | 193.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.311 | 0.304 | 0.311 | 0.003 | 0.309 | 0.308 | 143.4 | 1.00× |
| tebako-python | 5 | 0.736 | 0.724 | 0.755 | 0.013 | 0.738 | 0.735 | 189.6 | 0.42× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.744 | 0.726 | 0.761 | 0.025 | 0.744 | 0.742 | 194.5 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.021 | 0.021 | 0.023 | 0.001 | 0.021 | 0.020 | 143.4 | 1.00× |
| tebako-python | 5 | 0.474 | 0.460 | 0.480 | 0.008 | 0.472 | 0.473 | 195.9 | 0.04× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.468 | 0.467 | 0.468 | 0.000 | 0.468 | 0.466 | 196.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.079 | 0.077 | 0.079 | 0.001 | 0.078 | 0.077 | 143.4 | 1.00× |
| tebako-python | 5 | 0.554 | 0.548 | 0.565 | 0.006 | 0.555 | 0.553 | 195.3 | 0.14× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.559 | 0.556 | 0.562 | 0.004 | 0.559 | 0.557 | 191.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.035 | 0.035 | 0.035 | 0.000 | 0.035 | 0.034 | 143.4 | 1.00× |

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

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB

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
Versions: tebako 2.8.61

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.043 | 0.042 | 0.061 | 0.008 | 0.048 | 0.034 | 35.9 | 1.00× |
| tebako-java | 5 | 0.782 | 0.724 | 0.827 | 0.046 | 0.780 | 0.388 | 43.4 | 0.06× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.714 | 0.691 | 0.738 | 0.033 | 0.714 | 0.391 | 43.4 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.063 | 0.046 | 0.070 | 0.009 | 0.061 | 0.048 | 36.9 | 1.00× |
| tebako-java | 5 | 0.758 | 0.682 | 0.974 | 0.117 | 0.777 | 0.402 | 44.2 | 0.08× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.602 | 0.585 | 0.619 | 0.024 | 0.602 | 0.336 | 44.3 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.058 | 0.050 | 0.067 | 0.006 | 0.058 | 0.042 | 40.1 | 1.00× |
| tebako-java | 5 | 0.607 | 0.527 | 0.735 | 0.082 | 0.609 | 0.322 | 111.6 | 0.10× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.609 | 0.607 | 0.611 | 0.002 | 0.609 | 0.312 | 111.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.108 | 0.065 | 0.161 | 0.034 | 0.110 | 0.086 | 41.9 | 1.00× |
| tebako-java | 5 | 0.664 | 0.498 | 0.718 | 0.089 | 0.648 | 0.363 | 47.4 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.688 | 0.633 | 0.742 | 0.077 | 0.688 | 0.392 | 47.3 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.258 | 0.246 | 0.299 | 0.025 | 0.270 | 0.357 | 76.2 | 1.00× |
| tebako-java | 5 | 0.905 | 0.766 | 0.994 | 0.097 | 0.883 | 0.645 | 83.5 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.788 | 0.753 | 0.822 | 0.049 | 0.788 | 0.606 | 82.7 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.037 | 0.017 | 0.041 | 0.010 | 0.033 | 0.016 | 11.6 | 1.00× |
| tebako-python | 5 | 0.273 | 0.257 | 0.289 | 0.014 | 0.274 | 0.255 | 128.6 | 0.14× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.272 | 0.271 | 0.272 | 0.000 | 0.272 | 0.254 | 130.2 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.224 | 0.219 | 0.230 | 0.004 | 0.225 | 0.209 | 11.6 | 1.00× |
| tebako-python | 5 | 0.533 | 0.485 | 0.713 | 0.092 | 0.554 | 0.497 | 130.1 | 0.42× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.671 | 0.578 | 0.763 | 0.131 | 0.671 | 0.645 | 129.9 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.040 | 0.034 | 0.043 | 0.003 | 0.039 | 0.024 | 13.7 | 1.00× |
| tebako-python | 5 | 0.294 | 0.275 | 0.331 | 0.022 | 0.302 | 0.266 | 198.4 | 0.13× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.273 | 0.267 | 0.280 | 0.009 | 0.273 | 0.258 | 197.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.065 | 0.058 | 0.086 | 0.013 | 0.071 | 0.057 | 20.0 | 1.00× |
| tebako-python | 5 | 0.431 | 0.401 | 0.439 | 0.018 | 0.422 | 0.405 | 138.6 | 0.15× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.403 | 0.401 | 0.406 | 0.003 | 0.403 | 0.389 | 139.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.050 | 0.036 | 0.069 | 0.013 | 0.049 | 0.035 | 18.2 | 1.00× |

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
Versions: tebako 2.8.61

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.038 | 0.032 | 0.041 | 0.004 | 0.037 | 0.030 | 8.7 | 1.00× |
| tebako-python | 5 | 0.466 | 0.446 | 0.483 | 0.014 | 0.466 | 0.448 | 127.7 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.466 | 0.463 | 0.469 | 0.004 | 0.466 | 0.454 | 127.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.543 | 0.518 | 0.563 | 0.020 | 0.541 | 0.541 | 8.8 | 1.00× |
| tebako-python | 5 | 1.434 | 1.408 | 1.524 | 0.046 | 1.444 | 1.418 | 127.7 | 0.38× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.447 | 1.433 | 1.460 | 0.019 | 1.447 | 1.422 | 127.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.043 | 0.041 | 0.076 | 0.015 | 0.051 | 0.039 | 10.9 | 1.00× |
| tebako-python | 5 | 0.494 | 0.490 | 0.868 | 0.167 | 0.569 | 0.479 | 194.9 | 0.09× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.501 | 0.498 | 0.504 | 0.004 | 0.501 | 0.483 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.128 | 0.123 | 0.401 | 0.122 | 0.182 | 0.122 | 17.3 | 1.00× |
| tebako-python | 5 | 0.760 | 0.698 | 0.902 | 0.083 | 0.771 | 0.727 | 134.7 | 0.17× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.720 | 0.712 | 0.729 | 0.012 | 0.720 | 0.695 | 134.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.064 | 0.061 | 0.073 | 0.005 | 0.065 | 0.061 | 14.0 | 1.00× |

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
Versions: tebako 2.8.61

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.064 | 0.061 | 0.065 | 0.002 | 0.063 | 0.094 | 37.0 | 1.00× |
| tebako-java | 5 | 0.617 | 0.613 | 0.647 | 0.014 | 0.624 | 0.531 | 177.9 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.620 | 0.617 | 0.624 | 0.005 | 0.620 | 0.531 | 177.9 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.074 | 0.074 | 0.077 | 0.001 | 0.075 | 0.078 | 37.4 | 1.00× |
| tebako-java | 5 | 0.630 | 0.620 | 0.639 | 0.008 | 0.631 | 0.547 | 177.9 | 0.12× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.656 | 0.648 | 0.664 | 0.012 | 0.656 | 0.570 | 177.9 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.091 | 0.090 | 0.101 | 0.004 | 0.093 | 0.141 | 39.5 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.154 | 0.154 | 0.159 | 0.002 | 0.156 | 0.250 | 42.7 | 1.00× |
| tebako-java | 5 | 0.684 | 0.666 | 0.748 | 0.033 | 0.691 | 0.531 | 177.9 | 0.23× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.689 | 0.681 | 0.696 | 0.011 | 0.689 | 0.508 | 177.9 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.268 | 0.257 | 0.297 | 0.016 | 0.274 | 0.625 | 77.0 | 1.00× |
| tebako-java | 5 | 0.818 | 0.793 | 0.838 | 0.017 | 0.815 | 0.547 | 177.9 | 0.33× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.827 | 0.822 | 0.831 | 0.007 | 0.827 | 0.531 | 177.9 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

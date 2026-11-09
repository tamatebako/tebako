# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.74

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.011 | 0.013 | 0.001 | 0.012 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.337 | 0.337 | 0.339 | 0.001 | 0.338 | 0.336 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.338 | 0.337 | 0.339 | 0.001 | 0.338 | 0.338 | 185.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.114 | 12.051 | 12.145 | 0.044 | 12.102 | 12.112 | 8.7 | 1.00× |
| tebako-python | 5 | 12.572 | 12.564 | 12.592 | 0.011 | 12.575 | 12.570 | 185.8 | 0.96× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.752 | 12.552 | 12.952 | 0.283 | 12.752 | 12.749 | 185.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.406 | 0.387 | 0.412 | 0.009 | 0.403 | 0.404 | 9.7 | 1.00× |
| tebako-python | 5 | 0.622 | 0.622 | 0.635 | 0.005 | 0.626 | 0.622 | 186.0 | 0.65× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.639 | 0.637 | 0.641 | 0.003 | 0.639 | 0.638 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.747 | 1.729 | 1.782 | 0.021 | 1.752 | 1.746 | 16.7 | 1.00× |
| tebako-python | 5 | 3.783 | 3.768 | 3.809 | 0.015 | 3.784 | 3.780 | 185.7 | 0.46× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.808 | 3.793 | 3.824 | 0.022 | 3.808 | 3.807 | 185.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.030 | 0.029 | 0.030 | 0.000 | 0.030 | 0.028 | 16.4 | 1.00× |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-gnu-arm64.sha256
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-21.0.12-linux-gnu-arm64.tar.gz.sha256

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.74

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.030 | 0.039 | 0.003 | 0.035 | 0.038 | 37.3 | 1.00× |
| tebako-java | 5 | 1.040 | 1.025 | 1.068 | 0.018 | 1.044 | 1.044 | 182.1 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.037 | 1.021 | 1.052 | 0.022 | 1.037 | 1.041 | 182.0 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.826 | 11.748 | 11.913 | 0.067 | 11.815 | 11.838 | 39.0 | 1.00× |
| tebako-java | 5 | 12.835 | 12.764 | 12.921 | 0.061 | 12.827 | 12.846 | 182.1 | 0.92× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.777 | 12.763 | 12.791 | 0.020 | 12.777 | 12.786 | 182.3 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.439 | 3.421 | 3.513 | 0.041 | 3.460 | 3.518 | 47.6 | 1.00× |
| tebako-java | 5 | 1.729 | 1.651 | 1.753 | 0.051 | 1.708 | 1.828 | 208.1 | 1.99× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.667 | 1.637 | 1.698 | 0.043 | 1.667 | 1.777 | 207.9 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.267 | 0.257 | 0.279 | 0.010 | 0.269 | 0.540 | 68.9 | 1.00× |
| tebako-java | 5 | 1.353 | 1.337 | 1.369 | 0.013 | 1.350 | 1.696 | 182.1 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.349 | 1.349 | 1.350 | 0.000 | 1.349 | 1.701 | 182.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.224 | 0.208 | 0.231 | 0.010 | 0.220 | 0.506 | 81.5 | 1.00× |
| tebako-java | 5 | 1.223 | 1.210 | 1.259 | 0.020 | 1.231 | 1.551 | 182.1 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.232 | 1.225 | 1.238 | 0.009 | 1.232 | 1.538 | 182.0 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.013 | 0.015 | 0.001 | 0.014 | 0.013 | 9.5 | 1.00× |
| tebako-python | 5 | 0.502 | 0.480 | 0.503 | 0.011 | 0.495 | 0.501 | 194.4 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.498 | 0.483 | 0.513 | 0.021 | 0.498 | 0.497 | 194.2 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 13.396 | 12.947 | 13.648 | 0.290 | 13.360 | 13.394 | 9.5 | 1.00× |
| tebako-python | 5 | 13.293 | 12.975 | 13.563 | 0.222 | 13.277 | 13.290 | 194.3 | 1.01× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 13.500 | 13.038 | 13.961 | 0.653 | 13.500 | 13.496 | 194.5 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.484 | 0.480 | 0.486 | 0.003 | 0.483 | 0.483 | 10.5 | 1.00× |
| tebako-python | 5 | 0.841 | 0.822 | 0.861 | 0.017 | 0.842 | 0.841 | 194.9 | 0.58× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.821 | 0.816 | 0.826 | 0.007 | 0.821 | 0.821 | 194.7 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.722 | 2.701 | 2.753 | 0.023 | 2.727 | 2.719 | 17.5 | 1.00× |
| tebako-python | 5 | 3.435 | 3.420 | 3.546 | 0.052 | 3.463 | 3.433 | 194.4 | 0.79× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.441 | 3.407 | 3.475 | 0.048 | 3.441 | 3.439 | 194.3 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.034 | 0.034 | 0.034 | 0.000 | 0.034 | 0.032 | 17.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.068 | 0.068 | 0.068 | 0.000 | 0.068 | 0.067 | 27.9 | 1.00× |
| tebako-ruby | 5 | 0.298 | 0.297 | 0.300 | 0.002 | 0.298 | 0.298 | 82.4 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.299 | 0.298 | 0.299 | 0.000 | 0.299 | 0.297 | 82.2 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.902 | 2.875 | 2.926 | 0.019 | 2.904 | 2.900 | 28.0 | 1.00× |
| tebako-ruby | 5 | 2.805 | 2.748 | 2.822 | 0.033 | 2.787 | 2.802 | 82.4 | 1.03× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.775 | 2.736 | 2.815 | 0.056 | 2.775 | 2.774 | 82.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.795 | 0.762 | 0.851 | 0.037 | 0.796 | 0.795 | 93.9 | 1.00× |
| tebako-ruby | 5 | 2.726 | 1.780 | 3.511 | 0.711 | 2.836 | 2.725 | 205.0 | 0.29× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 3.096 | 2.694 | 3.497 | 0.568 | 3.096 | 3.095 | 205.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.152 | 0.149 | 0.154 | 0.002 | 0.151 | 0.150 | 30.6 | 1.00× |
| tebako-ruby | 5 | 1.095 | 1.091 | 1.112 | 0.010 | 1.099 | 1.094 | 85.3 | 0.14× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.108 | 1.101 | 1.115 | 0.010 | 1.108 | 1.107 | 85.5 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.105 | 0.105 | 0.107 | 0.001 | 0.106 | 0.105 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.324 | 0.322 | 0.326 | 0.002 | 0.324 | 0.323 | 87.0 | 0.33× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.325 | 0.324 | 0.326 | 0.001 | 0.325 | 0.324 | 87.1 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
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

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-arm64.tar.gz.sha256
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-boot / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-arm64.tar.gz.sha256
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-arm64.tar.gz.sha256

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-linux-musl-x86_64.sha256
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-boot / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-linux-musl-x86_64.tar.gz.sha256
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-linux-musl-x86_64.tfs.sha256

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-arm64.tar.gz.sha256
- python-boot / on-system-python: unavailable — the paired arm 'tebako-python' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256)
- python-boot / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256
- python-fib / on-system-python: unavailable — the paired arm 'tebako-python' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256)
- python-fib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256
- python-stdlib / on-system-python: unavailable — the paired arm 'tebako-python' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256)
- python-stdlib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256
- python-ioread / on-system-python: unavailable — the paired arm 'tebako-python' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256)
- python-ioread / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256
- python-statloop / on-system-python: unavailable — the paired arm 'tebako-python' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256)
- python-statloop / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-arm64.tar.gz.sha256
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-arm64.sha256

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs)
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs)
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs)
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs)
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs)
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs: 504 fetching https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.17.4/tebako-runtime-0.17.4-ruby-3.3.13-macos-x86_64.tfs
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-boot / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-fib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-stdlib / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-ioread / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-statloop / tebako-python: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-python/releases/download/v0.3.3/tebako-runtime-0.3.3-python-3.14.7-macos-x86_64.tar.gz.sha256
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: cannot probe https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256: 504 fetching https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.8.50/tebako-runtime-2.8.50-temurin-21.0.12-macos-x86_64.tar.gz.sha256

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.72

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.061 | 0.059 | 0.063 | 0.001 | 0.061 | 0.078 | 331.1 | 1.00× |
| tebako-java | 5 | 0.755 | 0.745 | 0.789 | 0.017 | 0.759 | 0.750 | 463.0 | 0.08× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.754 | 0.752 | 0.755 | 0.002 | 0.754 | 0.758 | 462.8 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 12.003 | 11.976 | 12.008 | 0.013 | 11.997 | 11.953 | 331.4 | 1.00× |
| tebako-java | 5 | 12.683 | 12.646 | 12.731 | 0.035 | 12.685 | 12.609 | 463.2 | 0.95× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.646 | 12.627 | 12.666 | 0.027 | 12.646 | 12.625 | 463.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.506 | 1.447 | 2.224 | 0.321 | 1.667 | 1.672 | 337.4 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.007 | 0.986 | 1.028 | 0.019 | 1.008 | 1.469 | 348.5 | 1.00× |
| tebako-java | 5 | 1.113 | 1.069 | 1.118 | 0.020 | 1.103 | 1.500 | 475.8 | 0.91× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.088 | 1.072 | 1.105 | 0.023 | 1.088 | 1.586 | 478.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.250 | 0.237 | 0.260 | 0.009 | 0.248 | 0.531 | 337.7 | 1.00× |
| tebako-java | 5 | 0.954 | 0.951 | 1.013 | 0.026 | 0.969 | 1.281 | 470.4 | 0.26× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.964 | 0.955 | 0.973 | 0.013 | 0.964 | 1.297 | 469.5 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.167 | 0.160 | 0.170 | 0.004 | 0.167 | 0.156 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.241 | 0.234 | 0.256 | 0.008 | 0.243 | 0.250 | 68.3 | 0.69× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.235 | 0.235 | 0.236 | 0.000 | 0.235 | 0.234 | 68.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.614 | 2.571 | 2.626 | 0.024 | 2.606 | 2.609 | 51.6 | 1.00× |
| tebako-ruby | 5 | 2.689 | 2.678 | 2.728 | 0.019 | 2.697 | 2.688 | 68.4 | 0.97× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.756 | 2.695 | 2.817 | 0.086 | 2.756 | 2.750 | 68.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.006 | 2.957 | 3.177 | 0.084 | 3.033 | 3.000 | 117.7 | 1.00× |
| tebako-ruby | 5 | 2.599 | 2.581 | 2.705 | 0.051 | 2.624 | 2.594 | 134.6 | 1.16× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.598 | 2.574 | 2.623 | 0.034 | 2.598 | 2.594 | 134.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.608 | 3.575 | 3.621 | 0.018 | 3.606 | 3.578 | 53.8 | 1.00× |
| tebako-ruby | 5 | 1.327 | 1.322 | 1.345 | 0.009 | 1.330 | 1.328 | 68.9 | 2.72× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.326 | 1.313 | 1.338 | 0.018 | 1.326 | 1.320 | 69.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.224 | 0.222 | 0.231 | 0.003 | 0.225 | 0.219 | 53.0 | 1.00× |
| tebako-ruby | 5 | 0.259 | 0.258 | 0.266 | 0.003 | 0.261 | 0.250 | 69.2 | 0.87× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.268 | 0.267 | 0.270 | 0.002 | 0.268 | 0.266 | 69.1 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

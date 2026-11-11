# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.034 | 0.036 | 0.001 | 0.036 | 0.036 | 36.1 | 1.00× |
| tebako-java | 5 | 0.751 | 0.748 | 0.770 | 0.010 | 0.756 | 0.752 | 173.2 | 0.05× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.763 | 0.763 | 0.764 | 0.001 | 0.763 | 0.762 | 173.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.565 | 7.555 | 7.571 | 0.007 | 7.564 | 7.571 | 37.2 | 1.00× |
| tebako-java | 5 | 8.279 | 8.276 | 8.290 | 0.005 | 8.281 | 8.286 | 173.2 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.283 | 8.283 | 8.283 | 0.000 | 8.283 | 8.291 | 173.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.946 | 0.920 | 0.991 | 0.027 | 0.950 | 0.999 | 44.6 | 1.00× |
| tebako-java | 5 | 1.522 | 1.467 | 1.549 | 0.035 | 1.514 | 1.575 | 200.8 | 0.62× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.569 | 1.565 | 1.574 | 0.006 | 1.569 | 1.615 | 200.8 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.210 | 0.181 | 0.218 | 0.015 | 0.203 | 0.361 | 62.7 | 1.00× |
| tebako-java | 5 | 1.046 | 1.035 | 1.140 | 0.044 | 1.062 | 1.317 | 173.2 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.047 | 1.046 | 1.048 | 0.002 | 1.047 | 1.353 | 173.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.157 | 0.156 | 0.180 | 0.010 | 0.163 | 0.348 | 78.4 | 1.00× |
| tebako-java | 5 | 0.884 | 0.876 | 0.924 | 0.019 | 0.892 | 1.071 | 173.2 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.897 | 0.894 | 0.900 | 0.005 | 0.897 | 1.083 | 173.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.361 | 0.348 | 0.363 | 0.006 | 0.359 | 0.360 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.349 | 0.348 | 0.350 | 0.001 | 0.349 | 0.348 | 185.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.152 | 12.075 | 12.181 | 0.048 | 12.128 | 12.150 | 8.7 | 1.00× |
| tebako-python | 5 | 12.596 | 12.553 | 12.656 | 0.044 | 12.608 | 12.594 | 185.7 | 0.96× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.588 | 12.585 | 12.592 | 0.005 | 12.588 | 12.587 | 185.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.700 | 0.689 | 0.722 | 0.013 | 0.704 | 0.698 | 9.7 | 1.00× |
| tebako-python | 5 | 0.890 | 0.851 | 0.908 | 0.025 | 0.884 | 0.889 | 186.0 | 0.79× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.895 | 0.893 | 0.898 | 0.004 | 0.895 | 0.894 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.890 | 1.886 | 1.901 | 0.007 | 1.893 | 1.889 | 16.7 | 1.00× |
| tebako-python | 5 | 3.872 | 3.840 | 3.897 | 0.022 | 3.870 | 3.871 | 185.7 | 0.49× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.873 | 3.865 | 3.882 | 0.013 | 3.873 | 3.872 | 185.8 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.034 | 0.034 | 0.034 | 0.000 | 0.034 | 0.032 | 16.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.050 | 0.053 | 0.001 | 0.051 | 0.050 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.250 | 0.249 | 0.252 | 0.002 | 0.250 | 0.248 | 74.3 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.247 | 0.246 | 0.248 | 0.001 | 0.247 | 0.246 | 74.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.233 | 2.228 | 2.243 | 0.007 | 2.235 | 2.232 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.492 | 2.468 | 2.498 | 0.013 | 2.486 | 2.491 | 74.4 | 0.90× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.467 | 2.466 | 2.469 | 0.002 | 2.467 | 2.466 | 74.3 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.789 | 0.752 | 0.805 | 0.021 | 0.786 | 0.788 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.272 | 1.252 | 1.300 | 0.020 | 1.275 | 1.271 | 197.0 | 0.62× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.296 | 1.279 | 1.314 | 0.024 | 1.296 | 1.296 | 197.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.105 | 0.102 | 0.107 | 0.002 | 0.105 | 0.104 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.158 | 1.156 | 1.159 | 0.001 | 1.158 | 1.157 | 77.3 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.161 | 1.161 | 1.162 | 0.000 | 1.161 | 1.160 | 77.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.086 | 0.086 | 0.000 | 0.086 | 0.085 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.269 | 0.269 | 0.269 | 0.000 | 0.269 | 0.268 | 78.1 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.268 | 0.267 | 0.269 | 0.002 | 0.268 | 0.267 | 78.2 | — |

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

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.033 | 0.042 | 0.004 | 0.038 | 0.038 | 37.5 | 1.00× |
| tebako-java | 5 | 1.044 | 1.030 | 1.052 | 0.009 | 1.043 | 1.048 | 182.2 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.043 | 1.029 | 1.057 | 0.020 | 1.043 | 1.047 | 182.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.830 | 11.756 | 11.848 | 0.037 | 11.812 | 11.840 | 39.0 | 1.00× |
| tebako-java | 5 | 12.875 | 12.804 | 12.879 | 0.032 | 12.856 | 12.887 | 182.2 | 0.92× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.851 | 12.830 | 12.872 | 0.030 | 12.851 | 12.863 | 182.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.575 | 3.499 | 3.691 | 0.081 | 3.581 | 3.643 | 47.6 | 1.00× |
| tebako-java | 5 | 1.676 | 1.671 | 1.780 | 0.056 | 1.714 | 1.779 | 207.8 | 2.13× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.745 | 1.701 | 1.788 | 0.061 | 1.745 | 1.845 | 208.1 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.284 | 0.271 | 0.289 | 0.007 | 0.281 | 0.555 | 68.9 | 1.00× |
| tebako-java | 5 | 1.393 | 1.373 | 1.448 | 0.028 | 1.401 | 1.728 | 182.2 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.375 | 1.373 | 1.378 | 0.004 | 1.375 | 1.716 | 182.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.218 | 0.212 | 0.225 | 0.005 | 0.219 | 0.508 | 81.4 | 1.00× |
| tebako-java | 5 | 1.209 | 1.203 | 1.243 | 0.018 | 1.218 | 1.503 | 182.3 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.225 | 1.213 | 1.237 | 0.017 | 1.225 | 1.541 | 182.0 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.015 | 0.015 | 0.000 | 0.015 | 0.013 | 9.4 | 1.00× |
| tebako-python | 5 | 0.489 | 0.479 | 0.495 | 0.007 | 0.488 | 0.488 | 194.5 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.482 | 0.481 | 0.484 | 0.002 | 0.482 | 0.480 | 194.5 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 13.453 | 13.429 | 13.527 | 0.040 | 13.466 | 13.450 | 9.5 | 1.00× |
| tebako-python | 5 | 13.326 | 12.983 | 13.602 | 0.285 | 13.319 | 13.323 | 194.5 | 1.01× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 13.349 | 13.326 | 13.371 | 0.031 | 13.349 | 13.346 | 194.2 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.509 | 0.509 | 0.528 | 0.009 | 0.516 | 0.508 | 10.5 | 1.00× |
| tebako-python | 5 | 0.860 | 0.848 | 0.873 | 0.010 | 0.862 | 0.858 | 194.8 | 0.59× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.867 | 0.858 | 0.875 | 0.012 | 0.867 | 0.866 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.705 | 2.683 | 2.738 | 0.024 | 2.708 | 2.704 | 17.4 | 1.00× |
| tebako-python | 5 | 3.470 | 3.437 | 3.540 | 0.040 | 3.476 | 3.469 | 194.5 | 0.78× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.511 | 3.500 | 3.522 | 0.016 | 3.511 | 3.509 | 194.4 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.034 | 0.034 | 0.036 | 0.001 | 0.034 | 0.034 | 17.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.070 | 0.070 | 0.072 | 0.001 | 0.070 | 0.068 | 27.9 | 1.00× |
| tebako-ruby | 5 | 0.309 | 0.303 | 0.310 | 0.003 | 0.307 | 0.308 | 82.3 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.305 | 0.303 | 0.307 | 0.003 | 0.305 | 0.304 | 82.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.254 | 3.205 | 3.359 | 0.060 | 3.262 | 3.252 | 28.1 | 1.00× |
| tebako-ruby | 5 | 2.803 | 2.785 | 2.909 | 0.051 | 2.825 | 2.802 | 82.3 | 1.16× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.770 | 2.761 | 2.779 | 0.013 | 2.770 | 2.769 | 82.3 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.868 | 0.841 | 0.903 | 0.025 | 0.865 | 0.866 | 93.9 | 1.00× |
| tebako-ruby | 5 | 2.711 | 2.161 | 2.715 | 0.254 | 2.537 | 2.710 | 204.8 | 0.32× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.874 | 2.150 | 3.597 | 1.023 | 2.874 | 2.872 | 204.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.152 | 0.152 | 0.156 | 0.002 | 0.153 | 0.151 | 30.6 | 1.00× |
| tebako-ruby | 5 | 1.105 | 1.098 | 1.118 | 0.008 | 1.107 | 1.103 | 85.3 | 0.14× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.099 | 1.092 | 1.107 | 0.010 | 1.099 | 1.098 | 85.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.105 | 0.103 | 0.107 | 0.001 | 0.105 | 0.104 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.330 | 0.328 | 0.332 | 0.002 | 0.330 | 0.329 | 87.3 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.332 | 0.331 | 0.334 | 0.002 | 0.332 | 0.331 | 87.1 | — |

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

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.033 | 0.030 | 0.034 | 0.002 | 0.032 | 0.032 | 35.9 | 1.00× |
| tebako-java | 5 | 0.920 | 0.917 | 1.078 | 0.070 | 0.954 | 0.920 | 174.7 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.955 | 0.954 | 0.956 | 0.002 | 0.955 | 0.955 | 174.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.557 | 7.554 | 7.559 | 0.002 | 7.556 | 7.560 | 37.3 | 1.00× |
| tebako-java | 5 | 8.466 | 8.461 | 8.720 | 0.114 | 8.515 | 8.470 | 174.7 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.469 | 8.469 | 8.470 | 0.000 | 8.469 | 8.474 | 174.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.805 | 0.753 | 1.233 | 0.196 | 0.892 | 0.853 | 44.8 | 1.00× |
| tebako-java | 5 | 1.706 | 1.580 | 1.747 | 0.069 | 1.673 | 1.760 | 204.4 | 0.47× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.719 | 1.709 | 1.728 | 0.013 | 1.719 | 1.765 | 203.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.186 | 0.181 | 0.190 | 0.003 | 0.186 | 0.349 | 65.2 | 1.00× |
| tebako-java | 5 | 1.256 | 1.221 | 1.266 | 0.018 | 1.252 | 1.563 | 174.7 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.257 | 1.242 | 1.273 | 0.022 | 1.257 | 1.567 | 174.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.161 | 0.153 | 0.173 | 0.007 | 0.162 | 0.358 | 78.7 | 1.00× |
| tebako-java | 5 | 1.073 | 1.060 | 1.075 | 0.006 | 1.070 | 1.281 | 174.7 | 0.15× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.084 | 1.082 | 1.086 | 0.003 | 1.084 | 1.294 | 174.8 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.048 | 0.050 | 0.001 | 0.050 | 0.048 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.256 | 0.254 | 0.256 | 0.001 | 0.255 | 0.254 | 76.1 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.253 | 0.250 | 0.256 | 0.005 | 0.253 | 0.251 | 76.1 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.224 | 2.213 | 2.238 | 0.010 | 2.225 | 2.222 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.564 | 2.552 | 2.574 | 0.009 | 2.563 | 2.563 | 76.2 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.556 | 2.548 | 2.564 | 0.011 | 2.556 | 2.555 | 76.1 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.680 | 0.669 | 0.713 | 0.018 | 0.684 | 0.680 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.767 | 1.185 | 1.842 | 0.270 | 1.662 | 1.767 | 199.6 | 0.38× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.449 | 1.160 | 1.738 | 0.409 | 1.449 | 1.448 | 199.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.098 | 0.098 | 0.100 | 0.001 | 0.099 | 0.097 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.235 | 1.232 | 1.240 | 0.003 | 1.235 | 1.234 | 79.1 | 0.08× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.235 | 1.231 | 1.238 | 0.005 | 1.235 | 1.234 | 79.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.084 | 0.084 | 0.084 | 0.000 | 0.084 | 0.083 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.275 | 0.275 | 0.277 | 0.001 | 0.275 | 0.274 | 81.5 | 0.30× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.275 | 0.275 | 0.275 | 0.000 | 0.275 | 0.274 | 81.5 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-boot / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15983.9 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.038 | 0.035 | 0.043 | 0.003 | 0.038 | 0.040 | 37.0 | 1.00× |
| tebako-java | 5 | 1.399 | 1.395 | 1.433 | 0.016 | 1.405 | 1.401 | 175.6 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.404 | 1.394 | 1.414 | 0.014 | 1.404 | 1.407 | 175.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.821 | 11.773 | 11.874 | 0.038 | 11.824 | 11.831 | 39.0 | 1.00× |
| tebako-java | 5 | 13.227 | 13.211 | 13.238 | 0.011 | 13.225 | 13.238 | 175.6 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 13.199 | 13.198 | 13.199 | 0.000 | 13.199 | 13.210 | 175.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.460 | 3.393 | 3.558 | 0.070 | 3.467 | 3.530 | 47.6 | 1.00× |
| tebako-java | 5 | 2.039 | 2.033 | 2.054 | 0.008 | 2.041 | 2.145 | 209.5 | 1.70× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.053 | 2.046 | 2.059 | 0.009 | 2.053 | 2.151 | 209.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.266 | 0.263 | 0.286 | 0.010 | 0.272 | 0.533 | 68.9 | 1.00× |
| tebako-java | 5 | 1.766 | 1.754 | 1.796 | 0.016 | 1.769 | 2.163 | 175.6 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.778 | 1.771 | 1.785 | 0.010 | 1.778 | 2.177 | 175.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.221 | 0.213 | 0.225 | 0.004 | 0.220 | 0.513 | 81.5 | 1.00× |
| tebako-java | 5 | 1.599 | 1.592 | 1.605 | 0.006 | 1.598 | 1.925 | 175.7 | 0.14× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.586 | 1.575 | 1.598 | 0.016 | 1.586 | 1.930 | 175.6 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.068 | 0.068 | 0.070 | 0.001 | 0.069 | 0.067 | 27.8 | 1.00× |
| tebako-ruby | 5 | 0.343 | 0.340 | 0.345 | 0.002 | 0.343 | 0.341 | 79.5 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.342 | 0.342 | 0.343 | 0.001 | 0.342 | 0.342 | 80.1 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.909 | 2.879 | 3.048 | 0.067 | 2.933 | 2.907 | 28.0 | 1.00× |
| tebako-ruby | 5 | 2.818 | 2.770 | 3.056 | 0.114 | 2.856 | 2.817 | 79.6 | 1.03× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.858 | 2.846 | 2.869 | 0.016 | 2.858 | 2.856 | 79.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.787 | 0.760 | 0.840 | 0.032 | 0.792 | 0.787 | 93.9 | 1.00× |
| tebako-ruby | 5 | 2.090 | 1.571 | 2.121 | 0.236 | 1.991 | 2.089 | 202.0 | 0.38× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.823 | 1.550 | 2.096 | 0.386 | 1.823 | 1.821 | 201.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.151 | 0.150 | 0.156 | 0.003 | 0.152 | 0.149 | 30.6 | 1.00× |
| tebako-ruby | 5 | 1.459 | 1.439 | 1.505 | 0.025 | 1.468 | 1.457 | 82.6 | 0.10× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.458 | 1.441 | 1.475 | 0.024 | 1.458 | 1.456 | 82.5 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.103 | 0.103 | 0.104 | 0.000 | 0.103 | 0.103 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.366 | 0.364 | 0.368 | 0.002 | 0.365 | 0.364 | 84.6 | 0.28× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.367 | 0.366 | 0.368 | 0.002 | 0.367 | 0.365 | 84.6 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-boot / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-fib / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- python-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the musl runtime exe dynamically links libintl.so.8 — no boot on a bare musl system (tebako-runtime-python#76)
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.057 | 0.053 | 0.077 | 0.010 | 0.061 | 0.038 | 33.9 | 1.00× |
| tebako-java | 5 | 1.247 | 1.090 | 1.384 | 0.120 | 1.267 | 1.135 | 190.6 | 0.05× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.213 | 1.144 | 1.282 | 0.098 | 1.213 | 1.057 | 190.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.246 | 8.741 | 10.500 | 0.762 | 9.458 | 9.244 | 35.0 | 1.00× |
| tebako-java | 5 | 11.205 | 9.624 | 11.799 | 0.943 | 10.793 | 11.069 | 191.4 | 0.83× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.898 | 12.299 | 13.498 | 0.847 | 12.898 | 12.668 | 191.5 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.117 | 0.995 | 1.519 | 0.203 | 1.192 | 1.169 | 44.5 | 1.00× |
| tebako-java | 5 | 2.274 | 1.928 | 2.376 | 0.175 | 2.229 | 2.176 | 266.2 | 0.49× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.095 | 1.912 | 2.277 | 0.258 | 2.095 | 1.938 | 266.2 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.328 | 0.298 | 0.381 | 0.037 | 0.342 | 0.505 | 60.3 | 1.00× |
| tebako-java | 5 | 1.782 | 1.542 | 1.986 | 0.178 | 1.782 | 1.722 | 215.8 | 0.18× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.741 | 1.659 | 1.824 | 0.117 | 1.741 | 1.744 | 216.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.237 | 0.177 | 0.330 | 0.057 | 0.240 | 0.273 | 74.1 | 1.00× |
| tebako-java | 5 | 1.250 | 1.038 | 1.386 | 0.128 | 1.227 | 1.214 | 230.8 | 0.19× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.330 | 1.166 | 1.494 | 0.232 | 1.330 | 1.238 | 230.1 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.018 | 0.015 | 0.036 | 0.009 | 0.021 | 0.015 | 11.4 | 1.00× |
| tebako-python | 5 | 0.239 | 0.224 | 0.260 | 0.014 | 0.243 | 0.227 | 131.5 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.254 | 0.241 | 0.268 | 0.019 | 0.254 | 0.226 | 131.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 9.245 | 8.513 | 9.302 | 0.330 | 9.098 | 9.223 | 11.4 | 1.00× |
| tebako-python | 5 | 11.078 | 10.925 | 11.227 | 0.110 | 11.073 | 11.018 | 131.4 | 0.83× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.967 | 10.957 | 10.977 | 0.015 | 10.967 | 10.935 | 131.0 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.704 | 0.600 | 0.785 | 0.073 | 0.701 | 0.676 | 14.4 | 1.00× |
| tebako-python | 5 | 0.540 | 0.516 | 0.716 | 0.082 | 0.574 | 0.523 | 200.1 | 1.30× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.565 | 0.541 | 0.589 | 0.034 | 0.565 | 0.552 | 197.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.724 | 1.692 | 1.884 | 0.078 | 1.750 | 1.706 | 20.4 | 1.00× |
| tebako-python | 5 | 3.661 | 3.589 | 3.764 | 0.077 | 3.680 | 3.635 | 141.5 | 0.47× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.865 | 3.808 | 3.921 | 0.080 | 3.865 | 3.830 | 140.1 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.054 | 0.051 | 0.085 | 0.015 | 0.062 | 0.048 | 18.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.039 | 0.035 | 0.063 | 0.012 | 0.043 | 0.033 | 12.3 | 1.00× |
| tebako-ruby | 5 | 0.210 | 0.199 | 0.217 | 0.007 | 0.209 | 0.185 | 65.2 | 0.19× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.240 | 0.227 | 0.254 | 0.019 | 0.240 | 0.207 | 66.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.174 | 2.171 | 2.260 | 0.038 | 2.195 | 2.167 | 12.5 | 1.00× |
| tebako-ruby | 5 | 2.375 | 2.313 | 2.490 | 0.071 | 2.386 | 2.340 | 64.8 | 0.92× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.329 | 2.293 | 2.365 | 0.051 | 2.329 | 2.311 | 64.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.566 | 1.462 | 1.698 | 0.089 | 1.573 | 1.539 | 151.5 | 1.00× |
| tebako-ruby | 5 | 0.933 | 0.904 | 1.103 | 0.080 | 0.962 | 0.912 | 268.3 | 1.68× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.051 | 0.960 | 1.142 | 0.129 | 1.051 | 1.032 | 268.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.139 | 0.114 | 0.186 | 0.028 | 0.141 | 0.119 | 15.1 | 1.00× |
| tebako-ruby | 5 | 1.170 | 1.126 | 1.476 | 0.141 | 1.239 | 1.156 | 67.4 | 0.12× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.132 | 1.128 | 1.136 | 0.006 | 1.132 | 1.115 | 68.6 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.068 | 0.056 | 0.087 | 0.012 | 0.071 | 0.054 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.221 | 0.207 | 0.236 | 0.011 | 0.221 | 0.205 | 69.9 | 0.31× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.221 | 0.211 | 0.231 | 0.014 | 0.221 | 0.206 | 70.2 | — |

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

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.144 | 0.139 | 0.255 | 0.050 | 0.167 | 0.146 | 25.3 | 1.00× |
| tebako-java | 5 | 2.051 | 1.694 | 2.576 | 0.351 | 2.041 | 1.930 | 155.4 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.126 | 1.792 | 2.460 | 0.473 | 2.126 | 1.711 | 155.5 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 14.768 | 12.549 | 17.683 | 1.980 | 14.907 | 14.750 | 25.8 | 1.00× |
| tebako-java | 5 | 18.014 | 13.716 | 21.530 | 3.033 | 17.949 | 17.264 | 156.4 | 0.82× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 15.645 | 15.174 | 16.117 | 0.667 | 15.645 | 15.500 | 156.3 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.826 | 1.736 | 1.982 | 0.106 | 1.852 | 1.903 | 34.6 | 1.00× |
| tebako-java | 5 | 3.718 | 3.013 | 4.382 | 0.505 | 3.710 | 3.564 | 230.4 | 0.49× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 4.941 | 4.703 | 5.179 | 0.336 | 4.941 | 4.644 | 230.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.315 | 0.913 | 1.441 | 0.252 | 1.191 | 1.951 | 68.5 | 1.00× |
| tebako-java | 5 | 3.761 | 3.518 | 4.588 | 0.474 | 3.999 | 4.346 | 176.2 | 0.35× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 3.990 | 2.894 | 5.086 | 1.550 | 3.990 | 3.954 | 175.9 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.440 | 0.385 | 0.524 | 0.057 | 0.450 | 0.751 | 63.9 | 1.00× |
| tebako-java | 5 | 2.005 | 1.746 | 2.457 | 0.296 | 2.093 | 2.342 | 168.0 | 0.22× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.011 | 1.904 | 2.119 | 0.152 | 2.011 | 2.267 | 168.4 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.077 | 0.093 | 0.006 | 0.086 | 0.074 | 9.6 | 1.00× |
| tebako-ruby | 5 | 0.359 | 0.354 | 0.495 | 0.062 | 0.385 | 0.353 | 59.5 | 0.24× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.351 | 0.351 | 0.351 | 0.000 | 0.351 | 0.344 | 59.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 7.195 | 7.124 | 7.511 | 0.152 | 7.244 | 7.177 | 9.7 | 1.00× |
| tebako-ruby | 5 | 4.044 | 3.968 | 4.199 | 0.092 | 4.060 | 4.035 | 59.5 | 1.78× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.049 | 3.984 | 4.115 | 0.093 | 4.049 | 4.037 | 59.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.876 | 2.843 | 2.967 | 0.047 | 2.889 | 2.860 | 74.3 | 1.00× |
| tebako-ruby | 5 | 3.528 | 3.444 | 3.553 | 0.044 | 3.513 | 3.506 | 172.6 | 0.82× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 3.493 | 3.472 | 3.514 | 0.030 | 3.493 | 3.472 | 172.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.270 | 0.247 | 0.353 | 0.043 | 0.285 | 0.256 | 11.8 | 1.00× |
| tebako-ruby | 5 | 2.080 | 1.698 | 2.730 | 0.414 | 2.065 | 2.067 | 61.9 | 0.13× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.894 | 1.859 | 1.930 | 0.051 | 1.894 | 1.885 | 61.9 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.137 | 0.130 | 0.145 | 0.006 | 0.137 | 0.123 | 14.6 | 1.00× |
| tebako-ruby | 5 | 0.400 | 0.395 | 0.402 | 0.003 | 0.399 | 0.388 | 63.3 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.405 | 0.404 | 0.406 | 0.002 | 0.405 | 0.388 | 63.3 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-boot / tebako-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-fib / tebako-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- python-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the published macos-x86_64 runtime computes pure-Python at ~half the on-system speed (tebako-runtime-python#79, covenant run 42525964214) — the pair is gapped until the factory ships a corrected leg
- java-boot / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-fib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-stdlib / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-ioread / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-statloop / on-system-java [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.049 | 0.048 | 0.051 | 0.001 | 0.049 | 0.078 | 331.2 | 1.00× |
| tebako-java | 5 | 0.648 | 0.642 | 0.887 | 0.106 | 0.710 | 0.609 | 463.2 | 0.08× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.652 | 0.631 | 0.672 | 0.029 | 0.652 | 0.617 | 463.4 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.133 | 9.131 | 9.247 | 0.050 | 9.158 | 9.094 | 331.6 | 1.00× |
| tebako-java | 5 | 9.748 | 9.716 | 10.217 | 0.218 | 9.881 | 9.672 | 463.0 | 0.94× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 10.141 | 9.994 | 10.288 | 0.208 | 10.141 | 9.703 | 463.4 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.359 | 1.346 | 1.418 | 0.037 | 1.378 | 1.453 | 337.2 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.662 | 0.652 | 0.690 | 0.016 | 0.667 | 1.094 | 349.9 | 1.00× |
| tebako-java | 5 | 0.883 | 0.864 | 0.904 | 0.015 | 0.882 | 1.141 | 476.4 | 0.75× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.897 | 0.893 | 0.901 | 0.006 | 0.897 | 1.164 | 478.1 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.194 | 0.176 | 0.201 | 0.010 | 0.193 | 0.453 | 337.7 | 1.00× |
| tebako-java | 5 | 0.808 | 0.775 | 0.924 | 0.060 | 0.823 | 0.984 | 470.1 | 0.24× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.808 | 0.808 | 0.808 | 0.000 | 0.808 | 1.055 | 470.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.123 | 0.122 | 0.124 | 0.001 | 0.123 | 0.125 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.185 | 0.184 | 0.187 | 0.001 | 0.185 | 0.188 | 68.4 | 0.66× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.188 | 0.186 | 0.190 | 0.003 | 0.188 | 0.188 | 68.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.888 | 1.881 | 1.966 | 0.035 | 1.906 | 1.906 | 51.6 | 1.00× |
| tebako-ruby | 5 | 1.880 | 1.875 | 1.899 | 0.009 | 1.883 | 1.875 | 68.4 | 1.00× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.906 | 1.890 | 1.922 | 0.023 | 1.906 | 1.898 | 68.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.814 | 2.798 | 2.969 | 0.071 | 2.851 | 2.812 | 117.7 | 1.00× |
| tebako-ruby | 5 | 2.357 | 2.342 | 2.414 | 0.029 | 2.371 | 2.359 | 134.6 | 1.19× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.374 | 2.360 | 2.389 | 0.021 | 2.374 | 2.359 | 134.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.260 | 2.250 | 2.275 | 0.010 | 2.261 | 2.250 | 53.7 | 1.00× |
| tebako-ruby | 5 | 1.028 | 1.023 | 1.029 | 0.003 | 1.027 | 1.016 | 69.1 | 2.20× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.029 | 1.028 | 1.030 | 0.002 | 1.029 | 1.016 | 69.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.160 | 0.159 | 0.168 | 0.004 | 0.162 | 0.172 | 53.0 | 1.00× |
| tebako-ruby | 5 | 0.208 | 0.205 | 0.208 | 0.001 | 0.207 | 0.203 | 69.0 | 0.77× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.208 | 0.208 | 0.208 | 0.000 | 0.208 | 0.195 | 69.2 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.032 | 0.037 | 0.002 | 0.035 | 0.037 | 36.2 | 1.00× |
| tebako-java | 5 | 0.757 | 0.749 | 0.769 | 0.007 | 0.758 | 0.759 | 173.2 | 0.05× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.771 | 0.768 | 0.774 | 0.004 | 0.771 | 0.772 | 173.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.558 | 7.556 | 7.567 | 0.004 | 7.561 | 7.565 | 37.2 | 1.00× |
| tebako-java | 5 | 8.306 | 8.303 | 8.309 | 0.003 | 8.306 | 8.310 | 173.2 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.308 | 8.307 | 8.309 | 0.001 | 8.308 | 8.315 | 173.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.097 | 1.075 | 1.116 | 0.016 | 1.096 | 1.157 | 44.8 | 1.00× |
| tebako-java | 5 | 1.662 | 1.626 | 1.671 | 0.018 | 1.658 | 1.712 | 200.8 | 0.66× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.659 | 1.654 | 1.664 | 0.007 | 1.659 | 1.715 | 200.9 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.202 | 0.199 | 0.219 | 0.008 | 0.207 | 0.405 | 66.7 | 1.00× |
| tebako-java | 5 | 1.050 | 1.048 | 1.068 | 0.009 | 1.056 | 1.357 | 173.2 | 0.19× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.072 | 1.054 | 1.091 | 0.026 | 1.072 | 1.382 | 173.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.180 | 0.178 | 0.186 | 0.003 | 0.181 | 0.380 | 78.8 | 1.00× |
| tebako-java | 5 | 0.914 | 0.902 | 0.926 | 0.010 | 0.915 | 1.122 | 173.2 | 0.20× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.923 | 0.913 | 0.932 | 0.014 | 0.923 | 1.130 | 173.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.364 | 0.360 | 0.373 | 0.005 | 0.366 | 0.364 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.367 | 0.364 | 0.369 | 0.004 | 0.367 | 0.366 | 185.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.106 | 12.096 | 12.141 | 0.021 | 12.115 | 12.105 | 8.7 | 1.00× |
| tebako-python | 5 | 12.604 | 12.573 | 12.640 | 0.027 | 12.606 | 12.601 | 185.7 | 0.96× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.612 | 12.601 | 12.623 | 0.016 | 12.612 | 12.609 | 185.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.670 | 0.642 | 0.699 | 0.024 | 0.673 | 0.669 | 9.7 | 1.00× |
| tebako-python | 5 | 0.850 | 0.793 | 0.868 | 0.029 | 0.843 | 0.848 | 186.0 | 0.79× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.908 | 0.878 | 0.937 | 0.041 | 0.908 | 0.906 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.883 | 1.865 | 1.914 | 0.020 | 1.889 | 1.881 | 16.7 | 1.00× |
| tebako-python | 5 | 3.867 | 3.857 | 3.900 | 0.018 | 3.872 | 3.865 | 185.7 | 0.49× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.890 | 3.880 | 3.901 | 0.015 | 3.890 | 3.889 | 185.8 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.032 | 0.032 | 0.034 | 0.001 | 0.032 | 0.031 | 16.3 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.053 | 0.050 | 0.053 | 0.001 | 0.052 | 0.051 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.254 | 0.252 | 0.256 | 0.001 | 0.254 | 0.253 | 74.3 | 0.21× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.256 | 0.255 | 0.256 | 0.001 | 0.256 | 0.255 | 74.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.244 | 2.233 | 2.255 | 0.010 | 2.243 | 2.244 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.504 | 2.482 | 2.654 | 0.070 | 2.530 | 2.503 | 74.6 | 0.90× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.564 | 2.482 | 2.646 | 0.116 | 2.564 | 2.563 | 74.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.866 | 0.816 | 1.226 | 0.169 | 0.928 | 0.865 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.344 | 1.294 | 1.357 | 0.030 | 1.329 | 1.343 | 197.1 | 0.64× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.337 | 1.334 | 1.340 | 0.004 | 1.337 | 1.337 | 197.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.103 | 0.102 | 0.107 | 0.002 | 0.103 | 0.101 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.157 | 1.152 | 1.157 | 0.002 | 1.156 | 1.155 | 77.3 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.154 | 1.153 | 1.155 | 0.001 | 1.154 | 1.153 | 77.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.086 | 0.088 | 0.001 | 0.087 | 0.085 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.271 | 0.271 | 0.273 | 0.001 | 0.272 | 0.270 | 78.1 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.272 | 0.271 | 0.273 | 0.001 | 0.272 | 0.271 | 77.9 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.035 | 0.030 | 0.037 | 0.003 | 0.034 | 0.038 | 37.7 | 1.00× |
| tebako-java | 5 | 0.839 | 0.830 | 0.913 | 0.034 | 0.853 | 0.842 | 182.2 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.885 | 0.861 | 0.908 | 0.033 | 0.885 | 0.888 | 182.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.093 | 9.046 | 9.115 | 0.027 | 9.085 | 9.102 | 39.1 | 1.00× |
| tebako-java | 5 | 9.932 | 9.876 | 9.959 | 0.034 | 9.920 | 9.925 | 182.2 | 0.92× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 9.930 | 9.917 | 9.942 | 0.018 | 9.930 | 9.938 | 182.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.875 | 0.871 | 0.878 | 0.003 | 0.875 | 0.953 | 48.4 | 1.00× |
| tebako-java | 5 | 1.388 | 1.378 | 1.397 | 0.007 | 1.387 | 1.467 | 208.3 | 0.63× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.404 | 1.392 | 1.415 | 0.016 | 1.404 | 1.489 | 208.1 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.224 | 0.222 | 0.257 | 0.015 | 0.232 | 0.446 | 69.9 | 1.00× |
| tebako-java | 5 | 1.100 | 1.096 | 1.139 | 0.019 | 1.111 | 1.376 | 182.1 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.116 | 1.095 | 1.137 | 0.029 | 1.116 | 1.394 | 182.4 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.178 | 0.159 | 0.287 | 0.052 | 0.194 | 0.397 | 81.2 | 1.00× |
| tebako-java | 5 | 0.994 | 0.979 | 1.005 | 0.011 | 0.994 | 1.215 | 182.1 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.019 | 1.013 | 1.024 | 0.008 | 1.019 | 1.249 | 182.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 9.5 | 1.00× |
| tebako-python | 5 | 0.394 | 0.390 | 0.400 | 0.004 | 0.395 | 0.394 | 194.5 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.393 | 0.390 | 0.397 | 0.005 | 0.393 | 0.392 | 194.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 11.026 | 9.977 | 11.184 | 0.603 | 10.658 | 11.023 | 9.5 | 1.00× |
| tebako-python | 5 | 10.442 | 10.216 | 10.526 | 0.122 | 10.424 | 10.438 | 194.6 | 1.06× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.625 | 10.559 | 10.692 | 0.094 | 10.625 | 10.623 | 194.5 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.482 | 0.476 | 0.484 | 0.003 | 0.481 | 0.481 | 10.6 | 1.00× |
| tebako-python | 5 | 0.751 | 0.742 | 0.793 | 0.021 | 0.757 | 0.749 | 194.9 | 0.64× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.749 | 0.745 | 0.753 | 0.006 | 0.749 | 0.747 | 195.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.443 | 2.396 | 2.458 | 0.024 | 2.433 | 2.441 | 17.5 | 1.00× |
| tebako-python | 5 | 2.818 | 2.792 | 2.843 | 0.020 | 2.816 | 2.815 | 194.4 | 0.87× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.808 | 2.805 | 2.811 | 0.004 | 2.808 | 2.806 | 194.4 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.030 | 0.029 | 0.030 | 0.000 | 0.030 | 0.028 | 17.5 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.057 | 0.054 | 0.057 | 0.001 | 0.056 | 0.055 | 28.2 | 1.00× |
| tebako-ruby | 5 | 0.242 | 0.242 | 0.248 | 0.003 | 0.244 | 0.242 | 82.6 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.245 | 0.244 | 0.246 | 0.001 | 0.245 | 0.244 | 82.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.273 | 2.237 | 2.294 | 0.022 | 2.268 | 2.272 | 28.2 | 1.00× |
| tebako-ruby | 5 | 2.067 | 2.042 | 2.263 | 0.092 | 2.099 | 2.065 | 82.7 | 1.10× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.058 | 2.055 | 2.061 | 0.004 | 2.058 | 2.057 | 80.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.734 | 0.702 | 0.776 | 0.029 | 0.734 | 0.733 | 94.0 | 1.00× |
| tebako-ruby | 5 | 1.627 | 1.360 | 2.512 | 0.486 | 1.771 | 1.626 | 204.9 | 0.45× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.359 | 1.371 | 3.347 | 1.397 | 2.359 | 2.357 | 205.1 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.130 | 0.130 | 0.132 | 0.001 | 0.130 | 0.129 | 30.7 | 1.00× |
| tebako-ruby | 5 | 0.888 | 0.880 | 0.899 | 0.008 | 0.888 | 0.886 | 85.3 | 0.15× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.895 | 0.886 | 0.904 | 0.013 | 0.895 | 0.894 | 85.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.084 | 0.082 | 0.087 | 0.002 | 0.084 | 0.083 | 34.4 | 1.00× |
| tebako-ruby | 5 | 0.261 | 0.261 | 0.264 | 0.002 | 0.262 | 0.261 | 87.3 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.262 | 0.261 | 0.263 | 0.002 | 0.262 | 0.261 | 87.3 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.029 | 0.022 | 0.034 | 0.005 | 0.029 | 0.027 | 35.6 | 1.00× |
| tebako-java | 5 | 0.948 | 0.938 | 0.963 | 0.010 | 0.948 | 0.949 | 174.7 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.985 | 0.974 | 0.995 | 0.015 | 0.985 | 0.987 | 174.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.557 | 7.550 | 7.559 | 0.004 | 7.556 | 7.562 | 37.2 | 1.00× |
| tebako-java | 5 | 8.462 | 8.451 | 8.475 | 0.008 | 8.463 | 8.467 | 174.7 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.475 | 8.463 | 8.487 | 0.017 | 8.475 | 8.481 | 174.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.694 | 0.685 | 0.698 | 0.005 | 0.694 | 0.750 | 44.9 | 1.00× |
| tebako-java | 5 | 1.557 | 1.514 | 1.605 | 0.034 | 1.555 | 1.619 | 204.4 | 0.45× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.663 | 1.640 | 1.685 | 0.032 | 1.663 | 1.724 | 204.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.214 | 0.189 | 0.231 | 0.016 | 0.210 | 0.409 | 67.7 | 1.00× |
| tebako-java | 5 | 1.231 | 1.223 | 1.296 | 0.030 | 1.245 | 1.545 | 174.7 | 0.17× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.244 | 1.228 | 1.260 | 0.023 | 1.244 | 1.552 | 174.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.157 | 0.150 | 0.167 | 0.007 | 0.158 | 0.353 | 78.6 | 1.00× |
| tebako-java | 5 | 1.089 | 1.067 | 1.112 | 0.019 | 1.086 | 1.304 | 174.7 | 0.14× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.105 | 1.092 | 1.118 | 0.018 | 1.105 | 1.321 | 174.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.052 | 0.050 | 0.053 | 0.001 | 0.052 | 0.050 | 12.6 | 1.00× |
| tebako-ruby | 5 | 0.256 | 0.254 | 0.258 | 0.002 | 0.256 | 0.254 | 76.2 | 0.21× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.256 | 0.254 | 0.259 | 0.003 | 0.256 | 0.256 | 76.2 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.221 | 2.217 | 2.245 | 0.012 | 2.227 | 2.220 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.560 | 2.553 | 2.660 | 0.045 | 2.580 | 2.559 | 76.2 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.575 | 2.567 | 2.582 | 0.011 | 2.575 | 2.574 | 76.2 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.719 | 0.705 | 0.739 | 0.013 | 0.722 | 0.719 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.834 | 1.743 | 1.884 | 0.052 | 1.823 | 1.833 | 199.6 | 0.39× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.409 | 1.146 | 1.671 | 0.371 | 1.409 | 1.407 | 199.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.098 | 0.096 | 0.098 | 0.001 | 0.097 | 0.097 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.233 | 1.229 | 1.242 | 0.005 | 1.234 | 1.231 | 79.1 | 0.08× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.233 | 1.231 | 1.235 | 0.003 | 1.233 | 1.232 | 79.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.086 | 0.086 | 0.000 | 0.086 | 0.084 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.276 | 0.275 | 0.277 | 0.001 | 0.276 | 0.275 | 81.5 | 0.31× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.277 | 0.275 | 0.279 | 0.003 | 0.277 | 0.276 | 81.4 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.040 | 0.038 | 0.042 | 0.002 | 0.040 | 0.042 | 37.4 | 1.00× |
| tebako-java | 5 | 1.502 | 1.476 | 1.507 | 0.013 | 1.496 | 1.500 | 175.6 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.482 | 1.481 | 1.482 | 0.001 | 1.482 | 1.483 | 175.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.735 | 11.708 | 11.797 | 0.035 | 11.738 | 11.746 | 39.0 | 1.00× |
| tebako-java | 5 | 13.236 | 13.207 | 13.258 | 0.020 | 13.234 | 13.251 | 175.6 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 13.229 | 13.194 | 13.263 | 0.049 | 13.229 | 13.236 | 175.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.015 | 1.006 | 1.035 | 0.012 | 1.018 | 1.116 | 48.6 | 1.00× |
| tebako-java | 5 | 2.163 | 2.149 | 2.189 | 0.014 | 2.165 | 2.265 | 209.4 | 0.47× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.199 | 2.196 | 2.201 | 0.003 | 2.199 | 2.315 | 209.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.296 | 0.294 | 0.325 | 0.013 | 0.301 | 0.596 | 70.8 | 1.00× |
| tebako-java | 5 | 1.881 | 1.860 | 1.900 | 0.016 | 1.879 | 2.296 | 175.6 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.928 | 1.916 | 1.940 | 0.017 | 1.928 | 2.425 | 175.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.227 | 0.210 | 0.238 | 0.011 | 0.226 | 0.518 | 81.0 | 1.00× |
| tebako-java | 5 | 1.704 | 1.686 | 1.718 | 0.013 | 1.700 | 2.037 | 175.6 | 0.13× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.705 | 1.701 | 1.709 | 0.006 | 1.705 | 2.045 | 175.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.074 | 0.074 | 0.075 | 0.001 | 0.074 | 0.073 | 27.9 | 1.00× |
| tebako-ruby | 5 | 0.352 | 0.350 | 0.354 | 0.002 | 0.352 | 0.350 | 79.4 | 0.21× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.354 | 0.352 | 0.355 | 0.002 | 0.354 | 0.353 | 79.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.953 | 2.901 | 3.106 | 0.097 | 2.998 | 2.953 | 28.0 | 1.00× |
| tebako-ruby | 5 | 2.746 | 2.710 | 2.811 | 0.040 | 2.748 | 2.745 | 79.6 | 1.08× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.706 | 2.701 | 2.710 | 0.006 | 2.706 | 2.705 | 79.3 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.922 | 0.897 | 0.945 | 0.020 | 0.919 | 0.921 | 93.9 | 1.00× |
| tebako-ruby | 5 | 1.816 | 1.766 | 2.517 | 0.387 | 2.068 | 1.814 | 201.8 | 0.51× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.818 | 1.785 | 1.850 | 0.046 | 1.818 | 1.817 | 202.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.178 | 0.176 | 0.179 | 0.001 | 0.178 | 0.177 | 30.6 | 1.00× |
| tebako-ruby | 5 | 1.445 | 1.439 | 1.464 | 0.010 | 1.447 | 1.444 | 82.7 | 0.12× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.453 | 1.451 | 1.454 | 0.002 | 1.453 | 1.450 | 82.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.114 | 0.112 | 0.116 | 0.002 | 0.114 | 0.111 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.377 | 0.375 | 0.390 | 0.006 | 0.380 | 0.375 | 84.5 | 0.30× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.380 | 0.379 | 0.380 | 0.000 | 0.380 | 0.378 | 83.4 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.054 | 0.042 | 0.092 | 0.021 | 0.060 | 0.037 | 34.0 | 1.00× |
| tebako-java | 5 | 1.202 | 1.087 | 1.237 | 0.059 | 1.188 | 1.045 | 190.6 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.190 | 1.042 | 1.337 | 0.209 | 1.190 | 1.054 | 190.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.951 | 9.608 | 10.381 | 0.289 | 9.980 | 9.870 | 34.9 | 1.00× |
| tebako-java | 5 | 11.546 | 10.925 | 12.110 | 0.422 | 11.530 | 11.267 | 191.6 | 0.86× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 11.644 | 11.394 | 11.893 | 0.353 | 11.644 | 11.430 | 191.5 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.986 | 0.941 | 1.116 | 0.070 | 1.007 | 1.028 | 43.8 | 1.00× |
| tebako-java | 5 | 2.014 | 1.574 | 2.200 | 0.239 | 1.962 | 1.891 | 266.6 | 0.49× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.842 | 1.752 | 1.932 | 0.127 | 1.842 | 1.765 | 266.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.318 | 0.238 | 0.403 | 0.060 | 0.315 | 0.409 | 59.0 | 1.00× |
| tebako-java | 5 | 1.576 | 1.555 | 1.611 | 0.022 | 1.583 | 1.615 | 216.3 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.551 | 1.521 | 1.581 | 0.042 | 1.551 | 1.596 | 216.0 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.226 | 0.192 | 0.330 | 0.058 | 0.252 | 0.337 | 73.8 | 1.00× |
| tebako-java | 5 | 1.426 | 1.396 | 1.564 | 0.067 | 1.447 | 1.417 | 230.1 | 0.16× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.744 | 1.741 | 1.747 | 0.004 | 1.744 | 1.732 | 230.0 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.038 | 0.018 | 0.043 | 0.010 | 0.035 | 0.018 | 11.3 | 1.00× |
| tebako-python | 5 | 0.267 | 0.228 | 0.306 | 0.028 | 0.268 | 0.247 | 130.2 | 0.14× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.264 | 0.256 | 0.271 | 0.010 | 0.264 | 0.239 | 130.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 9.647 | 9.140 | 10.632 | 0.681 | 9.854 | 9.613 | 11.5 | 1.00× |
| tebako-python | 5 | 11.504 | 10.846 | 12.367 | 0.585 | 11.611 | 11.474 | 131.4 | 0.84× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.802 | 10.657 | 10.946 | 0.204 | 10.802 | 10.777 | 132.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.652 | 0.643 | 0.749 | 0.045 | 0.670 | 0.641 | 13.4 | 1.00× |
| tebako-python | 5 | 0.513 | 0.470 | 0.574 | 0.040 | 0.511 | 0.477 | 197.9 | 1.27× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.486 | 0.477 | 0.494 | 0.012 | 0.486 | 0.457 | 199.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.905 | 1.753 | 2.233 | 0.181 | 1.942 | 1.902 | 20.4 | 1.00× |
| tebako-python | 5 | 3.848 | 3.734 | 4.135 | 0.149 | 3.890 | 3.826 | 141.6 | 0.50× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.876 | 3.714 | 4.037 | 0.228 | 3.876 | 3.841 | 141.2 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.053 | 0.052 | 0.054 | 0.001 | 0.053 | 0.043 | 18.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.060 | 0.053 | 0.063 | 0.004 | 0.058 | 0.049 | 12.7 | 1.00× |
| tebako-ruby | 5 | 0.249 | 0.240 | 0.262 | 0.009 | 0.249 | 0.230 | 64.8 | 0.24× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.250 | 0.235 | 0.265 | 0.021 | 0.250 | 0.232 | 65.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.417 | 2.331 | 2.442 | 0.050 | 2.395 | 2.347 | 12.5 | 1.00× |
| tebako-ruby | 5 | 2.629 | 2.413 | 2.775 | 0.137 | 2.618 | 2.605 | 64.7 | 0.92× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.680 | 2.668 | 2.692 | 0.017 | 2.680 | 2.657 | 64.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.459 | 2.095 | 2.881 | 0.303 | 2.451 | 2.416 | 152.7 | 1.00× |
| tebako-ruby | 5 | 1.670 | 0.926 | 2.216 | 0.469 | 1.629 | 1.655 | 268.6 | 1.47× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.301 | 1.152 | 1.450 | 0.211 | 1.301 | 1.291 | 279.2 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.165 | 0.141 | 0.188 | 0.017 | 0.164 | 0.143 | 15.3 | 1.00× |
| tebako-ruby | 5 | 1.358 | 1.253 | 1.388 | 0.054 | 1.345 | 1.336 | 67.4 | 0.12× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.233 | 1.215 | 1.250 | 0.025 | 1.233 | 1.211 | 67.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.087 | 0.070 | 0.091 | 0.009 | 0.085 | 0.078 | 19.6 | 1.00× |
| tebako-ruby | 5 | 0.263 | 0.245 | 0.286 | 0.017 | 0.265 | 0.257 | 70.5 | 0.33× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.285 | 0.283 | 0.288 | 0.003 | 0.285 | 0.270 | 69.8 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.166 | 0.140 | 0.305 | 0.067 | 0.193 | 0.177 | 25.3 | 1.00× |
| tebako-java | 5 | 2.688 | 1.867 | 3.424 | 0.583 | 2.583 | 2.156 | 155.9 | 0.06× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.467 | 2.042 | 2.891 | 0.600 | 2.467 | 2.156 | 155.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 18.716 | 15.859 | 21.902 | 2.527 | 18.758 | 18.734 | 25.8 | 1.00× |
| tebako-java | 5 | 21.762 | 18.317 | 23.415 | 2.521 | 21.019 | 21.180 | 156.4 | 0.86× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 20.021 | 18.340 | 21.701 | 2.377 | 20.021 | 19.912 | 156.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 2.120 | 1.988 | 2.563 | 0.256 | 2.220 | 2.226 | 34.6 | 1.00× |
| tebako-java | 5 | 4.660 | 3.472 | 5.254 | 0.684 | 4.582 | 4.415 | 230.3 | 0.45× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 4.367 | 4.148 | 4.587 | 0.310 | 4.367 | 4.387 | 230.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.779 | 0.576 | 1.258 | 0.276 | 0.851 | 1.478 | 69.4 | 1.00× |
| tebako-java | 5 | 3.438 | 2.679 | 4.486 | 0.658 | 3.478 | 3.475 | 176.3 | 0.23× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.881 | 2.679 | 3.084 | 0.286 | 2.881 | 3.206 | 176.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.529 | 0.524 | 0.912 | 0.166 | 0.626 | 0.941 | 63.4 | 1.00× |
| tebako-java | 5 | 2.550 | 2.120 | 2.800 | 0.245 | 2.506 | 2.826 | 167.2 | 0.21× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.537 | 2.365 | 2.708 | 0.242 | 2.537 | 2.639 | 167.4 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.153 | 0.136 | 0.172 | 0.014 | 0.152 | 0.134 | 9.7 | 1.00× |
| tebako-ruby | 5 | 0.596 | 0.573 | 0.632 | 0.022 | 0.600 | 0.579 | 59.6 | 0.26× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.691 | 0.616 | 0.766 | 0.106 | 0.691 | 0.671 | 59.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 10.148 | 9.895 | 10.886 | 0.432 | 10.366 | 10.122 | 9.7 | 1.00× |
| tebako-ruby | 5 | 6.285 | 5.747 | 6.788 | 0.382 | 6.254 | 6.264 | 59.6 | 1.61× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 5.952 | 5.580 | 6.325 | 0.527 | 5.952 | 5.934 | 59.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 4.092 | 3.338 | 4.140 | 0.407 | 3.825 | 4.081 | 74.6 | 1.00× |
| tebako-ruby | 5 | 4.844 | 4.074 | 5.482 | 0.569 | 4.705 | 4.828 | 172.7 | 0.84× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 5.194 | 5.004 | 5.385 | 0.270 | 5.194 | 5.170 | 172.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.330 | 0.316 | 0.450 | 0.056 | 0.353 | 0.326 | 11.8 | 1.00× |
| tebako-ruby | 5 | 2.409 | 2.141 | 2.720 | 0.210 | 2.436 | 2.391 | 62.0 | 0.14× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.467 | 2.267 | 2.666 | 0.282 | 2.467 | 2.443 | 62.0 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.167 | 0.159 | 0.169 | 0.004 | 0.166 | 0.150 | 14.8 | 1.00× |
| tebako-ruby | 5 | 0.491 | 0.476 | 0.512 | 0.013 | 0.493 | 0.479 | 63.4 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.477 | 0.477 | 0.478 | 0.001 | 0.477 | 0.464 | 63.3 | — |

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
Versions: tebako 2.8.75

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.066 | 0.064 | 0.068 | 0.002 | 0.066 | 0.078 | 331.1 | 1.00× |
| tebako-java | 5 | 0.763 | 0.753 | 0.804 | 0.021 | 0.770 | 0.781 | 462.9 | 0.09× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.762 | 0.759 | 0.765 | 0.004 | 0.762 | 0.742 | 463.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 12.959 | 11.943 | 14.082 | 0.831 | 12.842 | 12.953 | 331.2 | 1.00× |
| tebako-java | 5 | 12.909 | 12.652 | 14.294 | 0.705 | 13.230 | 12.812 | 463.4 | 1.00× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.655 | 12.637 | 12.674 | 0.027 | 12.655 | 12.609 | 463.5 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.867 | 1.821 | 2.103 | 0.115 | 1.908 | 2.016 | 337.3 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.054 | 1.047 | 1.073 | 0.012 | 1.058 | 1.594 | 349.8 | 1.00× |
| tebako-java | 5 | 1.121 | 1.089 | 1.133 | 0.019 | 1.116 | 1.516 | 476.1 | 0.94× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.116 | 1.098 | 1.134 | 0.025 | 1.116 | 1.562 | 476.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.261 | 0.243 | 0.274 | 0.012 | 0.259 | 0.578 | 337.9 | 1.00× |
| tebako-java | 5 | 0.958 | 0.926 | 0.975 | 0.018 | 0.954 | 1.266 | 470.1 | 0.27× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.957 | 0.942 | 0.971 | 0.021 | 0.957 | 1.227 | 470.4 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.179 | 0.179 | 0.186 | 0.003 | 0.182 | 0.172 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.243 | 0.241 | 0.256 | 0.006 | 0.246 | 0.234 | 68.3 | 0.74× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.244 | 0.240 | 0.248 | 0.006 | 0.244 | 0.242 | 68.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.601 | 2.588 | 2.631 | 0.016 | 2.604 | 2.594 | 51.6 | 1.00× |
| tebako-ruby | 5 | 2.703 | 2.682 | 2.779 | 0.038 | 2.715 | 2.688 | 68.4 | 0.96× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.728 | 2.697 | 2.758 | 0.043 | 2.728 | 2.711 | 68.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.202 | 3.111 | 3.268 | 0.059 | 3.203 | 3.172 | 117.7 | 1.00× |
| tebako-ruby | 5 | 2.507 | 2.477 | 2.542 | 0.024 | 2.510 | 2.500 | 134.4 | 1.28× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.498 | 2.486 | 2.509 | 0.016 | 2.498 | 2.461 | 134.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.693 | 3.661 | 3.747 | 0.033 | 3.699 | 3.656 | 53.9 | 1.00× |
| tebako-ruby | 5 | 1.341 | 1.335 | 1.344 | 0.003 | 1.340 | 1.344 | 68.9 | 2.75× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.360 | 1.345 | 1.375 | 0.021 | 1.360 | 1.367 | 69.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.231 | 0.229 | 0.234 | 0.002 | 0.231 | 0.234 | 53.0 | 1.00× |
| tebako-ruby | 5 | 0.264 | 0.262 | 0.270 | 0.004 | 0.266 | 0.266 | 69.1 | 0.88× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.268 | 0.268 | 0.268 | 0.000 | 0.268 | 0.258 | 69.3 | — |

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

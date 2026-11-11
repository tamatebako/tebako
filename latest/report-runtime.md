# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.6 GiB
Versions: tebako 2.8.76

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.034 | 0.030 | 0.074 | 0.019 | 0.041 | 0.033 | 36.1 | 1.00× |
| tebako-java | 5 | 0.746 | 0.733 | 0.755 | 0.008 | 0.745 | 0.746 | 173.2 | 0.05× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.746 | 0.736 | 0.757 | 0.015 | 0.746 | 0.747 | 173.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.559 | 7.554 | 7.584 | 0.012 | 7.565 | 7.562 | 37.2 | 1.00× |
| tebako-java | 5 | 8.272 | 8.263 | 8.283 | 0.007 | 8.273 | 8.279 | 173.2 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.278 | 8.271 | 8.285 | 0.010 | 8.278 | 8.285 | 173.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.799 | 0.734 | 0.844 | 0.041 | 0.789 | 0.843 | 44.9 | 1.00× |
| tebako-java | 5 | 1.425 | 1.363 | 1.480 | 0.054 | 1.424 | 1.484 | 201.0 | 0.56× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.411 | 1.393 | 1.428 | 0.025 | 1.411 | 1.459 | 200.9 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.184 | 0.181 | 0.188 | 0.003 | 0.185 | 0.342 | 64.5 | 1.00× |
| tebako-java | 5 | 1.002 | 0.995 | 1.016 | 0.009 | 1.005 | 1.287 | 173.2 | 0.18× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.039 | 1.037 | 1.040 | 0.002 | 1.039 | 1.323 | 173.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.164 | 0.150 | 0.176 | 0.009 | 0.164 | 0.365 | 78.4 | 1.00× |
| tebako-java | 5 | 0.879 | 0.878 | 0.900 | 0.009 | 0.883 | 1.072 | 173.2 | 0.19× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.880 | 0.870 | 0.891 | 0.015 | 0.880 | 1.062 | 173.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.352 | 0.344 | 0.365 | 0.009 | 0.354 | 0.350 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.351 | 0.346 | 0.357 | 0.008 | 0.351 | 0.351 | 185.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.061 | 12.051 | 12.167 | 0.049 | 12.087 | 12.059 | 8.7 | 1.00× |
| tebako-python | 5 | 12.607 | 12.554 | 12.616 | 0.028 | 12.592 | 12.606 | 185.7 | 0.96× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.579 | 12.571 | 12.586 | 0.011 | 12.579 | 12.577 | 185.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.454 | 0.442 | 0.508 | 0.032 | 0.470 | 0.452 | 9.7 | 1.00× |
| tebako-python | 5 | 0.684 | 0.684 | 0.733 | 0.022 | 0.698 | 0.683 | 186.0 | 0.66× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.685 | 0.673 | 0.697 | 0.017 | 0.685 | 0.684 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.841 | 1.812 | 1.860 | 0.019 | 1.842 | 1.840 | 16.7 | 1.00× |
| tebako-python | 5 | 3.885 | 3.820 | 3.894 | 0.030 | 3.871 | 3.883 | 185.7 | 0.47× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.799 | 3.778 | 3.819 | 0.029 | 3.799 | 3.797 | 185.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.032 | 0.030 | 0.032 | 0.001 | 0.031 | 0.030 | 16.3 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.048 | 0.048 | 0.048 | 0.000 | 0.048 | 0.048 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.248 | 0.246 | 0.248 | 0.001 | 0.247 | 0.246 | 74.3 | 0.19× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.247 | 0.244 | 0.250 | 0.004 | 0.247 | 0.246 | 74.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.232 | 2.222 | 2.250 | 0.011 | 2.234 | 2.231 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.494 | 2.485 | 2.507 | 0.008 | 2.495 | 2.493 | 74.4 | 0.89× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.488 | 2.479 | 2.496 | 0.012 | 2.488 | 2.486 | 74.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.681 | 0.615 | 0.770 | 0.060 | 0.677 | 0.680 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.199 | 1.133 | 1.204 | 0.030 | 1.184 | 1.197 | 197.0 | 0.57× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.183 | 1.169 | 1.196 | 0.019 | 1.183 | 1.182 | 196.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.103 | 0.100 | 0.105 | 0.002 | 0.103 | 0.101 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.160 | 1.154 | 1.161 | 0.003 | 1.158 | 1.159 | 77.3 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.154 | 1.154 | 1.154 | 0.000 | 1.154 | 1.153 | 77.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.084 | 0.082 | 0.088 | 0.002 | 0.085 | 0.083 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.265 | 0.265 | 0.273 | 0.004 | 0.267 | 0.264 | 78.1 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.270 | 0.269 | 0.271 | 0.001 | 0.270 | 0.269 | 78.1 | — |

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
| on-system-java | 5 | 0.032 | 0.031 | 0.032 | 0.001 | 0.032 | 0.035 | 37.5 | 1.00× |
| tebako-java | 5 | 0.845 | 0.831 | 0.872 | 0.016 | 0.846 | 0.841 | 182.2 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.868 | 0.843 | 0.892 | 0.035 | 0.868 | 0.870 | 182.3 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.075 | 9.060 | 9.212 | 0.067 | 9.116 | 9.080 | 39.1 | 1.00× |
| tebako-java | 5 | 9.916 | 9.889 | 9.949 | 0.023 | 9.915 | 9.923 | 182.3 | 0.92× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 9.920 | 9.908 | 9.931 | 0.016 | 9.920 | 9.929 | 182.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.852 | 0.852 | 0.859 | 0.003 | 0.854 | 0.919 | 48.3 | 1.00× |
| tebako-java | 5 | 1.400 | 1.378 | 1.451 | 0.029 | 1.406 | 1.489 | 208.3 | 0.61× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.411 | 1.397 | 1.424 | 0.019 | 1.411 | 1.488 | 208.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.224 | 0.220 | 0.232 | 0.005 | 0.225 | 0.447 | 69.4 | 1.00× |
| tebako-java | 5 | 1.110 | 1.094 | 1.135 | 0.018 | 1.113 | 1.397 | 182.2 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.141 | 1.128 | 1.154 | 0.018 | 1.141 | 1.459 | 182.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.173 | 0.162 | 0.188 | 0.011 | 0.174 | 0.385 | 81.7 | 1.00× |
| tebako-java | 5 | 0.995 | 0.971 | 1.038 | 0.026 | 0.997 | 1.221 | 182.2 | 0.17× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.983 | 0.979 | 0.986 | 0.005 | 0.983 | 1.215 | 182.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 9.5 | 1.00× |
| tebako-python | 5 | 0.400 | 0.393 | 0.403 | 0.004 | 0.399 | 0.400 | 194.6 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.401 | 0.399 | 0.403 | 0.003 | 0.401 | 0.400 | 194.5 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 10.595 | 9.994 | 11.811 | 0.745 | 10.723 | 10.593 | 9.6 | 1.00× |
| tebako-python | 5 | 10.684 | 10.150 | 10.752 | 0.282 | 10.519 | 10.679 | 194.5 | 0.99× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.596 | 10.301 | 10.891 | 0.417 | 10.596 | 10.593 | 194.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.436 | 0.434 | 0.438 | 0.002 | 0.436 | 0.436 | 10.5 | 1.00× |
| tebako-python | 5 | 0.750 | 0.743 | 0.756 | 0.005 | 0.749 | 0.749 | 195.0 | 0.58× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.751 | 0.749 | 0.753 | 0.003 | 0.751 | 0.751 | 195.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.458 | 2.421 | 2.477 | 0.020 | 2.455 | 2.457 | 17.5 | 1.00× |
| tebako-python | 5 | 2.813 | 2.801 | 2.828 | 0.010 | 2.812 | 2.811 | 194.7 | 0.87× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.833 | 2.829 | 2.836 | 0.005 | 2.833 | 2.831 | 194.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.030 | 0.030 | 0.032 | 0.001 | 0.030 | 0.029 | 17.5 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.057 | 0.057 | 0.059 | 0.001 | 0.057 | 0.056 | 28.0 | 1.00× |
| tebako-ruby | 5 | 0.246 | 0.243 | 0.253 | 0.004 | 0.248 | 0.246 | 82.4 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.244 | 0.244 | 0.244 | 0.000 | 0.244 | 0.244 | 82.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.279 | 2.263 | 2.291 | 0.011 | 2.276 | 2.277 | 28.2 | 1.00× |
| tebako-ruby | 5 | 2.063 | 2.040 | 2.239 | 0.081 | 2.097 | 2.063 | 82.5 | 1.10× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.079 | 2.062 | 2.097 | 0.024 | 2.079 | 2.079 | 82.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.755 | 0.698 | 0.811 | 0.041 | 0.756 | 0.755 | 94.0 | 1.00× |
| tebako-ruby | 5 | 2.550 | 2.097 | 3.392 | 0.570 | 2.778 | 2.549 | 204.9 | 0.30× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.595 | 2.595 | 2.595 | 0.000 | 2.595 | 2.594 | 205.3 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.132 | 0.130 | 0.138 | 0.003 | 0.133 | 0.131 | 30.7 | 1.00× |
| tebako-ruby | 5 | 0.894 | 0.886 | 0.911 | 0.010 | 0.895 | 0.894 | 85.7 | 0.15× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.894 | 0.891 | 0.898 | 0.004 | 0.894 | 0.894 | 85.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.084 | 0.084 | 0.084 | 0.000 | 0.084 | 0.082 | 34.4 | 1.00× |
| tebako-ruby | 5 | 0.261 | 0.261 | 0.284 | 0.010 | 0.266 | 0.260 | 87.3 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.262 | 0.261 | 0.263 | 0.002 | 0.262 | 0.261 | 87.4 | — |

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
| on-system-java | 5 | 0.032 | 0.030 | 0.034 | 0.002 | 0.032 | 0.034 | 35.9 | 1.00× |
| tebako-java | 5 | 0.938 | 0.914 | 0.943 | 0.012 | 0.934 | 0.939 | 174.7 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.964 | 0.963 | 0.966 | 0.002 | 0.964 | 0.964 | 174.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.556 | 7.555 | 7.562 | 0.003 | 7.557 | 7.562 | 37.2 | 1.00× |
| tebako-java | 5 | 8.465 | 8.456 | 8.478 | 0.008 | 8.467 | 8.471 | 174.7 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.477 | 8.476 | 8.477 | 0.001 | 8.477 | 8.482 | 174.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.772 | 0.742 | 0.789 | 0.017 | 0.768 | 0.829 | 44.8 | 1.00× |
| tebako-java | 5 | 1.665 | 1.649 | 1.701 | 0.019 | 1.669 | 1.726 | 204.4 | 0.46× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.705 | 1.703 | 1.706 | 0.003 | 1.705 | 1.758 | 204.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.205 | 0.187 | 0.206 | 0.010 | 0.199 | 0.388 | 65.5 | 1.00× |
| tebako-java | 5 | 1.245 | 1.228 | 1.270 | 0.015 | 1.247 | 1.563 | 174.7 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.248 | 1.232 | 1.264 | 0.022 | 1.248 | 1.556 | 174.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.160 | 0.150 | 0.169 | 0.008 | 0.160 | 0.354 | 78.7 | 1.00× |
| tebako-java | 5 | 1.094 | 1.070 | 1.101 | 0.015 | 1.088 | 1.316 | 174.7 | 0.15× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.101 | 1.095 | 1.107 | 0.008 | 1.101 | 1.325 | 174.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.048 | 0.057 | 0.003 | 0.051 | 0.049 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.256 | 0.250 | 0.260 | 0.004 | 0.256 | 0.255 | 76.1 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.254 | 0.254 | 0.254 | 0.000 | 0.254 | 0.253 | 76.1 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.232 | 2.222 | 2.237 | 0.006 | 2.230 | 2.230 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.564 | 2.561 | 2.613 | 0.022 | 2.574 | 2.563 | 76.2 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.570 | 2.567 | 2.573 | 0.004 | 2.570 | 2.569 | 76.2 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.777 | 0.721 | 0.797 | 0.033 | 0.760 | 0.775 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.810 | 1.227 | 1.842 | 0.313 | 1.595 | 1.809 | 199.6 | 0.43× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.527 | 1.235 | 1.820 | 0.413 | 1.527 | 1.527 | 199.4 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.098 | 0.098 | 0.100 | 0.001 | 0.099 | 0.098 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.234 | 1.230 | 1.240 | 0.004 | 1.234 | 1.233 | 79.0 | 0.08× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.234 | 1.233 | 1.235 | 0.002 | 1.234 | 1.233 | 79.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.084 | 0.086 | 0.001 | 0.085 | 0.084 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.275 | 0.275 | 0.277 | 0.001 | 0.276 | 0.274 | 81.5 | 0.31× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.277 | 0.277 | 0.277 | 0.000 | 0.277 | 0.275 | 81.5 | — |

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
| on-system-java | 5 | 0.026 | 0.026 | 0.029 | 0.001 | 0.026 | 0.029 | 37.2 | 1.00× |
| tebako-java | 5 | 1.024 | 1.004 | 1.027 | 0.011 | 1.018 | 1.025 | 175.6 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.033 | 1.014 | 1.053 | 0.027 | 1.033 | 1.034 | 175.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 6.343 | 6.329 | 6.470 | 0.059 | 6.365 | 6.346 | 39.1 | 1.00× |
| tebako-java | 5 | 7.354 | 7.297 | 8.128 | 0.357 | 7.492 | 7.358 | 175.6 | 0.86× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 7.334 | 7.319 | 7.348 | 0.020 | 7.334 | 7.338 | 175.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.475 | 0.463 | 0.482 | 0.007 | 0.474 | 0.542 | 48.7 | 1.00× |
| tebako-java | 5 | 1.401 | 1.386 | 1.435 | 0.019 | 1.406 | 1.471 | 209.6 | 0.34× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.453 | 1.425 | 1.482 | 0.041 | 1.453 | 1.520 | 209.6 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.194 | 0.187 | 0.203 | 0.007 | 0.194 | 0.385 | 71.1 | 1.00× |
| tebako-java | 5 | 1.274 | 1.266 | 1.363 | 0.041 | 1.289 | 1.550 | 175.7 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.272 | 1.264 | 1.281 | 0.012 | 1.272 | 1.541 | 175.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.136 | 0.126 | 0.139 | 0.005 | 0.135 | 0.313 | 81.9 | 1.00× |
| tebako-java | 5 | 1.133 | 1.117 | 1.151 | 0.014 | 1.134 | 1.337 | 175.7 | 0.12× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.132 | 1.122 | 1.143 | 0.015 | 1.132 | 1.308 | 175.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.042 | 0.042 | 0.044 | 0.001 | 0.042 | 0.042 | 28.1 | 1.00× |
| tebako-ruby | 5 | 0.254 | 0.245 | 0.256 | 0.004 | 0.252 | 0.252 | 79.6 | 0.17× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.255 | 0.245 | 0.264 | 0.013 | 0.255 | 0.254 | 79.8 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.499 | 1.464 | 1.587 | 0.051 | 1.519 | 1.498 | 28.2 | 1.00× |
| tebako-ruby | 5 | 1.559 | 1.537 | 1.562 | 0.010 | 1.554 | 1.557 | 79.6 | 0.96× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.545 | 1.544 | 1.547 | 0.002 | 1.545 | 1.544 | 79.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.457 | 0.455 | 0.462 | 0.003 | 0.458 | 0.455 | 94.0 | 1.00× |
| tebako-ruby | 5 | 1.396 | 0.989 | 1.419 | 0.186 | 1.321 | 1.394 | 201.8 | 0.33× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.210 | 1.008 | 1.413 | 0.286 | 1.210 | 1.209 | 201.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.106 | 0.104 | 0.106 | 0.001 | 0.105 | 0.104 | 30.7 | 1.00× |
| tebako-ruby | 5 | 1.162 | 1.118 | 1.172 | 0.022 | 1.153 | 1.161 | 82.5 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.189 | 1.188 | 1.190 | 0.002 | 1.189 | 1.188 | 82.5 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.067 | 0.065 | 0.068 | 0.001 | 0.066 | 0.065 | 34.5 | 1.00× |
| tebako-ruby | 5 | 0.269 | 0.262 | 0.277 | 0.006 | 0.270 | 0.267 | 84.7 | 0.25× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.270 | 0.268 | 0.271 | 0.002 | 0.270 | 0.269 | 84.6 | — |

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
| on-system-java | 5 | 0.036 | 0.036 | 0.061 | 0.012 | 0.044 | 0.030 | 34.0 | 1.00× |
| tebako-java | 5 | 1.087 | 0.921 | 1.197 | 0.118 | 1.062 | 0.946 | 190.7 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.202 | 0.893 | 1.510 | 0.437 | 1.202 | 0.953 | 188.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 10.280 | 9.602 | 11.093 | 0.671 | 10.361 | 10.257 | 35.0 | 1.00× |
| tebako-java | 5 | 10.970 | 10.730 | 13.253 | 1.082 | 11.553 | 10.821 | 191.6 | 0.94× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 10.816 | 10.537 | 11.096 | 0.395 | 10.816 | 10.642 | 191.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.011 | 0.965 | 1.269 | 0.124 | 1.065 | 1.025 | 43.8 | 1.00× |
| tebako-java | 5 | 1.562 | 1.356 | 1.884 | 0.218 | 1.563 | 1.416 | 266.2 | 0.65× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.710 | 1.710 | 1.711 | 0.001 | 1.710 | 1.617 | 266.1 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.294 | 0.281 | 0.333 | 0.021 | 0.304 | 0.487 | 60.9 | 1.00× |
| tebako-java | 5 | 1.435 | 1.250 | 1.574 | 0.131 | 1.428 | 1.498 | 215.4 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.473 | 1.420 | 1.525 | 0.074 | 1.473 | 1.472 | 215.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.224 | 0.168 | 0.284 | 0.047 | 0.233 | 0.326 | 73.8 | 1.00× |
| tebako-java | 5 | 1.213 | 1.108 | 1.533 | 0.202 | 1.297 | 1.228 | 230.8 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.364 | 1.319 | 1.409 | 0.064 | 1.364 | 1.355 | 230.7 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.035 | 0.032 | 0.048 | 0.006 | 0.038 | 0.026 | 11.5 | 1.00× |
| tebako-python | 5 | 0.335 | 0.315 | 0.359 | 0.018 | 0.339 | 0.307 | 131.4 | 0.10× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.394 | 0.392 | 0.395 | 0.002 | 0.394 | 0.355 | 131.1 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 9.548 | 9.103 | 10.305 | 0.472 | 9.579 | 9.521 | 11.4 | 1.00× |
| tebako-python | 5 | 10.951 | 10.761 | 12.069 | 0.553 | 11.215 | 10.924 | 130.3 | 0.87× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.793 | 10.775 | 10.811 | 0.026 | 10.793 | 10.757 | 130.4 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.616 | 0.574 | 0.646 | 0.029 | 0.609 | 0.602 | 14.5 | 1.00× |
| tebako-python | 5 | 0.461 | 0.431 | 0.477 | 0.019 | 0.454 | 0.444 | 198.4 | 1.34× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.464 | 0.453 | 0.475 | 0.015 | 0.464 | 0.450 | 199.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.691 | 1.679 | 1.750 | 0.030 | 1.706 | 1.689 | 20.3 | 1.00× |
| tebako-python | 5 | 3.773 | 3.624 | 3.818 | 0.091 | 3.723 | 3.748 | 141.5 | 0.45× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.970 | 3.856 | 4.084 | 0.161 | 3.970 | 3.944 | 143.3 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.037 | 0.032 | 0.051 | 0.008 | 0.039 | 0.031 | 18.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.065 | 0.057 | 0.073 | 0.007 | 0.065 | 0.050 | 12.5 | 1.00× |
| tebako-ruby | 5 | 0.273 | 0.258 | 0.302 | 0.017 | 0.279 | 0.258 | 64.5 | 0.24× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.335 | 0.298 | 0.372 | 0.052 | 0.335 | 0.291 | 65.8 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.787 | 2.594 | 2.829 | 0.094 | 2.743 | 2.762 | 12.8 | 1.00× |
| tebako-ruby | 5 | 2.621 | 2.588 | 2.716 | 0.054 | 2.642 | 2.599 | 66.2 | 1.06× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.733 | 2.713 | 2.752 | 0.028 | 2.733 | 2.713 | 64.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.039 | 1.762 | 2.282 | 0.193 | 2.015 | 2.017 | 183.8 | 1.00× |
| tebako-ruby | 5 | 1.390 | 1.201 | 2.259 | 0.435 | 1.538 | 1.368 | 269.4 | 1.47× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.525 | 1.500 | 1.550 | 0.035 | 1.525 | 1.494 | 287.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.153 | 0.124 | 0.182 | 0.023 | 0.155 | 0.142 | 14.8 | 1.00× |
| tebako-ruby | 5 | 1.418 | 1.328 | 1.451 | 0.052 | 1.398 | 1.365 | 67.3 | 0.11× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.325 | 1.263 | 1.387 | 0.087 | 1.325 | 1.308 | 68.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.088 | 0.079 | 0.095 | 0.007 | 0.086 | 0.067 | 19.4 | 1.00× |
| tebako-ruby | 5 | 0.261 | 0.246 | 0.299 | 0.021 | 0.266 | 0.241 | 70.3 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.320 | 0.319 | 0.320 | 0.001 | 0.320 | 0.307 | 71.0 | — |

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
| on-system-java | 5 | 0.165 | 0.151 | 0.191 | 0.016 | 0.168 | 0.171 | 25.3 | 1.00× |
| tebako-java | 5 | 2.482 | 1.996 | 2.659 | 0.283 | 2.396 | 2.124 | 155.4 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.239 | 2.235 | 2.244 | 0.006 | 2.239 | 2.060 | 155.4 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 14.442 | 14.139 | 16.115 | 0.793 | 14.730 | 14.446 | 25.8 | 1.00× |
| tebako-java | 5 | 17.390 | 15.413 | 17.706 | 0.963 | 16.950 | 17.118 | 156.4 | 0.83× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 17.954 | 17.505 | 18.403 | 0.635 | 17.954 | 17.654 | 156.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 2.062 | 1.817 | 2.241 | 0.155 | 2.029 | 2.097 | 35.0 | 1.00× |
| tebako-java | 5 | 3.249 | 3.149 | 3.449 | 0.127 | 3.288 | 3.354 | 230.3 | 0.63× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 3.718 | 3.640 | 3.795 | 0.110 | 3.718 | 3.683 | 230.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.860 | 0.743 | 1.538 | 0.327 | 0.980 | 1.623 | 70.7 | 1.00× |
| tebako-java | 5 | 2.821 | 2.374 | 3.255 | 0.388 | 2.833 | 3.433 | 176.3 | 0.30× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.324 | 2.238 | 2.409 | 0.121 | 2.324 | 2.798 | 176.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.479 | 0.347 | 0.793 | 0.172 | 0.512 | 0.856 | 63.6 | 1.00× |
| tebako-java | 5 | 2.091 | 1.924 | 2.256 | 0.134 | 2.095 | 2.279 | 168.9 | 0.23× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.209 | 2.139 | 2.279 | 0.099 | 2.209 | 2.362 | 168.8 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.126 | 0.115 | 0.154 | 0.017 | 0.134 | 0.116 | 9.4 | 1.00× |
| tebako-ruby | 5 | 0.535 | 0.524 | 0.557 | 0.016 | 0.539 | 0.518 | 59.6 | 0.24× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.511 | 0.509 | 0.513 | 0.003 | 0.511 | 0.501 | 59.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 9.514 | 9.144 | 9.775 | 0.287 | 9.494 | 9.493 | 9.7 | 1.00× |
| tebako-ruby | 5 | 5.303 | 5.137 | 5.519 | 0.158 | 5.349 | 5.290 | 59.6 | 1.79× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.967 | 4.925 | 5.009 | 0.060 | 4.967 | 4.954 | 59.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.639 | 3.473 | 3.756 | 0.107 | 3.642 | 3.626 | 74.3 | 1.00× |
| tebako-ruby | 5 | 4.332 | 4.015 | 4.955 | 0.383 | 4.404 | 4.319 | 173.2 | 0.84× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.186 | 4.151 | 4.222 | 0.051 | 4.186 | 4.169 | 172.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.308 | 0.293 | 0.351 | 0.024 | 0.312 | 0.292 | 11.6 | 1.00× |
| tebako-ruby | 5 | 1.958 | 1.948 | 2.385 | 0.188 | 2.067 | 1.944 | 61.9 | 0.16× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.319 | 2.256 | 2.383 | 0.090 | 2.319 | 2.303 | 62.0 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.190 | 0.168 | 0.293 | 0.050 | 0.210 | 0.175 | 14.9 | 1.00× |
| tebako-ruby | 5 | 0.559 | 0.507 | 0.657 | 0.058 | 0.567 | 0.542 | 63.5 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.525 | 0.523 | 0.527 | 0.003 | 0.525 | 0.509 | 63.5 | — |

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
| on-system-java | 5 | 0.073 | 0.067 | 0.083 | 0.007 | 0.073 | 0.109 | 331.1 | 1.00× |
| tebako-java | 5 | 0.790 | 0.774 | 0.812 | 0.015 | 0.794 | 0.781 | 463.3 | 0.09× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.822 | 0.800 | 0.844 | 0.031 | 0.822 | 0.836 | 463.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 13.803 | 13.461 | 14.231 | 0.362 | 13.847 | 13.703 | 331.3 | 1.00× |
| tebako-java | 5 | 14.773 | 14.107 | 15.973 | 0.759 | 14.766 | 14.750 | 463.3 | 0.93× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 14.169 | 14.166 | 14.173 | 0.004 | 14.169 | 14.141 | 463.8 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.167 | 3.103 | 3.236 | 0.052 | 3.170 | 3.281 | 337.4 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.895 | 0.876 | 0.919 | 0.016 | 0.895 | 1.391 | 349.9 | 1.00× |
| tebako-java | 5 | 1.064 | 1.063 | 1.088 | 0.013 | 1.073 | 1.469 | 476.1 | 0.84× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.084 | 1.077 | 1.092 | 0.010 | 1.084 | 1.492 | 477.5 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.263 | 0.246 | 0.276 | 0.013 | 0.263 | 0.625 | 337.6 | 1.00× |
| tebako-java | 5 | 0.967 | 0.945 | 0.990 | 0.016 | 0.969 | 1.328 | 470.5 | 0.27× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.968 | 0.958 | 0.979 | 0.015 | 0.968 | 1.281 | 469.9 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.152 | 0.149 | 0.169 | 0.008 | 0.155 | 0.156 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.231 | 0.229 | 0.239 | 0.004 | 0.232 | 0.219 | 68.4 | 0.66× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.241 | 0.228 | 0.253 | 0.017 | 0.241 | 0.227 | 68.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.193 | 2.179 | 2.375 | 0.082 | 2.237 | 2.203 | 51.6 | 1.00× |
| tebako-ruby | 5 | 2.288 | 2.237 | 2.346 | 0.043 | 2.290 | 2.266 | 68.4 | 0.96× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.241 | 2.231 | 2.250 | 0.014 | 2.241 | 2.219 | 68.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 5.162 | 5.061 | 5.408 | 0.131 | 5.209 | 5.172 | 117.7 | 1.00× |
| tebako-ruby | 5 | 3.953 | 3.898 | 4.027 | 0.062 | 3.960 | 3.906 | 134.5 | 1.31× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 3.911 | 3.911 | 3.912 | 0.001 | 3.911 | 3.883 | 134.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.872 | 2.832 | 2.899 | 0.027 | 2.867 | 2.859 | 53.8 | 1.00× |
| tebako-ruby | 5 | 1.231 | 1.226 | 1.256 | 0.012 | 1.236 | 1.234 | 69.0 | 2.33× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.233 | 1.230 | 1.236 | 0.004 | 1.233 | 1.250 | 69.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.206 | 0.203 | 0.213 | 0.004 | 0.207 | 0.203 | 53.0 | 1.00× |
| tebako-ruby | 5 | 0.254 | 0.250 | 0.262 | 0.005 | 0.256 | 0.250 | 69.1 | 0.81× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.259 | 0.256 | 0.262 | 0.004 | 0.259 | 0.250 | 69.2 | — |

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

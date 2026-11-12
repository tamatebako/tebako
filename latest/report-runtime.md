# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.032 | 0.030 | 0.034 | 0.002 | 0.032 | 0.033 | 35.9 | 1.00× |
| tebako-java | 5 | 0.744 | 0.739 | 0.751 | 0.005 | 0.745 | 0.745 | 173.2 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.767 | 0.761 | 0.773 | 0.009 | 0.767 | 0.767 | 173.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.560 | 7.554 | 7.561 | 0.003 | 7.559 | 7.564 | 37.2 | 1.00× |
| tebako-java | 5 | 8.278 | 8.273 | 8.287 | 0.006 | 8.280 | 8.285 | 173.2 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.281 | 8.274 | 8.288 | 0.010 | 8.281 | 8.288 | 173.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.909 | 0.869 | 0.924 | 0.023 | 0.899 | 0.956 | 44.8 | 1.00× |
| tebako-java | 5 | 1.499 | 1.455 | 1.528 | 0.026 | 1.493 | 1.556 | 201.0 | 0.61× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.564 | 1.523 | 1.606 | 0.059 | 1.564 | 1.625 | 201.0 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.211 | 0.194 | 0.213 | 0.008 | 0.207 | 0.407 | 67.9 | 1.00× |
| tebako-java | 5 | 1.018 | 1.014 | 1.026 | 0.006 | 1.019 | 1.311 | 173.2 | 0.21× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.070 | 1.052 | 1.088 | 0.025 | 1.070 | 1.366 | 173.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.167 | 0.150 | 0.170 | 0.008 | 0.164 | 0.361 | 78.6 | 1.00× |
| tebako-java | 5 | 0.880 | 0.864 | 0.893 | 0.011 | 0.881 | 1.073 | 173.2 | 0.19× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.899 | 0.893 | 0.905 | 0.008 | 0.899 | 1.086 | 173.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.013 | 0.000 | 0.013 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.350 | 0.348 | 0.365 | 0.008 | 0.355 | 0.349 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.349 | 0.348 | 0.350 | 0.001 | 0.349 | 0.348 | 185.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.170 | 12.121 | 12.196 | 0.032 | 12.166 | 12.167 | 8.7 | 1.00× |
| tebako-python | 5 | 12.583 | 12.578 | 12.617 | 0.018 | 12.593 | 12.579 | 185.7 | 0.97× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.604 | 12.601 | 12.607 | 0.004 | 12.604 | 12.601 | 185.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.512 | 0.494 | 0.562 | 0.026 | 0.520 | 0.511 | 9.7 | 1.00× |
| tebako-python | 5 | 0.749 | 0.731 | 0.758 | 0.011 | 0.745 | 0.749 | 186.0 | 0.68× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.752 | 0.741 | 0.762 | 0.015 | 0.752 | 0.751 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.836 | 1.820 | 1.847 | 0.010 | 1.835 | 1.834 | 16.7 | 1.00× |
| tebako-python | 5 | 3.844 | 3.837 | 3.868 | 0.013 | 3.851 | 3.843 | 185.7 | 0.48× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.858 | 3.855 | 3.861 | 0.004 | 3.858 | 3.856 | 185.8 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.032 | 0.032 | 0.032 | 0.000 | 0.032 | 0.030 | 16.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.052 | 0.051 | 0.053 | 0.001 | 0.052 | 0.051 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.250 | 0.250 | 0.253 | 0.001 | 0.251 | 0.250 | 74.4 | 0.21× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.250 | 0.250 | 0.250 | 0.000 | 0.250 | 0.249 | 74.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.241 | 2.227 | 2.248 | 0.008 | 2.238 | 2.240 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.518 | 2.471 | 2.545 | 0.028 | 2.511 | 2.517 | 74.4 | 0.89× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.486 | 2.483 | 2.489 | 0.004 | 2.486 | 2.485 | 74.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.781 | 0.762 | 0.796 | 0.013 | 0.778 | 0.780 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.276 | 1.250 | 1.293 | 0.016 | 1.276 | 1.275 | 196.9 | 0.61× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.276 | 1.260 | 1.293 | 0.023 | 1.276 | 1.275 | 197.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.105 | 0.100 | 0.119 | 0.008 | 0.106 | 0.103 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.157 | 1.153 | 1.160 | 0.003 | 1.157 | 1.156 | 77.3 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.159 | 1.158 | 1.161 | 0.002 | 1.159 | 1.158 | 77.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.086 | 0.086 | 0.000 | 0.086 | 0.085 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.269 | 0.269 | 0.271 | 0.001 | 0.269 | 0.268 | 78.1 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.270 | 0.269 | 0.271 | 0.001 | 0.270 | 0.268 | 78.3 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.039 | 0.033 | 0.047 | 0.006 | 0.040 | 0.044 | 37.2 | 1.00× |
| tebako-java | 5 | 1.067 | 1.059 | 1.087 | 0.011 | 1.069 | 1.066 | 181.9 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.053 | 1.041 | 1.064 | 0.016 | 1.053 | 1.055 | 182.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.842 | 11.748 | 11.858 | 0.045 | 11.818 | 11.852 | 39.0 | 1.00× |
| tebako-java | 5 | 12.822 | 12.794 | 12.883 | 0.036 | 12.834 | 12.834 | 182.1 | 0.92× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.843 | 12.817 | 12.869 | 0.036 | 12.843 | 12.855 | 182.0 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.548 | 3.452 | 3.811 | 0.145 | 3.572 | 3.629 | 47.6 | 1.00× |
| tebako-java | 5 | 1.723 | 1.714 | 1.799 | 0.041 | 1.748 | 1.832 | 207.8 | 2.06× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.805 | 1.800 | 1.810 | 0.007 | 1.805 | 1.909 | 208.1 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.273 | 0.264 | 0.284 | 0.008 | 0.274 | 0.533 | 68.9 | 1.00× |
| tebako-java | 5 | 1.361 | 1.346 | 1.394 | 0.020 | 1.367 | 1.715 | 182.1 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.360 | 1.343 | 1.377 | 0.024 | 1.360 | 1.703 | 182.1 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.225 | 0.213 | 0.229 | 0.006 | 0.223 | 0.536 | 81.2 | 1.00× |
| tebako-java | 5 | 1.239 | 1.212 | 1.253 | 0.016 | 1.233 | 1.556 | 182.0 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.257 | 1.256 | 1.259 | 0.002 | 1.257 | 1.571 | 182.0 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.013 | 0.015 | 0.001 | 0.014 | 0.013 | 9.5 | 1.00× |
| tebako-python | 5 | 0.487 | 0.471 | 0.514 | 0.017 | 0.493 | 0.486 | 194.5 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.482 | 0.480 | 0.483 | 0.002 | 0.482 | 0.480 | 194.2 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 13.320 | 13.043 | 13.745 | 0.272 | 13.386 | 13.319 | 9.5 | 1.00× |
| tebako-python | 5 | 13.244 | 13.102 | 13.742 | 0.265 | 13.327 | 13.242 | 194.4 | 1.01× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 13.302 | 13.071 | 13.533 | 0.326 | 13.302 | 13.300 | 194.3 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.525 | 0.507 | 0.536 | 0.014 | 0.523 | 0.524 | 10.5 | 1.00× |
| tebako-python | 5 | 0.882 | 0.876 | 0.903 | 0.011 | 0.886 | 0.879 | 194.9 | 0.60× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.886 | 0.885 | 0.887 | 0.001 | 0.886 | 0.885 | 194.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.792 | 2.768 | 2.810 | 0.017 | 2.790 | 2.790 | 17.5 | 1.00× |
| tebako-python | 5 | 3.443 | 3.428 | 3.551 | 0.052 | 3.472 | 3.442 | 194.5 | 0.81× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.561 | 3.516 | 3.606 | 0.064 | 3.561 | 3.560 | 193.6 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.034 | 0.034 | 0.034 | 0.000 | 0.034 | 0.033 | 17.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.070 | 0.068 | 0.072 | 0.002 | 0.070 | 0.068 | 28.0 | 1.00× |
| tebako-ruby | 5 | 0.307 | 0.304 | 0.309 | 0.002 | 0.307 | 0.306 | 82.5 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.304 | 0.303 | 0.304 | 0.001 | 0.304 | 0.303 | 82.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.913 | 2.854 | 3.033 | 0.068 | 2.935 | 2.912 | 28.0 | 1.00× |
| tebako-ruby | 5 | 2.815 | 2.785 | 2.824 | 0.015 | 2.811 | 2.814 | 82.4 | 1.03× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.894 | 2.826 | 2.961 | 0.096 | 2.894 | 2.891 | 80.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.808 | 0.795 | 0.816 | 0.009 | 0.805 | 0.805 | 93.9 | 1.00× |
| tebako-ruby | 5 | 1.635 | 1.551 | 3.579 | 0.913 | 2.241 | 1.634 | 205.2 | 0.49× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.790 | 2.782 | 2.797 | 0.011 | 2.790 | 2.788 | 202.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.150 | 0.149 | 0.154 | 0.002 | 0.151 | 0.148 | 30.6 | 1.00× |
| tebako-ruby | 5 | 1.097 | 1.091 | 1.124 | 0.013 | 1.102 | 1.095 | 85.4 | 0.14× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.090 | 1.087 | 1.093 | 0.004 | 1.090 | 1.089 | 85.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.104 | 0.103 | 0.106 | 0.001 | 0.104 | 0.103 | 34.2 | 1.00× |
| tebako-ruby | 5 | 0.328 | 0.325 | 0.333 | 0.003 | 0.329 | 0.327 | 87.0 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.327 | 0.326 | 0.328 | 0.002 | 0.327 | 0.326 | 87.0 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.034 | 0.034 | 0.037 | 0.001 | 0.035 | 0.036 | 36.2 | 1.00× |
| tebako-java | 5 | 0.962 | 0.941 | 0.969 | 0.011 | 0.960 | 0.962 | 174.7 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.971 | 0.968 | 0.974 | 0.004 | 0.971 | 0.973 | 174.8 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.559 | 7.555 | 7.562 | 0.003 | 7.559 | 7.567 | 37.2 | 1.00× |
| tebako-java | 5 | 8.502 | 8.493 | 8.521 | 0.012 | 8.505 | 8.509 | 174.7 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.483 | 8.475 | 8.492 | 0.012 | 8.483 | 8.490 | 174.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.927 | 0.900 | 0.943 | 0.018 | 0.922 | 0.972 | 44.5 | 1.00× |
| tebako-java | 5 | 1.769 | 1.751 | 1.795 | 0.016 | 1.771 | 1.833 | 204.3 | 0.52× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.800 | 1.774 | 1.825 | 0.036 | 1.800 | 1.856 | 204.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.191 | 0.176 | 0.203 | 0.010 | 0.192 | 0.383 | 66.9 | 1.00× |
| tebako-java | 5 | 1.244 | 1.242 | 1.254 | 0.005 | 1.247 | 1.563 | 174.7 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.263 | 1.263 | 1.263 | 0.000 | 1.263 | 1.577 | 174.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.166 | 0.165 | 0.173 | 0.003 | 0.168 | 0.356 | 78.7 | 1.00× |
| tebako-java | 5 | 1.088 | 1.071 | 1.112 | 0.015 | 1.091 | 1.323 | 174.7 | 0.15× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.126 | 1.115 | 1.137 | 0.015 | 1.126 | 1.334 | 174.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.050 | 0.051 | 0.000 | 0.050 | 0.049 | 12.6 | 1.00× |
| tebako-ruby | 5 | 0.258 | 0.252 | 0.260 | 0.003 | 0.257 | 0.257 | 76.1 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.257 | 0.256 | 0.259 | 0.002 | 0.257 | 0.256 | 76.3 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.230 | 2.222 | 2.245 | 0.008 | 2.232 | 2.229 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.574 | 2.556 | 2.590 | 0.015 | 2.573 | 2.573 | 76.1 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.607 | 2.548 | 2.665 | 0.082 | 2.607 | 2.605 | 76.3 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.893 | 0.862 | 0.935 | 0.026 | 0.897 | 0.892 | 78.5 | 1.00× |
| tebako-ruby | 5 | 1.939 | 1.342 | 2.002 | 0.315 | 1.744 | 1.937 | 199.6 | 0.46× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.041 | 1.988 | 2.094 | 0.074 | 2.041 | 2.040 | 199.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.113 | 0.109 | 0.117 | 0.003 | 0.113 | 0.112 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.252 | 1.247 | 1.261 | 0.005 | 1.253 | 1.250 | 79.1 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.251 | 1.250 | 1.252 | 0.002 | 1.251 | 1.250 | 79.0 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.090 | 0.088 | 0.090 | 0.001 | 0.090 | 0.088 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.284 | 0.283 | 0.285 | 0.001 | 0.284 | 0.283 | 81.5 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.281 | 0.281 | 0.281 | 0.000 | 0.281 | 0.281 | 81.4 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.035 | 0.042 | 0.003 | 0.038 | 0.040 | 37.6 | 1.00× |
| tebako-java | 5 | 1.408 | 1.388 | 1.411 | 0.010 | 1.403 | 1.411 | 175.6 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.389 | 1.388 | 1.390 | 0.002 | 1.389 | 1.393 | 175.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 11.827 | 11.793 | 11.877 | 0.036 | 11.835 | 11.841 | 39.0 | 1.00× |
| tebako-java | 5 | 13.258 | 13.197 | 13.285 | 0.035 | 13.255 | 13.269 | 175.6 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 13.216 | 13.175 | 13.258 | 0.059 | 13.216 | 13.228 | 175.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.546 | 3.490 | 3.558 | 0.028 | 3.539 | 3.618 | 47.6 | 1.00× |
| tebako-java | 5 | 2.113 | 2.106 | 2.133 | 0.011 | 2.118 | 2.229 | 209.4 | 1.68× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.143 | 2.140 | 2.145 | 0.003 | 2.143 | 2.249 | 209.5 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.276 | 0.268 | 0.286 | 0.007 | 0.277 | 0.544 | 68.9 | 1.00× |
| tebako-java | 5 | 1.799 | 1.789 | 1.848 | 0.024 | 1.808 | 2.194 | 175.6 | 0.15× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.825 | 1.806 | 1.844 | 0.026 | 1.825 | 2.277 | 175.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.221 | 0.212 | 0.241 | 0.012 | 0.222 | 0.524 | 81.8 | 1.00× |
| tebako-java | 5 | 1.603 | 1.582 | 1.619 | 0.014 | 1.603 | 1.913 | 175.6 | 0.14× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.609 | 1.607 | 1.611 | 0.003 | 1.609 | 1.949 | 175.6 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.070 | 0.068 | 0.072 | 0.002 | 0.070 | 0.068 | 28.0 | 1.00× |
| tebako-ruby | 5 | 0.345 | 0.343 | 0.349 | 0.003 | 0.345 | 0.343 | 79.6 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.346 | 0.343 | 0.348 | 0.004 | 0.346 | 0.344 | 79.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.905 | 2.868 | 3.088 | 0.087 | 2.937 | 2.903 | 28.0 | 1.00× |
| tebako-ruby | 5 | 2.820 | 2.806 | 2.867 | 0.024 | 2.826 | 2.819 | 79.9 | 1.03× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.838 | 2.810 | 2.866 | 0.040 | 2.838 | 2.837 | 79.8 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.779 | 0.764 | 0.821 | 0.028 | 0.790 | 0.778 | 93.8 | 1.00× |
| tebako-ruby | 5 | 2.172 | 1.579 | 2.210 | 0.314 | 1.959 | 2.170 | 202.0 | 0.36× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.204 | 2.192 | 2.217 | 0.017 | 2.204 | 2.203 | 201.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.152 | 0.152 | 0.157 | 0.002 | 0.153 | 0.151 | 30.5 | 1.00× |
| tebako-ruby | 5 | 1.462 | 1.454 | 1.478 | 0.011 | 1.466 | 1.461 | 82.1 | 0.10× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.465 | 1.453 | 1.476 | 0.016 | 1.465 | 1.463 | 82.6 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.106 | 0.105 | 0.109 | 0.002 | 0.106 | 0.105 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.371 | 0.370 | 0.372 | 0.001 | 0.371 | 0.369 | 84.8 | 0.28× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.371 | 0.370 | 0.371 | 0.001 | 0.371 | 0.370 | 84.2 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.044 | 0.027 | 0.085 | 0.021 | 0.051 | 0.033 | 34.0 | 1.00× |
| tebako-java | 5 | 1.142 | 1.009 | 1.410 | 0.146 | 1.177 | 1.000 | 190.7 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.066 | 1.023 | 1.109 | 0.061 | 1.066 | 0.930 | 190.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 10.353 | 9.822 | 10.520 | 0.315 | 10.211 | 10.316 | 34.9 | 1.00× |
| tebako-java | 5 | 11.751 | 11.335 | 12.254 | 0.328 | 11.788 | 11.526 | 191.6 | 0.88× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 10.996 | 10.928 | 11.063 | 0.096 | 10.996 | 10.864 | 191.4 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.126 | 0.880 | 1.273 | 0.169 | 1.069 | 1.114 | 43.6 | 1.00× |
| tebako-java | 5 | 1.831 | 1.434 | 1.967 | 0.225 | 1.761 | 1.748 | 266.1 | 0.61× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.154 | 1.910 | 2.397 | 0.345 | 2.154 | 1.996 | 266.2 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.367 | 0.243 | 0.405 | 0.066 | 0.352 | 0.540 | 60.5 | 1.00× |
| tebako-java | 5 | 1.806 | 1.737 | 2.272 | 0.217 | 1.920 | 1.835 | 215.5 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.723 | 1.583 | 1.863 | 0.198 | 1.723 | 1.745 | 215.3 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.198 | 0.167 | 0.303 | 0.065 | 0.230 | 0.321 | 74.4 | 1.00× |
| tebako-java | 5 | 1.254 | 1.143 | 1.734 | 0.246 | 1.355 | 1.276 | 230.4 | 0.16× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.088 | 1.057 | 1.119 | 0.044 | 1.088 | 1.083 | 230.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.035 | 0.030 | 0.040 | 0.005 | 0.035 | 0.024 | 11.4 | 1.00× |
| tebako-python | 5 | 0.295 | 0.283 | 0.374 | 0.037 | 0.308 | 0.278 | 131.4 | 0.12× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.344 | 0.270 | 0.419 | 0.106 | 0.344 | 0.325 | 132.4 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 10.716 | 10.430 | 11.528 | 0.464 | 10.871 | 10.665 | 11.4 | 1.00× |
| tebako-python | 5 | 12.793 | 11.840 | 13.390 | 0.575 | 12.688 | 12.743 | 131.4 | 0.84× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.527 | 12.461 | 12.593 | 0.093 | 12.527 | 12.469 | 131.1 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.602 | 0.548 | 0.670 | 0.050 | 0.596 | 0.588 | 13.5 | 1.00× |
| tebako-python | 5 | 0.462 | 0.429 | 0.485 | 0.022 | 0.457 | 0.436 | 197.6 | 1.30× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.490 | 0.475 | 0.504 | 0.020 | 0.490 | 0.464 | 199.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 2.138 | 1.830 | 2.792 | 0.392 | 2.221 | 2.132 | 20.4 | 1.00× |
| tebako-python | 5 | 4.186 | 4.071 | 4.497 | 0.175 | 4.263 | 4.155 | 143.0 | 0.51× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.945 | 3.937 | 3.954 | 0.012 | 3.945 | 3.922 | 140.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.041 | 0.035 | 0.061 | 0.010 | 0.044 | 0.036 | 18.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.110 | 0.070 | 0.157 | 0.035 | 0.105 | 0.090 | 12.7 | 1.00× |
| tebako-ruby | 5 | 0.352 | 0.333 | 0.396 | 0.024 | 0.358 | 0.341 | 66.1 | 0.31× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.376 | 0.357 | 0.396 | 0.028 | 0.376 | 0.353 | 66.0 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.743 | 2.669 | 3.094 | 0.170 | 2.799 | 2.711 | 12.8 | 1.00× |
| tebako-ruby | 5 | 2.948 | 2.801 | 3.514 | 0.319 | 3.101 | 2.926 | 65.8 | 0.93× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 3.061 | 2.830 | 3.293 | 0.328 | 3.061 | 3.037 | 65.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.300 | 1.965 | 2.398 | 0.177 | 2.225 | 2.240 | 151.0 | 1.00× |
| tebako-ruby | 5 | 1.993 | 1.529 | 2.015 | 0.219 | 1.850 | 1.960 | 290.3 | 1.15× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.524 | 1.475 | 1.572 | 0.069 | 1.524 | 1.470 | 269.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.185 | 0.162 | 0.246 | 0.036 | 0.195 | 0.161 | 14.7 | 1.00× |
| tebako-ruby | 5 | 1.419 | 1.344 | 1.959 | 0.255 | 1.507 | 1.404 | 68.3 | 0.13× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.339 | 1.299 | 1.379 | 0.056 | 1.339 | 1.312 | 67.9 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.103 | 0.081 | 0.115 | 0.013 | 0.100 | 0.092 | 19.1 | 1.00× |
| tebako-ruby | 5 | 0.299 | 0.279 | 0.378 | 0.039 | 0.314 | 0.286 | 72.1 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.307 | 0.283 | 0.331 | 0.034 | 0.307 | 0.289 | 70.5 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.120 | 0.114 | 0.126 | 0.005 | 0.120 | 0.109 | 25.3 | 1.00× |
| tebako-java | 5 | 1.334 | 1.274 | 1.972 | 0.323 | 1.532 | 1.263 | 155.5 | 0.09× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.499 | 1.312 | 1.686 | 0.264 | 1.499 | 1.225 | 155.4 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 13.834 | 12.014 | 14.556 | 1.157 | 13.291 | 13.838 | 25.8 | 1.00× |
| tebako-java | 5 | 15.141 | 13.709 | 21.930 | 3.316 | 16.113 | 14.652 | 156.4 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 15.688 | 14.306 | 17.070 | 1.954 | 15.688 | 15.555 | 156.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 2.055 | 1.757 | 2.388 | 0.292 | 2.067 | 2.000 | 34.6 | 1.00× |
| tebako-java | 5 | 3.158 | 2.699 | 3.629 | 0.345 | 3.131 | 3.206 | 230.3 | 0.65× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.690 | 2.585 | 2.795 | 0.148 | 2.690 | 2.620 | 230.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.504 | 0.480 | 0.860 | 0.162 | 0.573 | 0.917 | 69.8 | 1.00× |
| tebako-java | 5 | 2.090 | 1.847 | 2.674 | 0.322 | 2.150 | 2.464 | 176.4 | 0.24× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.765 | 2.628 | 2.901 | 0.193 | 2.765 | 3.119 | 176.3 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.430 | 0.326 | 0.494 | 0.068 | 0.434 | 0.799 | 63.7 | 1.00× |
| tebako-java | 5 | 2.067 | 1.854 | 2.503 | 0.238 | 2.128 | 2.152 | 168.4 | 0.21× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.903 | 1.753 | 2.053 | 0.212 | 1.903 | 2.023 | 167.3 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.108 | 0.102 | 0.132 | 0.014 | 0.116 | 0.099 | 9.6 | 1.00× |
| tebako-ruby | 5 | 0.441 | 0.423 | 0.551 | 0.052 | 0.459 | 0.430 | 59.5 | 0.25× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.432 | 0.428 | 0.437 | 0.006 | 0.432 | 0.419 | 59.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 8.003 | 7.800 | 8.975 | 0.475 | 8.218 | 7.980 | 9.7 | 1.00× |
| tebako-ruby | 5 | 4.287 | 4.191 | 4.523 | 0.150 | 4.338 | 4.275 | 59.4 | 1.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.144 | 4.089 | 4.198 | 0.077 | 4.144 | 4.124 | 59.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.990 | 2.795 | 3.303 | 0.187 | 3.001 | 2.969 | 74.6 | 1.00× |
| tebako-ruby | 5 | 3.528 | 3.421 | 3.881 | 0.185 | 3.602 | 3.512 | 172.7 | 0.85× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 3.416 | 3.389 | 3.443 | 0.039 | 3.416 | 3.401 | 172.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.255 | 0.249 | 0.296 | 0.019 | 0.262 | 0.237 | 11.7 | 1.00× |
| tebako-ruby | 5 | 1.684 | 1.655 | 1.777 | 0.051 | 1.700 | 1.668 | 62.2 | 0.15× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.691 | 1.687 | 1.695 | 0.006 | 1.691 | 1.680 | 62.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.145 | 0.132 | 0.168 | 0.015 | 0.151 | 0.129 | 14.8 | 1.00× |
| tebako-ruby | 5 | 0.432 | 0.413 | 0.532 | 0.048 | 0.448 | 0.409 | 63.4 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.424 | 0.423 | 0.425 | 0.001 | 0.424 | 0.408 | 63.1 | — |

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
Versions: tebako 2.8.77

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.068 | 0.064 | 0.074 | 0.004 | 0.069 | 0.094 | 331.1 | 1.00× |
| tebako-java | 5 | 0.764 | 0.757 | 0.786 | 0.012 | 0.767 | 0.750 | 463.5 | 0.09× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.777 | 0.775 | 0.779 | 0.003 | 0.777 | 0.773 | 463.6 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 13.351 | 13.332 | 13.370 | 0.017 | 13.353 | 13.344 | 331.3 | 1.00× |
| tebako-java | 5 | 14.054 | 14.029 | 14.074 | 0.018 | 14.055 | 14.031 | 463.8 | 0.95× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 14.060 | 14.055 | 14.066 | 0.008 | 14.060 | 14.039 | 463.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 3.544 | 3.382 | 3.961 | 0.218 | 3.591 | 3.531 | 337.4 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.019 | 1.005 | 1.110 | 0.043 | 1.034 | 1.594 | 349.7 | 1.00× |
| tebako-java | 5 | 1.214 | 1.205 | 1.240 | 0.013 | 1.218 | 1.672 | 477.1 | 0.84× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.172 | 1.157 | 1.187 | 0.021 | 1.172 | 1.617 | 477.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.261 | 0.250 | 0.305 | 0.022 | 0.268 | 0.625 | 337.9 | 1.00× |
| tebako-java | 5 | 0.966 | 0.957 | 1.011 | 0.025 | 0.978 | 1.297 | 470.3 | 0.27× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.959 | 0.952 | 0.966 | 0.010 | 0.959 | 1.312 | 470.0 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.162 | 0.153 | 0.171 | 0.007 | 0.162 | 0.156 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.230 | 0.225 | 0.236 | 0.004 | 0.230 | 0.219 | 68.4 | 0.70× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.236 | 0.233 | 0.238 | 0.004 | 0.236 | 0.234 | 68.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.193 | 2.148 | 2.445 | 0.120 | 2.235 | 2.219 | 51.6 | 1.00× |
| tebako-ruby | 5 | 2.273 | 2.223 | 2.386 | 0.063 | 2.288 | 2.250 | 68.4 | 0.97× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.262 | 2.240 | 2.284 | 0.031 | 2.262 | 2.250 | 68.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 5.220 | 5.195 | 5.254 | 0.022 | 5.221 | 5.203 | 117.7 | 1.00× |
| tebako-ruby | 5 | 3.984 | 3.957 | 4.004 | 0.020 | 3.985 | 3.969 | 134.5 | 1.31× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.006 | 3.992 | 4.020 | 0.020 | 4.006 | 3.969 | 134.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.858 | 2.836 | 2.880 | 0.018 | 2.857 | 2.844 | 53.8 | 1.00× |
| tebako-ruby | 5 | 1.233 | 1.228 | 1.238 | 0.004 | 1.233 | 1.219 | 69.1 | 2.32× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.230 | 1.228 | 1.233 | 0.004 | 1.230 | 1.211 | 69.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.209 | 0.203 | 0.209 | 0.003 | 0.207 | 0.203 | 53.0 | 1.00× |
| tebako-ruby | 5 | 0.256 | 0.256 | 0.261 | 0.002 | 0.257 | 0.281 | 69.2 | 0.82× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.262 | 0.259 | 0.264 | 0.003 | 0.262 | 0.258 | 69.4 | — |

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

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.6 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15983.9 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

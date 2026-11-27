# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB · image ubuntu24-arm64/20261122.193.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.030 | 0.028 | 0.038 | 0.004 | 0.032 | 0.032 | 35.9 | 1.00× |
| tebako-java | 5 | 0.742 | 0.717 | 0.755 | 0.015 | 0.738 | 0.743 | 173.2 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.733 | 0.731 | 0.736 | 0.004 | 0.733 | 0.734 | 173.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.556 | 7.552 | 7.557 | 0.002 | 7.555 | 7.561 | 37.2 | 1.00× |
| tebako-java | 5 | 8.274 | 8.257 | 8.358 | 0.041 | 8.294 | 8.269 | 173.2 | 0.91× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.270 | 8.264 | 8.275 | 0.008 | 8.270 | 8.276 | 173.2 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.898 | 0.880 | 0.942 | 0.023 | 0.903 | 0.957 | 44.8 | 1.00× |
| tebako-java | 5 | 1.539 | 1.510 | 1.558 | 0.021 | 1.535 | 1.581 | 201.0 | 0.58× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.534 | 1.523 | 1.545 | 0.016 | 1.534 | 1.584 | 200.9 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.190 | 0.182 | 0.193 | 0.004 | 0.188 | 0.346 | 64.5 | 1.00× |
| tebako-java | 5 | 1.025 | 1.013 | 1.055 | 0.017 | 1.028 | 1.305 | 173.2 | 0.19× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.019 | 1.016 | 1.022 | 0.004 | 1.019 | 1.310 | 173.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.157 | 0.155 | 0.187 | 0.013 | 0.165 | 0.355 | 78.4 | 1.00× |
| tebako-java | 5 | 0.889 | 0.865 | 0.899 | 0.014 | 0.883 | 1.057 | 173.2 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.886 | 0.875 | 0.896 | 0.015 | 0.886 | 1.073 | 173.2 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.011 | 0.013 | 0.001 | 0.013 | 0.011 | 8.7 | 1.00× |
| tebako-python | 5 | 0.346 | 0.343 | 0.348 | 0.002 | 0.346 | 0.345 | 185.7 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.348 | 0.347 | 0.349 | 0.001 | 0.348 | 0.347 | 185.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 12.125 | 12.067 | 12.324 | 0.100 | 12.151 | 12.124 | 8.7 | 1.00× |
| tebako-python | 5 | 12.566 | 12.563 | 12.589 | 0.013 | 12.574 | 12.564 | 185.7 | 0.96× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 12.578 | 12.557 | 12.598 | 0.029 | 12.578 | 12.576 | 185.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.426 | 0.420 | 0.453 | 0.015 | 0.433 | 0.425 | 9.7 | 1.00× |
| tebako-python | 5 | 0.651 | 0.641 | 0.664 | 0.009 | 0.652 | 0.650 | 186.0 | 0.65× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.652 | 0.651 | 0.654 | 0.002 | 0.652 | 0.651 | 186.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.781 | 1.765 | 1.834 | 0.032 | 1.795 | 1.780 | 16.6 | 1.00× |
| tebako-python | 5 | 3.785 | 3.775 | 3.796 | 0.008 | 3.785 | 3.783 | 185.7 | 0.47× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.790 | 3.777 | 3.802 | 0.018 | 3.790 | 3.788 | 185.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.030 | 0.030 | 0.030 | 0.000 | 0.030 | 0.028 | 16.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.051 | 0.050 | 0.053 | 0.001 | 0.051 | 0.050 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.244 | 0.242 | 0.245 | 0.001 | 0.244 | 0.243 | 74.3 | 0.21× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.244 | 0.244 | 0.245 | 0.001 | 0.244 | 0.243 | 74.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.237 | 2.214 | 2.245 | 0.012 | 2.233 | 2.237 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.481 | 2.463 | 2.485 | 0.009 | 2.477 | 2.480 | 74.4 | 0.90× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.489 | 2.478 | 2.500 | 0.016 | 2.489 | 2.489 | 74.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.632 | 0.631 | 0.703 | 0.034 | 0.656 | 0.631 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.153 | 1.121 | 1.213 | 0.038 | 1.164 | 1.153 | 197.0 | 0.55× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.124 | 1.110 | 1.139 | 0.021 | 1.124 | 1.123 | 197.1 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.201 | 0.199 | 0.203 | 0.001 | 0.201 | 0.199 | 67.9 | — |
| tebako-ruby | 5 | 0.568 | 0.568 | 0.576 | 0.004 | 0.571 | 0.567 | 74.6 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 0.270 | 0.199 | 0.341 | 0.100 | 0.270 | 0.198 | 68.0 | — |
| tebako-ruby | 2 | 0.569 | 0.567 | 0.570 | 0.002 | 0.569 | 0.567 | 74.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.101 | 0.100 | 0.103 | 0.001 | 0.101 | 0.100 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.150 | 1.149 | 1.153 | 0.002 | 1.151 | 1.149 | 77.3 | 0.09× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.152 | 1.147 | 1.157 | 0.007 | 1.152 | 1.151 | 77.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.086 | 0.084 | 0.086 | 0.001 | 0.086 | 0.084 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.263 | 0.263 | 0.265 | 0.001 | 0.264 | 0.262 | 78.1 | 0.33× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.265 | 0.265 | 0.265 | 0.000 | 0.265 | 0.263 | 78.2 | — |

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

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB · image ubuntu24/20261122.23.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.027 | 0.026 | 0.029 | 0.001 | 0.027 | 0.030 | 37.0 | 1.00× |
| tebako-java | 5 | 0.707 | 0.694 | 0.715 | 0.009 | 0.706 | 0.709 | 182.2 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.707 | 0.703 | 0.710 | 0.005 | 0.707 | 0.708 | 182.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 10.044 | 9.995 | 10.076 | 0.030 | 10.041 | 10.051 | 39.0 | 1.00× |
| tebako-java | 5 | 10.729 | 10.661 | 10.735 | 0.030 | 10.713 | 10.738 | 182.2 | 0.94× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 10.632 | 10.584 | 10.680 | 0.067 | 10.632 | 10.641 | 181.8 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.039 | 0.950 | 1.093 | 0.057 | 1.020 | 1.092 | 47.7 | 1.00× |
| tebako-java | 5 | 1.573 | 1.508 | 1.601 | 0.037 | 1.567 | 1.649 | 207.2 | 0.66× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.583 | 1.581 | 1.585 | 0.003 | 1.583 | 1.644 | 207.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.138 | 0.134 | 0.142 | 0.003 | 0.138 | 0.276 | 65.7 | 1.00× |
| tebako-java | 5 | 0.874 | 0.871 | 0.890 | 0.008 | 0.878 | 1.034 | 182.1 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.879 | 0.879 | 0.880 | 0.001 | 0.879 | 1.041 | 182.2 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.155 | 0.148 | 0.161 | 0.006 | 0.155 | 0.353 | 82.0 | 1.00× |
| tebako-java | 5 | 0.838 | 0.818 | 0.844 | 0.010 | 0.835 | 1.039 | 182.3 | 0.18× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.834 | 0.831 | 0.837 | 0.004 | 0.834 | 1.047 | 182.3 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.011 | 0.011 | 0.011 | 0.000 | 0.011 | 0.010 | 9.5 | 1.00× |
| tebako-python | 5 | 0.317 | 0.312 | 0.324 | 0.004 | 0.318 | 0.315 | 194.6 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.321 | 0.316 | 0.326 | 0.007 | 0.321 | 0.319 | 193.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 8.222 | 8.171 | 8.249 | 0.030 | 8.220 | 8.220 | 9.5 | 1.00× |
| tebako-python | 5 | 8.343 | 8.317 | 8.362 | 0.019 | 8.337 | 8.342 | 194.6 | 0.99× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 8.322 | 8.312 | 8.332 | 0.014 | 8.322 | 8.320 | 194.4 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.605 | 0.570 | 0.640 | 0.027 | 0.605 | 0.603 | 10.5 | 1.00× |
| tebako-python | 5 | 0.783 | 0.744 | 0.827 | 0.033 | 0.783 | 0.782 | 194.9 | 0.77× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.719 | 0.718 | 0.721 | 0.002 | 0.719 | 0.718 | 194.8 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.313 | 1.288 | 1.367 | 0.036 | 1.323 | 1.311 | 17.5 | 1.00× |
| tebako-python | 5 | 2.199 | 2.193 | 2.220 | 0.013 | 2.205 | 2.197 | 194.6 | 0.60× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.205 | 2.196 | 2.215 | 0.013 | 2.205 | 2.204 | 194.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.025 | 0.025 | 0.027 | 0.001 | 0.026 | 0.025 | 17.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.047 | 0.046 | 0.047 | 0.000 | 0.046 | 0.045 | 28.1 | 1.00× |
| tebako-ruby | 5 | 0.200 | 0.200 | 0.201 | 0.001 | 0.200 | 0.199 | 82.7 | 0.23× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.203 | 0.202 | 0.204 | 0.001 | 0.203 | 0.201 | 82.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.734 | 1.727 | 1.750 | 0.009 | 1.738 | 1.733 | 28.3 | 1.00× |
| tebako-ruby | 5 | 1.881 | 1.868 | 1.905 | 0.015 | 1.886 | 1.881 | 82.7 | 0.92× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.884 | 1.883 | 1.886 | 0.002 | 1.884 | 1.882 | 82.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.042 | 0.943 | 1.080 | 0.052 | 1.027 | 1.039 | 94.2 | 1.00× |
| tebako-ruby | 5 | 1.492 | 1.440 | 2.302 | 0.444 | 1.794 | 1.490 | 204.9 | 0.70× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.305 | 1.646 | 2.964 | 0.932 | 2.305 | 2.305 | 205.0 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.181 | 0.180 | 0.190 | 0.004 | 0.183 | 0.179 | 74.8 | — |
| tebako-ruby | 5 | 0.437 | 0.435 | 0.444 | 0.004 | 0.440 | 0.437 | 82.8 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 0.184 | 0.184 | 0.185 | 0.000 | 0.184 | 0.180 | 74.8 | — |
| tebako-ruby | 2 | 0.435 | 0.433 | 0.437 | 0.003 | 0.435 | 0.434 | 82.6 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.080 | 0.080 | 0.081 | 0.000 | 0.080 | 0.079 | 30.8 | 1.00× |
| tebako-ruby | 5 | 0.732 | 0.725 | 0.735 | 0.004 | 0.731 | 0.732 | 85.6 | 0.11× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.737 | 0.735 | 0.738 | 0.002 | 0.737 | 0.735 | 85.6 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.074 | 0.073 | 0.074 | 0.000 | 0.074 | 0.072 | 34.4 | 1.00× |
| tebako-ruby | 5 | 0.217 | 0.217 | 0.218 | 0.000 | 0.217 | 0.216 | 87.2 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.218 | 0.218 | 0.218 | 0.000 | 0.218 | 0.216 | 87.5 | — |

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

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15941.5 GiB · image ubuntu24-arm64/20261122.193.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.033 | 0.030 | 0.037 | 0.003 | 0.033 | 0.032 | 35.4 | 1.00× |
| tebako-java | 5 | 0.950 | 0.936 | 0.971 | 0.015 | 0.950 | 0.951 | 174.7 | 0.03× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.984 | 0.981 | 0.987 | 0.004 | 0.984 | 0.986 | 174.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 7.571 | 7.558 | 9.575 | 0.898 | 7.968 | 7.562 | 37.2 | 1.00× |
| tebako-java | 5 | 8.495 | 8.484 | 8.514 | 0.011 | 8.497 | 8.503 | 174.7 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 8.505 | 8.486 | 8.523 | 0.026 | 8.505 | 8.509 | 174.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.913 | 0.884 | 0.917 | 0.016 | 0.903 | 0.961 | 44.8 | 1.00× |
| tebako-java | 5 | 1.795 | 1.750 | 1.815 | 0.027 | 1.785 | 1.851 | 204.4 | 0.51× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.792 | 1.772 | 1.812 | 0.028 | 1.792 | 1.856 | 204.4 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.220 | 0.205 | 0.248 | 0.017 | 0.223 | 0.422 | 67.0 | 1.00× |
| tebako-java | 5 | 1.318 | 1.298 | 1.334 | 0.013 | 1.316 | 1.649 | 174.7 | 0.17× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.326 | 1.325 | 1.326 | 0.001 | 1.326 | 1.661 | 174.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.169 | 0.162 | 0.182 | 0.008 | 0.171 | 0.368 | 78.6 | 1.00× |
| tebako-java | 5 | 1.114 | 1.097 | 1.129 | 0.012 | 1.113 | 1.331 | 174.7 | 0.15× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.135 | 1.129 | 1.140 | 0.007 | 1.135 | 1.361 | 174.7 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.053 | 0.051 | 0.055 | 0.001 | 0.053 | 0.051 | 12.4 | 1.00× |
| tebako-ruby | 5 | 0.257 | 0.253 | 0.259 | 0.003 | 0.256 | 0.257 | 76.2 | 0.20× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.254 | 0.251 | 0.257 | 0.004 | 0.254 | 0.253 | 76.1 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.236 | 2.222 | 2.635 | 0.180 | 2.314 | 2.234 | 12.6 | 1.00× |
| tebako-ruby | 5 | 2.564 | 2.560 | 2.577 | 0.007 | 2.566 | 2.563 | 76.2 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.571 | 2.562 | 2.579 | 0.012 | 2.571 | 2.570 | 76.3 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.670 | 0.640 | 0.682 | 0.016 | 0.668 | 0.670 | 78.4 | 1.00× |
| tebako-ruby | 5 | 1.851 | 1.804 | 1.893 | 0.032 | 1.852 | 1.851 | 199.6 | 0.36× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.866 | 1.847 | 1.885 | 0.027 | 1.866 | 1.865 | 199.7 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.226 | 0.225 | 0.227 | 0.001 | 0.226 | 0.225 | 69.3 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 0.230 | 0.227 | 0.234 | 0.005 | 0.230 | 0.224 | 69.4 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.105 | 0.103 | 0.107 | 0.002 | 0.105 | 0.104 | 15.2 | 1.00× |
| tebako-ruby | 5 | 1.241 | 1.239 | 1.245 | 0.003 | 1.242 | 1.240 | 79.1 | 0.08× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.239 | 1.236 | 1.243 | 0.005 | 1.239 | 1.237 | 79.1 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.088 | 0.088 | 0.090 | 0.001 | 0.089 | 0.088 | 18.9 | 1.00× |
| tebako-ruby | 5 | 0.278 | 0.276 | 0.280 | 0.002 | 0.278 | 0.277 | 81.5 | 0.32× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.275 | 0.274 | 0.276 | 0.002 | 0.275 | 0.274 | 81.4 | — |

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

Failed runs:

- ruby-spawn / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15701.6 GiB · image ubuntu24/20261122.23.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.029 | 0.027 | 0.034 | 0.003 | 0.030 | 0.031 | 37.2 | 1.00× |
| tebako-java | 5 | 1.269 | 1.253 | 1.284 | 0.012 | 1.269 | 1.270 | 175.6 | 0.02× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.265 | 1.264 | 1.265 | 0.001 | 1.265 | 1.269 | 175.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.963 | 9.951 | 9.995 | 0.020 | 9.972 | 9.970 | 39.1 | 1.00× |
| tebako-java | 5 | 11.217 | 11.197 | 11.282 | 0.036 | 11.232 | 11.224 | 175.6 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 11.245 | 11.228 | 11.261 | 0.024 | 11.245 | 11.254 | 175.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.035 | 0.979 | 1.055 | 0.028 | 1.027 | 1.107 | 47.7 | 1.00× |
| tebako-java | 5 | 2.178 | 2.130 | 2.198 | 0.026 | 2.170 | 2.249 | 209.4 | 0.48× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.175 | 2.174 | 2.176 | 0.001 | 2.175 | 2.240 | 209.2 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.148 | 0.137 | 0.160 | 0.011 | 0.148 | 0.278 | 67.9 | 1.00× |
| tebako-java | 5 | 1.542 | 1.519 | 1.557 | 0.017 | 1.539 | 1.825 | 175.6 | 0.10× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.588 | 1.547 | 1.629 | 0.058 | 1.588 | 1.970 | 175.7 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.157 | 0.153 | 0.181 | 0.011 | 0.161 | 0.392 | 81.6 | 1.00× |
| tebako-java | 5 | 1.403 | 1.399 | 1.411 | 0.006 | 1.405 | 1.634 | 175.6 | 0.11× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.406 | 1.403 | 1.408 | 0.003 | 1.406 | 1.626 | 175.6 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.046 | 0.044 | 0.047 | 0.001 | 0.046 | 0.044 | 28.1 | 1.00× |
| tebako-ruby | 5 | 0.335 | 0.334 | 0.340 | 0.003 | 0.336 | 0.334 | 79.4 | 0.14× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.334 | 0.334 | 0.334 | 0.000 | 0.334 | 0.332 | 79.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.736 | 1.722 | 1.851 | 0.053 | 1.756 | 1.734 | 28.3 | 1.00× |
| tebako-ruby | 5 | 1.975 | 1.955 | 2.319 | 0.156 | 2.041 | 1.973 | 79.5 | 0.88× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.967 | 1.960 | 1.973 | 0.009 | 1.967 | 1.965 | 79.5 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.877 | 0.859 | 0.943 | 0.033 | 0.891 | 0.875 | 94.1 | 1.00× |
| tebako-ruby | 5 | 1.438 | 1.383 | 1.798 | 0.170 | 1.496 | 1.436 | 201.9 | 0.61× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.800 | 1.781 | 1.819 | 0.027 | 1.800 | 1.799 | 202.3 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.273 | 0.272 | 0.274 | 0.001 | 0.273 | 0.271 | 71.2 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 0.275 | 0.273 | 0.277 | 0.003 | 0.275 | 0.273 | 71.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.080 | 0.079 | 0.081 | 0.001 | 0.080 | 0.078 | 30.8 | 1.00× |
| tebako-ruby | 5 | 1.655 | 1.651 | 1.659 | 0.004 | 1.655 | 1.654 | 82.2 | 0.05× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.655 | 1.655 | 1.656 | 0.001 | 1.655 | 1.654 | 82.3 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.074 | 0.072 | 0.074 | 0.001 | 0.073 | 0.072 | 34.3 | 1.00× |
| tebako-ruby | 5 | 0.354 | 0.353 | 0.360 | 0.003 | 0.356 | 0.353 | 84.8 | 0.21× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.353 | 0.352 | 0.354 | 0.001 | 0.353 | 0.352 | 84.2 | — |

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

Failed runs:

- ruby-spawn / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-15 · aarch64 · 3 cpus · 7168.0 GiB · image macos15/20261118.0550.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.046 | 0.044 | 0.101 | 0.025 | 0.062 | 0.037 | 34.0 | 1.00× |
| tebako-java | 5 | 1.202 | 1.161 | 1.332 | 0.071 | 1.237 | 1.050 | 190.7 | 0.04× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.143 | 1.127 | 1.160 | 0.023 | 1.143 | 0.993 | 190.7 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 9.341 | 9.113 | 9.546 | 0.182 | 9.341 | 9.318 | 35.0 | 1.00× |
| tebako-java | 5 | 10.529 | 10.305 | 10.689 | 0.139 | 10.509 | 10.329 | 191.4 | 0.89× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 10.638 | 10.504 | 10.771 | 0.189 | 10.638 | 10.455 | 191.7 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.952 | 0.890 | 0.985 | 0.035 | 0.945 | 0.985 | 43.7 | 1.00× |
| tebako-java | 5 | 1.650 | 1.495 | 1.944 | 0.163 | 1.681 | 1.607 | 266.6 | 0.58× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.754 | 1.701 | 1.806 | 0.074 | 1.754 | 1.671 | 266.2 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.247 | 0.212 | 0.300 | 0.035 | 0.245 | 0.389 | 60.3 | 1.00× |
| tebako-java | 5 | 1.255 | 1.208 | 1.369 | 0.065 | 1.263 | 1.266 | 216.2 | 0.20× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.157 | 1.116 | 1.197 | 0.057 | 1.157 | 1.239 | 216.1 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.228 | 0.162 | 0.232 | 0.032 | 0.208 | 0.296 | 74.2 | 1.00× |
| tebako-java | 5 | 1.167 | 1.089 | 1.597 | 0.203 | 1.259 | 1.169 | 230.3 | 0.20× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.574 | 1.388 | 1.761 | 0.264 | 1.574 | 1.399 | 229.9 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.038 | 0.019 | 0.045 | 0.010 | 0.036 | 0.023 | 11.4 | 1.00× |
| tebako-python | 5 | 0.369 | 0.310 | 0.408 | 0.046 | 0.359 | 0.287 | 132.4 | 0.10× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.323 | 0.320 | 0.326 | 0.004 | 0.323 | 0.272 | 132.2 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 9.078 | 8.885 | 10.415 | 0.628 | 9.397 | 8.992 | 11.5 | 1.00× |
| tebako-python | 5 | 10.679 | 10.366 | 13.585 | 1.340 | 11.206 | 10.652 | 132.0 | 0.85× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 10.805 | 10.769 | 10.842 | 0.052 | 10.805 | 10.756 | 131.2 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.630 | 0.625 | 0.652 | 0.011 | 0.635 | 0.624 | 13.8 | 1.00× |
| tebako-python | 5 | 0.478 | 0.472 | 0.490 | 0.008 | 0.481 | 0.457 | 199.7 | 1.32× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.451 | 0.438 | 0.465 | 0.020 | 0.451 | 0.431 | 199.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.688 | 1.651 | 1.751 | 0.040 | 1.699 | 1.666 | 20.4 | 1.00× |
| tebako-python | 5 | 3.556 | 3.513 | 3.724 | 0.087 | 3.584 | 3.529 | 141.3 | 0.47× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 3.582 | 3.522 | 3.642 | 0.085 | 3.582 | 3.565 | 140.2 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.036 | 0.030 | 0.064 | 0.014 | 0.042 | 0.032 | 18.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.062 | 0.048 | 0.075 | 0.010 | 0.062 | 0.049 | 12.7 | 1.00× |
| tebako-ruby | 5 | 0.234 | 0.226 | 0.256 | 0.013 | 0.240 | 0.215 | 64.9 | 0.26× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.238 | 0.238 | 0.238 | 0.000 | 0.238 | 0.221 | 66.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.272 | 2.218 | 2.537 | 0.127 | 2.318 | 2.263 | 12.5 | 1.00× |
| tebako-ruby | 5 | 2.420 | 2.388 | 2.844 | 0.191 | 2.516 | 2.399 | 65.5 | 0.94× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.481 | 2.457 | 2.504 | 0.034 | 2.481 | 2.450 | 65.0 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.958 | 1.853 | 2.215 | 0.144 | 2.002 | 1.939 | 151.5 | 1.00× |
| tebako-ruby | 5 | 1.551 | 1.356 | 2.255 | 0.369 | 1.620 | 1.518 | 270.4 | 1.26× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.122 | 1.066 | 1.178 | 0.079 | 1.122 | 1.097 | 269.4 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.941 | 0.907 | 1.043 | 0.056 | 0.967 | 0.573 | 68.6 | — |
| tebako-ruby | 5 | 0.440 | 0.383 | 0.456 | 0.036 | 0.424 | 0.358 | 65.6 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 1.070 | 1.026 | 1.114 | 0.062 | 1.070 | 0.591 | 68.8 | — |
| tebako-ruby | 2 | 0.414 | 0.394 | 0.433 | 0.028 | 0.414 | 0.379 | 66.7 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.149 | 0.119 | 0.154 | 0.014 | 0.141 | 0.128 | 14.7 | 1.00× |
| tebako-ruby | 5 | 1.208 | 1.174 | 1.247 | 0.026 | 1.210 | 1.183 | 69.0 | 0.12× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.221 | 1.206 | 1.236 | 0.021 | 1.221 | 1.206 | 67.4 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.090 | 0.067 | 0.095 | 0.011 | 0.086 | 0.064 | 19.0 | 1.00× |
| tebako-ruby | 5 | 0.260 | 0.230 | 0.270 | 0.018 | 0.253 | 0.246 | 70.8 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.303 | 0.272 | 0.333 | 0.043 | 0.303 | 0.283 | 69.1 | — |

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

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB · image macos15/20260824.0482.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.127 | 0.114 | 0.141 | 0.010 | 0.128 | 0.119 | 25.3 | 1.00× |
| tebako-java | 5 | 2.187 | 1.617 | 3.965 | 0.948 | 2.386 | 1.660 | 155.9 | 0.06× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.476 | 2.432 | 2.521 | 0.063 | 2.476 | 2.138 | 155.4 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 14.234 | 13.917 | 15.834 | 0.922 | 14.701 | 14.242 | 25.8 | 1.00× |
| tebako-java | 5 | 18.835 | 16.094 | 21.346 | 1.985 | 18.592 | 18.602 | 156.4 | 0.76× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 15.816 | 15.686 | 15.946 | 0.184 | 15.816 | 15.622 | 156.1 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.792 | 1.741 | 1.992 | 0.099 | 1.820 | 1.896 | 34.6 | 1.00× |
| tebako-java | 5 | 3.297 | 2.904 | 3.638 | 0.271 | 3.263 | 3.077 | 230.3 | 0.54× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 3.203 | 2.983 | 3.423 | 0.311 | 3.203 | 3.089 | 230.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.669 | 0.558 | 0.720 | 0.069 | 0.643 | 1.069 | 67.8 | 1.00× |
| tebako-java | 5 | 2.502 | 2.466 | 3.491 | 0.437 | 2.737 | 2.994 | 176.3 | 0.27× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.309 | 2.170 | 2.448 | 0.197 | 2.309 | 2.658 | 176.4 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.417 | 0.398 | 0.574 | 0.072 | 0.455 | 0.793 | 63.2 | 1.00× |
| tebako-java | 5 | 1.956 | 1.923 | 2.411 | 0.233 | 2.106 | 2.130 | 169.3 | 0.21× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 2.002 | 1.848 | 2.157 | 0.219 | 2.002 | 2.244 | 167.8 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.063 | 0.050 | 0.072 | 0.008 | 0.061 | 0.056 | 8.7 | 1.00× |
| tebako-python | 5 | 0.965 | 0.872 | 1.127 | 0.112 | 0.987 | 0.935 | 130.5 | 0.07× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.172 | 1.152 | 1.193 | 0.029 | 1.172 | 1.045 | 130.3 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 25.985 | 25.626 | 46.160 | 8.828 | 30.640 | 25.970 | 8.8 | 1.00× |
| tebako-python | 5 | 52.619 | 52.225 | 67.321 | 6.502 | 55.797 | 52.506 | 129.7 | 0.49× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 52.171 | 52.124 | 52.218 | 0.067 | 52.171 | 52.099 | 129.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 1.068 | 1.048 | 1.158 | 0.044 | 1.082 | 1.059 | 9.9 | 1.00× |
| tebako-python | 5 | 1.317 | 1.310 | 1.342 | 0.015 | 1.325 | 1.287 | 196.0 | 0.81× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.375 | 1.303 | 1.446 | 0.101 | 1.375 | 1.355 | 196.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 4.288 | 4.157 | 4.843 | 0.271 | 4.412 | 4.269 | 17.6 | 1.00× |
| tebako-python | 5 | 5.939 | 5.839 | 6.053 | 0.085 | 5.960 | 5.922 | 136.8 | 0.72× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 5.919 | 5.875 | 5.962 | 0.061 | 5.919 | 5.886 | 136.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.079 | 0.073 | 0.082 | 0.004 | 0.078 | 0.069 | 14.0 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.182 | 0.115 | 0.221 | 0.047 | 0.166 | 0.168 | 9.7 | 1.00× |
| tebako-ruby | 5 | 0.654 | 0.474 | 1.122 | 0.266 | 0.737 | 0.629 | 59.6 | 0.28× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.492 | 0.450 | 0.533 | 0.059 | 0.492 | 0.482 | 59.8 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 8.205 | 8.115 | 8.854 | 0.320 | 8.396 | 8.189 | 9.6 | 1.00× |
| tebako-ruby | 5 | 4.794 | 4.638 | 5.089 | 0.172 | 4.809 | 4.775 | 59.4 | 1.71× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.613 | 4.598 | 4.628 | 0.022 | 4.613 | 4.597 | 59.8 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.586 | 3.368 | 4.935 | 0.647 | 3.849 | 3.570 | 74.6 | 1.00× |
| tebako-ruby | 5 | 4.716 | 3.878 | 5.372 | 0.686 | 4.639 | 4.705 | 172.6 | 0.76× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 4.406 | 4.032 | 4.781 | 0.530 | 4.406 | 4.391 | 172.8 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 2.587 | 2.357 | 2.880 | 0.203 | 2.578 | 2.270 | 66.9 | — |
| tebako-ruby | 5 | 1.220 | 1.056 | 1.611 | 0.214 | 1.285 | 1.131 | 60.0 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 2.368 | 2.338 | 2.398 | 0.042 | 2.368 | 2.163 | 66.9 | — |
| tebako-ruby | 2 | 1.316 | 1.254 | 1.378 | 0.088 | 1.316 | 1.144 | 59.9 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.448 | 0.413 | 0.479 | 0.025 | 0.444 | 0.428 | 11.9 | 1.00× |
| tebako-ruby | 5 | 2.781 | 2.741 | 2.942 | 0.085 | 2.825 | 2.765 | 62.0 | 0.16× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.779 | 2.750 | 2.808 | 0.041 | 2.779 | 2.766 | 61.9 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.154 | 0.151 | 0.183 | 0.014 | 0.159 | 0.140 | 14.8 | 1.00× |
| tebako-ruby | 5 | 0.460 | 0.444 | 0.505 | 0.028 | 0.471 | 0.443 | 63.3 | 0.34× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.454 | 0.446 | 0.463 | 0.012 | 0.454 | 0.440 | 63.1 | — |

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

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB · image win25-vs2026/20261123.321.1
Versions: tebako 2.8.86

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.064 | 0.060 | 0.074 | 0.005 | 0.065 | 0.078 | 330.9 | 1.00× |
| tebako-java | 5 | 0.780 | 0.754 | 0.838 | 0.032 | 0.786 | 0.734 | 463.0 | 0.08× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.762 | 0.761 | 0.762 | 0.001 | 0.762 | 0.797 | 463.2 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 12.046 | 11.976 | 12.065 | 0.039 | 12.025 | 12.031 | 331.3 | 1.00× |
| tebako-java | 5 | 12.680 | 12.668 | 12.774 | 0.044 | 12.699 | 12.672 | 463.7 | 0.95× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 12.724 | 12.708 | 12.740 | 0.023 | 12.724 | 12.719 | 463.6 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 1.587 | 1.531 | 1.679 | 0.054 | 1.592 | 1.719 | 337.5 | 1.00× |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.989 | 0.955 | 1.022 | 0.025 | 0.988 | 1.547 | 349.5 | 1.00× |
| tebako-java | 5 | 1.102 | 1.073 | 1.125 | 0.020 | 1.098 | 1.469 | 475.8 | 0.90× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 1.116 | 1.110 | 1.122 | 0.008 | 1.116 | 1.523 | 476.9 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.262 | 0.255 | 0.276 | 0.008 | 0.262 | 0.609 | 338.1 | 1.00× |
| tebako-java | 5 | 0.951 | 0.923 | 0.957 | 0.013 | 0.946 | 1.250 | 469.7 | 0.28× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.952 | 0.950 | 0.955 | 0.003 | 0.952 | 1.312 | 469.8 | — |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.151 | 0.150 | 0.154 | 0.002 | 0.152 | 0.141 | 51.6 | 1.00× |
| tebako-ruby | 5 | 0.239 | 0.238 | 0.249 | 0.004 | 0.241 | 0.234 | 68.4 | 0.63× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.248 | 0.244 | 0.252 | 0.006 | 0.248 | 0.234 | 68.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.600 | 2.570 | 2.616 | 0.018 | 2.595 | 2.578 | 51.6 | 1.00× |
| tebako-ruby | 5 | 2.714 | 2.680 | 2.779 | 0.037 | 2.718 | 2.703 | 68.3 | 0.96× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.717 | 2.701 | 2.732 | 0.022 | 2.717 | 2.711 | 68.6 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 3.188 | 3.097 | 3.214 | 0.056 | 3.161 | 3.188 | 117.7 | 1.00× |
| tebako-ruby | 5 | 2.725 | 2.682 | 2.789 | 0.040 | 2.728 | 2.719 | 134.6 | 1.17× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 2.706 | 2.692 | 2.721 | 0.021 | 2.706 | 2.695 | 134.6 | — |

### ruby-spawn — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 5 | 0.764 | 0.730 | 0.903 | 0.080 | 0.801 | 0.719 | 36.3 | — |

### ruby-spawn — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| extract-ruby | 2 | 0.912 | 0.882 | 0.942 | 0.042 | 0.912 | 0.695 | 36.4 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 2.870 | 2.853 | 2.902 | 0.022 | 2.878 | 2.859 | 53.8 | 1.00× |
| tebako-ruby | 5 | 1.353 | 1.335 | 1.363 | 0.012 | 1.351 | 1.328 | 68.9 | 2.12× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.339 | 1.337 | 1.340 | 0.002 | 1.339 | 1.320 | 69.0 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.195 | 0.194 | 0.205 | 0.005 | 0.197 | 0.188 | 52.9 | 1.00× |
| tebako-ruby | 5 | 0.269 | 0.262 | 0.272 | 0.004 | 0.268 | 0.281 | 69.0 | 0.72× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.265 | 0.263 | 0.267 | 0.003 | 0.265 | 0.266 | 69.3 | — |

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

- ruby-spawn / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-ioread / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15941.5 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15701.6 GiB; macos-arm64 = macos-15 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

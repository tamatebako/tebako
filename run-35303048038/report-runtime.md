# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.011 | 0.011 | 0.011 | 0.000 | 0.011 | 0.010 | 153.4 | 1.00× |
| tebako-python | 5 | 0.311 | 0.303 | 0.311 | 0.004 | 0.309 | 0.309 | 181.6 | 0.03× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.307 | 0.303 | 0.311 | 0.006 | 0.307 | 0.305 | 181.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.272 | 0.270 | 0.274 | 0.002 | 0.272 | 0.270 | 153.4 | 1.00× |
| tebako-python | 5 | 0.570 | 0.566 | 0.580 | 0.005 | 0.572 | 0.568 | 181.6 | 0.48× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.569 | 0.566 | 0.572 | 0.004 | 0.569 | 0.567 | 181.6 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.017 | 0.017 | 0.017 | 0.000 | 0.017 | 0.016 | 153.4 | 1.00× |
| tebako-python | 5 | 0.315 | 0.311 | 0.319 | 0.004 | 0.315 | 0.314 | 184.5 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.318 | 0.317 | 0.319 | 0.001 | 0.318 | 0.317 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.050 | 0.050 | 0.052 | 0.001 | 0.050 | 0.049 | 153.4 | 1.00× |
| tebako-python | 5 | 0.401 | 0.393 | 0.403 | 0.004 | 0.399 | 0.399 | 181.6 | 0.12× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.395 | 0.393 | 0.397 | 0.003 | 0.395 | 0.394 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.027 | 0.029 | 0.001 | 0.029 | 0.027 | 153.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.048 | 0.048 | 0.048 | 0.000 | 0.048 | 0.047 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.118 | 0.117 | 0.118 | 0.000 | 0.118 | 0.117 | 153.4 | 0.41× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.118 | 0.118 | 0.118 | 0.000 | 0.118 | 0.117 | 153.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.243 | 0.241 | 0.245 | 0.001 | 0.243 | 0.242 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.321 | 0.319 | 0.323 | 0.002 | 0.321 | 0.320 | 153.4 | 0.76× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.321 | 0.319 | 0.323 | 0.003 | 0.321 | 0.320 | 153.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.062 | 0.062 | 0.064 | 0.001 | 0.062 | 0.061 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.136 | 0.136 | 0.138 | 0.001 | 0.137 | 0.135 | 175.8 | 0.46× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.137 | 0.136 | 0.138 | 0.001 | 0.137 | 0.136 | 175.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.052 | 0.052 | 0.054 | 0.001 | 0.053 | 0.052 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.130 | 0.128 | 0.130 | 0.001 | 0.129 | 0.128 | 153.4 | 0.40× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.131 | 0.130 | 0.132 | 0.001 | 0.131 | 0.129 | 153.4 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.081 | 0.080 | 0.081 | 0.000 | 0.081 | 0.079 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.133 | 0.132 | 0.134 | 0.001 | 0.133 | 0.132 | 153.4 | 0.60× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.134 | 0.134 | 0.134 | 0.000 | 0.134 | 0.133 | 153.4 | — |

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
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.9

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.035 | 0.042 | 0.003 | 0.038 | 0.041 | 158.7 | 1.00× |
| tebako-java | 5 | 0.518 | 0.501 | 0.539 | 0.015 | 0.520 | 0.521 | 188.5 | 0.07× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.528 | 0.487 | 0.568 | 0.057 | 0.528 | 0.530 | 188.5 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.049 | 0.048 | 0.058 | 0.005 | 0.052 | 0.056 | 158.7 | 1.00× |
| tebako-java | 5 | 0.569 | 0.519 | 0.587 | 0.029 | 0.561 | 0.551 | 188.4 | 0.09× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.516 | 0.509 | 0.523 | 0.010 | 0.516 | 0.521 | 188.4 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.078 | 0.076 | 0.078 | 0.001 | 0.077 | 0.084 | 158.7 | 1.00× |
| tebako-java | 5 | 0.520 | 0.509 | 0.545 | 0.016 | 0.525 | 0.524 | 190.5 | 0.15× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.536 | 0.535 | 0.538 | 0.002 | 0.536 | 0.543 | 190.3 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.102 | 0.097 | 0.111 | 0.005 | 0.103 | 0.139 | 158.7 | 1.00× |
| tebako-java | 5 | 0.626 | 0.554 | 0.634 | 0.033 | 0.612 | 0.662 | 188.4 | 0.16× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.542 | 0.538 | 0.546 | 0.005 | 0.542 | 0.576 | 188.6 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.219 | 0.207 | 0.231 | 0.010 | 0.217 | 0.538 | 158.7 | 1.00× |
| tebako-java | 5 | 0.697 | 0.676 | 0.734 | 0.025 | 0.701 | 1.004 | 188.6 | 0.31× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.721 | 0.717 | 0.724 | 0.005 | 0.721 | 1.037 | 188.5 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.015 | 0.015 | 0.000 | 0.015 | 0.014 | 158.7 | 1.00× |
| tebako-python | 5 | 0.332 | 0.321 | 0.340 | 0.007 | 0.332 | 0.330 | 197.6 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.421 | 0.416 | 0.426 | 0.007 | 0.421 | 0.420 | 194.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.315 | 0.298 | 0.321 | 0.009 | 0.313 | 0.313 | 158.7 | 1.00× |
| tebako-python | 5 | 0.610 | 0.575 | 0.666 | 0.034 | 0.612 | 0.608 | 196.9 | 0.52× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.596 | 0.588 | 0.604 | 0.012 | 0.596 | 0.594 | 195.9 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.021 | 0.021 | 0.021 | 0.000 | 0.021 | 0.020 | 158.7 | 1.00× |
| tebako-python | 5 | 0.347 | 0.325 | 0.350 | 0.010 | 0.341 | 0.346 | 199.6 | 0.06× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.328 | 0.325 | 0.331 | 0.004 | 0.328 | 0.327 | 196.9 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.081 | 0.079 | 0.082 | 0.001 | 0.080 | 0.079 | 158.7 | 1.00× |
| tebako-python | 5 | 0.430 | 0.424 | 0.445 | 0.009 | 0.433 | 0.429 | 197.0 | 0.19× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.413 | 0.412 | 0.414 | 0.001 | 0.413 | 0.411 | 196.5 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.038 | 0.035 | 0.038 | 0.001 | 0.037 | 0.036 | 158.7 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.075 | 0.073 | 0.077 | 0.002 | 0.075 | 0.075 | 158.7 | 1.00× |
| tebako-ruby | 5 | 0.141 | 0.137 | 0.154 | 0.007 | 0.143 | 0.141 | 158.7 | 0.53× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.138 | 0.137 | 0.139 | 0.002 | 0.138 | 0.137 | 158.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.328 | 0.323 | 0.335 | 0.005 | 0.328 | 0.326 | 158.7 | 1.00× |
| tebako-ruby | 5 | 0.361 | 0.358 | 0.363 | 0.002 | 0.361 | 0.360 | 158.7 | 0.91× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.362 | 0.358 | 0.365 | 0.005 | 0.362 | 0.361 | 158.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.098 | 0.098 | 0.104 | 0.003 | 0.099 | 0.098 | 158.7 | 1.00× |
| tebako-ruby | 5 | 0.173 | 0.170 | 0.179 | 0.003 | 0.174 | 0.173 | 204.1 | 0.57× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.173 | 0.172 | 0.174 | 0.002 | 0.173 | 0.172 | 204.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.083 | 0.081 | 0.086 | 0.002 | 0.083 | 0.083 | 158.7 | 1.00× |
| tebako-ruby | 5 | 0.154 | 0.150 | 0.156 | 0.003 | 0.153 | 0.152 | 158.7 | 0.54× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.150 | 0.149 | 0.151 | 0.001 | 0.150 | 0.149 | 158.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.110 | 0.110 | 0.114 | 0.002 | 0.111 | 0.109 | 158.7 | 1.00× |
| tebako-ruby | 5 | 0.156 | 0.153 | 0.162 | 0.003 | 0.156 | 0.154 | 158.7 | 0.71× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.159 | 0.158 | 0.160 | 0.002 | 0.159 | 0.158 | 158.7 | — |

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

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.3 GiB
Versions: tebako 2.8.9

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.050 | 0.050 | 0.000 | 0.050 | 0.048 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.114 | 0.109 | 0.116 | 0.002 | 0.113 | 0.112 | 105.7 | 0.44× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.115 | 0.114 | 0.116 | 0.001 | 0.115 | 0.112 | 105.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.245 | 0.243 | 0.247 | 0.002 | 0.245 | 0.244 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.319 | 0.317 | 0.321 | 0.002 | 0.319 | 0.317 | 105.7 | 0.77× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.325 | 0.323 | 0.328 | 0.003 | 0.325 | 0.325 | 105.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.062 | 0.060 | 0.066 | 0.002 | 0.063 | 0.061 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.132 | 0.130 | 0.134 | 0.001 | 0.132 | 0.131 | 176.3 | 0.47× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.134 | 0.134 | 0.134 | 0.000 | 0.134 | 0.133 | 176.4 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.054 | 0.052 | 0.056 | 0.002 | 0.054 | 0.054 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.124 | 0.120 | 0.124 | 0.002 | 0.123 | 0.122 | 105.7 | 0.44× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.123 | 0.122 | 0.124 | 0.001 | 0.123 | 0.121 | 105.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.085 | 0.081 | 0.085 | 0.002 | 0.084 | 0.083 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.130 | 0.128 | 0.132 | 0.001 | 0.130 | 0.128 | 105.7 | 0.65× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.129 | 0.128 | 0.130 | 0.001 | 0.129 | 0.129 | 105.7 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
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
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.9

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.042 | 0.041 | 0.042 | 0.000 | 0.042 | 0.040 | 114.4 | 1.00× |
| tebako-ruby | 5 | 0.079 | 0.079 | 0.081 | 0.001 | 0.079 | 0.078 | 114.4 | 0.53× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.080 | 0.079 | 0.081 | 0.001 | 0.080 | 0.079 | 114.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.169 | 0.169 | 0.173 | 0.002 | 0.171 | 0.169 | 114.4 | 1.00× |
| tebako-ruby | 5 | 0.184 | 0.181 | 0.190 | 0.003 | 0.184 | 0.182 | 114.4 | 0.92× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.183 | 0.182 | 0.184 | 0.001 | 0.183 | 0.181 | 114.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.056 | 0.056 | 0.058 | 0.001 | 0.056 | 0.055 | 114.4 | 1.00× |
| tebako-ruby | 5 | 0.100 | 0.099 | 0.101 | 0.001 | 0.100 | 0.099 | 202.6 | 0.56× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.128 | 0.101 | 0.154 | 0.037 | 0.128 | 0.126 | 201.7 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.050 | 0.048 | 0.050 | 0.001 | 0.049 | 0.048 | 114.4 | 1.00× |
| tebako-ruby | 5 | 0.087 | 0.087 | 0.087 | 0.000 | 0.087 | 0.085 | 114.4 | 0.57× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.088 | 0.087 | 0.089 | 0.001 | 0.088 | 0.087 | 114.4 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.064 | 0.062 | 0.064 | 0.001 | 0.064 | 0.062 | 114.4 | 1.00× |
| tebako-ruby | 5 | 0.091 | 0.089 | 0.093 | 0.001 | 0.091 | 0.090 | 114.4 | 0.71× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.092 | 0.091 | 0.093 | 0.001 | 0.092 | 0.091 | 114.4 | — |

Unavailable arms:

- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
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
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.9

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.056 | 0.036 | 0.112 | 0.035 | 0.071 | 0.036 | 35.8 | 1.00× |
| tebako-java | 5 | 0.605 | 0.487 | 0.698 | 0.075 | 0.599 | 0.292 | 42.0 | 0.09× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.517 | 0.488 | 0.547 | 0.042 | 0.517 | 0.296 | 42.1 | — |

### java-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.036 | 0.033 | 0.053 | 0.008 | 0.040 | 0.034 | 36.9 | 1.00× |
| tebako-java | 5 | 0.557 | 0.521 | 0.801 | 0.113 | 0.602 | 0.295 | 42.8 | 0.06× |

### java-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.452 | 0.428 | 0.475 | 0.033 | 0.452 | 0.253 | 42.8 | — |

### java-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.071 | 0.048 | 0.077 | 0.011 | 0.066 | 0.045 | 40.1 | 1.00× |
| tebako-java | 5 | 0.569 | 0.508 | 0.641 | 0.049 | 0.574 | 0.285 | 110.0 | 0.13× |

### java-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.531 | 0.522 | 0.541 | 0.013 | 0.531 | 0.291 | 110.0 | — |

### java-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.068 | 0.064 | 0.091 | 0.011 | 0.074 | 0.068 | 40.5 | 1.00× |
| tebako-java | 5 | 0.580 | 0.469 | 0.902 | 0.164 | 0.636 | 0.318 | 46.2 | 0.12× |

### java-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.546 | 0.515 | 0.577 | 0.044 | 0.546 | 0.306 | 45.8 | — |

### java-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.204 | 0.154 | 0.247 | 0.042 | 0.197 | 0.254 | 76.4 | 1.00× |
| tebako-java | 5 | 0.665 | 0.622 | 0.790 | 0.066 | 0.687 | 0.507 | 81.9 | 0.31× |

### java-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.607 | 0.594 | 0.621 | 0.019 | 0.607 | 0.502 | 81.6 | — |

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.018 | 0.016 | 0.019 | 0.001 | 0.018 | 0.014 | 11.5 | 1.00× |
| tebako-python | 5 | 0.267 | 0.227 | 0.300 | 0.030 | 0.262 | 0.238 | 128.3 | 0.07× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.268 | 0.263 | 0.272 | 0.007 | 0.268 | 0.244 | 128.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.224 | 0.207 | 0.229 | 0.009 | 0.220 | 0.214 | 11.7 | 1.00× |
| tebako-python | 5 | 0.466 | 0.434 | 0.495 | 0.023 | 0.467 | 0.448 | 128.3 | 0.48× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.469 | 0.428 | 0.510 | 0.058 | 0.469 | 0.459 | 128.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.035 | 0.028 | 0.045 | 0.007 | 0.036 | 0.020 | 13.6 | 1.00× |
| tebako-python | 5 | 0.255 | 0.244 | 0.272 | 0.011 | 0.255 | 0.233 | 196.5 | 0.14× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.238 | 0.233 | 0.242 | 0.006 | 0.238 | 0.220 | 196.6 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.069 | 0.051 | 0.107 | 0.021 | 0.074 | 0.056 | 19.9 | 1.00× |
| tebako-python | 5 | 0.421 | 0.395 | 0.493 | 0.044 | 0.441 | 0.413 | 139.3 | 0.16× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.352 | 0.341 | 0.364 | 0.016 | 0.352 | 0.335 | 137.2 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.040 | 0.036 | 0.045 | 0.003 | 0.040 | 0.026 | 18.2 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.039 | 0.036 | 0.059 | 0.011 | 0.045 | 0.034 | 12.8 | 1.00× |
| tebako-ruby | 5 | 0.096 | 0.094 | 0.114 | 0.008 | 0.101 | 0.084 | 58.6 | 0.40× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.100 | 0.097 | 0.103 | 0.004 | 0.100 | 0.080 | 59.2 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.260 | 0.240 | 0.275 | 0.016 | 0.257 | 0.243 | 12.7 | 1.00× |
| tebako-ruby | 5 | 0.292 | 0.280 | 0.327 | 0.019 | 0.298 | 0.272 | 59.3 | 0.89× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.320 | 0.316 | 0.324 | 0.006 | 0.320 | 0.299 | 59.0 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.081 | 0.060 | 0.091 | 0.011 | 0.078 | 0.053 | 47.5 | 1.00× |
| tebako-ruby | 5 | 0.120 | 0.119 | 0.135 | 0.007 | 0.123 | 0.111 | 157.8 | 0.67× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.134 | 0.132 | 0.135 | 0.002 | 0.134 | 0.112 | 157.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.070 | 0.058 | 0.079 | 0.009 | 0.070 | 0.055 | 13.7 | 1.00× |
| tebako-ruby | 5 | 0.116 | 0.098 | 0.130 | 0.012 | 0.114 | 0.097 | 58.7 | 0.60× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.111 | 0.106 | 0.115 | 0.006 | 0.111 | 0.099 | 59.9 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.085 | 0.080 | 0.098 | 0.008 | 0.088 | 0.063 | 19.3 | 1.00× |
| tebako-ruby | 5 | 0.119 | 0.109 | 0.141 | 0.014 | 0.123 | 0.107 | 63.5 | 0.71× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.112 | 0.109 | 0.115 | 0.005 | 0.112 | 0.090 | 63.4 | — |

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
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.043 | 0.038 | 0.052 | 0.005 | 0.044 | 0.035 | 8.7 | 1.00× |
| tebako-python | 5 | 0.834 | 0.487 | 0.920 | 0.202 | 0.723 | 0.813 | 127.8 | 0.05× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.496 | 0.495 | 0.497 | 0.001 | 0.496 | 0.474 | 127.7 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.831 | 0.757 | 1.287 | 0.217 | 0.915 | 0.750 | 8.9 | 1.00× |
| tebako-python | 5 | 2.599 | 1.788 | 3.330 | 0.667 | 2.653 | 1.978 | 128.2 | 0.32× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.058 | 1.872 | 2.245 | 0.264 | 2.058 | 1.955 | 127.8 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.055 | 0.052 | 0.068 | 0.006 | 0.057 | 0.048 | 10.9 | 1.00× |
| tebako-python | 5 | 0.694 | 0.634 | 0.713 | 0.036 | 0.678 | 0.679 | 195.0 | 0.08× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.777 | 0.727 | 0.828 | 0.071 | 0.777 | 0.713 | 195.2 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.167 | 0.131 | 0.220 | 0.036 | 0.175 | 0.158 | 17.4 | 1.00× |
| tebako-python | 5 | 1.024 | 0.842 | 1.276 | 0.171 | 1.030 | 0.984 | 136.6 | 0.16× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.024 | 0.968 | 1.080 | 0.079 | 1.024 | 0.998 | 135.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.100 | 0.083 | 0.121 | 0.016 | 0.099 | 0.095 | 14.1 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.088 | 0.082 | 0.138 | 0.027 | 0.105 | 0.084 | 9.7 | 1.00× |
| tebako-ruby | 5 | 0.217 | 0.204 | 0.282 | 0.031 | 0.231 | 0.210 | 52.6 | 0.41× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.197 | 0.195 | 0.199 | 0.003 | 0.197 | 0.185 | 52.5 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.755 | 0.727 | 0.759 | 0.013 | 0.750 | 0.744 | 9.7 | 1.00× |
| tebako-ruby | 5 | 0.580 | 0.564 | 0.673 | 0.056 | 0.612 | 0.568 | 52.7 | 1.30× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.681 | 0.535 | 0.828 | 0.207 | 0.681 | 0.664 | 52.8 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.106 | 0.101 | 0.108 | 0.003 | 0.105 | 0.097 | 45.2 | 1.00× |
| tebako-ruby | 5 | 0.229 | 0.228 | 0.233 | 0.002 | 0.230 | 0.219 | 153.0 | 0.46× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.231 | 0.227 | 0.235 | 0.006 | 0.231 | 0.221 | 153.1 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.099 | 0.096 | 0.102 | 0.002 | 0.099 | 0.092 | 10.4 | 1.00× |
| tebako-ruby | 5 | 0.192 | 0.189 | 0.201 | 0.005 | 0.194 | 0.185 | 53.0 | 0.51× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.194 | 0.193 | 0.195 | 0.002 | 0.194 | 0.184 | 53.0 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.136 | 0.129 | 0.145 | 0.006 | 0.136 | 0.129 | 14.8 | 1.00× |
| tebako-ruby | 5 | 0.235 | 0.230 | 0.242 | 0.004 | 0.236 | 0.222 | 56.0 | 0.58× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.233 | 0.231 | 0.235 | 0.003 | 0.233 | 0.222 | 56.5 | — |

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
- java-boot / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256)
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-fib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256)
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-stdlib / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256)
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-ioread / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256)
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-statloop / on-system-java: unavailable — the paired arm 'tebako-java' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256)
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256

Failed runs:

- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.9

### java-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-java | 5 | 0.060 | 0.059 | 0.063 | 0.002 | 0.061 | 0.094 | 36.7 | 1.00× |
| tebako-java | 5 | 0.578 | 0.574 | 0.616 | 0.018 | 0.587 | 0.516 | 177.7 | 0.10× |

### java-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-java | 2 | 0.620 | 0.574 | 0.666 | 0.065 | 0.620 | 0.562 | 177.7 | — |

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-boot / tebako-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-fib / on-system-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-fib / tebako-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-stdlib / on-system-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-stdlib / tebako-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-ioread / on-system-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-ioread / tebako-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-statloop / on-system-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
- ruby-statloop / tebako-ruby: unavailable — the version probe failed: acquire: the version probe `\\?\D:\a\tebako\tebako\tebako-rs\out-runtime\targets\tebako-ruby\tebako-runtime-0.16.25-3.3.12-windows-ucrt64 -v` failed (exit -1073741511) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-probe-tebako-ruby.log
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

Failed runs:

- java-fib / on-system-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-fib / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-stdlib / on-system-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-stdlib / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-ioread / on-system-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-ioread / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-statloop / on-system-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-statloop / tebako-java [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.3 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

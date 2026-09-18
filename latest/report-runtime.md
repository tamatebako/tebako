# tebako benchmark report — runtime-on-system-vs-tebako

## linux-gnu-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.013 | 0.011 | 0.013 | 0.001 | 0.012 | 0.011 | 153.4 | 1.00× |
| tebako-python | 5 | 0.315 | 0.307 | 0.317 | 0.005 | 0.312 | 0.313 | 181.6 | 0.04× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.309 | 0.309 | 0.309 | 0.000 | 0.309 | 0.307 | 181.6 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.272 | 0.270 | 0.278 | 0.003 | 0.272 | 0.270 | 153.4 | 1.00× |
| tebako-python | 5 | 0.576 | 0.570 | 0.581 | 0.005 | 0.575 | 0.576 | 181.6 | 0.47× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.580 | 0.578 | 0.583 | 0.003 | 0.580 | 0.579 | 181.7 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.017 | 0.017 | 0.019 | 0.001 | 0.017 | 0.017 | 153.4 | 1.00× |
| tebako-python | 5 | 0.325 | 0.323 | 0.329 | 0.002 | 0.326 | 0.324 | 184.4 | 0.05× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.327 | 0.325 | 0.330 | 0.003 | 0.327 | 0.327 | 184.4 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.052 | 0.050 | 0.052 | 0.001 | 0.051 | 0.049 | 153.4 | 1.00× |
| tebako-python | 5 | 0.399 | 0.397 | 0.410 | 0.006 | 0.402 | 0.398 | 181.7 | 0.13× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.408 | 0.408 | 0.408 | 0.000 | 0.408 | 0.407 | 181.7 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.029 | 0.029 | 0.029 | 0.000 | 0.029 | 0.028 | 153.4 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.048 | 0.048 | 0.050 | 0.001 | 0.048 | 0.047 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.120 | 0.120 | 0.122 | 0.001 | 0.121 | 0.120 | 153.4 | 0.40× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.121 | 0.120 | 0.122 | 0.001 | 0.121 | 0.120 | 153.4 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.245 | 0.243 | 0.251 | 0.003 | 0.246 | 0.245 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.326 | 0.323 | 0.330 | 0.003 | 0.326 | 0.325 | 153.4 | 0.75× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.323 | 0.323 | 0.323 | 0.000 | 0.323 | 0.323 | 153.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.064 | 0.064 | 0.066 | 0.001 | 0.065 | 0.064 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.142 | 0.142 | 0.144 | 0.001 | 0.143 | 0.141 | 175.9 | 0.45× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.141 | 0.140 | 0.142 | 0.001 | 0.141 | 0.141 | 175.8 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.054 | 0.054 | 0.056 | 0.001 | 0.054 | 0.053 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.134 | 0.132 | 0.134 | 0.001 | 0.133 | 0.133 | 153.4 | 0.40× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.134 | 0.134 | 0.134 | 0.000 | 0.134 | 0.134 | 153.4 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.083 | 0.083 | 0.083 | 0.000 | 0.083 | 0.081 | 153.4 | 1.00× |
| tebako-ruby | 5 | 0.136 | 0.134 | 0.136 | 0.001 | 0.135 | 0.134 | 153.4 | 0.61× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.134 | 0.134 | 0.134 | 0.000 | 0.134 | 0.134 | 153.4 | — |

Unavailable arms:

- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-gnu-arm64.sha256
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- ruby-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)

## linux-gnu-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.015 | 0.015 | 0.015 | 0.000 | 0.015 | 0.014 | 159.7 | 1.00× |
| tebako-python | 5 | 0.312 | 0.310 | 0.313 | 0.001 | 0.311 | 0.309 | 199.3 | 0.05× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.312 | 0.312 | 0.312 | 0.000 | 0.312 | 0.310 | 197.8 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.341 | 0.306 | 0.348 | 0.017 | 0.335 | 0.340 | 159.7 | 1.00× |
| tebako-python | 5 | 0.588 | 0.583 | 0.593 | 0.004 | 0.588 | 0.586 | 197.8 | 0.58× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.591 | 0.589 | 0.593 | 0.003 | 0.591 | 0.588 | 198.5 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.021 | 0.021 | 0.021 | 0.000 | 0.021 | 0.020 | 159.7 | 1.00× |
| tebako-python | 5 | 0.319 | 0.318 | 0.320 | 0.001 | 0.319 | 0.318 | 197.8 | 0.07× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.322 | 0.322 | 0.323 | 0.001 | 0.322 | 0.321 | 200.0 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.085 | 0.083 | 0.085 | 0.001 | 0.084 | 0.083 | 159.7 | 1.00× |
| tebako-python | 5 | 0.405 | 0.403 | 0.409 | 0.003 | 0.405 | 0.403 | 197.9 | 0.21× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.405 | 0.404 | 0.405 | 0.001 | 0.405 | 0.404 | 199.4 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.037 | 0.037 | 0.037 | 0.000 | 0.037 | 0.036 | 159.7 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.069 | 0.068 | 0.071 | 0.001 | 0.069 | 0.068 | 159.7 | 1.00× |
| tebako-ruby | 5 | 0.130 | 0.130 | 0.132 | 0.001 | 0.131 | 0.130 | 159.7 | 0.53× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.130 | 0.130 | 0.131 | 0.000 | 0.130 | 0.129 | 159.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.328 | 0.327 | 0.337 | 0.004 | 0.330 | 0.326 | 159.7 | 1.00× |
| tebako-ruby | 5 | 0.342 | 0.341 | 0.343 | 0.001 | 0.342 | 0.340 | 159.7 | 0.96× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.342 | 0.339 | 0.344 | 0.003 | 0.342 | 0.341 | 159.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.093 | 0.091 | 0.093 | 0.001 | 0.092 | 0.092 | 159.7 | 1.00× |
| tebako-ruby | 5 | 0.170 | 0.165 | 0.170 | 0.002 | 0.168 | 0.168 | 204.1 | 0.55× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.170 | 0.170 | 0.170 | 0.000 | 0.170 | 0.168 | 205.0 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.081 | 0.081 | 0.085 | 0.002 | 0.082 | 0.080 | 159.7 | 1.00× |
| tebako-ruby | 5 | 0.146 | 0.145 | 0.147 | 0.001 | 0.146 | 0.145 | 159.7 | 0.56× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.147 | 0.147 | 0.147 | 0.000 | 0.147 | 0.146 | 159.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.106 | 0.106 | 0.106 | 0.000 | 0.106 | 0.104 | 159.7 | 1.00× |
| tebako-ruby | 5 | 0.147 | 0.147 | 0.149 | 0.001 | 0.147 | 0.147 | 159.7 | 0.72× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.147 | 0.147 | 0.147 | 0.000 | 0.147 | 0.147 | 159.7 | — |

Unavailable arms:

- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- ruby-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)

## linux-musl-arm64

Runner: ubuntu-24.04-arm · aarch64 · 4 cpus · 15947.4 GiB
Versions: tebako 2.8.9

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.048 | 0.048 | 0.048 | 0.000 | 0.048 | 0.047 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.114 | 0.111 | 0.114 | 0.001 | 0.113 | 0.111 | 105.7 | 0.42× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.111 | 0.111 | 0.112 | 0.000 | 0.111 | 0.111 | 105.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.243 | 0.243 | 0.245 | 0.001 | 0.244 | 0.243 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.319 | 0.319 | 0.334 | 0.006 | 0.323 | 0.319 | 105.7 | 0.76× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.319 | 0.319 | 0.319 | 0.000 | 0.319 | 0.318 | 105.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.062 | 0.062 | 0.062 | 0.000 | 0.062 | 0.061 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.132 | 0.132 | 0.134 | 0.001 | 0.133 | 0.131 | 175.5 | 0.47× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.131 | 0.130 | 0.132 | 0.001 | 0.131 | 0.130 | 176.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.054 | 0.054 | 0.054 | 0.000 | 0.054 | 0.052 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.120 | 0.120 | 0.122 | 0.001 | 0.120 | 0.119 | 105.7 | 0.45× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.119 | 0.118 | 0.120 | 0.001 | 0.119 | 0.118 | 105.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.081 | 0.081 | 0.083 | 0.001 | 0.081 | 0.080 | 105.7 | 1.00× |
| tebako-ruby | 5 | 0.126 | 0.124 | 0.130 | 0.002 | 0.127 | 0.125 | 105.7 | 0.64× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.126 | 0.126 | 0.126 | 0.000 | 0.126 | 0.125 | 105.7 | — |

Unavailable arms:

- ruby-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- python-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- python-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- python-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- python-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-arm64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-arm64.sha256
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## linux-musl-x86_64

Runner: ubuntu-24.04 · x86_64 · 4 cpus · 15989.7 GiB
Versions: tebako 2.8.9

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.046 | 0.044 | 0.046 | 0.001 | 0.045 | 0.044 | 113.7 | 1.00× |
| tebako-ruby | 5 | 0.085 | 0.083 | 0.094 | 0.004 | 0.087 | 0.085 | 113.7 | 0.54× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.087 | 0.085 | 0.089 | 0.003 | 0.087 | 0.086 | 113.7 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.178 | 0.176 | 0.184 | 0.004 | 0.179 | 0.177 | 113.7 | 1.00× |
| tebako-ruby | 5 | 0.205 | 0.204 | 0.207 | 0.001 | 0.205 | 0.204 | 113.7 | 0.87× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.206 | 0.202 | 0.209 | 0.005 | 0.206 | 0.204 | 113.7 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.060 | 0.060 | 0.062 | 0.001 | 0.061 | 0.060 | 113.7 | 1.00× |
| tebako-ruby | 5 | 0.105 | 0.103 | 0.112 | 0.004 | 0.106 | 0.104 | 202.9 | 0.57× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.100 | 0.099 | 0.101 | 0.001 | 0.100 | 0.099 | 203.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.052 | 0.052 | 0.054 | 0.001 | 0.053 | 0.051 | 113.7 | 1.00× |
| tebako-ruby | 5 | 0.093 | 0.091 | 0.093 | 0.001 | 0.093 | 0.091 | 113.7 | 0.56× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.093 | 0.091 | 0.095 | 0.003 | 0.093 | 0.092 | 113.7 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.068 | 0.066 | 0.070 | 0.001 | 0.068 | 0.067 | 113.7 | 1.00× |
| tebako-ruby | 5 | 0.095 | 0.095 | 0.097 | 0.001 | 0.096 | 0.094 | 113.7 | 0.72× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.098 | 0.097 | 0.099 | 0.001 | 0.098 | 0.096 | 113.7 | — |

Unavailable arms:

- ruby-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- ruby-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- python-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- python-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- python-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- python-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- python-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-boot / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-fib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /home/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-python: unavailable — the version probe failed: acquire: the version probe `/home/runner/work/tebako/tebako/tebako-rs/out-runtime/targets/tebako-python/tebako-runtime-0.2.0-3.14.7-linux-musl-x86_64 --version` failed (exit 127) — see /home/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-probe-tebako-python.log
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-linux-musl-x86_64.sha256
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)

## macos-arm64

Runner: macos-14 · aarch64 · 3 cpus · 7168.0 GiB
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.019 | 0.016 | 0.024 | 0.003 | 0.019 | 0.012 | 11.4 | 1.00× |
| tebako-python | 5 | 0.247 | 0.213 | 0.278 | 0.026 | 0.240 | 0.238 | 129.9 | 0.08× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.238 | 0.236 | 0.240 | 0.003 | 0.238 | 0.232 | 130.1 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.218 | 0.216 | 0.246 | 0.012 | 0.223 | 0.207 | 11.4 | 1.00× |
| tebako-python | 5 | 0.434 | 0.418 | 0.484 | 0.026 | 0.440 | 0.415 | 129.8 | 0.50× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.454 | 0.453 | 0.455 | 0.002 | 0.454 | 0.433 | 130.1 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.043 | 0.033 | 0.045 | 0.005 | 0.040 | 0.018 | 13.6 | 1.00× |
| tebako-python | 5 | 0.242 | 0.238 | 0.288 | 0.021 | 0.251 | 0.226 | 198.3 | 0.18× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.245 | 0.228 | 0.262 | 0.023 | 0.245 | 0.229 | 197.8 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.072 | 0.058 | 0.086 | 0.012 | 0.070 | 0.058 | 20.0 | 1.00× |
| tebako-python | 5 | 0.372 | 0.345 | 0.442 | 0.039 | 0.385 | 0.357 | 139.3 | 0.19× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.387 | 0.364 | 0.410 | 0.033 | 0.387 | 0.372 | 138.9 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.038 | 0.037 | 0.040 | 0.001 | 0.038 | 0.025 | 17.9 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.053 | 0.036 | 0.061 | 0.012 | 0.050 | 0.034 | 12.7 | 1.00× |
| tebako-ruby | 5 | 0.099 | 0.089 | 0.103 | 0.007 | 0.096 | 0.077 | 60.8 | 0.54× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.105 | 0.102 | 0.108 | 0.004 | 0.105 | 0.084 | 60.9 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.240 | 0.224 | 0.247 | 0.009 | 0.236 | 0.222 | 12.6 | 1.00× |
| tebako-ruby | 5 | 0.284 | 0.274 | 0.292 | 0.008 | 0.282 | 0.264 | 60.8 | 0.84× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.289 | 0.282 | 0.296 | 0.010 | 0.289 | 0.265 | 60.4 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.067 | 0.058 | 0.081 | 0.008 | 0.068 | 0.042 | 48.1 | 1.00× |
| tebako-ruby | 5 | 0.100 | 0.092 | 0.116 | 0.009 | 0.102 | 0.081 | 159.5 | 0.67× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.096 | 0.093 | 0.098 | 0.004 | 0.096 | 0.083 | 157.5 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.057 | 0.049 | 0.065 | 0.006 | 0.058 | 0.041 | 13.9 | 1.00× |
| tebako-ruby | 5 | 0.096 | 0.092 | 0.101 | 0.003 | 0.096 | 0.075 | 60.6 | 0.60× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.096 | 0.095 | 0.096 | 0.001 | 0.096 | 0.075 | 61.6 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.066 | 0.062 | 0.080 | 0.008 | 0.068 | 0.053 | 19.3 | 1.00× |
| tebako-ruby | 5 | 0.100 | 0.095 | 0.120 | 0.010 | 0.103 | 0.086 | 64.3 | 0.66× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.105 | 0.103 | 0.106 | 0.002 | 0.105 | 0.084 | 64.5 | — |

Unavailable arms:

- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log)
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- ruby-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)

## macos-x86_64

Runner: macos-15-intel · x86_64 · 4 cpus · 14336.0 GiB
Versions: tebako 2.8.9

### python-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.056 | 0.038 | 0.058 | 0.009 | 0.050 | 0.040 | 8.7 | 1.00× |
| tebako-python | 5 | 0.655 | 0.617 | 0.980 | 0.149 | 0.718 | 0.627 | 127.9 | 0.09× |

### python-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 0.649 | 0.645 | 0.653 | 0.006 | 0.649 | 0.629 | 128.0 | — |

### python-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.853 | 0.757 | 1.051 | 0.121 | 0.895 | 0.815 | 8.9 | 1.00× |
| tebako-python | 5 | 2.460 | 2.070 | 2.596 | 0.217 | 2.395 | 2.403 | 127.9 | 0.35× |

### python-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 2.491 | 2.378 | 2.604 | 0.160 | 2.491 | 2.460 | 127.9 | — |

### python-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.085 | 0.066 | 0.103 | 0.017 | 0.085 | 0.076 | 10.9 | 1.00× |
| tebako-python | 5 | 0.991 | 0.792 | 1.471 | 0.253 | 1.045 | 0.939 | 195.6 | 0.09× |

### python-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.097 | 1.062 | 1.131 | 0.049 | 1.097 | 1.058 | 195.1 | — |

### python-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.247 | 0.233 | 0.399 | 0.070 | 0.282 | 0.235 | 17.4 | 1.00× |
| tebako-python | 5 | 1.651 | 1.166 | 2.018 | 0.366 | 1.587 | 1.625 | 136.1 | 0.15× |

### python-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-python | 2 | 1.656 | 1.630 | 1.683 | 0.038 | 1.656 | 1.461 | 136.8 | — |

### python-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-python | 5 | 0.100 | 0.098 | 0.109 | 0.004 | 0.102 | 0.093 | 14.0 | 1.00× |

### ruby-boot — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.143 | 0.112 | 0.172 | 0.021 | 0.143 | 0.130 | 9.7 | 1.00× |
| tebako-ruby | 5 | 0.347 | 0.281 | 0.518 | 0.099 | 0.368 | 0.334 | 52.4 | 0.41× |

### ruby-boot — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.306 | 0.291 | 0.320 | 0.020 | 0.306 | 0.295 | 52.6 | — |

### ruby-fib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 1.307 | 1.165 | 1.569 | 0.150 | 1.329 | 1.290 | 10.2 | 1.00× |
| tebako-ruby | 5 | 0.934 | 0.915 | 1.133 | 0.113 | 1.009 | 0.927 | 53.0 | 1.40× |

### ruby-fib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 1.134 | 0.970 | 1.298 | 0.232 | 1.134 | 1.021 | 53.0 | — |

### ruby-ioread — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.144 | 0.135 | 0.160 | 0.009 | 0.146 | 0.137 | 45.8 | 1.00× |
| tebako-ruby | 5 | 0.327 | 0.308 | 0.352 | 0.018 | 0.332 | 0.313 | 153.2 | 0.44× |

### ruby-ioread — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.342 | 0.341 | 0.343 | 0.002 | 0.342 | 0.333 | 153.2 | — |

### ruby-statloop — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.219 | 0.183 | 0.333 | 0.061 | 0.238 | 0.202 | 11.0 | 1.00× |
| tebako-ruby | 5 | 0.472 | 0.329 | 0.646 | 0.131 | 0.456 | 0.449 | 53.5 | 0.46× |

### ruby-statloop — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.319 | 0.318 | 0.319 | 0.001 | 0.319 | 0.301 | 53.2 | — |

### ruby-stdlib — warm

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| on-system-ruby | 5 | 0.231 | 0.173 | 0.256 | 0.038 | 0.216 | 0.221 | 15.0 | 1.00× |
| tebako-ruby | 5 | 0.321 | 0.299 | 0.344 | 0.019 | 0.325 | 0.310 | 56.2 | 0.72× |

### ruby-stdlib — cold (install/first-boot)

| target | n | median s | min s | max s | stdev s | mean s | cpu median s | peak RSS MiB | vs on-system |
|--------|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| tebako-ruby | 2 | 0.283 | 0.282 | 0.284 | 0.001 | 0.283 | 0.270 | 56.3 | — |

Unavailable arms:

- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- ruby-boot / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- ruby-fib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- ruby-stdlib / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- ruby-ioread / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- ruby-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- ruby-statloop / on-system-ruby [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- python-boot / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- python-fib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- python-stdlib / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- python-ioread / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- python-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- python-statloop / on-system-python [cold]: unavailable — on-system arms have no cold story — the toolchain's provisioning time is not tebako's comparison
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-boot / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-fib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-fib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-fib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-fib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-stdlib / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-stdlib / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-stdlib / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-stdlib' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-ioread / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-ioread / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-ioread / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-ioread' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/sources/_classes ./benchmarks/fixtures/java/Collections.java ./benchmarks/fixtures/java/Fib.java ./benchmarks/fixtures/java/IoRead.java ./benchmarks/fixtures/java/TreeWalk.java` failed (exit 2) — see /Users/runner/work/tebako/tebako/tebako-rs/out-runtime/logs/acquire-javac.log
- java-statloop / tebako-java: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256: not found: https://github.com/tamatebako/tebako-runtime-openjdk/releases/download/v2.7.0/tebako-runtime-2.7.0-21.0.12-macos-x86_64.sha256
- java-statloop / on-system-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / on-system-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-ruby: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)
- java-statloop / tebako-python: unavailable — the staged program could not start on this triplet: engine: workload 'java-statloop' uses {classes} but the leg compiled no classes (harness bug)

Failed runs:

- ruby-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-fib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-stdlib / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-ioread / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- ruby-statloop / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- python-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-fib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-stdlib / tebako-python [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-ioread / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- python-statloop / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / on-system-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)
- java-boot / tebako-ruby [warm] #0 (exit 1): failed — warmup: exit 1 (expected 0)
- java-boot / tebako-python [warm] #0 (exit 2): failed — warmup: exit 2 (expected 0)

## windows-ucrt64

Runner: windows-latest · x86_64 · 4 cpus · 16378.7 GiB
Versions: tebako 2.8.9

Unavailable arms:

- ruby-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- ruby-boot / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- ruby-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- ruby-boot / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- ruby-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- ruby-fib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- ruby-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- ruby-fib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- ruby-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- ruby-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- ruby-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- ruby-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- ruby-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- ruby-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- ruby-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- ruby-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- ruby-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- ruby-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- ruby-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- ruby-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- ruby-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- python-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- python-boot / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- python-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- python-boot / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- python-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- python-fib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- python-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- python-fib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- python-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- python-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- python-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- python-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- python-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- python-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- python-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- python-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- python-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- python-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- python-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- python-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- python-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- java-boot / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- java-boot / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-boot / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- java-boot / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- java-boot / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-boot / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- java-fib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- java-fib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-fib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- java-fib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- java-fib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-fib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- java-stdlib / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- java-stdlib / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-stdlib / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- java-stdlib / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- java-stdlib / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-stdlib / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- java-ioread / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- java-ioread / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-ioread / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- java-ioread / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- java-ioread / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-ioread / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)
- java-statloop / on-system-ruby: unavailable — the paired arm 'tebako-ruby' is unavailable — the on-system vs tebako comparison needs both arms (runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256)
- java-statloop / on-system-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-statloop / on-system-java: unavailable — the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log
- java-statloop / tebako-ruby: unavailable — runtime acquisition failed: acquire: download failed for https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256: not found: https://github.com/tamatebako/tebako-runtime-ruby/releases/download/v0.16.25/tebako-runtime-0.16.25-3.3.12-windows-ucrt64.exe.sha256
- java-statloop / tebako-python: unavailable — platforms.yaml declares this gap: the python runtime's windows tier has not shipped
- java-statloop / tebako-java: unavailable — the paired arm 'on-system-java' is unavailable — the on-system vs tebako comparison needs both arms (the in-leg javac compile failed: acquire: `javac -d \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\sources\_classes .\benchmarks/fixtures/java\Collections.java .\benchmarks/fixtures/java\Fib.java .\benchmarks/fixtures/java\IoRead.java .\benchmarks/fixtures/java\TreeWalk.java` failed (exit 2) — see \\?\D:\a\tebako\tebako\tebako-rs\out-runtime\logs\acquire-javac.log)

---

Runner metadata: linux-gnu-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-gnu-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; linux-musl-arm64 = ubuntu-24.04-arm / aarch64 / 4 cpus / 15947.4 GiB; linux-musl-x86_64 = ubuntu-24.04 / x86_64 / 4 cpus / 15989.7 GiB; macos-arm64 = macos-14 / aarch64 / 3 cpus / 7168.0 GiB; macos-x86_64 = macos-15-intel / x86_64 / 4 cpus / 14336.0 GiB; windows-ucrt64 = windows-latest / x86_64 / 4 cpus / 16378.7 GiB;

GitHub-hosted runners are shared, multi-tenant machines; treat differences under ~10% as noise and read min alongside median — min is the cross-noise-comparable figure (noise inflates, never deflates).

Version skew: the old world is frozen at the packed-mn tag's metanorma-cli while the v2 payload is current — compare ratios, not absolutes. Numbers across image formats are never mixed.

Peak RSS is file-backed-inclusive: mmap'd image pages are reclaimable under memory pressure, so a tebako arm's RSS delta overstates its real memory cost.

Method notes:

- Cold method: each cold iteration wipes the arm's caches before the measured run. For the tebako runtime arms the wipe covers the bench store and the per-target tempdir, so the measured run pays the driver's first mount and cache rebuild — the one-time, user-visible first-boot cost. The runtime pair's own download and verification happen at acquisition, outside the measured span.
- Teardown is not a separate workload: a no-op run's wall clock is boot+teardown by construction, and teardown is ≈free by design — the in-process VFS dies with the process; a measured exit that ever shows a tail is a regression signal.
- On-system arms report no cold numbers: the toolchain's provisioning time is not tebako's comparison.

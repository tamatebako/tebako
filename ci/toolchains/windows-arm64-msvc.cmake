# MSVC ARM64 cross toolchain for rnp-src's cmake deps (json-c, zlib,
# librnp), consumed via RNP_CMAKE_TOOLCHAIN by ci/windows-arm64-msvc-build.sh.
# The x64_arm64 vcvars env's cl targets arm64 on its own; Ninja (the
# leg's CMAKE_GENERATOR) drives the build. Declaring SYSTEM_NAME and
# SYSTEM_PROCESSOR marks the configure as cross, so cmake keeps
# host-only assumptions (try_run, host PATH suffixes) out of the dep
# builds. Compiler selection here is what suppresses rnp-src's gcc/g++
# default injection in build_librnp.
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR ARM64)
set(CMAKE_C_COMPILER cl)
set(CMAKE_CXX_COMPILER cl)

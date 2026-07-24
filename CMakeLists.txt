# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
# 1. Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
# 2. Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
# TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
# BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
# CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
# SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
# INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
# CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
# ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
# POSSIBILITY OF SUCH DAMAGE.

cmake_minimum_required (VERSION 3.20)

# For project( ... VERSION ... )
cmake_policy(SET CMP0048 NEW)

if ("-${TEBAKO_VERSION}" STREQUAL "-")
  message(FATAL_ERROR "Tebako version is not specified.")
endif()

project(tebako_packager VERSION ${TEBAKO_VERSION})

include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/def-external-project.cmake)

# ...................................................................
# Tebako packaging environment provisioning ('tebako setup')
#
# The gem presses packages against prebuilt tebako runtime packages
# (produced by tebako-runtime-ruby and resolved by Tebako::RuntimeManager);
# no Ruby is built here. The only artifact this project still provisions is
# the prebuilt libtfs package set -- mkdwarfs, which the press pipeline runs
# to build application images, and the libtfs static libraries/headers the
# runtime link consumes.
#
# SETUP_MODE is accepted for command-line compatibility with previous
# releases; provisioning is the only mode this project has.

option(SETUP_MODE "Tebako setup" ON)

# This is the default that will be overwritten for MacOS and MSys
set(GNU_BASH "bash")

execute_process(
  COMMAND "${GNU_BASH}"
          "-c"
          "echo \$OSTYPE"
  RESULT_VARIABLE OSTYPE_RES
  OUTPUT_VARIABLE OSTYPE_TXT
  OUTPUT_STRIP_TRAILING_WHITESPACE
)
if(OSTYPE_RES EQUAL 0)
  message(STATUS "OSTYPE: '${OSTYPE_TXT}'")
else(OSTYPE_RES EQUAL 0)
  message(FATAL_ERROR "Failed to detect OSTYPE: ${OSTYPE_TXT}")
endif(OSTYPE_RES EQUAL 0)

set(IS_GNU OFF)
set(IS_MUSL OFF)
set(IS_MSYS OFF)
set(IS_DARWIN OFF)

if("${OSTYPE_TXT}" MATCHES "^linux-gnu.*")
  set(IS_GNU ON)
elseif("${OSTYPE_TXT}" MATCHES "^linux-musl.*")
  set(IS_MUSL ON)
elseif("${OSTYPE_TXT}" MATCHES "^msys*" OR "${OSTYPE_TXT}" MATCHES "^cygwin*")
  set(IS_MSYS ON)
elseif("${OSTYPE_TXT}" MATCHES "^darwin.*")
  set(IS_DARWIN ON)
endif()

# Various locations for external projects
set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps CACHE STRING "Dependencies' folder'")
set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_SRC_DIR ${DEPS}/src)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

# ...................................................................
# libtfs (former libdwarfs-wr): DWARFS_WR_TAG selects the libtfs GitHub
# release the prebuilt package is deployed from
# NOTE: the env override in def_ext_prj_g (tebako-tools) is currently broken
# (`if (DEFINED ENV{${TAG})` is missing a closing brace, so the test never
# fires); this default is the operative tag selection.
def_ext_prj_g(DWARFS_WR "v0.12.10")

# Provisions the prebuilt libtfs package, the prebuilt mkdwarfs, and the
# transitive static dependencies the libtfs package does not ship.
include(${CMAKE_SOURCE_DIR}/cmake/tebako-libtfs.cmake)

message("Configuration summary:")
message(STATUS "libtfs: v${LIBTFS_VERSION} prebuilt package (${LIBTFS_PLATFORM}) at ${DEPS_LIB_DIR}")
message(STATUS "mkdwarfs: ${LIBTFS_MKDWARFS}")

# Everything (download, SHA256 verification, deployment to ${DEPS}) was done
# at configure time by cmake/tebako-libtfs.cmake; the targets only keep the
# 'setup' build entry point intact.
add_custom_target(${DWARFS_WR_PRJ}
  COMMAND ${CMAKE_COMMAND} -E echo "libtfs v${LIBTFS_VERSION} prebuilt package [${LIBTFS_PLATFORM}] deployed to ${DEPS}"
)

add_custom_target(setup
  ${CMAKE_COMMAND} -E echo "Tebako setup has completed"
  DEPENDS ${DWARFS_WR_PRJ}
)

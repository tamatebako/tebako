# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of tebako
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
# Version 3.20 for cmake_path

# For project( ... VERSION ... )
cmake_policy(SET CMP0048 NEW)
# DOWNLOAD_EXTRACT_TIMESTAMP option default = true
if(${CMAKE_VERSION} VERSION_GREATER "3.23.10")
  cmake_policy(SET CMP0135 NEW)
endif()

if ("-${TEBAKO_VERSION}" STREQUAL "-")
  message(FATAL_ERROR "Tebako version is not specified.")
endif()

project(tebako_packager VERSION ${TEBAKO_VERSION})

include(ExternalProject)
# This is the default that will be overwritten for MacOS and MSys
set(GNU_BASH "bash")

include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/macos-environment.cmake)
include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/msys-environment.cmake)
include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/def-external-project.cmake)

option(SETUP_MODE "Tebako setup" OFF)
if(NOT LOG_LEVEL)
  set(LOG_LEVEL "error")
endif()

if (NOT ${SETUP_MODE})
  message(STATUS "Running tebako press script")

  if ("-${PCKG}" STREQUAL "-")
    message(FATAL_ERROR "Project OUTPUT PACKAGE is not specified.")
  endif()

  cmake_path(IS_RELATIVE PCKG IS_PK_RELATIVE)
  if(${IS_PK_RELATIVE})
    message(FATAL_ERROR "Path to output package shall be absolute. Relative path '${PCKG}' is not allowed.")
  else()
    cmake_path(SET APP_NAME NORMALIZE ${PCKG})
  endif()

  message("Running tebako press script")
else()
  message("Running tebako setup script")
endif()

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
set(RB_W32 OFF)
set(RUBY_WITHOUT_EXT "dbm,win32,win32ole,-test-/*")
set(DWARFS_PRELOAD OFF)
set(WITH_PATCHELF OFF)

if("${OSTYPE_TXT}" MATCHES "^linux-gnu.*")
  set(IS_GNU ON)
  if(REMOVE_GLIBC_PRIVATE)
    set(WITH_PATCHELF ON)
  endif(REMOVE_GLIBC_PRIVATE)
elseif("${OSTYPE_TXT}" MATCHES "^linux-musl.*")
  set(IS_MUSL ON)
elseif("${OSTYPE_TXT}" MATCHES "^msys*")
  set(IS_MSYS ON)
  # set(DWARFS_PRELOAD ON)
  set(RB_W32 ON)
  set(RUBY_WITHOUT_EXT "dbm,syslog,pty,gdbm,readline,-test-/*")
elseif("${OSTYPE_TXT}" MATCHES "^darwin.*")
  set(IS_DARWIN ON)
  if(${RUBY_VER} VERSION_LESS "3.1.0")
    set(OPENSSL_VER "1.1")
  else(${RUBY_VER} VERSION_LESS "3.1.0")
    set(OPENSSL_VER "3")
  endif(${RUBY_VER} VERSION_LESS "3.1.0")
  set(BUILD_OPENSSL_ROOT_DIR "${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}")
endif()

if(IS_DARWIN)
  execute_process(
    COMMAND "sysctl" "-n" "hw.ncpu"
    RESULT_VARIABLE NCORES_RES
    OUTPUT_VARIABLE NCORES
    OUTPUT_STRIP_TRAILING_WHITESPACE
  )
else(IS_DARWIN)
  execute_process(
    COMMAND "nproc" "--all"
    RESULT_VARIABLE NCORES_RES
    OUTPUT_VARIABLE NCORES
    OUTPUT_STRIP_TRAILING_WHITESPACE
  )
endif(IS_DARWIN)

if(NCORES_RES EQUAL 0)
  message(STATUS "NCORES: ${NCORES}")
else(NCORES_RES EQUAL 0)
  set(NCORES 4)
  message(WARNING "Failed to detect NCORES, resetting to ${NCORES} (default)")
endif(NCORES_RES EQUAL 0)

# Various locations for external projects
set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps CACHE STRING "Dependencies' folder'")
set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_SRC_DIR ${DEPS}/src)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

set(EXE ${CMAKE_CURRENT_SOURCE_DIR}/exe)

# Project resources that are used during CMake configuration stage
set(DATA_RES_DIR  ${CMAKE_CURRENT_SOURCE_DIR}/resources)

# ...................................................................
# External projects

if ("-${RUBY_VER}" STREQUAL "-" OR "-${RUBY_HASH}" STREQUAL "-")
  message(FATAL_ERROR "Ruby version is not specified")
endif()

set(RUBY_PRJ _ruby_${RUBY_VER})
set(RUBY_SOURCE_DIR ${DEPS}/src/${RUBY_PRJ})
set(RUBY_BINARY_DIR ${DEPS}/src/${RUBY_PRJ})
set(RUBY_STASH_DIR ${DEPS}/stash_${RUBY_VER})

string(SUBSTRING ${RUBY_VER} 0 3 RUBY_VER_BASE)
string(CONCAT RUBY_API_VER ${RUBY_VER_BASE} ".0")

#if(DWARFS_PRELOAD)
#  def_ext_prj_t(LIBDWARFS_WR "0.5.8" "7bf8e5b4432f35b65f6034f614067c2018995c1bebaf935e8cdddc1a3e045c01")
#
#  string(REGEX REPLACE "^([0-9]+)\\.([0-9]+)\\.([0-9]+).*" "\\1;\\2;\\3" LIBDWARFS_WR_VER_COMPONENTS ${LIBDWARFS_WR_VER})
#
#  list(GET LIBDWARFS_WR_VER_COMPONENTS 0 LIBDWARFS_WR_VER_MAJOR)
#  list(GET LIBDWARFS_WR_VER_COMPONENTS 1 LIBDWARFS_WR_VER_MINOR)
#  list(GET LIBDWARFS_WR_VER_COMPONENTS 2 LIBDWARFS_WR_VER_PATCH)
#  set (LIBDWARFS_WR_VER_M ${LIBDWARFS_WR_VER_MAJOR}.${LIBDWARFS_WR_VER_MINOR}.${LIBDWARFS_WR_VER_PATCH})
#else(DWARFS_PRELOAD)
def_ext_prj_g(DWARFS_WR "v0.9.3")
#endif(DWARFS_PRELOAD)

def_ext_prj_g(PATCHELF "65e14792061c298f1d2bc44becd48a10cbf0bc81")

set(LIBYAML_RUBY_OPTION "")
if(${RUBY_VER} VERSION_LESS "3.2.0")
  set(LIBYAML_RUBY_OPTION "--enable-bundled-libyaml")
endif(${RUBY_VER} VERSION_LESS "3.2.0")

message("Configuration summary:")
message(STATUS "ruby: v${RUBY_VER} at ${RUBY_SOURCE_DIR}")

if(DWARFS_PRELOAD)
  message(STATUS "dwarfs with tebako wrapper: deploying v${LIBDWARFS_WR_VER} to ${LIBDWARFS_WR_SOURCE_DIR}")
else(DWARFS_PRELOAD)
  message(STATUS "dwarfs with tebako wrapper: @${DWARFS_WR_TAG} at ${DWARFS_WR_SOURCE_DIR}")
endif(DWARFS_PRELOAD)

# ...................................................................
# Filesystem locations

# DATA_SRC_DIR folder is used to collect all files that need to be packaged
set(DATA_SRC_DIR  ${CMAKE_CURRENT_BINARY_DIR}/s)
# DATA_PRE_DIR folder is used to build gems  that need to be packaged
set(DATA_PRE_DIR  ${CMAKE_CURRENT_BINARY_DIR}/r)
# DATA_BIN_DIR folder is used to create packaged filesystem
set(DATA_BIN_DIR  ${CMAKE_CURRENT_BINARY_DIR}/p)
# DATA_BIN_FILE is packaged filesystem itself
set(DATA_BIN_FILE ${DATA_BIN_DIR}/fs.bin)
# Target binary directory
set (TBD ${DATA_SRC_DIR}/bin)
# Target library directory
set (TLIBD ${DATA_SRC_DIR}/lib)
# Target 'local' directory
set (TLD ${DATA_SRC_DIR}/local)
# TGD folder is used to install gems
set (TGD ${DATA_SRC_DIR}/lib/ruby/gems/${RUBY_API_VER})
# This is actually a constant that shall match libdwarfs-wr TEBAKO_MOUNT_POINT at tebako-common.h
if(${IS_MSYS})
  set(FS_MOUNT_POINT "A:/__tebako_memfs__")
else(${IS_MSYS})
  set(FS_MOUNT_POINT "/__tebako_memfs__")
endif(${IS_MSYS})

message(STATUS "DATA_SRC_DIR: ${DATA_SRC_DIR}")
message(STATUS "DATA_PRE_DIR: ${DATA_PRE_DIR}")
message(STATUS "DATA_BIN_DIR: ${DATA_BIN_DIR}")
message(STATUS "DATA_BIN_FILE: ${DATA_BIN_FILE}")
message(STATUS "Target binary directory: ${TBD}")
message(STATUS "Target library directory: ${TLIBD}")
message(STATUS "Target local directory: ${TLD}")
message(STATUS "Target Gem directory: ${TGD}")
message(STATUS "FS_MOUNT_POINT: ${FS_MOUNT_POINT}")
message(STATUS "Building for Win32 Ruby (RB_W32): ${RB_W32}")
message(STATUS "Removing GLIBC_PRIVATE reference: ${WITH_PATCHELF}")

# ...................................................................
# Other options

message(STATUS "Not building Ruby extensions: ${RUBY_WITHOUT_EXT}")

# ...................................................................
# DwarFS with tebako wrapper

# ...................................................................
# The libraries that are build by DwarFS project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light
# These forward-declarations and BUILD_BYPRODICTS are required to support 'Ninja'
# Otherwise add_dependencies would be enough for 'Unix makefiles' generator

set(__LIBDWARFS_WR "${DEPS_LIB_DIR}/libdwarfs-wr.a")
set(__LIBDWARFS "${DEPS_LIB_DIR}/libdwarfs.a")
set(__LIBFOLLY "${DEPS_LIB_DIR}/libfolly.a")
set(__LIBFSST "${DEPS_LIB_DIR}/libfsst.a")
set(__LIBT_METADATA "${DEPS_LIB_DIR}/libmetadata_thrift.a")
set(__LIBT_LIGHT "${DEPS_LIB_DIR}/libthrift_light.a")
set(__LIBXXHASH "${DEPS_LIB_DIR}/libxxhash.a")
set(__LIBZSTD "${DEPS_LIB_DIR}/libzstd.a")
set(__LIBARCHIVE "${DEPS_LIB_DIR}/libarchive.a")

if(DWARFS_PRELOAD)
  ExternalProject_Add(${LIBDWARFS_WR_PRJ}
    PREFIX ${DEPS}
    URL https://github.com/tamatebako/libdwarfs/releases/download/v${LIBDWARFS_WR_VER}/libdwarfs-wr-${LIBDWARFS_WR_VER_M}-mingw-ucrt64.7z
    URL_HASH SHA256=${LIBDWARFS_WR_HASH}
    DOWNLOAD_NO_PROGRESS true
    SOURCE_DIR ${LIBDWARFS_WR_SOURCE_DIR}
    UPDATE_COMMAND ""
    CONFIGURE_COMMAND ""
    BUILD_COMMAND ""
    INSTALL_COMMAND ${CMAKE_COMMAND} -E make_directory ${DEPS}
            COMMAND ${CMAKE_COMMAND} -E copy_directory ${LIBDWARFS_WR_SOURCE_DIR} ${DEPS}
    TEST_COMMAND ""
  )
else(DWARFS_PRELOAD)
  ExternalProject_Add(${DWARFS_WR_PRJ}
    GIT_SHALLOW true
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/tamatebako/libdwarfs.git
    GIT_TAG ${DWARFS_WR_TAG}
    SOURCE_DIR ${DWARFS_WR_SOURCE_DIR}
    BINARY_DIR ${DWARFS_WR_BINARY_DIR}
    UPDATE_COMMAND ""
    BUILD_COMMAND ${CMAKE_COMMAND} --build ${DWARFS_WR_BINARY_DIR} --parallel ${NCORES}
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS}
               -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
               -DWITH_TESTS:BOOL=OFF
               -DWITH_ASAN:BOOL=OFF
               -DWITH_COVERAGE:BOOL=OFF
               -DTEBAKO_BUILD_SCOPE=MKD
               -DRB_W32=${RB_W32}
               -GNinja
    BUILD_BYPRODUCTS ${__LIBDWARFS_WR}
                     ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA}
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
  )
endif(DWARFS_PRELOAD)

if(IS_GNU)
  ExternalProject_Add(${PATCHELF_PRJ}
    GIT_SHALLOW true
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/chitao1234/patchelf.git
    GIT_TAG ${PATCHELF_WR_TAG}
    SOURCE_DIR ${PATCHELF_SOURCE_DIR}
    BINARY_DIR ${PATCHELF_BINARY_DIR}
    UPDATE_COMMAND ""
    CONFIGURE_COMMAND ${CMAKE_COMMAND} -E chdir ${PATCHELF_SOURCE_DIR} ./bootstrap.sh
              COMMAND ${PATCHELF_SOURCE_DIR}/configure
                  --srcdir=${PATCHELF_SOURCE_DIR}
                  --prefix=${DEPS}
    TEST_COMMAND ""
  )
endif(IS_GNU)

# ...................................................................
# Ruby

set(RUBY_L_FLAGS "-L${DEPS_LIB_DIR} -L${CMAKE_CURRENT_BINARY_DIR}")
set(RUBY_C_FLAGS "-fPIC -I${DEPS_INCLUDE_DIR} -I${CMAKE_CURRENT_SOURCE_DIR}/include")

# Shadow
# https://github.com/deivid-rodriguez/byebug/issues/825
# (it happens under some conditions though it is hard to explain when)
if (CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -fdeclspec")
endif()

set(C_FLAGS_DEST cflags)

if(IS_DARWIN)
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -I${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/include -I${BREW_PREFIX}/opt/zlib/include -I${BREW_PREFIX}/include")
  string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -L${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/lib -L${BREW_PREFIX}/opt/zlib/lib  -L${BREW_PREFIX}/lib")
  set(OPENSSL_RUBY_OPTION "--with-openssl-dir=${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}")
else(IS_DARWIN)
  if(IS_MUSL)
    string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DENABLE_PATH_CHECK=0")
  endif(IS_MUSL)

  if(RB_W32)
    string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DRB_W32=1")
    string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -l:libstdc++.a -L${TLIBD}")
    set(C_FLAGS_DEST cppflags)
  endif(RB_W32)

  # Ruby 2.7 configure script error
  if(${RUBY_VER} VERSION_LESS "3.0.0")
    set(C_FLAGS_DEST cppflags)
  endif()
endif(IS_DARWIN)

message(STATUS "Ruby build ${C_FLAGS_DEST}='${RUBY_C_FLAGS}'")
message(STATUS "Ruby build LDFLAGS='${RUBY_L_FLAGS}'")
message(STATUS "openssl Ruby option='${OPENSSL_RUBY_OPTION}'")
message(STATUS "libyaml Ruby option='${LIBYAML_RUBY_OPTION}'")

ExternalProject_Add(${RUBY_PRJ}
  PREFIX ${DEPS}
  URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
  URL_HASH SHA256=${RUBY_HASH}
  DOWNLOAD_NO_PROGRESS true
  SOURCE_DIR ${RUBY_SOURCE_DIR}
  BUILD_IN_SOURCE true
  CONFIGURE_COMMAND ""
  BUILD_COMMAND ruby ${EXE}/tebako-packager pass1 ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${FS_MOUNT_POINT} ${DATA_SRC_DIR} ${RUBY_VER}
  # Make it for MacOS otherwise LDFLAGS are invalid
  # COMMAND ${CMAKE_COMMAND} -E make_directory ${DEPS_LIB_DIR}
  COMMAND ${GNU_BASH} -c "${RUBY_SOURCE_DIR}/configure  ${OPENSSL_RUBY_OPTION} ${LIBYAML_RUBY_OPTION} \
                                                        --without-gmp                         \
                                                        --disable-dtrace                      \
                                                        --disable-debug-env                   \
                                                        --disable-shared                      \
                                                        --disable-install-doc                 \
                                                        --with-static-linked-ext              \
                                                        --with-out-ext=${RUBY_WITHOUT_EXT}    \
                                                        --prefix=${DATA_SRC_DIR}              \
                                                        ${C_FLAGS_DEST}=\"${RUBY_C_FLAGS}\"   \
                                                        LDFLAGS=\"${RUBY_L_FLAGS}\""
  COMMAND   ruby ${EXE}/tebako-packager pass2 ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${DEPS_LIB_DIR} ${DATA_SRC_DIR} ${RUBY_STASH_DIR} ${RUBY_VER}
  INSTALL_COMMAND  ""
)

add_dependencies(${RUBY_PRJ} ${DWARFS_WR_PRJ})

if (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has completed"
    DEPENDS ${DWARFS_WR_PRJ} ${RUBY_PRJ}
  )
  if(IS_GNU)
    add_dependencies(setup ${PATCHELF_PRJ})
  endif(IS_GNU)

  else (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has been verified"
    DEPENDS ${DWARFS_WR_PRJ} ${RUBY_PRJ}
  )
  if(IS_GNU)
    add_dependencies(setup ${PATCHELF_PRJ})
  endif(IS_GNU)



  set(CMAKE_CXX_STANDARD      20)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
  set(CMAKE_CXX_EXTENSIONS   OFF)

# ...................................................................
# Packaged filesystem

  add_custom_target(packaged_filesystem
    COMMAND ruby ${DEPS_BIN_DIR}/deploy.rb ${RUBY_VER}
    DEPENDS setup
    BYPRODUCTS ${DATA_BIN_FILE}
  )

  set(CMAKE_CXX_FLAGS "${RUBY_C_FLAGS}")

  add_library(tebako-fs STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-main.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/include/tebako/tebako-main.h
    ${DEPS_SRC_DIR}/tebako/tebako-fs.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/include/tebako/tebako-fs.h
    ${DEPS_INCLUDE_DIR}/tebako/tebako-version.h
  )

  if(${RUBY_VER} VERSION_LESS "3.3.0" AND "${OSTYPE_TXT}" MATCHES "^msys*")
    target_compile_definitions(tebako-fs PUBLIC RB_W32_PRE_33)
  endif(${RUBY_VER} VERSION_LESS "3.3.0" AND "${OSTYPE_TXT}" MATCHES "^msys*")

  add_dependencies(tebako-fs packaged_filesystem)

  add_custom_target(tebako COMMAND ruby ${EXE}/tebako-packager finalize ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${APP_NAME} ${RUBY_VER} ${DEPS_BIN_DIR}/patchelf ${WITH_PATCHELF})
  add_dependencies(tebako setup tebako-fs)

endif(${SETUP_MODE})

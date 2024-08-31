# Copyright (c) 2021-2024 [Ribose Inc](https://www.ribose.com).
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
  message("Running tebako press script")

  if ("-${ROOT}" STREQUAL "-")
    message(FATAL_ERROR "Project ROOT is not specified.")
  endif()

  if ("-${ENTRANCE}" STREQUAL "-")
    message(FATAL_ERROR "Project ENTRANCE is not specified.")
  endif()

  if ("-${PCKG}" STREQUAL "-")
    message(FATAL_ERROR "Project OUTPUT PACKAGE is not specified.")
  endif()

  cmake_path(SET FS_ENTRANCE NORMALIZE ${ENTRANCE})

  cmake_path(IS_RELATIVE ROOT IS_RT_RELATIVE)
  if(${IS_RT_RELATIVE})
    message(FATAL_ERROR "Path to root shall be absolute. Relative path '${FS_ROOT}' is not allowed.")
  else()
    cmake_path(SET FS_ROOT NORMALIZE ${ROOT})
  endif()

  cmake_path(IS_ABSOLUTE FS_ENTRANCE IS_EN_ABSOLUTE)
  if(${IS_EN_ABSOLUTE})
    cmake_path(IS_PREFIX FS_ROOT ${FS_ENTRANCE} IS_FS_EN_PREFIX)
    if(NOT ${IS_FS_EN_PREFIX})
      message(FATAL_ERROR "Entry point ${FS_ENTRANCE} is not in the project root ${FS_ROOT}")
    endif(NOT ${IS_FS_EN_PREFIX})
    set(FS_ENTR_TMP ${FS_ENTRANCE})
    cmake_path(RELATIVE_PATH FS_ENTRANCE BASE_DIRECTORY ${FS_ROOT})
    message("Absolute path to entry point '${FS_ENTR_TMP}' will be reduced '${FS_ENTRANCE}' relative to '${FS_ROOT}'")
  endif(${IS_EN_ABSOLUTE})

  cmake_path(IS_RELATIVE PCKG IS_PK_RELATIVE)
  if(${IS_PK_RELATIVE})
    message(FATAL_ERROR "Path to output package shall be absolute. Relative path '${PCKG}' is not allowed.")
  else()
    cmake_path(SET APP_NAME NORMALIZE ${PCKG})
  endif()

  message("Packaging ${FS_ROOT} folder with ${FS_ENTRANCE} entry point to ${APP_NAME}")
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

set(IS_MUSL OFF)
set(IS_MSYS OFF)
set(IS_DARWIN OFF)
set(RB_W32 OFF)
set(RUBY_WITHOUT_EXT "dbm,win32,win32ole,-test-/*")
set(RUBY_NAME "ruby")
set(EXE_SUFFIX "")
set(DWARFS_PRELOAD OFF)

if("${OSTYPE_TXT}" MATCHES "^linux-musl.*")
  set(IS_MUSL ON)
elseif("${OSTYPE_TXT}" MATCHES "^msys*")
  set(IS_MSYS ON)
  # set(DWARFS_PRELOAD ON)
  set(RB_W32 ON)
  set(RUBY_WITHOUT_EXT "dbm,syslog,pty,gdbm,readline,-test-/*")
  set(EXE_SUFFIX ".exe")
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
  set(RUBY_VER "3.1.4")
  set(RUBY_HASH "a3d55879a0dfab1d7141fdf10d22a07dbf8e5cdc4415da1bde06127d5cc3c7b6")
  message(STATUS "Using default Ruby version ${RUBY_VER}")
endif()

set(RUBY_NAME ruby)
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
def_ext_prj_g(DWARFS_WR "v0.5.9")
#endif(DWARFS_PRELOAD)

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

if(PACKAGE_NEEDS_CWD)
  set(NEEDS_CWD true)
  set(PACKAGE_CWD_FULL ${FS_MOUNT_POINT}/${PACKAGE_CWD})
else(PACKAGE_NEEDS_CWD)
  set(NEEDS_CWD false)
  set(PACKAGE_CWD "")
  set(PACKAGE_CWD_FULL "")
endif(PACKAGE_NEEDS_CWD)

message(STATUS "DATA_SRC_DIR: ${DATA_SRC_DIR}")
message(STATUS "DATA_PRE_DIR: ${DATA_PRE_DIR}")
message(STATUS "DATA_BIN_DIR: ${DATA_BIN_DIR}")
message(STATUS "DATA_BIN_FILE: ${DATA_BIN_FILE}")
message(STATUS "Target binary directory: ${TBD}")
message(STATUS "Target library directory: ${TLIBD}")
message(STATUS "Target local directory: ${TLD}")
message(STATUS "Target Gem directory:: ${TGD}")
message(STATUS "FS_MOUNT_POINT: ${FS_MOUNT_POINT}")
message(STATUS "Building for Win32 Ruby (RB_W32): ${RB_W32}")

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

# ...................................................................
# Ruby

set(RUBY_L_FLAGS "-L${DEPS_LIB_DIR} -L${CMAKE_CURRENT_BINARY_DIR}")
set(RUBY_C_FLAGS "-pthread -I${DEPS_INCLUDE_DIR} -I${CMAKE_CURRENT_SOURCE_DIR}/include")

# Shadow
# https://github.com/deivid-rodriguez/byebug/issues/825
# (it happens under some conditions though it is hard to explain when)
if (CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -fdeclspec")
endif()

set(C_FLAGS_DEST cppflags)

if(IS_DARWIN)
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -I${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/include -I${BREW_PREFIX}/opt/zlib/include -I${BREW_PREFIX}/include")
  string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -L${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/lib -L${BREW_PREFIX}/opt/zlib/lib  -L${BREW_PREFIX}/lib")
  set(OPENSSL_RUBY_OPTION "--with-openssl-dir=${BREW_PREFIX}/opt/openssl@${OPENSSL_VER}")
  set(C_FLAGS_DEST cflags)
else(IS_DARWIN)
  string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -static-libgcc")

  if(IS_MUSL)
    string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DENABLE_PATH_CHECK=0")
  endif(IS_MUSL)

  if(RB_W32)
    string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DRB_W32=1")
    string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -l:libstdc++.a -L${TLIBD}")
  endif(RB_W32)
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
  PATCH_COMMAND ruby ${EXE}/tebako-packager pass1 ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${FS_MOUNT_POINT} ${DATA_SRC_DIR} ${RUBY_VER}
  CONFIGURE_COMMAND ${GNU_BASH} -c "${RUBY_SOURCE_DIR}/configure  ${OPENSSL_RUBY_OPTION} ${LIBYAML_RUBY_OPTION} \
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
  COMMAND          ruby ${EXE}/tebako-packager pass1a ${RUBY_SOURCE_DIR}
  BUILD_COMMAND    make -j${NCORES}
  INSTALL_COMMAND  make install -j${NCORES}
  COMMAND          ${GNU_BASH} -c "ruby ${EXE}/tebako-packager pass2 ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${DEPS_LIB_DIR} ${DATA_SRC_DIR} ${RUBY_STASH_DIR} ${RUBY_VER}"
)

add_dependencies(${RUBY_PRJ} ${DWARFS_WR_PRJ} ${LIBDWARFS_WR_PRJ})

if (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has completed"
    DEPENDS ${DWARFS_WR_PRJ} ${RUBY_PRJ}
  )
else (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has been verified"
    DEPENDS ${DWARFS_WR_PRJ} ${RUBY_PRJ}
  )

  file(GLOB GEMSPECS LIST_DIRECTORIES false ${FS_ROOT}/*.gemspec)
  file(GLOB GEMFILES LIST_DIRECTORIES false ${FS_ROOT}/Gemfile)
  file(GLOB GEMS     LIST_DIRECTORIES false ${FS_ROOT}/*.gem)

  list(LENGTH GEMSPECS GSLENGTH)
  list(LENGTH GEMFILES GFLENGTH)
  list(LENGTH GEMS     GLENGTH)

  if(GSLENGTH GREATER 0)
    if(GSLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gemspecs detected in ${FS_ROOT}")
    else(GSLENGTH GREATER 1)
      if(GFLENGTH GREATER 0)
#  Found xxx.gemspec and Gemfile
        set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
      else(GFLENGTH GREATER 0)
#  Found xxx.gemspec but no Gemfile
        set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
      endif(GFLENGTH GREATER 0)
    endif(GSLENGTH GREATER 1)
  elseif(GFLENGTH GREATER 0)
#  Found Gemfile but no gemspec
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
  elseif(GLENGTH GREATER 0)
  # xxx.gem, no gemspec, no gemfile
    if(GLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gems detected in ${FS_ROOT}")
    else(GLENGTH GREATER 1)
      set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
    endif(GLENGTH GREATER 1)
  else()
# No gem, no gemspec, no gemfile
# Simple ruby script
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
  endif()

  add_custom_target(source_filesystem
      COMMAND ruby ${EXE}/tebako-packager deploy ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${RUBY_STASH_DIR}
                                                 ${DATA_SRC_DIR} ${DATA_PRE_DIR} ${DATA_BIN_DIR}
                                                 ${FS_ROOT} ${FS_ENTRANCE} ${FS_MOUNT_POINT} ${APP_NAME} ${RUBY_VER}
                                                 ${PACKAGE_CWD}
      DEPENDS ${RUBY_PRJ}
  )

# ...................................................................
# Packaged filesystem

  configure_file(
    ${DATA_RES_DIR}/tebako-version.h.in
    ${DEPS_INCLUDE_DIR}/tebako/tebako-version.h
    @ONLY
  )

  configure_file(
    ${DATA_RES_DIR}/tebako-fs.cpp.in
    ${DEPS_SRC_DIR}/tebako/tebako-fs.cpp
    @ONLY
  )

  add_custom_target(packaged_filesystem
    COMMAND ${GNU_BASH} -c "chmod +x ${DEPS_BIN_DIR}/mkdwarfs${EXE_SUFFIX}"
    # No progress below is critical since in reporting mode mkdwarfs tries to work directly with terminal
    # and it creates issues for stderr redirects (&2 > 1) used by Ninja and our test srcipts
    # It may be fixable bit is very difficult to reproduce in test environment
    COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR} --no-progress
    COMMAND ${CMAKE_COMMAND} -E touch ${DEPS_SRC_DIR}/tebako/tebako-fs.cpp
    DEPENDS setup source_filesystem
    VERBATIM
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

  add_custom_target(tebako COMMAND ruby ${EXE}/tebako-packager finalize ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${APP_NAME} ${RUBY_VER})
  add_dependencies(tebako setup tebako-fs)

endif(${SETUP_MODE})

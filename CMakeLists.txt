# Copyright (c) 2021-2023 [Ribose Inc](https://www.ribose.com).
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

include(${CMAKE_CURRENT_SOURCE_DIR}/tools/cmake-scripts/version.cmake)
determine_version("${CMAKE_CURRENT_SOURCE_DIR}" TEBAKO)

project(tebako_packager VERSION ${TEBAKO_VERSION})

execute_process(
  COMMAND pwd
  RESULT_VARIABLE PWD_RES
  OUTPUT_VARIABLE FS_CURRENT_RAW
  OUTPUT_STRIP_TRAILING_WHITESPACE
)

if(NOT (PWD_RES EQUAL 0))
  message(FATAL_ERROR "Could not determine current folder")
endif()

if (MINGW)
  execute_process(
    COMMAND "cygpath" "-w" ${FS_CURRENT_RAW}
    RESULT_VARIABLE CYGPATH_RES
    OUTPUT_VARIABLE FS_CURRENT
    OUTPUT_STRIP_TRAILING_WHITESPACE
  )
  if(NOT (CYGPATH_RES EQUAL 0))
    message(FATAL_ERROR "Could not map MSys path to Windows")
  endif()
else(MINGW)
  set(FS_CURRENT ${FS_CURRENT_RAW})
endif(MINGW)

set(RUBY_DEBUG_BUILD OFF)

include(ExternalProject)
# This is the default that will be overwritten for MacOS and MSys
set(GNU_BASH "bash")

include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/macos-environment.cmake)
include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/msys-environment.cmake)
include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/def-external-project.cmake)

include(${CMAKE_SOURCE_DIR}/cmake/def_tty_colours.cmake)

option(SETUP_MODE "Tebako setup" OFF)
if(NOT LOG_LEVEL)
  set(LOG_LEVEL "error")
endif()

set(START_WITH_PACKAGED_FILESYSTEM OFF CACHE BOOL "Start workflow from 'packaged filesystem' target!!!" FORCE)

if (NOT ${SETUP_MODE})
  message("Running tebako press script")
  if (${START_WITH_PACKAGED_FILESYSTEM})
    message("${ColourBoldRed}*** Starting workflow from 'packaged filesystem' target ***${ColourReset}")
  endif(${START_WITH_PACKAGED_FILESYSTEM})

  if ("-${ROOT}" STREQUAL "-")
    message(FATAL_ERROR "Project ROOT is not specified.")
  endif()

  if ("-${ENTRANCE}" STREQUAL "-")
    message(FATAL_ERROR "Project ENTRANCE is not specified.")
  endif()

  cmake_path(SET FS_ENTRANCE NORMALIZE ${ENTRANCE})

  cmake_path(IS_RELATIVE ROOT IS_RT_RELATIVE)
  if(${IS_RT_RELATIVE})
    cmake_path(SET FS_ROOT NORMALIZE ${FS_CURRENT}/${ROOT})
    message("Relative path to root '${ROOT}' will be extended to '${FS_ROOT}'")
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

  if ("-${PCKG}" STREQUAL "-")
    set(APP_NAME "${FS_CURRENT}/tebako")
  else()
    cmake_path(IS_RELATIVE PCKG IS_PK_RELATIVE)
    if(${IS_PK_RELATIVE})
      cmake_path(SET APP_NAME NORMALIZE ${FS_CURRENT}/${PCKG})
    else()
      cmake_path(SET APP_NAME NORMALIZE ${PCKG})
    endif()
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
set(FORCE_RUBY_PLATFORM "false")
set(RUBY_WITHOUT_EXT "dbm,win32,win32ole")
set(RUBY_NAME "ruby")
set(RUBY_SUFFIX "")
set(EXE_SUFFIX "")
set(CMD_SUFFIX "")
set(BAT_SUFFIX "")
set(RUBY_MAKEFILE "")

if("${OSTYPE_TXT}" MATCHES "^linux-musl.*")
  set(IS_MUSL ON)
  set(FORCE_RUBY_PLATFORM "true")
elseif("${OSTYPE_TXT}" MATCHES "^msys*")
  set(IS_MSYS ON)
  set(RB_W32 ON)
  set(RUBY_WITHOUT_EXT "dbm,syslog,pty,gdbm,readline")
  set(RUBY_SUFFIX "w")
  set(EXE_SUFFIX ".exe")
  set(CMD_SUFFIX ".cmd")
  set(BAT_SUFFIX ".bat")
  set(RUBY_MAKEFILE "--file=GNUmakefile")
elseif("${OSTYPE_TXT}" MATCHES "^darwin.*")
  set(IS_DARWIN ON)
  set(BUILD_OPENSSL_ROOT_DIR "${BUILD_BREW_PREFIX}/opt/openssl@1.1")
endif()

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

# May be overwritten on MSys platform by setup-openssl.cmake
set(WITH_OPENSSL_BUILD OFF)

if(IS_MSYS)
# ${RUBY_VER} VERSION_LESS "3.0.0"
  include(${CMAKE_SOURCE_DIR}/tools/cmake-scripts/setup-openssl.cmake)
endif(IS_MSYS)

def_ext_prj_g(INCBIN "348e36b")
def_ext_prj_g(DWARFS_WR "v0.3.4")

if (DEFINED ENV{BUNDLER_VER})
 set(BUNDLER_VER $ENV{BUNDLER_VER})
 set(BUNDLER_ANNOTATION "environment")
else()
  set(BUNDLER_VER "2.3.22")
  set(BUNDLER_ANNOTATION "default")
endif()

find_library(_LIBNCURSES "libncurses.a")
if(${_LIBNCURSES} STREQUAL "_LIBNCURSES-NOTFOUND")
  set(WITH_NCURSES_BUILD ON)
  def_ext_prj_t(NCURSES  "6.3" "97fc51ac2b085d4cde31ef4d2c3122c21abc217e9090a43a30fc5ec21684e059")
else()
  set(WITH_NCURSES_BUILD OFF)
endif()

set(LIBYAML_RUBY_OPTION "")
if(${RUBY_VER} VERSION_LESS "3.2.0")
  set(LIBYAML_RUBY_OPTION "--enable-bundled-libyaml")
endif(${RUBY_VER} VERSION_LESS "3.2.0")

message("Configuration summary:")
message(STATUS "ruby: v${RUBY_VER} at ${RUBY_SOURCE_DIR}")
message(STATUS "bundler version: ${BUNDLER_VER} (${BUNDLER_ANNOTATION})")
if(WITH_OPENSSL_BUILD)
  message(STATUS "openssl: building @${OPENSSL_TAG} at ${OPENSSL_SOURCE_DIR}")
endif(WITH_OPENSSL_BUILD)

if(WITH_NCURSES_BUILD)
  message(STATUS "ncurses: building v${NCURSES_VER} at ${NCURSES_SOURCE_DIR}")
else()
  message(STATUS "ncurses: ${_LIBNCURSES}")
endif(WITH_NCURSES_BUILD)

message(STATUS "incbin: @${INCBIN_TAG} at ${INCBIN_SOURCE_DIR}")
message(STATUS "dwarfs with tebako wrapper: @${DWARFS_WR_TAG} at ${DWARFS_WR_SOURCE_DIR}")

# ...................................................................
# Filesystem locations

# DATA_SRC_DIR folder is used to collect all files that need to be packaged
set(DATA_SRC_DIR  ${CMAKE_CURRENT_BINARY_DIR}/source_filesystem)
# DATA_PRE_DIR folder is used to build gems  that need to be packaged
set(DATA_PRE_DIR  ${CMAKE_CURRENT_BINARY_DIR}/prepare_filesystem)
# DATA_BIN_DIR folder is used to create packaged filesystem
set(DATA_BIN_DIR  ${CMAKE_CURRENT_BINARY_DIR}/packaged_filesystem)
# DATA_BIN_FILE is packaged filesystem itself
set(DATA_BIN_FILE ${DATA_BIN_DIR}/fs.bin)
# Target binary directory
set (TBD ${DATA_SRC_DIR}/bin)
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
message(STATUS "TBD: ${TBD}")
message(STATUS "TLD: ${TLD}")
message(STATUS "TGD: ${TGD}")
message(STATUS "FS_MOUNT_POINT: ${FS_MOUNT_POINT}")
message(STATUS "Building for Win32 Ruby (RB_W32): ${RB_W32}")

# ...................................................................
# Other options

message(STATUS "Force Ruby platform for Gems: ${FORCE_RUBY_PLATFORM}")
message(STATUS "Not building Ruby extensions: ${RUBY_WITHOUT_EXT}")

# ...................................................................
# incbin

ExternalProject_Add(${INCBIN_PRJ}
   PREFIX ${DEPS}
   GIT_REPOSITORY https://github.com/graphitemaster/incbin.git
   GIT_TAG ${INCBIN_TAG}
   BUILD_IN_SOURCE true
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ${CMAKE_COMMAND} -E make_directory ${DEPS_INCLUDE_DIR}/incbin
   COMMAND ${CMAKE_COMMAND} -E copy_if_different ${INCBIN_SOURCE_DIR}/incbin.h ${DEPS_INCLUDE_DIR}/incbin
)

# ...................................................................
# OpenSSL
if(WITH_OPENSSL_BUILD)
  message(STATUS "Building OpenSSL since Ruby 2.x requires 1.1.x")
  def_ext_prj_g(OPENSSL "OpenSSL_1_1_1s")

  set(__LIBSSL "${DEPS}/lib/libssl.a")
  set(__LIBCRYPTO "${DEPS}/lib/libcrypto.a")

  ExternalProject_Add(${OPENSSL_PRJ}
    PREFIX ${DEPS}
    GIT_REPOSITORY "https://github.com/openssl/openssl.git"
    GIT_TAG ${OPENSSL_TAG}
    UPDATE_COMMAND ""
    SOURCE_DIR ${OPENSSL_SOURCE_DIR}
    BINARY_DIR ${OPENSSL_BINARY_DIR}
    CONFIGURE_COMMAND   ${GNU_BASH} -c "${OPENSSL_SOURCE_DIR}/config          \
                                                        --openssldir=${DEPS}  \
                                                        --prefix=${DEPS}"
    BUILD_BYPRODUCTS ${__LIBSSL} ${__LIBCRYPTO}
  )
endif(WITH_OPENSSL_BUILD)

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

ExternalProject_Add(${DWARFS_WR_PRJ}
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/tamatebako/libdwarfs.git
    GIT_TAG ${DWARFS_WR_TAG}
    SOURCE_DIR ${DWARFS_WR_SOURCE_DIR}
    BINARY_DIR ${DWARFS_WR_BINARY_DIR}
    UPDATE_COMMAND ""
    BUILD_COMMAND ${CMAKE_COMMAND} --build ${DWARFS_WR_BINARY_DIR} --parallel 4
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS}
               -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
               -DWITH_TESTS:BOOL=OFF
               -DWITH_ASAN:BOOL=OFF
               -DWITH_COVERAGE:BOOL=OFF
               -DTEBAKO_BUILD_SCOPE=MKD
               -DRB_W32=${RB_W32}
    BUILD_BYPRODUCTS ${__LIBDWARFS_WR}
                     ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA}
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
)
if(WITH_OPENSSL_BUILD)
  add_dependencies(${DWARFS_WR_PRJ} ${OPENSSL_PRJ} )
endif(WITH_OPENSSL_BUILD)

if(WITH_NCURSES_BUILD)
  ExternalProject_Add(${NCURSES_PRJ}
    PREFIX ${DEPS}
    URL https://ftp.gnu.org/pub/gnu/ncurses/ncurses-${NCURSES_VER}.tar.gz
    URL_HASH SHA256=${NCURSES_HASH}
    DOWNLOAD_NO_PROGRESS true
    SOURCE_DIR ${NCURSES_SOURCE_DIR}
    UPDATE_COMMAND ""
    CONFIGURE_COMMAND ${NCURSES_SOURCE_DIR}/configure
                          --without-shared
                          --without-cxx-shared
                          --without-tests
                          --prefix=${DEPS}
                          --srcdir=${NCURSES_SOURCE_DIR}
    BUILD_COMMAND     make
    INSTALL_COMMAND   make install
  )
endif(WITH_NCURSES_BUILD)

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
  if(${RUBY_VER} VERSION_LESS "3.2.2")
    set(OPENSSL_VER "1.1")
  else(${RUBY_VER} VERSION_LESS "3.2.2")
    set(OPENSSL_VER "3")
  endif(${RUBY_VER} VERSION_LESS "3.2.2")
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -I${BUILD_BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/include -I${BUILD_BREW_PREFIX}/opt/zlib/include -I${BUILD_BREW_PREFIX}/include")
  string(CONCAT RUBY_L_FLAGS ${RUBY_L_FLAGS} " -L${BUILD_BREW_PREFIX}/opt/openssl@${OPENSSL_VER}/lib -L${BUILD_BREW_PREFIX}/opt/zlib/lib  -L${BUILD_BREW_PREFIX}/lib")
  set(OPENSSL_RUBY_OPTION "--with-openssl-dir=${BUILD_BREW_PREFIX}/opt/openssl@${OPENSSL_VER}")
  set(C_FLAGS_DEST cflags)
endif(IS_DARWIN)

if(IS_MUSL)
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DENABLE_PATH_CHECK=0")
endif(IS_MUSL)

if(RB_W32)
  string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS} " -DRB_W32=1")
endif(RB_W32)

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
  BUILD_COMMAND    make ${RUBY_MAKEFILE}
  INSTALL_COMMAND  make ${RUBY_MAKEFILE} install
  COMMAND          ruby ${EXE}/tebako-packager pass2 ${OSTYPE_TXT} ${RUBY_SOURCE_DIR} ${DEPS_LIB_DIR} ${DATA_SRC_DIR} ${RUBY_STASH_DIR} ${RUBY_VER}
)

add_dependencies(${RUBY_PRJ} ${DWARFS_WR_PRJ})
if(WITH_NCURSES_BUILD)
  add_dependencies(${RUBY_PRJ} ${NCURSES_PRJ})
endif(WITH_NCURSES_BUILD)

if(WITH_OPENSSL_BUILD)
  add_dependencies(${RUBY_PRJ} ${OPENSSL_PRJ} )
endif(WITH_OPENSSL_BUILD)

if (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has completed"
    DEPENDS ${DWARFS_WR_PRJ} ${INCBIN_PRJ} ${RUBY_PRJ}
  )
else (${SETUP_MODE})
  add_custom_target(setup
    ${CMAKE_COMMAND} -E echo "Tebako setup has been verified"
    DEPENDS ${DWARFS_WR_PRJ} ${INCBIN_PRJ} ${RUBY_PRJ}
  )

  file(GLOB GEMSPECS LIST_DIRECTORIES false ${FS_ROOT}/*.gemspec)
  file(GLOB GEMFILES LIST_DIRECTORIES false ${FS_ROOT}/Gemfile)
  file(GLOB GEMS     LIST_DIRECTORIES false ${FS_ROOT}/*.gem)

  list(LENGTH GEMSPECS GSLENGTH)
  list(LENGTH GEMFILES GFLENGTH)
  list(LENGTH GEMS     GLENGTH)

  add_custom_target(clean_filesystem
      COMMAND ruby ${EXE}/tebako-packager deploy ${RUBY_STASH_DIR} ${DATA_SRC_DIR} ${DATA_PRE_DIR} ${DATA_BIN_DIR}
      DEPENDS ${RUBY_PRJ}
  )
  if(GSLENGTH GREATER 0)
    if(GSLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gemspecs detected in ${FS_ROOT}")
    else(GSLENGTH GREATER 1)
      list(GET GEMSPECS 0 GEMSPEC)
      list(LENGTH GEMFILES GFLENGTH)
      if(GFLENGTH GREATER 0)
#  Found xxx.gemspec and Gemfile
        message("Collecting gem from gemspec ${GEMSPEC} and Gemfile")
        set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
        add_custom_target(source_filesystem
           COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${DATA_PRE_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
           COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
           COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                       ${TBD}/gem${CMD_SUFFIX} install bundler -v '${BUNDLER_VER}' --source 'https://rubygems.org/' --no-document --install-dir ${TGD}
           COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                       ${TBD}/bundle${CMD_SUFFIX} config set --local force_ruby_platform ${FORCE_RUBY_PLATFORM}
           COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E  env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                       ${TBD}/bundle${CMD_SUFFIX} install
           COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                       ${TBD}/bundle${CMD_SUFFIX} exec ${TBD}/gem${CMD_SUFFIX} build ${GEMSPEC}
           COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                       ${TBD}/gem${CMD_SUFFIX} install *.gem --verbose --no-document --install-dir ${TGD}
           COMMAND test -f  ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                   (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist" &&
                    ${CMAKE_COMMAND} -E false )
           DEPENDS clean_filesystem
        )
      else(GFLENGTH GREATER 0)
#  Found xxx.gemspec but no Gemfile
         message("Collecting gem from gemspec ${GEMSPEC}")
         set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
         message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
         add_custom_target(source_filesystem
            COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${DATA_PRE_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
            COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                        ${TBD}/gem${CMD_SUFFIX} build ${GEMSPEC}
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                        ${TBD}/gem${CMD_SUFFIX} install *.gem --verbose --no-document --install-dir ${TGD}
            COMMAND test -f  ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                    (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist" &&
                     ${CMAKE_COMMAND} -E false )
            DEPENDS clean_filesystem
         )
      endif(GFLENGTH GREATER 0)
    endif(GSLENGTH GREATER 1)
  elseif(GFLENGTH GREATER 0)
#  Found Gemfile but no gemspec
    message("Deploying Gemfile")
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
    message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
    add_custom_target(source_filesystem
       COMMAND test -f ${FS_ROOT}/${FS_ENTRANCE} ||
                  (${CMAKE_COMMAND} -E echo "Entry point ${FS_ROOT}/${FS_ENTRANCE} does not exist or is not accessible" &&
                   ${CMAKE_COMMAND} -E false )
       COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
       COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${TLD} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                           ${TBD}/gem${CMD_SUFFIX} install bundler -v '${BUNDLER_VER}'
                                              --source 'https://rubygems.org/' --no-document --install-dir ${TGD}
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                           ${TBD}/bundle${CMD_SUFFIX} config build.ffi --disable-system-libffi
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                           ${TBD}/bundle${CMD_SUFFIX} config set --local force_ruby_platform ${FORCE_RUBY_PLATFORM}
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                           ${TBD}/bundle${CMD_SUFFIX} install --jobs=4
#       COMMAND_EXPAND_LISTS
# [TODO] Open question - how to deploy a project to source filesystem
#        ruby packer does bundle install --deployment --binstubs
# [TODO] --jobs=4 probably can be more intellectual
#       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
#                                    ${TBD}/bundle lock
#       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
#                                    ${TBD}/bundle install --deployment --jobs=4
        DEPENDS clean_filesystem
    )
  elseif(GLENGTH GREATER 0)
  # xxx.gem, no gemspec, no gemfile
    list(GET GEMS 0 GEM)
    if(GLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gems detected in ${FS_ROOT}")
    else(GLENGTH GREATER 1)
      message("Installing Ruby gem from ${GEM}")
      set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
      message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
      add_custom_target(source_filesystem
         COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${DATA_PRE_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
         COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1
                                     ${TBD}/gem${CMD_SUFFIX} install ${GEM} --verbose --no-document --install-dir ${TGD}
         COMMAND test -f ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                  (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist or is not accessible" &&
                   ${CMAKE_COMMAND} -E false )
         DEPENDS clean_filesystem
      )
    endif(GLENGTH GREATER 1)
  else()
# No gem, no gemspec, no gemfile
# Simple ruby script
    message("Collecting simple Ruby script from ${FS_ROOT}")
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
    message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
    add_custom_target(source_filesystem
       COMMAND test -f ${FS_ROOT}/${FS_ENTRANCE} ||
                  (${CMAKE_COMMAND} -E echo "Entry point ${FS_ROOT}/${FS_ENTRANCE} does not exist or is not accessible" &&
                   ${CMAKE_COMMAND} -E false )
       COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
       COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${TLD} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
       DEPENDS clean_filesystem
    )
  endif()

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

  if (${START_WITH_PACKAGED_FILESYSTEM})
    add_custom_target(packaged_filesystem
      COMMAND chmod +x ${DEPS_BIN_DIR}/*
      COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
      COMMAND ${CMAKE_COMMAND} -E touch ${DEPS_SRC_DIR}/tebako-fs.cpp
      BYPRODUCTS ${DATA_BIN_FILE}
    )
  else(${START_WITH_PACKAGED_FILESYSTEM})
    add_custom_target(packaged_filesystem
# [TODO] Open question - whether we shall delete ruby binaries from memfs or not
#        For debugging purposes it is very handy to have it here
#
      COMMAND ${CMAKE_COMMAND} -E rm -rf  ${DATA_SRC_DIR}/share  ${DATA_SRC_DIR}/include   ${DATA_SRC_DIR}/lib/pkgconfig
                                          ${DATA_SRC_DIR}/bin/bundle${CMD_SUFFIX}  ${DATA_SRC_DIR}/bin/bundle
                                          ${DATA_SRC_DIR}/bin/bundler${CMD_SUFFIX} ${DATA_SRC_DIR}/bin/bundler
                                          ${DATA_SRC_DIR}/bin/bundle${BAT_SUFFIX}  ${DATA_SRC_DIR}/bin/bundler${BAT_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/erb${CMD_SUFFIX}     ${DATA_SRC_DIR}/bin/gem${CMD_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/irb${CMD_SUFFIX}     ${DATA_SRC_DIR}/bin/racc${CMD_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/racc2y${CMD_SUFFIX}  ${DATA_SRC_DIR}/bin/rake${BAT_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/rake                 ${DATA_SRC_DIR}/bin/rdoc${CMD_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/ri${CMD_SUFFIX}      ${DATA_SRC_DIR}/bin/y2racc${CMD_SUFFIX}
                                          ${DATA_SRC_DIR}/bin/ruby${EXE_SUFFIX}    ${DATA_SRC_DIR}/bin/ruby${RUBY_SUFFIX}${EXE_SUFFIX}

      COMMAND ${GNU_BASH} -c "rm -f ${DATA_SRC_DIR}/**/*.a"
      COMMAND ${GNU_BASH} -c "rm -f ${DATA_SRC_DIR}/**/*.o"
      COMMAND chmod +x ${DEPS_BIN_DIR}/*
      COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
      COMMAND ${CMAKE_COMMAND} -E touch ${DEPS_SRC_DIR}/tebako/tebako-fs.cpp
      DEPENDS setup source_filesystem
      BYPRODUCTS ${DATA_BIN_FILE}
    )
  endif (${START_WITH_PACKAGED_FILESYSTEM})

  set(CMAKE_CXX_FLAGS "${RUBY_C_FLAGS}")

  add_library(tebako-fs STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-main.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/include/tebako/tebako-main.h
    ${DEPS_SRC_DIR}/tebako/tebako-fs.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/include/tebako/tebako-fs.h
    ${DEPS_INCLUDE_DIR}/tebako/tebako-version.h
  )

  add_dependencies(tebako-fs ${INCBIN_PRJ} packaged_filesystem)

  if (${RUBY_VER} VERSION_LESS "3.0.0")
    add_custom_target(patched_ruby
      COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_SOURCE_DIR} make ${RUBY_MAKEFILE}
    )
  else()
    add_custom_target(patched_ruby
      COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_SOURCE_DIR} make ruby ${RUBY_MAKEFILE}
      COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_SOURCE_DIR} make ${RUBY_MAKEFILE}
    )
  endif()

  add_dependencies(patched_ruby setup tebako-fs)

  add_custom_target(tebako ALL
    COMMAND ${CMAKE_COMMAND} -E copy ${RUBY_SOURCE_DIR}/ruby${RUBY_SUFFIX}${EXE_SUFFIX} ${APP_NAME}${EXE_SUFFIX}
    COMMAND ${CMAKE_COMMAND} -E echo "Tebako packaging has completed"
    DEPENDS patched_ruby
  )

endif(${SETUP_MODE})

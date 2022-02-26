# Copyright (c) 2021-2022 [Ribose Inc](https://www.ribose.com).
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
cmake_policy(SET CMP0048 NEW)
# For project( ... VERSION ... )

project(tebako_packager VERSION 0.3.4)
execute_process(COMMAND pwd OUTPUT_VARIABLE FS_CURRENT)
string(REGEX REPLACE "\n$" "" FS_CURRENT "${FS_CURRENT}")

# External dependencies
# ---------------------------------------------------------------------------------------------------
# DwarFS dependencies:
#        ronn binutils-dev libarchive-dev libevent-dev libjemalloc-dev acl-dev \
#        libdouble-conversion-dev libiberty-dev liblz4-dev liblzma-dev libssl-dev \
#        libboost-context-dev libboost-filesystem-dev libboost-program-options-dev \
#        libboost-regex-dev libboost-system-dev libboost-thread-dev \
#        libunwind-dev libdwarf-dev libelf-dev libfmt-dev libfuse3-dev libgoogle-glog-dev \
#        bison flex fuse3 pkg-config libsodium-dev
# ---------------------------------------------------------------------------------------------------
# Ruby dependencies:
#        libffi-dev libgdbm-dev libyaml-dev libncurses-dev libreadline-dev
# ---------------------------------------------------------------------------------------------------
# For Rails testing only:
#        libsqlite3-dev
# ---------------------------------------------------------------------------------------------------

include(ExternalProject)
include(${CMAKE_SOURCE_DIR}/cmake/def_ext_prj.cmake)
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
  set(SETUP_COMPLETED_MESSAGE "Tebako setup has been verified")
else()
  message("Running tebako setup script")
  set(SETUP_COMPLETED_MESSAGE "Tebako setup has been completed")
endif()

set(BOOST_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)

# Various locations for external projects
set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps CACHE STRING "Dependencies' folder'")
set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

set(MKDWARFS ${DEPS_BIN_DIR}/mkdwarfs)
set(CROSS_COMPILING OFF)

if(TEBAKO_BUILD_TARGET)
  if(NOT ${TEBAKO_BUILD_TARGET} STREQUAL "guess")
    if(${CMAKE_HOST_SYSTEM_NAME} MATCHES "Darwin")
      if(${TEBAKO_BUILD_TARGET} STREQUAL "arm64-apple-macos11")
        set(HOST_OPTION --host=aarch64-apple-darwin --with-coroutine=ucontext --with-arch=arm64)
        set(CMAKE_OSX_ARCHITECTURES arm64)
      elseif(${TEBAKO_BUILD_TARGET} STREQUAL "x86_64-apple-macos11")
        set(HOST_OPTION --host=x86_64-apple-darwin --with-coroutine=ucontext --with-arch=x86_64)
        set(CMAKE_OSX_ARCHITECTURES x86_64)
      else()
        message(FATAL_ERROR "Cannot cross-compile for target ${TEBAKO_BUILD_TARGET}")
      endif()
      add_compile_options(--target=${TEBAKO_BUILD_TARGET})
      add_link_options(--target=${TEBAKO_BUILD_TARGET})
      set(XLDFLAGS " -F/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/System/Library/Frameworks/ -framework CoreFoundation -framework Security ")
      set(XCCFLAGS " --target=${TEBAKO_BUILD_TARGET} -D_XOPEN_SOURCE=1 ")
      set(MKDWARFS ${DEPS}/host/bin/mkdwarfs)
      set(DWARFS_WR_OPTION "-DTEBAKO_BUILD_TARGET=${TEBAKO_BUILD_TARGET}")
      set(CROSS_COMPILING ON)
    else(${CMAKE_HOST_SYSTEM_NAME} MATCHES "Darwin")
      message(${CMAKE_HOST_SYSTEM_NAME})
      message(FATAL_ERROR "Cross-compilation is supported on MacOS only")
    endif(${CMAKE_HOST_SYSTEM_NAME} MATCHES "Darwin")
  endif(NOT ${TEBAKO_BUILD_TARGET} STREQUAL "guess")  
endif(TEBAKO_BUILD_TARGET)

set(GNU_BASH "bash")
include(${CMAKE_SOURCE_DIR}/cmake/def_darwin_env.cmake)

set(RUBY_CFLAGS " -O3 -fno-fast-math -ggdb3 -Os -fdata-sections -ffunction-sections " )
string(CONCAT RUBY_CFLAGS ${RUBY_CFLAGS} " -pthread -fPIC -pipe -I${DEPS_INCLUDE_DIR} -I${CMAKE_CURRENT_SOURCE_DIR}/include ")
# https://stackoverflow.com/questions/46123505/assembling-with-gcc-causes-weird-relocation-error-with-regards-to-data
if("${CMAKE_CXX_COMPILER_ID}" STREQUAL "GNU")
  string(CONCAT RUBY_CFLAGS ${RUBY_CFLAGS} " -no-pie ")
endif()
set(RUBY_LDFLAGS " -L${DEPS_LIB_DIR} -L${CMAKE_CURRENT_BINARY_DIR} ")
if(CMAKE_HOST_SYSTEM_NAME MATCHES "Darwin")
  if(CROSS_COMPILING)
  string(CONCAT RUBY_CFLAGS ${RUBY_CFLAGS} ${XCCFLAGS})
    string(CONCAT RUBY_CFLAGS ${RUBY_CFLAGS} ${RUBY_TARGET_BREW_INCLUDE})
    string(CONCAT RUBY_LDFLAGS ${RUBY_LDFLAGS} ${XLDFLAGS})   
    string(CONCAT RUBY_LDFLAGS ${RUBY_LDFLAGS} ${RUBY_TARGET_BREW_LIB})
  else(CROSS_COMPILING)
    string(CONCAT RUBY_CFLAGS ${RUBY_CFLAGS} ${RUBY_BUILD_BREW_INCLUDE})
    string(CONCAT RUBY_LDFLAGS ${RUBY_LDFLAGS} ${RUBY_BUILD_BREW_LIB})
  endif(CROSS_COMPILING)
endif(CMAKE_HOST_SYSTEM_NAME MATCHES "Darwin")

message("Configuraion summary:")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Build target: ${TEBAKO_BUILD_TARGET}")
message(STATUS "Extra c compiler flags: ${XCCFLAGS}")
message(STATUS "Extra linker flags: ${XLDFLAGS}")
message(STATUS "configure cross-compile options: ${HOST_OPTION}")
message(STATUS "CMake OSX architecture flag: ${CMAKE_OSX_ARCHITECTURES}")
message(STATUS "Build environment homebrew: ${BREW_PREFIX}")
message(STATUS "Target environment homebrew: ${TARGET_BREW_PREFIX}")
message(STATUS "mkdwarfs: ${MKDWARFS}")
message(STATUS "DWARFS_WR option: ${DWARFS_WR_OPTION}")
message(STATUS "GNU compatible bash: ${GNU_BASH}")
message(STATUS "Ruby CFLAGS: ${RUBY_CFLAGS}")
message(STATUS "Ruby LDFLAGS: ${RUBY_LDFLAGS}")

# ...................................................................
# External projects

def_ext_prj_t(RUBY "2.7.5" "2755b900a21235b443bb16dadd9032f784d4a88f143d852bc5d154f22b8781f1")
def_ext_prj_t(RUBY "2.7.5" "2755b900a21235b443bb16dadd9032f784d4a88f143d852bc5d154f22b8781f1" TARGET)
# def_ext_prj_t(RUBY "2.7.5" "2755b900a21235b443bb16dadd9032f784d4a88f143d852bc5d154f22b8781f1" BASE)

string(SUBSTRING ${RUBY_VER} 0 3 RUBY_VER_BASE)
string(CONCAT RUBY_API_VER ${RUBY_VER_BASE} ".0")

def_ext_prj_g(INCBIN "348e36b")
def_ext_prj_g(DWARFS_WR "v0.1.10")

if (DEFINED ENV{BUNDLER_VER})
set(BUNDLER_VER $ENV{BUNDLER_VER})
set(BUNDLER_ANNOTATION "environment")
else()
  set(BUNDLER_VER "2.2.3")
  set(BUNDLER_ANNOTATION "default")
endif()

if(CROSS_COMPILING)
  def_ext_prj_g(DWARFS "libdwarfs_wr_v0.1.7" HOST)
endif()

message("Collecting packages:")
message(STATUS "ruby build environmemt      -  v${RUBY_VER} at ${RUBY_SOURCE_DIR}")
message(STATUS "ruby target environmemt     -  v${RUBY_VER} at ${RUBY_TARGET_SOURCE_DIR}")
#if(CROSS_COMPILING)
#  message(STATUS "ruby base environmemt       -  v${RUBY_VER} at ${RUBY_BASE_SOURCE_DIR}")
#endif()
message(STATUS "incbin                      -  @${INCBIN_TAG} at ${INCBIN_SOURCE_DIR}")
message(STATUS "dwarfs with tebako wrapper  -  @${DWARFS_WR_TAG} at ${DWARFS_WR_SOURCE_DIR}")
if(CROSS_COMPILING)
  message(STATUS "dwarfs build host version   -  @${DWARFS_TAG} at ${DWARFS_HOST_SOURCE_DIR}")
endif()
message(STATUS "Using bundler ${BUNDLER_VER} per ${BUNDLER_ANNOTATION} setting")


# ...................................................................
# Filesystem locations

set(PATCHES ${CMAKE_CURRENT_SOURCE_DIR}/patches)
# Project resources that are used during CMake configuration stage
set(DATA_RES_DIR  ${CMAKE_CURRENT_SOURCE_DIR}/resources)
# DATA_TEMPLATE_DIR clean memfs template
set(DATA_TEMPLATE_DIR  ${DEPS}/template_filesystem)
# DATA_SRC_DIR folder is used to collect all files that need to be packaged
set(DATA_SRC_DIR  ${CMAKE_CURRENT_BINARY_DIR}/source_filesystem)
# DATA_PRE_DIR folder is used to build gems  that need to be packaged
set(DATA_PRE_DIR  ${CMAKE_CURRENT_BINARY_DIR}/prepare_filesystem)
# DATA_BIN_DIR folder is used to create packaged filesystem
set(DATA_BIN_DIR  ${CMAKE_CURRENT_BINARY_DIR}/packaged_filesystem)
# DATA_BIN_FILE is packaged filesystem itself
set(DATA_BIN_FILE ${DATA_BIN_DIR}/fs.bin)
# TBD target binary directory
set (TBD ${DATA_SRC_DIR}/bin)
# TLD target "local" directory
set (TLD ${DATA_SRC_DIR}/local)
# TGD target Gem directory
set (TGD ${DATA_SRC_DIR}/lib/ruby/gems/${RUBY_API_VER})
# TSD target site_ruby directory
if(CROSS_COMPILING)
  set (TSD ${DATA_SRC_DIR}/lib/ruby/${RUBY_API_VER}/${CMAKE_OSX_ARCHITECTURES}-darwin)
endif(CROSS_COMPILING)

if(CROSS_COMPILING)
  set(GEM_COMMAND ruby ${TBD}/gem)
  set(BUNDLE_COMMAND ruby ${TBD}/bundle)
else(CROSS_COMPILING)
  set(GEM_COMMAND ${TBD}/ruby ${TBD}/gem)
  set(BUNDLE_COMMAND ${TBD}/ruby ${TBD}/bundle)
endif(CROSS_COMPILING)

# This is actually a constant that shall match libdwarfs-wr TEBAKO_MOUNT_POINT at tebako-common.h
set(FS_MOUNT_POINT "/__tebako_memfs__")
set(FS_APP_HOME "/.")

message("Packaging specification:")
message(STATUS "memfs template directory:     ${DATA_TEMPLATE_DIR}")
message(STATUS "gem build directory:          ${DATA_PRE_DIR}")
message(STATUS "memfs source directory:       ${DATA_SRC_DIR}")
message(STATUS "memfs binary directory:       ${TBD}")
message(STATUS "memfs 'local' directory:      ${TLD}")
message(STATUS "memfs GEM_HOME:               ${TGD}")
message(STATUS "target arch config directory: ${TSD}")
message(STATUS "packaged memfs file:          ${DATA_BIN_FILE}")
message(STATUS "memfs mount point:            ${FS_MOUNT_POINT}")
message(STATUS "application starts at:        ${FS_APP_HOME}")
message(STATUS "build gem command:            ${GEM_COMMAND}")
message(STATUS "build bundle command:         ${BUNDLE_COMMAND}")

# ...................................................................
# incbin

ExternalProject_Add(${INCBIN_PRJ}
   PREFIX ${DEPS}
   GIT_REPOSITORY https://github.com/graphitemaster/incbin.git
   GIT_TAG ${INCBIN_TAG}
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ${CMAKE_COMMAND} -E make_directory ${DEPS_INCLUDE_DIR}/incbin
   COMMAND ${CMAKE_COMMAND} -E copy_if_different ${INCBIN_SOURCE_DIR}/incbin.h ${DEPS_INCLUDE_DIR}/incbin
)

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
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS}
               -DWITH_TESTS:BOOL=OFF
               -DTARGET_HOMEBREW=${_TARGET_HOMEBREW}
               ${DWARFS_WR_OPTION}
    BUILD_BYPRODUCTS ${__LIBDWARFS_WR}
                     ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA}
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
)

# ...................................................................
# Ruby

ExternalProject_Add(${RUBY_PRJ}
  PREFIX ${DEPS}
  URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
  URL_HASH SHA256=${RUBY_HASH}
  DOWNLOAD_NO_PROGRESS true
  SOURCE_DIR ${RUBY_SOURCE_DIR}
  BINARY_DIR ${RUBY_BINARY_DIR}
  PATCH_COMMAND ${GNU_BASH} ${PATCHES}/ruby/pass1.sh ${RUBY_SOURCE_DIR} ${RUBY_BINARY_DIR} ${_TARGET_HOMEBREW}
  UPDATE_COMMAND ""
  CONFIGURE_COMMAND ${RUBY_SOURCE_DIR}/configure
                    --enable-bundled-libyaml
                    --without-gmp
                    --disable-dtrace
                    --enable-debug-env
                    --disable-install-doc
                    --with-static-linked-ext
                    --srcdir=${RUBY_SOURCE_DIR}
                    --prefix=${DATA_SRC_DIR}
                    CFLAGS=${RUBY_CFLAGS}
                    LDFLAGS=${RUBY_LDFLAGS}
                    ${HOST_OPTION}                    
  INSTALL_COMMAND ""
)

# This is required since DWARFS_WR_PRJ creates a directory refeernced by ${RUBY_LDFLAGS}
# configure fails if the directory does not exist
add_dependencies(${RUBY_PRJ} ${DWARFS_WR_PRJ})

add_custom_target(template_filesystem
  COMMAND ${CMAKE_COMMAND} -E echo "template_filesystem: creating '${DATA_TEMPLATE_DIR}'"
  COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR} ${DATA_TEMPLATE_DIR}
  COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_SRC_DIR}
  COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_TEMPLATE_DIR}
  COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make install
  COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${DATA_SRC_DIR} -DTARGET_DIR=${DATA_TEMPLATE_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
  DEPENDS ${RUBY_PRJ} ${INCBIN_PRJ}
)

if(CROSS_COMPILING)
  ExternalProject_Add(${DWARFS_HOST_PRJ}
    PREFIX ${DEPS}/host
    GIT_REPOSITORY https://github.com/tamatebako/dwarfs.git
    GIT_TAG ${DWARFS_TAG}
    UPDATE_COMMAND ""
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS}/host
             -DWITH_TESTS=OFF
             -DTEBAKO_BUILD_SCOPE=MKD
             -DCMAKE_BUILD_TYPE=Release
    SOURCE_DIR ${DWARFS_HOST_SOURCE_DIR}
    BINARY_DIR ${DWARFS_HOST_BINARY_DIR}
  )
endif(CROSS_COMPILING)

add_custom_target(clean_filesystem ALL
  COMMAND ${CMAKE_COMMAND} -E echo "clean_filesystem: creating '${DATA_SRC_DIR}', '${DATA_PRE_DIR}', '${DATA_BIN_DIR}'"
  COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR} ${DATA_PRE_DIR} ${DATA_BIN_DIR}
  COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${DATA_TEMPLATE_DIR} -DTARGET_DIR=${DATA_SRC_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
  COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_PRE_DIR}
  COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_BIN_DIR}
  DEPENDS template_filesystem
)

if (${SETUP_MODE})
  add_custom_target(source_filesystem COMMAND ${CMAKE_COMMAND} -E true DEPENDS clean_filesystem)    
else(${SETUP_MODE})
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
      list(GET GEMSPECS 0 GEMSPEC)
      list(LENGTH GEMFILES GFLENGTH)
      if(GFLENGTH GREATER 0)
#  Found xxx.gemspec and Gemfile
        message("Collecting gem from gemspec ${GEMSPEC} and Gemfile")
        set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
        add_custom_target(source_filesystem
            COMMAND ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install bundler -v '${BUNDLER_VER}' --source 'https://rubygems.org/' --no-document --install-dir ${TGD} --bindir ${TBD}        
            COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${DATA_PRE_DIR} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} install
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} exec ${GEM_COMMAND} build ${GEMSPEC}
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install *.gem --verbose --no-document --install-dir ${TGD} --bindir ${TBD}
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
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} build ${GEMSPEC}
            COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
            COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install *.gem --verbose --no-document --install-dir ${TGD} --bindir ${TBD}
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
    if(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
      message(FATAL_ERROR "Entry point ${FS_ROOT}/${FS_ENTRANCE} does not exist or is not accessible")
    endif(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
    message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
    set(FS_APP_HOME "/local")
    message("Application will start at ${FS_MOUNT_POINT}${FS_APP_HOME}")
    if(CROSS_COMPILING)
      add_custom_target(source_filesystem
        COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
        COMMAND ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install bundler -v '${BUNDLER_VER}' --source 'https://rubygems.org/' --no-document --install-dir ${TGD} --bindir ${TBD}
        COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${TLD} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
        COMMAND ${CMAKE_COMMAND} -E make_directory ${BREW_PREFIX}/lib/ruby/site_ruby
        COMMAND ${CMAKE_COMMAND} -E copy_if_different ${TSD}/rbconfig.rb ${BREW_PREFIX}/lib/ruby/site_ruby
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} config set --local build.ffi --disable-system-libffi
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} config set --local bin ${TBD}
#      COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} config --local deployment true 
#      COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} update --bundler
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} lock
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} install --jobs=4
        DEPENDS clean_filesystem
      )
    else(CROSS_COMPILING)
      add_custom_target(source_filesystem
        COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
        COMMAND ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install bundler -v '${BUNDLER_VER}' --source 'https://rubygems.org/' --no-document --install-dir ${TGD} --bindir ${TBD}
        COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${TLD} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} config set --local build.ffi --disable-system-libffi
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} config set --local bin ${TBD}
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} lock
        COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${BUNDLE_COMMAND} install --jobs=4
        DEPENDS clean_filesystem
      )
    endif(CROSS_COMPILING)  
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
        COMMAND ${CMAKE_COMMAND} -E chdir ${DATA_PRE_DIR} ${CMAKE_COMMAND} -E env GEM_HOME=${TGD} GEM_PATH=${TGD} ${GEM_COMMAND} install ${GEM} --verbose --no-document --install-dir ${TGD} --bindir ${TBD}
        COMMAND test -f ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                  (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist" &&
                   ${CMAKE_COMMAND} -E false )
        DEPENDS clean_filesystem
      )
    endif(GLENGTH GREATER 1)
  else()
# No gem, no gemspec, no gemfile
# Simple ruby script
    message("Collecting simple Ruby script from ${FS_ROOT}")
    if(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
      message(FATAL_ERROR "Entry point ${FS_ROOT}/${FS_ENTRANCE} does not exist or is not accessible")
    endif(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
    set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
    message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
    add_custom_target(source_filesystem
      COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
      COMMAND ${CMAKE_COMMAND} -DSOURCE_DIR=${FS_ROOT} -DTARGET_DIR=${TLD} -P ${CMAKE_SOURCE_DIR}/cmake/copy_dir.cmake
      DEPENDS clean_filesystem
    )
  endif()
endif(${SETUP_MODE})

# ...................................................................
# Packaged filesystem

configure_file(
  ${DATA_RES_DIR}/tebako-version.h.in
  ${CMAKE_CURRENT_SOURCE_DIR}/include/tebako-version.h
  @ONLY
)

configure_file(
  ${DATA_RES_DIR}/tebako-fs.cpp.in
  ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
  @ONLY
)

if (${START_WITH_PACKAGED_FILESYSTEM})
  add_custom_target(packaged_filesystem
    COMMAND chmod +x ${DEPS_BIN_DIR}/*
    COMMAND ${MKDWARFS} -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
    COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
    BYPRODUCTS ${DATA_BIN_FILE}
  )
else(${START_WITH_PACKAGED_FILESYSTEM})
  add_custom_target(packaged_filesystem
# [TODO] Open question - whether we shall delete ruby binaries from memfs or not
#        For debugging purposes it is very handy to have it here

#    COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR}/share   ${DATA_SRC_DIR}/include  ${DATA_SRC_DIR}/lib/*.a ${DATA_SRC_DIR}/lib/pkgconfig
#                                       ${DATA_SRC_DIR}/bin/bundle  ${DATA_SRC_DIR}/bin/bundler  ${DATA_SRC_DIR}/bin/erb  ${DATA_SRC_DIR}/bin/gem
#                                       ${DATA_SRC_DIR}/bin/irb ${DATA_SRC_DIR}/bin/racc  ${DATA_SRC_DIR}/bin/racc2y ${DATA_SRC_DIR}/bin/rake
#                                       ${DATA_SRC_DIR}/bin/rdoc ${DATA_SRC_DIR}/bin/ri ${DATA_SRC_DIR}/bin/ruby ${DATA_SRC_DIR}/bin/y2racc
    COMMAND chmod +x ${DEPS_BIN_DIR}/*
    COMMAND ${MKDWARFS} -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
    COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
    DEPENDS source_filesystem
    BYPRODUCTS ${DATA_BIN_FILE}
  )
endif (${START_WITH_PACKAGED_FILESYSTEM})

if(CROSS_COMPILING)
  add_dependencies(packaged_filesystem ${DWARFS_HOST_PRJ})
endif(CROSS_COMPILING)  

set(CMAKE_CXX_FLAGS ${RUBY_CFLAGS})

add_library(tebako-fs STATIC
  src/tebako-main.cpp
  src/tebako-fs.cpp
  include/tebako-fs.h
  include/tebako-version.h
  include/tebako-main.h
)

ExternalProject_Add(${RUBY_TARGET_PRJ}
  PREFIX ${DEPS}
  URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
  URL_HASH SHA256=${RUBY_HASH}
  DOWNLOAD_NO_PROGRESS true
  SOURCE_DIR ${RUBY_TARGET_SOURCE_DIR}
  BINARY_DIR ${RUBY_TARGET_BINARY_DIR}
  PATCH_COMMAND ${GNU_BASH} ${PATCHES}/ruby/pass2.sh ${RUBY_TARGET_SOURCE_DIR} ${RUBY_TARGET_BINARY_DIR} ${_TARGET_HOMEBREW}
  CONFIGURE_COMMAND ${RUBY_TARGET_SOURCE_DIR}/configure
                    --enable-bundled-libyaml
                    --without-gmp
                    --disable-dtrace
                    --enable-debug-env
                    --disable-install-doc
                    --with-static-linked-ext
                    --srcdir=${RUBY_TARGET_SOURCE_DIR}
                    --prefix=${DEPS}/tmp
                    CFLAGS=${RUBY_CFLAGS} 
                    LDFLAGS=${RUBY_LDFLAGS}
                    ${HOST_OPTION}
  INSTALL_COMMAND ""
)

add_dependencies(tebako-fs ${INCBIN_PRJ} packaged_filesystem)
add_dependencies(${RUBY_TARGET_PRJ} tebako-fs)

add_custom_target(setup
  ${CMAKE_COMMAND} -E echo "${SETUP_COMPLETED_MESSAGE}"
  DEPENDS ${INCBIN_PRJ} ${RUBY_TARGET_PRJ} ${DWARFS_HOST_PRJ} ${RUBY_PRJ} ${RUBY_BASE_PRJ}
)

if(NOT ${SETUP_MODE})
  add_custom_target(tebako ALL
    COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_TARGET_BINARY_DIR} make main 
    COMMAND ${CMAKE_COMMAND} -E copy ${RUBY_TARGET_BINARY_DIR}/ruby ${APP_NAME}
    COMMAND ${CMAKE_COMMAND} -E echo "Tebako packaging has completed"
    DEPENDS setup
  )
endif(NOT ${SETUP_MODE})

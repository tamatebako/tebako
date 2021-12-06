# Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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


project(tebako_packager VERSION 0.2.0)
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
include(${CMAKE_SOURCE_DIR}/cmake/ensure_bundler.cmake)

option(SETUP_MODE "Tebako setup" OFF)

if (NOT ${SETUP_MODE})
  message("Running tebako press script")
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

set(PATCHES ${CMAKE_CURRENT_SOURCE_DIR}/patches)

# Project resources that are used during CMake configuration stage
set(DATA_RES_DIR  ${CMAKE_CURRENT_SOURCE_DIR}/resources)

if (NOT ${SETUP_MODE})
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

# This is actually a constant that shall match libdwarfs-wr TEBAKO_MOUNT_POINT at tebako-common.h
  set(FS_MOUNT_POINT "/__tebako_memfs__")
  set(FS_APP_HOME "/.")

endif(NOT ${SETUP_MODE})

# ...................................................................
# External projects

def_ext_prj_t(RUBY "2.7.4" "3043099089608859fc8cce7f9fdccaa1f53a462457e3838ec3b25a7d609fbc5b")
#def_ext_prj_t(RUBY "2.6.3" "577fd3795f22b8d91c1d4e6733637b0394d4082db659fccf224c774a2b1c82fb")

string(SUBSTRING ${RUBY_VER} 0 3 RUBY_VER_BASE)
string(CONCAT RUBY_API_VER ${RUBY_VER_BASE} ".0")
def_ext_prj_t(BUNDLER  "2.2.3"  "6acefda4aeb34cb3d69aff06affce10424d69f484402a9f7f5577e8c698070db")

def_ext_prj_g(INCBIN "348e36b")
def_ext_prj_g(DWARFS_WR "b1c1cb3")

message("................................................")
message("Collecting packages:")
message("ruby                        - " v${RUBY_VER}           " at " ${RUBY_SOURCE_DIR})
message("bundler                     - " v${BUNDLER_VER}        " at " ${BUNDLER_SOURCE_DIR})
message("incbin                      - " @${INCBIN_TAG}         " at " ${INCBIN_SOURCE_DIR})
message("dwarfs with tebako wrapper  - " @${DWARFS_WR_TAG}      " at " ${DWARFS_WR_SOURCE_DIR})
message("................................................")

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
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} -DWITH_TESTS:BOOL=OFF
    BUILD_BYPRODUCTS ${__LIBDWARFS_WR}
                     ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA}
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
)

# ...................................................................
# Ruby

if(RUBY_DEBUG_BUILD)
  set(RUBY_C_FLAGS_1 "-g -O0 -pipe")
else()
  set(RUBY_C_FLAGS_1 "-O3 -fno-fast-math -ggdb3 -Os -fdata-sections -ffunction-sections")
endif()

set(RUBY_C_FLAGS_2 "-fPIC -pipe -I${DEPS_INCLUDE_DIR} -I${CMAKE_CURRENT_SOURCE_DIR}/include")
string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS_1} " " ${RUBY_C_FLAGS_2})
set(RUBY_L_FLAGS "-L${DEPS_LIB_DIR} -L${CMAKE_CURRENT_BINARY_DIR}")

message("Ruby build CFLAGS='${RUBY_C_FLAGS}'")
message("Ruby build LDFLAGS='${RUBY_L_FLAGS}'")

ExternalProject_Add(${RUBY_PRJ}
   PREFIX ${DEPS}
   URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
   URL_HASH SHA256=${RUBY_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${RUBY_SOURCE_DIR}
   BINARY_DIR ${RUBY_BINARY_DIR}
   PATCH_COMMAND ${PATCHES}/ruby/pass1.sh ${RUBY_SOURCE_DIR} ${RUBY_BINARY_DIR}
   CONFIGURE_COMMAND ${RUBY_SOURCE_DIR}/configure
                     --enable-bundled-libyaml
                     --without-gmp
                     --disable-dtrace
                     --enable-debug-env
                     --disable-install-doc
                     --with-static-linked-ext
                     --with-jemalloc
                     --srcdir=${RUBY_SOURCE_DIR}
                     --prefix=${DATA_SRC_DIR}
                     CFLAGS=${RUBY_C_FLAGS}
                     LDFLAGS=${RUBY_L_FLAGS}
   INSTALL_COMMAND ""
)

add_dependencies(${RUBY_PRJ} ${DWARFS_WR_PRJ})

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

  set (TBD ${DATA_SRC_DIR}/bin)
  set (TLD ${DATA_SRC_DIR}/local)
# TGD folder is used to install gems
  set (TGD ${DATA_SRC_DIR}/lib/ruby/gems/${RUBY_API_VER})

  file(GLOB GEMSPECS LIST_DIRECTORIES false ${FS_ROOT}/*.gemspec)
  file(GLOB GEMFILES LIST_DIRECTORIES false ${FS_ROOT}/Gemfile)
  file(GLOB GEMS     LIST_DIRECTORIES false ${FS_ROOT}/*.gem)

  list(LENGTH GEMSPECS GSLENGTH)
  list(LENGTH GEMFILES GFLENGTH)
  list(LENGTH GEMS     GLENGTH)

  add_custom_target(clean_filesystem
           COMMAND ${CMAKE_COMMAND} -E echo "clean_filesystem: creating '${DATA_SRC_DIR}', '${DATA_PRE_DIR}', '${DATA_BIN_DIR}'"
           COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR} ${DATA_PRE_DIR} ${DATA_BIN_DIR}
           COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_SRC_DIR}
           COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_PRE_DIR}
           COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_BIN_DIR}
           COMMAND ${PATCHES}/ruby/pass1.sh ${RUBY_SOURCE_DIR} ${RUBY_BINARY_DIR}
           COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make install
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
        ensure_bundler(${BUNDLER_VER} ${BUNDLER_HASH} ${BUNDLER_SOURCE_DIR})
        add_custom_target(source_filesystem
           COMMAND ${TBD}/gem install ${BUNDLER_SOURCE_DIR}/bundler-${BUNDLER_VER}.gem --verbose --no-document --install-dir ${TGD}
#           COMMAND ${TBD}/gem install rake -v '13.0.6' --source 'https://rubygems.org/' --install-dir ${TGD}
#           COMMAND ${TBD}/gem install bundler:2.1.4 --verbose --no-document --install-dir ${TGD}
           COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${DATA_PRE_DIR}
           COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && export PATH=${TBD}:$PATH && export TEBAKO_PASS_THROUGH=1 && bundle install
           COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && export PATH=${TBD}:$PATH && export TEBAKO_PASS_THROUGH=1 && bundle exec gem build ${GEMSPEC}
           COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
           COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && export PATH=${TBD}:$PATH && export TEBAKO_PASS_THROUGH=1 &&
                   gem install *.gem --verbose --no-document --install-dir ${TGD}
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
            COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${DATA_PRE_DIR}
            COMMAND unset GEM_HOME && unset GEM_PATH && export PATH=${TBD}:$PATH && cd ${DATA_PRE_DIR} && export TEBAKO_PASS_THROUGH=1 && gem build ${GEMSPEC}
            COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
            COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && export PATH=${TBD}:$PATH && export TEBAKO_PASS_THROUGH=1 &&
                    gem install *.gem --verbose --no-document --install-dir ${TGD}
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
    ensure_bundler(${BUNDLER_VER} ${BUNDLER_HASH} ${BUNDLER_SOURCE_DIR})
    add_custom_target(source_filesystem
#       COMMAND ${TBD}/gem install ${BUNDLER_SOURCE_DIR}/bundler-${BUNDLER_VER}.gem --verbose --no-document --install-dir ${TGD}
       COMMAND ${TBD}/gem install bundler:2.2.3
       COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
       COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${TLD}
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1 ${TBD}/bundle lock
       COMMAND ${CMAKE_COMMAND} -E chdir ${TLD} ${CMAKE_COMMAND} -E env --unset=GEM_HOME --unset=GEM_PATH TEBAKO_PASS_THROUGH=1 ${TBD}/bundle install --deployment --jobs=4
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
         COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${DATA_PRE_DIR}
         COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && export PATH=${TBD}:$PATH && export TEBAKO_PASS_THROUGH=1 &&
                  gem install ${GEM} --verbose --no-document --install-dir ${TGD}
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
       COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${TLD}
       DEPENDS clean_filesystem
    )
  endif()

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

  add_custom_target(packaged_filesystem
#     COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR}/share   ${DATA_SRC_DIR}/include  ${DATA_SRC_DIR}/lib/*.a ${DATA_SRC_DIR}/lib/pkgconfig
#                                        ${DATA_SRC_DIR}/bin/bundle  ${DATA_SRC_DIR}/bin/bundler  ${DATA_SRC_DIR}/bin/erb  ${DATA_SRC_DIR}/bin/gem
#                                        ${DATA_SRC_DIR}/bin/irb ${DATA_SRC_DIR}/bin/racc  ${DATA_SRC_DIR}/bin/racc2y ${DATA_SRC_DIR}/bin/rake
#                                        ${DATA_SRC_DIR}/bin/rdoc ${DATA_SRC_DIR}/bin/ri ${DATA_SRC_DIR}/bin/ruby ${DATA_SRC_DIR}/bin/y2racc
     COMMAND chmod +x ${DEPS_BIN_DIR}/*
     COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
     COMMAND ${CMAKE_COMMAND} -E touch ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
     DEPENDS setup source_filesystem
     BYPRODUCTS ${DATA_BIN_FILE}
  )

  set(CMAKE_CXX_FLAGS "${RUBY_C_FLAGS}")

  add_library(tebako-fs STATIC
    src/tebako-main.cpp
    src/tebako-fs.cpp
    include/tebako-fs.h
    include/tebako-version.h
    include/tebako-main.h)

  add_dependencies(tebako-fs ${INCBIN_PRJ} packaged_filesystem)

  add_custom_target(patched_ruby
     COMMAND ${PATCHES}/ruby/pass2.sh ${RUBY_SOURCE_DIR} ${RUBY_BINARY_DIR}
     COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make
     DEPENDS setup tebako-fs
  )

  add_custom_target(tebako ALL
    COMMAND ${CMAKE_COMMAND} -E copy ${RUBY_BINARY_DIR}/ruby ${APP_NAME}
    COMMAND ${CMAKE_COMMAND} -E echo "Tebako packaging has completed"
    DEPENDS patched_ruby
  )

endif(${SETUP_MODE})

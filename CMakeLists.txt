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

project(tebako_packager VERSION 0.1.2)
# ...................................................................
#        sudo apt-get install \
#        ronn binutils-dev libarchive-dev libevent-dev libjemalloc-dev acl-dev \
#        libdouble-conversion-dev libiberty-dev liblz4-dev liblzma-dev libssl-dev \
#        libboost-context-dev libboost-filesystem-dev libboost-program-options-dev \
#        libboost-regex-dev libboost-system-dev libboost-thread-dev \
#        libunwind-dev libdwarf-dev libelf-dev libfmt-dev libfuse3-dev libgoogle-glog-dev 
#    ++  Already installed at GHA but may require local installation: 
#        bison flex fuse3 pkg-config libsodium-dev

include(ExternalProject)
include(${CMAKE_SOURCE_DIR}/cmake/def_ext_prj.cmake)


# Developer mode removes the burden of nested projects which takes a lot under WSL/VisualStudio/Ninja environment
# Use with caution, please
option(DEVELOPER_MODE "Developer mode" OFF)
option(RUBY_DEBUG_BUILD "Ruby debug build" OFF)

message("Welcome to tebako packager configuration script!")
if ("-${ROOT}" STREQUAL "-")
  message(FATAL_ERROR "Project ROOT is not specified.")
endif()

if ("-${ENTRANCE}" STREQUAL "-")
  message(FATAL_ERROR "Project ENTRANCE is not specified.")
endif()

cmake_path(SET FS_ENTRANCE NORMALIZE ${ENTRANCE})
cmake_path(SET FS_ROOT NORMALIZE ${ROOT})

cmake_path(IS_RELATIVE FS_ROOT IS_RT_RELATIVE)
if(${IS_RT_RELATIVE})
  message(FATAL_ERROR "Please provide absolute path to root folder. '${FS_ROOT}' is relative path and cannot be handled properly")
endif()

message("Packaging ${FS_ROOT} folder with ${FS_ENTRANCE} entry point")
message("................................................")

set(Boost_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)


# Various locations for external projects 
set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps CACHE STRING "Dependencies' folder'")
set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

# Project resources that are used during CMake configuration stage
set(DATA_RES_DIR  ${CMAKE_CURRENT_SOURCE_DIR}/resources)

# Inject version information to C++ code
configure_file(
   ${DATA_RES_DIR}/version.h.in 
   ${CMAKE_CURRENT_SOURCE_DIR}/include/version.h
   @ONLY
)


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
# This is a mount point for packaged filesystem at the target computer
# TODO: this is supposed to become obsolete when fuse dependency is eliminated
#       otherwise it shall be reconsidered 
set(FS_MOUNT_POINT "/home/tebako")

# Clean previous run if any
file(REMOVE_RECURSE ${DATA_SRC_DIR})
file(REMOVE_RECURSE ${DATA_PRE_DIR})
file(REMOVE_RECURSE ${DATA_BIN_DIR})
file(REMOVE ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp)

# ...................................................................
# External projects

def_ext_prj_t(RUBY     "2.7.4" "3043099089608859fc8cce7f9fdccaa1f53a462457e3838ec3b25a7d609fbc5b")
string(SUBSTRING ${RUBY_VER} 0 3 RUBY_VER_BASE)
string(CONCAT RUBY_API_VER ${RUBY_VER_BASE} ".0")
def_ext_prj_t(ZLIB     "1.2.11" "c3e5e9fdd5004dcb542feda5ee4f0ff0744628baf8ed2dd5d66f8ca1197cb1a1")
def_ext_prj_t(GDBM     "1.13"   "9d252cbd7d793f7b12bcceaddda98d257c14f4d1890d851c386c37207000a253")
def_ext_prj_t(LIBFFI   "3.2.1"  "980ca30a8d76f963fca722432b1fe5af77d7a4e4d2eac5144fbc5374d4c596609a293440573f4294207e1bdd9fda80ad1e1cafb2ffb543df5a275bc3bd546483")
def_ext_prj_t(NCURSES  "6.2"    "30306e0c76e0f9f1f0de987cf1c82a5c21e1ce6568b9227f7da5b71cbea86c9d")
def_ext_prj_t(OPENSSL  "1.1.0h" "5835626cde9e99656585fc7aaa2302a73a7e1340bf8c14fd635a62c66802a517")
string(SUBSTRING ${OPENSSL_VER} 0 5 OPENSSL_VER_BASE)
def_ext_prj_t(READLINE "7.0"    "750d437185286f40a369e1e4f4764eda932b9459b5ec9a731628393dd3d32334")
def_ext_prj_t(YAML     "0.1.7"  "8088e457264a98ba451a90b8661fcb4f9d6f478f7265d48322a196cec2480729")
def_ext_prj_t(BUNDLER  "2.2.3"  "6acefda4aeb34cb3d69aff06affce10424d69f484402a9f7f5577e8c698070db")
def_ext_prj_g(DWARFS "78401c3")
def_ext_prj_g(INCBIN "348e36b")


message("Collecting packages:")
message("ruby     - " v${RUBY_VER}           " at " ${RUBY_SOURCE_DIR})
message("zlib     - " v${ZLIB_VER}           " at " ${ZLIB_SOURCE_DIR})
message("gdbm     - " v${GDBM_VER}           " at " ${GDBM_SOURCE_DIR})
message("libffi   - " v${LIBFFI_VER}         " at " ${LIBFFI_SOURCE_DIR})
message("ncurses  - " v${NCURSES_VER}        " at " ${NCURSES_SOURCE_DIR})
message("openssl  - " v${OPENSSL_VER}        " at " ${OPENSSL_SOURCE_DIR})
message("readline - " v${READLINE_VER}       " at " ${READLINE_SOURCE_DIR}) 
message("yaml     - " v${YAML_VER}           " at " ${YAML_SOURCE_DIR})
message("bundler  - " v${BUNDLER_VER}        " at " ${BUNDLER_SOURCE_DIR}) 
message("incbin   - " @${INCBIN_TAG}         " at " ${INCBIN_SOURCE_DIR}) 
message("dwarfs   - " @${DWARFS_TAG}         " at " ${DWARFS_SOURCE_DIR}) 
message("................................................")

# ...................................................................
# incbin
# ExternalProject_Add is used for incbin project since we do not need 
# anything at configure step. We want to fetch some files before the 
# build, that's it.  

ExternalProject_Add(${INCBIN_PRJ}
   PREFIX ${DEPS}
   GIT_REPOSITORY https://github.com/graphitemaster/incbin.git
   GIT_TAG ${INCBIN_TAG}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ""
)

# ...................................................................
# Ruby dependencies

ExternalProject_Add(${ZLIB_PRJ}
   PREFIX ${DEPS}
   URL https://zlib.net/zlib-${ZLIB_VER}.tar.gz
   URL_HASH SHA256=${ZLIB_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${ZLIB_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND cd ${ZLIB_SOURCE_DIR} && 
                     ./configure --static --prefix=${DEPS}
   BUILD_COMMAND      cd ${ZLIB_SOURCE_DIR} && make 
   INSTALL_COMMAND    cd ${ZLIB_SOURCE_DIR} && make install
)

ExternalProject_Add(${OPENSSL_PRJ}
   PREFIX ${DEPS}
   URL https://www.openssl.org/source/old/${OPENSSL_VER_BASE}/openssl-${OPENSSL_VER}.tar.gz
   URL_HASH SHA256=${OPENSSL_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${OPENSSL_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND cd ${OPENSSL_SOURCE_DIR} && ./config no-shared --prefix=${DEPS}
   BUILD_COMMAND     cd ${OPENSSL_SOURCE_DIR} && export LD_LIBRARY_PATH="${DEPS}/lib:$LD_LIBRARY_PATH" && make 
   INSTALL_COMMAND   cd ${OPENSSL_SOURCE_DIR} && make install
)

ExternalProject_Add(${GDBM_PRJ}
   PREFIX ${DEPS}
   URL https://ftp.gnu.org/pub/gnu/gdbm/gdbm-${GDBM_VER}.tar.gz
   URL_HASH SHA256=${GDBM_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${GDBM_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND cd ${GDBM_SOURCE_DIR} && 
                     ./configure --with-pic --enable-libgdbm-compat --disable-shared 
                                 --enable-static --without-readline --prefix=${DEPS}
   BUILD_COMMAND     cd ${GDBM_SOURCE_DIR} && make 
   INSTALL_COMMAND   cd ${GDBM_SOURCE_DIR} && make install
)

ExternalProject_Add(${YAML_PRJ}
   PREFIX ${DEPS}
   URL https://pyyaml.org/download/libyaml/yaml-${YAML_VER}.tar.gz
   URL_HASH SHA256=${YAML_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${YAML_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND ${YAML_SOURCE_DIR}/configure --with-pic --disable-shared --enable-static 
                     --prefix=${DEPS} --srcdir=${YAML_SOURCE_DIR}
   BUILD_COMMAND     make 
   INSTALL_COMMAND   make install
)

ExternalProject_Add(${LIBFFI_PRJ}
   PREFIX ${DEPS}
   URL https://gcc.gnu.org/pub/libffi/libffi-${LIBFFI_VER}.tar.gz
   URL_HASH SHA512=${LIBFFI_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${LIBFFI_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND ${LIBFFI_SOURCE_DIR}/configure --with-pic --disable-shared --enable-static 
                                 --srcdir=${LIBFFI_SOURCE_DIR} --prefix=${DEPS}
   BUILD_COMMAND     make 
   INSTALL_COMMAND   make install
)

ExternalProject_Add(${NCURSES_PRJ}
   PREFIX ${DEPS}
   URL https://ftp.gnu.org/pub/gnu/ncurses/ncurses-${NCURSES_VER}.tar.gz
   URL_HASH SHA256=${NCURSES_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${NCURSES_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND ${NCURSES_SOURCE_DIR}/configure --without-shared --without-cxx-shared 
                     --prefix=${DEPS} --srcdir=${NCURSES_SOURCE_DIR}
   BUILD_COMMAND     make 
   INSTALL_COMMAND   make install
)

ExternalProject_Add(${READLINE_PRJ}
   PREFIX ${DEPS}
   URL https://ftp.gnu.org/gnu/readline/readline-${READLINE_VER}.tar.gz
   URL_HASH SHA256=${READLINE_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${READLINE_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND ${READLINE_SOURCE_DIR}/configure --disable-shared --enable-static 
                     --srcdir=${READLINE_SOURCE_DIR} --prefix=${DEPS} cflags=-fPIC
   BUILD_COMMAND     make 
   INSTALL_COMMAND   make install
)

# ...................................................................
# Ruby

set(RBD ${DEPS}/bin)
set(RID ${DEPS}/include)
set(RLD ${DEPS}/lib)

if(RUBY_DEBUG_BUILD)
  set(RUBY_C_FLAGS_1 "-fPIC -g -O0 -pipe")
else()
  set(RUBY_C_FLAGS_1 "-fPIC -O3 -fno-fast-math -ggdb3 -Os -fdata-sections -ffunction-sections -pipe")
endif()
set(RUBY_C_FLAGS_2 "-I${RID} -I${RID}/${NCURSES_NAME} -I${RID}/${OPENSSL_NAME} \
                  -I${RID}/${READLINE_NAME} -I${RLD}/${LIBFFI_NAME}-${LIBFFI_VER}/include")
string(CONCAT RUBY_C_FLAGS ${RUBY_C_FLAGS_1} " " ${RUBY_C_FLAGS_2})
set(RUBY_L_FLAGS "-L${RLD}")

#   COMMAND           sed -i "s/\#option nodynamic/option nodynamic/g" ${RUBY_SOURCE_DIR}/ext/Setup
#   --disable-shared
#   --with-vendordir = ${RLD}

ExternalProject_Add(${RUBY_PRJ}
   PREFIX ${DEPS}
   URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
   URL_HASH SHA256=${RUBY_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${RUBY_SOURCE_DIR}
   BINARY_DIR ${RUBY_BINARY_DIR}
   CONFIGURE_COMMAND ${RUBY_SOURCE_DIR}/configure 
                     --enable-bundled-libyaml 
                     --without-gmp 
                     --disable-dtrace 
                     --enable-debug-env 
                     --disable-install-doc
                     --with-static-linked-ext
                     --srcdir=${RUBY_SOURCE_DIR} 
                     --prefix=${DATA_SRC_DIR}
                     cflags=${RUBY_C_FLAGS} 
                     LDFLAGS=${RUBY_L_FLAGS}
   INSTALL_COMMAND ""  
)
 
add_dependencies(${RUBY_PRJ} ${ZLIB_PRJ} ${GDBM_PRJ} ${YAML_PRJ} ${LIBFFI_PRJ} 
                             ${NCURSES_PRJ} ${READLINE_PRJ} ${OPENSSL_PRJ})


set (TBD ${DATA_SRC_DIR}/bin)
set (TLD ${DATA_SRC_DIR}/local)
# TGD folder is used to install gems 
set (TGD ${DATA_SRC_DIR}/lib/ruby/gems/${RUBY_API_VER})

file(GLOB GEMSPECS LIST_DIRECTORIES false ${ROOT}/*.gemspec)
file(GLOB GEMFILES LIST_DIRECTORIES false ${ROOT}/Gemfile)
file(GLOB GEMS     LIST_DIRECTORIES false ${ROOT}/*.gem)

list(LENGTH GEMSPECS GSLENGTH)
list(LENGTH GEMS     GLENGS)

if(GSLENGTH GREATER 0)
   if(GSLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gemspecs detected in ${ROOT}")
   else(GSLENGTH GREATER 1)
      list(GET GEMSPECS 0 GEMSPEC)
      message("Collecting gem from gemspec ${GEMSPEC}")
      
      list(LENGTH GEMFILES GFLENGTH)
      if(GFLENGTH GREATER 0)
        ExternalProject_Add(${BUNDLER_PRJ}
          PREFIX ${DEPS}
          URL https://rubygems.org/downloads/bundler-${BUNDLER_VER}.gem
          URL_HASH SHA256=${BUNDLER_HASH}
          DOWNLOAD_NO_PROGRESS true
          SOURCE_DIR ${BUNDLER_SOURCE_DIR}
          UPDATE_COMMAND ""
          CONFIGURE_COMMAND ""
          BUILD_COMMAND ""
          INSTALL_COMMAND ${DEPS}\bin\gem install 
                          ${BUNDLER_SOURCE_DIR}/bundler-${BUNDLER_VER}.gem
                          --verbose --no-document
                          --install-dir ${TGD}
         )
      else(GFLENGTH GREATER 0)
#  Found x.gemspec but no Gemfile
         set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
         message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
         add_custom_target(_SOURCE_FILESYSTEM  
            COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make install
            COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_PRE_DIR}
            COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${DATA_PRE_DIR}
            COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && ${TBD}/gem build ${GEMSPEC}
            COMMAND ${CMAKE_COMMAND} -E make_directory ${TGD}
            COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && ${TBD}/gem install *.gem --verbose --no-document --install-dir ${TGD}
            COMMAND ${CMAKE_COMMAND} -E touch ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                    (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist" &&
                     ${CMAKE_COMMAND} -E false )
            DEPENDS ${RUBY_PRJ} 
         )
      endif()
   endif()  
elseif(GLENGTH GREATER 0)
# xxx.gem, no gemspec, nogemfile
   if(GLENGTH GREATER 1)
      message(FATAL_ERROR "Multiple gems detected in ${ROOT}")
   else()
      set(FS_ENTRY_POINT "/bin/${FS_ENTRANCE}")
      message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
      add_custom_target(_SOURCE_FILESYSTEM  
          COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make install
          COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_PRE_DIR}
          COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${DATA_PRE_DIR}
          COMMAND unset GEM_HOME && unset GEM_PATH && cd ${DATA_PRE_DIR} && ${TBD}/gem install *.gem --verbose --no-document --install-dir ${TGD}
          COMMAND ${CMAKE_COMMAND} -E touch ${DATA_SRC_DIR}${FS_ENTRY_POINT} ||
                  (${CMAKE_COMMAND} -E echo "Entry point ${DATA_SRC_DIR}${FS_ENTRY_POINT} does not exist" &&
                   ${CMAKE_COMMAND} -E false )
          DEPENDS ${RUBY_PRJ} 
         )
   endif()
else()
# No gemspecs
# Simple ruby script
   message("Collecting simple Ruby script from ${FS_ROOT}") 
   cmake_path(IS_ABSOLUTE FS_ENTRANCE IS_EN_ABSOLUTE)
   if(${IS_EN_ABSOLUTE})
      cmake_path(IS_PREFIX FS_ROOT ${FS_ENTRANCE} IS_FS_EN_PREFIX)
      if(NOT ${IS_FS_EN_PREFIX})
         message(FATAL_ERROR "Entry point ${FS_ENTRANCE} is not in the project root ${FS_ROOT}")
      endif(NOT ${IS_FS_EN_PREFIX})
      cmake_path(RELATIVE_PATH FS_ENTRANCE BASE_DIRECTORY ${FS_ROOT})
   endif(${IS_EN_ABSOLUTE})
   if(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
         message(FATAL_ERROR "Entry point ${FS_ROOT}/${FS_ENTRANCE} does not exist or is not accessible")
   endif(NOT EXISTS ${FS_ROOT}/${FS_ENTRANCE})
   set(FS_ENTRY_POINT "/local/${FS_ENTRANCE}")
   message("Target entry point will be at ${FS_MOUNT_POINT}${FS_ENTRY_POINT}")
   add_custom_target(_SOURCE_FILESYSTEM  
            COMMAND ${CMAKE_COMMAND} -E chdir ${RUBY_BINARY_DIR} make install
            COMMAND ${CMAKE_COMMAND} -E make_directory ${TLD}
            COMMAND ${CMAKE_COMMAND} -E copy_directory ${FS_ROOT} ${TLD}
            DEPENDS ${RUBY_PRJ}
   )
endif()

# ...................................................................
# DwarFS

# ...................................................................
# The libraries that are build by DwarFS project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light
# These forward-declarations and BUILD_BYPRODICTS are required to support 'Ninja'
# Otherwise add_dependencies would be enough for 'Unix makefiles' generator

set(__LIBDWARFS "${DWARFS_BINARY_DIR}/libdwarfs.a")
set(__LIBFOLLY "${DWARFS_BINARY_DIR}/folly/libfolly.a")
set(__LIBFSST "${DWARFS_BINARY_DIR}/libfsst.a")
set(__LIBT_METADATA "${DWARFS_BINARY_DIR}/libmetadata_thrift.a")
set(__LIBT_LIGHT "${DWARFS_BINARY_DIR}/libthrift_light.a")
set(__LIBXXHASH "${DWARFS_BINARY_DIR}/libxxhash.a")
set(__LIBZSTD "${DWARFS_BINARY_DIR}/zstd/build/cmake/lib/libzstd.a")
set(__LIBARCHIVE "${DWARFS_BINARY_DIR}/libarchive/lib/libarchive.a")

ExternalProject_Add(${DWARFS_PRJ}
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/tamatebako/dwarfs.git
    GIT_TAG ${DWARFS_TAG}
    PATCH_COMMAND ${CMAKE_COMMAND} -E copy_if_different ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.cpp ${DEPS}/src/_dwarfs/src/dwarfs  
    COMMAND       ${CMAKE_COMMAND} -E copy_if_different ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.h ${DEPS}/src/_dwarfs/include/dwarfs   
    UPDATE_COMMAND ""
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} -DSTATIC_BUILD_DO_NOT_USE:BOOL=ON
    SOURCE_DIR ${DWARFS_SOURCE_DIR}
    BINARY_DIR ${DWARFS_BINARY_DIR}
    BUILD_BYPRODUCTS ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA} 
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
)

# ...................................................................
# Packaged filesystem

add_custom_target(_PACKAGED_FILESYSTEM  
   COMMAND ${CMAKE_COMMAND} -E rm -rf ${DATA_SRC_DIR}/share   ${DATA_SRC_DIR}/include  ${DATA_SRC_DIR}/lib/*.a ${DATA_SRC_DIR}/lib/pkgconfig
#                                      ${DATA_SRC_DIR}/bin/b*  ${DATA_SRC_DIR}/bin/erb  ${DATA_SRC_DIR}/bin/gem ${DATA_SRC_DIR}/bin/irb 
#                                      ${DATA_SRC_DIR}/bin/ra* ${DATA_SRC_DIR}/bin/rdoc ${DATA_SRC_DIR}/bin/ri  ${DATA_SRC_DIR}/bin/y2racc
   COMMAND ${CMAKE_COMMAND} -E make_directory ${DATA_BIN_DIR}
   COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${DATA_SRC_DIR}
   DEPENDS ${DWARFS_PRJ} _SOURCE_FILESYSTEM
   BYPRODUCTS ${DATA_BIN_FILE}
)

configure_file(
   ${DATA_RES_DIR}/tebako-fs.cpp.in 
   ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
   @ONLY
)

# ...................................................................
# Executable

add_compile_options(-fdiagnostics-color=always)
add_link_options(-static -static-libgcc)
set(CMAKE_C_FLAGS_RELEASE "-DNDEBUG -O2 -g")
set(CMAKE_CXX_FLAGS_RELEASE "-DNDEBUG -O2 -g")

include_directories(${CMAKE_CURRENT_SOURCE_DIR}/include
                    ${INCBIN_SOURCE_DIR}
                    ${DWARFS_SOURCE_DIR}/folly ${DWARFS_BINARY_DIR}/folly ${DWARFS_SOURCE_DIR}/fbthrift 
                    ${DWARFS_BINARY_DIR}/thrift ${DWARFS_BINARY_DIR} ${DWARFS_SOURCE_DIR}/include)
add_compile_definitions(FUSE_USE_VERSION=35)

add_executable(
    tebako 
    "src/tebako.cpp"
    "src/tebako-dfs.cpp"
    "src/tebako-dfs-ops.cpp"
    "src/tebako-fs.cpp"
    "src/tebako-mfs.cpp"
    "include/tebako-fs.h"
    "include/tebako-dfs.h"
    "include/tebako-mfs.h"
    "include/version.h"
    )

add_dependencies(tebako _PACKAGED_FILESYSTEM  ${INCBIN_PRJ} ${DWARFS_PRJ})
set_property(TARGET tebako PROPERTY CXX_STANDARD 17)
set_property(TARGET tebako PROPERTY CXX_STANDARD_REQUIRED ON)
set_property(TARGET tebako PROPERTY CXX_EXTENSIONS OFF)

# ...................................................................
# The libraries that are build by dwarfs project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light libarchive

function(ADD_DWARFS_LIBRARY NAME PRJ)
  add_library(${NAME} STATIC IMPORTED)
  set_target_properties(${NAME} PROPERTIES IMPORTED_LOCATION ${_${NAME}})
  add_dependencies(${NAME} ${PRJ})
  target_link_libraries(tebako ${NAME})
endfunction()

add_dwarfs_library(_LIBDWARFS ${DWARFS_PRJ})
add_dwarfs_library(_LIBFSST ${DWARFS_PRJ})
add_dwarfs_library(_LIBFOLLY ${DWARFS_PRJ})
add_dwarfs_library(_LIBT_METADATA ${DWARFS_PRJ})
add_dwarfs_library(_LIBT_LIGHT ${DWARFS_PRJ})
add_dwarfs_library(_LIBXXHASH ${DWARFS_PRJ})
add_dwarfs_library(_LIBZSTD ${DWARFS_PRJ})
add_dwarfs_library(_LIBARCHIVE ${DWARFS_PRJ})

# ...................................................................
# System libraries
# Each library name given to the NAMES option is first considered
# as a library file name and then considered with platform-specific 
# prefixes (e.g. lib) and suffixes (e.g. .so). Therefore one may 
# specify library file names such as libfoo.a directly. This can be 
# used to locate static libraries on UNIX-like systems.
#

find_package(Boost 1.67 REQUIRED COMPONENTS context regex thread atomic date_time filesystem program_options system)

find_library(_LIBGLOG "libglog.a" REQUIRED)
find_library(_LIBFMT "libfmt.a" REQUIRED)
find_library(_LIBDC "libdouble-conversion.a" REQUIRED)
find_library(_LIBGFLAGS "libgflags.a" REQUIRED)
find_library(_LIBEVENT "libevent.a" REQUIRED)
find_library(_LIBACL "libacl.a" REQUIRED) 
find_library(_LIBCRYPTO "libcrypto.a" REQUIRED) 
find_library(_LIBLZ4 "liblz4.a" REQUIRED) 
find_library(_LIBLZMA "liblzma.a" REQUIRED) 
find_library(_LIBZ "libz.a" REQUIRED) 
find_library(_LIBLJEMALLOC "libjemalloc.a" REQUIRED) 
find_library(_LIBPTHREAD "libpthread.a" REQUIRED) 
find_library(_LIBFUSE "libfuse3.a" REQUIRED) 
find_library(_LIBDL "libdl.a" REQUIRED) 
find_library(_LIBC "libc.a" REQUIRED)  
find_library(_LIBM "libm.a" REQUIRED)
find_library(_LIBRT "librt.a" REQUIRED)
find_library(_LIBSSL "libssl.a" REQUIRED)
find_library(_LIBUNWIND "libunwind.a" REQUIRED)
find_library(_LIBIBERTY "libiberty.a" REQUIRED)

# ...................................................................
#  IMPORTANT !!!
#  MUST be in this order otherwise link may fail
#  -static-libgcc and gcc_eh below is all together an ugly trick to enforce static linking

target_link_libraries(tebako                              
  ${_LIBFMT}          ${Boost_LIBRARIES}  ${_LIBDC}           ${_LIBFUSE}                    
  ${_LIBGLOG}         ${_LIBGFLAGS}       ${_LIBEVENT}        ${_LIBIBERTY}
  ${_LIBACL}          ${_LIBSSL}          ${_LIBCRYPTO}       ${_LIBLZ4}
  ${_LIBZ}            ${_LIBJEMALLOC}     ${_LIBPTHREAD}      ${_LIBDL}           
  ${_LIBC}            ${_LIBM}            ${_LIBRT}           gcc_eh		                 
  ${_LIBUNWIND}       ${_LIBLZMA}
 )

# ...................................................................
#  Install
#
if(NOT DEVELOPER_MODE)
install(
  TARGETS tebako
  RUNTIME DESTINATION bin
 )
endif()

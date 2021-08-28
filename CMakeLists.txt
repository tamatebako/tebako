#
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

cmake_minimum_required (VERSION 3.13.4)
project(tebako_packager)
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

# Developer mode removes the burden of nested projects which takes a lot under WSL/VisualStudio/Ninja environment
# Use with caution, please
option(DEVELOPER_MODE "Developer mode" OFF)

set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps CACHE STRING "Dependencies' folder'")

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

set(Boost_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)



set(INCBIN_TAG "348e36b" CACHE STRING "incbin git tag")
set(DWARFS_TAG "78401c3" CACHE STRING "dwarFS git tag")

# ...................................................................
# DEF_VENDOR_PRJ
# This function defines version number and archieve hash for
# vendor code that is required to build Ruby
# For external project XXX this function will set the following variables:
# XXX_VER                -- package version
# XXX_HASH               -- tarball hash
# Version and hash value come from environment variables (GHA case)
# If environment variables are not set then default values are used
# XXX_NAME = xxx  (lowercase)   
# XXX_PRJ  = _xxx (lowercase)       -- CMake ExternalProject name
# XXX_SOURCE_DIR = ${DEPS}/src/_xxx -- Source folder for ExternalProject_Add 



function(DEF_VENDOR_PRJ NAME DEFAULT_VER DEFAULT_HASH)
  set(VER ${NAME}_VER)
  if (DEFINED ENV{${VER}})
    set(${VER} $ENV{${VER}} PARENT_SCOPE)
  else ()
    set(${VER} ${DEFAULT_VER} PARENT_SCOPE)
  endif()

  set(HASH ${NAME}_HASH)
  if (DEFINED ENV{${HASH})
    set(${HASH} $ENV{${HASH}} PARENT_SCOPE)
  else()
    set(${HASH} ${DEFAULT_HASH} PARENT_SCOPE)
  endif()
 
  set(R_NAME ${NAME}_NAME)
  string(TOLOWER ${NAME} TMP)
  set(${R_NAME} ${TMP} PARENT_SCOPE)
  set(R_PRJ  ${NAME}_PRJ)
  set(${R_PRJ} _${TMP} PARENT_SCOPE)
  set(R_DIR ${NAME}_SOURCE_DIR)
  set(${R_DIR} ${DEPS}/src/_${TMP} PARENT_SCOPE)
endfunction()

def_vendor_prj(RUBY     "2.6.3" "2347ed6ca5490a104ebd5684d2b9b5eefa6cd33c")
string(SUBSTRING ${RUBY_VER} 0 3 RUBY_VER_BASE)
def_vendor_prj(ZLIB     "1.2.11" "c3e5e9fdd5004dcb542feda5ee4f0ff0744628baf8ed2dd5d66f8ca1197cb1a1")
def_vendor_prj(GDBM     "1.13"   "9d252cbd7d793f7b12bcceaddda98d257c14f4d1890d851c386c37207000a253")
def_vendor_prj(LIBFFI   "3.2.1"  "980ca30a8d76f963fca722432b1fe5af77d7a4e4d2eac5144fbc5374d4c596609a293440573f4294207e1bdd9fda80ad1e1cafb2ffb543df5a275bc3bd546483")
def_vendor_prj(NCURSES  "6.2"    "30306e0c76e0f9f1f0de987cf1c82a5c21e1ce6568b9227f7da5b71cbea86c9d")
def_vendor_prj(OPENSSL  "1.1.0h" "5835626cde9e99656585fc7aaa2302a73a7e1340bf8c14fd635a62c66802a517")
#def_vendor_prj(OPENSSL  "1.1.0f" "12f746f3f2493b2f39da7ecf63d7ee19c6ac9ec6a4fcd8c229da8a522cb12765")
string(SUBSTRING ${OPENSSL_VER} 0 5 OPENSSL_VER_BASE)
def_vendor_prj(READLINE "7.0"    "750d437185286f40a369e1e4f4764eda932b9459b5ec9a731628393dd3d32334")
def_vendor_prj(YAML     "0.1.7"  "8088e457264a98ba451a90b8661fcb4f9d6f478f7265d48322a196cec2480729")
def_vendor_prj(BUNDLER  "2.2.3"  "6acefda4aeb34cb3d69aff06affce10424d69f484402a9f7f5577e8c698070db")

message("Collecting packages:")
message("ruby     - " ${RUBY_VER}           " at " ${RUBY_SOURCE_DIR})
message("zlib     - " ${ZLIB_VER}           " at " ${ZLIB_SOURCE_DIR})
message("gdbm     - " ${GDBM_VER}           " at " ${GDBM_SOURCE_DIR})
message("libffi   - " ${LIBFFI_VER}         " at " ${LIBFFI_SOURCE_DIR})
message("ncurses  - " ${NCURSES_VER}        " at " ${NCURSES_SOURCE_DIR})
message("openssl  - " ${OPENSSL_VER}        " at " ${OPENSSL_SOURCE_DIR})
message("readline - " ${READLINE_VER}       " at " ${READLINE_SOURCE_DIR}) 
message("yaml     - " ${YAML_VER}           " at " ${YAML_SOURCE_DIR})
message("bundler  - " ${BUNDLER_VER}        " at " ${BUNDLER_SOURCE_DIR}) 

# ...................................................................
# Ruby

 ExternalProject_Add(${RUBY_PRJ}
   PREFIX ${DEPS}
   URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
   URL_HASH SHA1=${RUBY_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${RUBY_SOURCE_DIR}
   CONFIGURE_COMMAND ${RUBY_SOURCE_DIR}/configure --srcdir=${RUBY_SOURCE_DIR} --prefix=${CMAKE_CURRENT_SOURCE_DIR}/resources/fs-1/ruby
 )


#set (R_BUNDLER_NAME bundler)
#set (R_BUNDLER_PRJ _${R_BUNDLER_NAME})
#set (R_BUNDLER_SOURCE_DIR ${DEPS}/src/${R_BUNDLER_PRJ})

# ExternalProject_Add(${R_BUNDLER_PRJ}
#   PREFIX ${DEPS}
#   URL https://rubygems.org/downloads/bundler-$(BUNDLER_VER).gem
#   URL_HASH SHA256=${R_BUNDLER_HASH}
#   DOWNLOAD_NO_PROGRESS true
#   SOURCE_DIR ${R_BUNDLER_SOURCE_DIR}
#   CONFIGURE_COMMAND ${R_BUNDLER_SOURCE_DIR}/configure --srcdir=${R_BUNDLER_SOURCE_DIR} --prefix=${CMAKE_CURRENT_SOURCE_DIR}/resources/fs-1/ruby
# )

 ExternalProject_Add(${ZLIB_PRJ}
   PREFIX ${DEPS}
   URL https://zlib.net/zlib-${ZLIB_VER}.tar.gz
   URL_HASH SHA256=${ZLIB_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${ZLIB_SOURCE_DIR}
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
   CONFIGURE_COMMAND cd ${OPENSSL_SOURCE_DIR} && ./config --prefix=${DEPS}
   BUILD_COMMAND     cd ${OPENSSL_SOURCE_DIR} && export LD_LIBRARY_PATH="${DEPS}/lib:$LD_LIBRARY_PATH" && make 
   INSTALL_COMMAND   "" #cd ${OPENSSL_SOURCE_DIR} && make install
 )

function (add_vendor_prj PRJ URL HASH DIR CFG)
  ExternalProject_Add(${PRJ}
   PREFIX ${DEPS}
   URL    ${URL}
   URL_HASH SHA256=${HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND cd ${DIR} && ./configure ${CFG}  --prefix=${DEPS}
   BUILD_COMMAND     cd ${DIR} && make 
   INSTALL_COMMAND   cd ${DIR} && make install
 )
endfunction()

 #add_vendor_prj(${GDBM_PRJ} "https://ftp.gnu.org/pub/gnu/gdbm/gdbm-${GDBM_VER}.tar.gz"
 #                ${GDBM_HASH} ${GDBM_SOURCE_DIR}
 #                " --with-pic --enable-libgdbm-compat --disable-shared --enable-static --without-readline")

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
   CONFIGURE_COMMAND cd ${YAML_SOURCE_DIR} && 
                     ./configure --with-pic --disable-shared --enable-static --prefix=${DEPS}
   BUILD_COMMAND     cd ${YAML_SOURCE_DIR} && make 
   INSTALL_COMMAND   cd ${YAML_SOURCE_DIR} && make install
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
   CONFIGURE_COMMAND cd ${NCURSES_SOURCE_DIR} && 
                     ./configure --without-shared --without-cxx-shared --prefix=${DEPS}
   BUILD_COMMAND     cd ${NCURSES_SOURCE_DIR} && make 
   INSTALL_COMMAND   cd ${NCURSES_SOURCE_DIR} && make install
 )

 ExternalProject_Add(${READLINE_PRJ}
   PREFIX ${DEPS}
   URL https://ftp.gnu.org/gnu/readline/readline-${READLINE_VER}.tar.gz
   URL_HASH SHA256=${READLINE_HASH}
   DOWNLOAD_NO_PROGRESS true
   SOURCE_DIR ${READLINE_SOURCE_DIR}
   UPDATE_COMMAND ""
   CONFIGURE_COMMAND cd ${READLINE_SOURCE_DIR} && 
                     ./configure --with-pic --disable-shared --enable-static --prefix=${DEPS}
   BUILD_COMMAND     cd ${READLINE_SOURCE_DIR} && make 
   INSTALL_COMMAND   cd ${READLINE_SOURCE_DIR} && make install
 )


 add_dependencies(${RUBY_PRJ} ${ZLIB_PRJ} ${GDBM_PRJ} ${YAML_PRJ} ${LIBFFI_PRJ} 
                              ${NCURSES_PRJ} ${READLINE_PRJ} ${OPENSSL_PRJ})


# ...................................................................
# incbin
# ExternalProject_Add is used for incbin project since we do not need 
# anything at configure step. We want to fetch some files before the 
# build, that's it.  

set (INCBIN_NAME incbin)
set (INCBIN_PRJ _${INCBIN_NAME})
set (INCBIN_SOURCE_DIR ${DEPS}/src/${INCBIN_PRJ})

if(NOT DEVELOPER_MODE)
 ExternalProject_Add(${INCBIN_PRJ}
   PREFIX ${DEPS}
   GIT_REPOSITORY https://github.com/graphitemaster/incbin.git
   GIT_TAG ${INCBIN_TAG}
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ""
 )
endif(NOT DEVELOPER_MODE)

# ...................................................................
# DwarFS
# ExternakProject_Add is used for incbin project since we do not need 
# anything at configure step. We want to fetch some files before the 
# build. that;s it.


set (DWARFS_NAME dwarfs)
set (DWARFS_PRJ _${DWARFS_NAME})
set (DWARFS_SOURCE_DIR ${DEPS}/src/${DWARFS_PRJ})
set (DWARFS_BINARY_DIR ${DEPS}/src/${DWARFS_PRJ}-build/)

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

if(NOT DEVELOPER_MODE)
 ExternalProject_Add(${DWARFS_PRJ}
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/tamatebako/dwarfs.git
    GIT_TAG ${DWARFS_TAG}
    PATCH_COMMAND cp ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.cpp ${DEPS}/src/_dwarfs/src/dwarfs && 
                  cp ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.h ${DEPS}/src/_dwarfs/include/dwarfs   
    CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} -DSTATIC_BUILD_DO_NOT_USE:BOOL=ON
    SOURCE_DIR ${DWARFS_SOURCE_DIR}
    BINARY_DIR ${DWARFS_BINARY_DIR}
    BUILD_BYPRODUCTS ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA} 
                     ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
 )
endif(NOT DEVELOPER_MODE)

# ...................................................................
# Packaged filesystem
# TODO: the chain of dependencies for fs is not correct and requires rethinking

set(DATA_BIN_DIR  ${CMAKE_CURRENT_BINARY_DIR}/packaged_filesystem)
set(DATA_BIN_FILE ${DATA_BIN_DIR}/fs.bin)
set(FS_MOUNT_POINT "/home/tebako")

add_custom_target(_PACKAGED_FILESYSTEM 
   COMMAND mkdir -p ${DATA_BIN_DIR}
   COMMAND ${DEPS_BIN_DIR}/mkdwarfs -o ${DATA_BIN_FILE} -i ${CMAKE_CURRENT_SOURCE_DIR}/resources/fs-0
   BYPRODUCTS ${DATA_BIN_FILE}
)

if(NOT DEVELOPER_MODE)
 add_dependencies(_PACKAGED_FILESYSTEM ${DWARFS_PRJ} ${INCBIN_PRJ})
endif(NOT DEVELOPER_MODE)
 add_dependencies(_PACKAGED_FILESYSTEM ${RUBY_PRJ})

configure_file(
   ${CMAKE_CURRENT_SOURCE_DIR}/resources/tebako-fs.cpp.in 
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

if(NOT DEVELOPER_MODE)
 add_dependencies(tebako ${DWARFS_PRJ})
endif(NOT DEVELOPER_MODE)

add_dependencies(tebako _PACKAGED_FILESYSTEM)
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

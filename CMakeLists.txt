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
include(FetchContent)

# Developer mode removes the burden of nested projects which takes a lot under WSL/VisualStudio/Ninja environment
# Use with caution, please
option(DEVELOPER_MODE "Developer mode" OFF)


option(INCBIN_TAG "incbin git tag" "348e36b")
option(DWARFS_TAG "dwarfs git tag" "78401c3")

if(NOT DEFINED(DEPS))
 set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps)
endif()

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)
set(DEPS_BIN_DIR ${DEPS}/bin)
set(DEPS_SBIN_DIR ${DEPS}/sbin)

set(Boost_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)

# ...................................................................
# Ruby

set (RUBY_NAME ruby)
set (RUBY_PRJ _${RUBY_NAME})
set (RUBY_SOURCE_DIR ${DEPS}/src/${RUBY_PRJ})

 ExternalProject_Add(${RUBY_PRJ}
   PREFIX ${DEPS}
   URL https://cache.ruby-lang.org/pub/ruby/${RUBY_VER_BASE}/ruby-${RUBY_VER}.tar.gz
   URL_HASH SHA1=${RUBY_HASH}
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ""
 )


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

# ...................................................................

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
   GIT_TAG 348e36b
   CONFIGURE_COMMAND ""
   BUILD_COMMAND ""
   INSTALL_COMMAND ""
 )
endif(NOT DEVELOPER_MODE)

# ...................................................................
# Each library name given to the NAMES option is first considered
# as a library file name and then considered with platform-specific 
# prefixes (e.g. lib) and suffixes (e.g. .so). Therefore one may 
# specify library file names such as libfoo.a directly. This can be 
# used to locate static libraries on UNIX-like systems.
#
#        sudo apt-get install \
#        ronn binutils-dev libarchive-dev libevent-dev libjemalloc-dev acl-dev \
#        libdouble-conversion-dev libiberty-dev liblz4-dev liblzma-dev libssl-dev \
#        libboost-context-dev libboost-filesystem-dev libboost-program-options-dev \
#        libboost-python-dev libboost-regex-dev libboost-system-dev libboost-thread-dev \
#        libunwind-dev libdwarf-dev libelf-dev libfmt-dev libfuse3-dev libgoogle-glog-dev 
#    ++  Already installed at GHA but may require local installation: bison flex fuse3 pkg-config libxml2-dev libsodium-dev
# ...................................................................

# Check if population has already been performed
#FetchContent_GetProperties(incbin)
#string(TOLOWER "incbin" lcName)
#if(NOT ${lcName}_POPULATED)
#  FetchContent_Populate(incbin)
#endif()

# ...................................................................


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
    GIT_REPOSITORY https://github.com/metanorma/dwarfs.git
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

if(NOT DEVELOPER_MODE)
 ExternalProject_Add(${DWARFS_PRJ}
    PREFIX ${DEPS}
    GIT_REPOSITORY https://github.com/metanorma/dwarfs.git
    GIT_TAG 78401c3
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
 add_dependencies(_PACKAGED_FILESYSTEM ${DWARFS_PRJ})
endif(NOT DEVELOPER_MODE)
 add_dependencies(_PACKAGED_FILESYSTEM ${RUBY_PRJ})

configure_file(
   ${CMAKE_CURRENT_SOURCE_DIR}/resources/tebako-fs.cpp.in 
   ${CMAKE_CURRENT_SOURCE_DIR}/src/tebako-fs.cpp
   @ONLY
)

# ...................................................................
# Executable
# ...................................................................

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

add_executable(tebako-0 src/tebako-0.cpp)
add_dependencies(tebako-0 ${DWARFS_PRJ})
set_property(TARGET tebako-0 PROPERTY CXX_STANDARD 17)
set_property(TARGET tebako-0 PROPERTY CXX_STANDARD_REQUIRED ON)
set_property(TARGET tebako-0 PROPERTY CXX_EXTENSIONS OFF)

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

install(
  TARGETS tebako
  RUNTIME DESTINATION bin
 )

cmake_minimum_required (VERSION 3.12)
include(ExternalProject)

project(tebako)

if(NOT DEFINED(DEPS))
   set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps)
endif()

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)


set (DWARFS_NAME dwarfs)
if(NOT DEFINED(DWARFS_VERSION))
   set (DWARFS_VERSION "0.5.6")
endif()


set (DWARFS_PRJ _${DWARFS_NAME})

ExternalProject_Add(${DWARFS_PRJ}
   PREFIX ${DEPS}
   URL https://github.com/mhx/dwarfs/releases/download/v${DWARFS_VERSION}/dwarfs-${DWARFS_VERSION}.tar.xz
   DOWNLOAD_NO_PROGRESS true
   CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} 
)

## -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE} ???

ExternalProject_Get_property(${DWARFS_PRJ} BINARY_DIR)


# ...................................................................
# The libraries that are build by dwarfs project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light
# ...................................................................
add_library(_LIBDWARFS STATIC IMPORTED)
set_target_properties(_LIBDWARFS PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/libdwarfs.a")

add_library(_LIBFOLLY STATIC IMPORTED)
set_target_properties(_LIBFOLLY PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/folly/libfolly.a")

add_library(_LIBFSST STATIC IMPORTED)
set_target_properties(_LIBFSST PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/libfsst.a")

add_library(_LIBXXHASH STATIC IMPORTED)
set_target_properties(_LIBXXHASH PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/libxxhash.a")

add_library(_LIBT_METADATA STATIC IMPORTED)
set_target_properties(_LIBT_METADATA PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/libmetadata_thrift.a")

add_library(_LIBT_LIGHT STATIC IMPORTED)
set_target_properties(_LIBT_LIGHT PROPERTIES IMPORTED_LOCATION "${BINARY_DIR}/libthrift_light.a")

# zstd/build/cmake/lib/libzstd.a  --> ${libzstd_static} 
# set by child project of ${DWARFS_TARGET}

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

set(Boost_USE_STATIC_LIBS ON) # only find static libs
find_package(Boost REQUIRED COMPONENTS context regex thread date_time filesystem program_options system)

find_library(_LIBFMT "libfmt.a" REQUIRED)
find_library(_LIBDC "libdouble-conversion.a" REQUIRED)
find_library(_LIBGFLAGS "libgflags.a" REQUIRED)
find_library(_LIBEVENT "libevent.a" REQUIRED)
find_library(_LIBARCH "libarchive.a" REQUIRED)
find_library(_LIBACL "libacl.a" REQUIRED) 
find_library(_LIBXML2 "libxml2.a" REQUIRED) 
find_library(_LIBCRYPTO "libcrypto.a" REQUIRED) 
find_library(_LIBIBERTY "libiberty.a" REQUIRED) 
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
# find_library(_LIBEH "libgcc_eh.a" REQUIRED)
#	!/usr/lib/gcc/x86_64-linux-gnu/10/libgcc_eh.a \
find_library(_LIBUNWIND "libunwind.a" REQUIRED)


message(${_LIBLZMA})

add_compile_options(-static -fdiagnostics-color=always)
set(CMAKE_C_FLAGS_RELEASE "-DNDEBUG -O2 -g")
set(CMAKE_CXX_FLAGS_RELEASE "-DNDEBUG -O2 -g")

add_executable(tebako-0 src/tebako-0.cpp)
add_dependencies(tebako-0 ${DWARFS_PRJ})

# ...................................................................
#  IMPORTANT !!!
#  ${_LIBUNWIND} ${_LIBLZMA}  -- MUST be in this order otherwise link step fails
# ...................................................................
target_link_libraries(tebako-0 
  ${_LIBDWARFS}       ${_LIBFOLLY}        ${_LIBXXHASH}       ${libzstd_static} 
  ${_LIBFMT}          ${_LIBFSST}         ${_LIBT_METADATA}   ${_LIBT_LIGHT}
  ${_LIBACL}          ${_LIBDC}           ${_LIBGFLAGS}       ${_LIBEVENT}
  ${_LIBCRYPTO}       ${_LIBIBERTY}       ${_LIBXML2}         ${Boost_LIBRARIES}
  ${_LIBLZ4}          ${_LIBZ}            ${_LIBJEMALLOC}     ${_LIBPTHREAD}
  ${_LIBDL}           ${_LIBC}            ${_LIBM}            ${_LIBRT}
  ${_LIBARCH}         ${_LIBFUSE}                   
  ${_LIBUNWIND}       ${_LIBLZMA}         ${_LIBB_PROGRAM_OPTIONS}     
  )






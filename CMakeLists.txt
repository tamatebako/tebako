cmake_minimum_required (VERSION 3.13.4)
include(ExternalProject)
include(FetchContent)

project(tebako)

if(NOT DEFINED(DEPS))
   set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps)
endif()

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)

set (DWARFS_NAME dwarfs)
set (DWARFS_VERSION "0.5.6")
set (DWARFS_CHECKSUM "4cf477b934567ac1e3d0f0da463dfd37")

# As of today only version "0.5.6" is supported 
# But potentially GHA may request to build someting diffferent
if(DEFINED(GHA_DWARFS_VERSION) AND (NOT DWARFS_VERSION EQUAL GHA_DWARFS_VERSION)) 
  message(FATAL_ERROR "DwarFs version ${DWARFS_VERSION} does not match GHA version ${GHA_DWARFS_VERSION}" )
endif()

if(DEFINED(GHA_DWARFS_CHECKSUM) AND (NOT DWARFS_CHECKSUM EQUAL GHA_DWARFS_CHECKSUM)) 
  message(FATAL_ERROR "DwarFs version ${DWARFS_CHECKSUM} does not match GHA version ${GHA_DWARFS_CHECKSUM}" )
endif()

set (DWARFS_PRJ _${DWARFS_NAME})

set(Boost_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)

find_package(Boost 1.67 REQUIRED COMPONENTS context regex thread atomic date_time filesystem program_options system)

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

find_library(_LIBGLOG "libglog.a" REQUIRED)
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
find_library(_LIBSSL "libssl.a" REQUIRED)
find_library(_LIBUNWIND "libunwind.a" REQUIRED)

# ...................................................................
# -- Release archive is used (and not git clone) since the repo misses version files 
#    that are generated but generated outside of normal build script (
# -- The patch:
#    - excludes dwarfsextract target that fails in static build because of libarchive problem (possible TODO)
#    - changes static_link.sh to point to /usr/lib/x86_64-linux-gnu  for all libaries (this is most likely related to the previous item and IS NOT PORTABLE)
#      TODO -- generate static_link.sh although it looks impossible  because of a ref to libgcc_eh that is version specific   <<<<<<<<<<<< BE CAREFUL    
#    - adds cmake_policy(SET CMP0077 NEW)  that is required to overwrite option from the command line [DSTATIC_BUILD_DO_NOT_USE=ON]

set(DWARFS_BINARY_DIR ${DEPS}/src/${DWARFS_PRJ}-build/)

# ...................................................................
# The libraries that are build by dwarfs project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light
# These forward-declarations and BUILD_BYPRODICTS are required to support 'Ninja'
# Otherwise add_dependencies would be enough for 'Unix makefiles' generator
# ...................................................................

set(__LIBDWARFS "${DWARFS_BINARY_DIR}/libdwarfs.a")
set(__LIBFOLLY "${DWARFS_BINARY_DIR}/folly/libfolly.a")
set(__LIBFSST "${DWARFS_BINARY_DIR}/libfsst.a")
set(__LIBT_METADATA "${DWARFS_BINARY_DIR}/libmetadata_thrift.a")
set(__LIBT_LIGHT "${DWARFS_BINARY_DIR}/libthrift_light.a")
set(__LIBXXHASH "${DWARFS_BINARY_DIR}/libxxhash.a")
set(__LIBZSTD "${DWARFS_BINARY_DIR}/zstd/build/cmake/lib/libzstd.a")

ExternalProject_Add(${DWARFS_PRJ}
   PREFIX ${DEPS}
   URL https://github.com/mhx/dwarfs/releases/download/v${DWARFS_VERSION}/dwarfs-${DWARFS_VERSION}.tar.xz
   DOWNLOAD_NO_PROGRESS true
   URL_HASH MD5=${DWARFS_CHECKSUM}
   PATCH_COMMAND cd ${DEPS}/src && patch -p1 < ../../patches/1.patch
   CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} -DSTATIC_BUILD_DO_NOT_USE=ON
   BINARY_DIR ${DWARFS_BINARY_DIR}
   BUILD_BYPRODUCTS ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA} ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD}
)

## -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE} ???

# ...................................................................
# The libraries that are build by dwarfs project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light
# ...................................................................
add_library(_LIBDWARFS STATIC IMPORTED)
set_target_properties(_LIBDWARFS PROPERTIES IMPORTED_LOCATION ${__LIBDWARFS})
add_dependencies(_LIBDWARFS ${DWARFS_PRJ})

add_library(_LIBFSST STATIC IMPORTED)
set_target_properties(_LIBFSST PROPERTIES IMPORTED_LOCATION ${__LIBFSST})
add_dependencies(_LIBFSST ${DWARFS_PRJ})

add_library(_LIBFOLLY STATIC IMPORTED)
set_target_properties(_LIBFOLLY PROPERTIES IMPORTED_LOCATION ${__LIBFOLLY})
add_dependencies(_LIBFOLLY ${DWARFS_PRJ})

add_library(_LIBT_METADATA STATIC IMPORTED)
set_target_properties(_LIBT_METADATA PROPERTIES IMPORTED_LOCATION ${__LIBT_METADATA})
add_dependencies(_LIBT_METADATA ${DWARFS_PRJ})

add_library(_LIBT_LIGHT STATIC IMPORTED)
set_target_properties(_LIBT_LIGHT PROPERTIES IMPORTED_LOCATION ${__LIBT_LIGHT})
add_dependencies(_LIBT_LIGHT ${DWARFS_PRJ})

add_library(_LIBXXHASH STATIC IMPORTED)
set_target_properties(_LIBXXHASH PROPERTIES IMPORTED_LOCATION ${__LIBXXHASH})
add_dependencies(_LIBXXHASH ${DWARFS_PRJ})

add_library(_LIBZSTD STATIC IMPORTED)
set_target_properties(_LIBZSTD PROPERTIES IMPORTED_LOCATION ${__LIBZSTD})
add_dependencies(_LIBZSTD ${DWARFS_PRJ})

# ...................................................................

add_compile_options(-fdiagnostics-color=always)
add_link_options(-static -static-libgcc)
set(CMAKE_C_FLAGS_RELEASE "-DNDEBUG -O2 -g")
set(CMAKE_CXX_FLAGS_RELEASE "-DNDEBUG -O2 -g")

ExternalProject_Get_property(${DWARFS_PRJ} SOURCE_DIR)
ExternalProject_Get_property(${DWARFS_PRJ} BINARY_DIR)
include_directories(${SOURCE_DIR}/folly ${BINARY_DIR}/folly ${SOURCE_DIR}/fbthrift ${BINARY_DIR}/thrift ${BINARY_DIR} ${SOURCE_DIR}/include)
add_compile_definitions(FUSE_USE_VERSION=35)

add_executable(tebako-0 src/tebako-0.cpp)
add_dependencies(tebako-0 ${DWARFS_PRJ} ${_LIBDWARFS})

# ...................................................................
#  IMPORTANT !!!
#  MUST be in this order otherwise link may fail
#  -static-libgcc and gcc_eh below is all together an ugly trick to enforce static linking
# ...................................................................
target_link_libraries(tebako-0 
  _LIBDWARFS          _LIBFSST            _LIBT_METADATA      _LIBT_LIGHT                            
  _LIBXXHASH          _LIBFOLLY           _LIBZSTD                             
  ${_LIBFMT}          ${Boost_LIBRARIES}  ${_LIBDC}           ${_LIBFUSE}                    
  ${_LIBGLOG}         ${_LIBGFLAGS}       ${_LIBEVENT}        ${_LIBARCH}
  ${_LIBACL}          ${_LIBXML2}         ${_LIBSSL}          ${_LIBCRYPTO}       
  ${_LIBIBERTY}       ${_LIBLZ4}          ${_LIBZ}            ${_LIBJEMALLOC}     
  ${_LIBPTHREAD}      ${_LIBDL}           ${_LIBC}            ${_LIBM}                       
  ${_LIBRT}			  gcc_eh			  ${_LIBUNWIND}       ${_LIBLZMA}
  )


FetchContent_Declare(
  incbin
  GIT_REPOSITORY https://github.com/graphitemaster/incbin.git
  GIT_TAG main
  )

# Check if population has already been performed
FetchContent_GetProperties(incbin)
string(TOLOWER "incbin" lcName)
if(NOT ${lcName}_POPULATED)
  FetchContent_Populate(incbin)
endif()



  set_property(TARGET tebako-0 PROPERTY CXX_STANDARD 17)
  set_property(TARGET tebako-0 PROPERTY CXX_STANDARD_REQUIRED ON)
  set_property(TARGET tebako-0 PROPERTY CXX_EXTENSIONS OFF)






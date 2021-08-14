cmake_minimum_required (VERSION 3.13.4)
include(ExternalProject)
include(FetchContent)

project(tebako)

if(NOT DEFINED(DEPS))
   set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps)
endif()

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)

set(Boost_USE_STATIC_LIBS   ON)
set(BOOST_LINK_STATIC       ON)
set(USE_STATIC_DEPS_ON_UNIX ON)
set(GFLAGS_SHARED          OFF)

find_package(Boost 1.67 REQUIRED COMPONENTS context regex thread atomic date_time filesystem program_options system)

# ...................................................................

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

set (DWARFS_NAME dwarfs)
set (DWARFS_PRJ _${DWARFS_NAME})
set (DWARFS_BINARY_DIR ${DEPS}/src/${DWARFS_PRJ}-build/)

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
set(__LIBARCHIVE "${DWARFS_BINARY_DIR}/libarchive/lib/libarchive.a")

ExternalProject_Add(${DWARFS_PRJ}
   PREFIX ${DEPS}
   GIT_REPOSITORY https://github.com/metanorma/dwarfs.git
   GIT_TAG main
   GIT_SHALLOW true
   PATCH_COMMAND cp ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.cpp ${DEPS}/src/_dwarfs/src/dwarfs && 
                 cp ${CMAKE_CURRENT_SOURCE_DIR}/patches/2.patch/version.h ${DEPS}/src/_dwarfs/include/dwarfs   
   CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} -DSTATIC_BUILD_DO_NOT_USE:BOOL=ON
   BINARY_DIR ${DWARFS_BINARY_DIR}
   BUILD_BYPRODUCTS ${__LIBDWARFS} ${__LIBFSST} ${__LIBFOLLY} ${__LIBT_METADATA} 
                    ${__LIBT_LIGHT} ${__LIBXXHASH} ${__LIBZSTD} ${__LIBARCHIVE}
)

## -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE} ???

# ...................................................................

add_compile_options(-fdiagnostics-color=always)
add_link_options(-static -static-libgcc)
set(CMAKE_C_FLAGS_RELEASE "-DNDEBUG -O2 -g")
set(CMAKE_CXX_FLAGS_RELEASE "-DNDEBUG -O2 -g")

ExternalProject_Get_property(${DWARFS_PRJ} SOURCE_DIR)
ExternalProject_Get_property(${DWARFS_PRJ} BINARY_DIR)
include_directories(${SOURCE_DIR}/folly ${BINARY_DIR}/folly ${SOURCE_DIR}/fbthrift ${BINARY_DIR}/thrift ${BINARY_DIR} ${SOURCE_DIR}/include)
add_compile_definitions(FUSE_USE_VERSION=35)

# ...................................................................

add_executable(tebako-0 src/tebako-0.cpp)
add_dependencies(tebako-0 ${DWARFS_PRJ})
set_property(TARGET tebako-0 PROPERTY CXX_STANDARD 17)
set_property(TARGET tebako-0 PROPERTY CXX_STANDARD_REQUIRED ON)
set_property(TARGET tebako-0 PROPERTY CXX_EXTENSIONS OFF)

# ...................................................................
# The libraries that are build by dwarfs project
# libdwarfs libfolly libfsst libxxhash libmetadata_thrift libthrift_light libarchive
# ...................................................................

function(ADD_DWARFS_LIBRARY NAME PRJ)
  add_library(${NAME} STATIC IMPORTED)
  set_target_properties(${NAME} PROPERTIES IMPORTED_LOCATION ${_${NAME}})
  add_dependencies(${NAME} ${PRJ})
  target_link_libraries(tebako-0 ${NAME})
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
#  IMPORTANT !!!
#  MUST be in this order otherwise link may fail
#  -static-libgcc and gcc_eh below is all together an ugly trick to enforce static linking
# ...................................................................
target_link_libraries(tebako-0                              
  ${_LIBFMT}          ${Boost_LIBRARIES}  ${_LIBDC}           ${_LIBFUSE}                    
  ${_LIBGLOG}         ${_LIBGFLAGS}       ${_LIBEVENT}        ${_LIBIBERTY}
  ${_LIBACL}          ${_LIBXML2}         ${_LIBSSL}          ${_LIBCRYPTO}       
  ${_LIBLZ4}          ${_LIBZ}            ${_LIBJEMALLOC}     
  ${_LIBPTHREAD}      ${_LIBDL}           ${_LIBC}            ${_LIBM}                       
  ${_LIBRT}           gcc_eh		  ${_LIBUNWIND}       ${_LIBLZMA}
  )








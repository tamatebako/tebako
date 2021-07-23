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


set (DWARFS_TARGET _${DWARFS_NAME})

ExternalProject_Add(${DWARFS_TARGET}
   PREFIX ${DEPS}
   URL https://github.com/mhx/dwarfs/releases/download/v${DWARFS_VERSION}/dwarfs-${DWARFS_VERSION}.tar.xz
   DOWNLOAD_NO_PROGRESS true
   CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} 
)


#libdwarfs.a \
#	libfsst.a \
#	libmetadata_thrift.a \
#	libthrift_light.a \
#	libxxhash.a \
#	folly/libfolly.a \
#	$fuse \
#	$extra \


#	!/usr/lib/gcc/x86_64-linux-gnu/10/libgcc_eh.a \

# ...................................................................
# Each library name given to the NAMES option is first considered
# as a library file name and then considered with platform-specific 
# prefixes (e.g. lib) and suffixes (e.g. .so). Therefore one may 
# specify library file names such as libfoo.a directly. This can be 
# used to locate static libraries on UNIX-like systems.
# ...................................................................

find_library(_LIBFMT "libfmt.a" REQUIRED)
find_library(_LIBB_CONTEXT "libboost_context.a" REQUIRED)
find_library(_LIBB_REGEX "libboost_regex.a" REQUIRED)
find_library(_LIBB_THREAD "libboost_thread.a" REQUIRED)
find_library(_LIBB_DATE_TIME "libboost_date_time.a" REQUIRED)
find_library(_LIBB_FILESYSTEM "libboost_filesystem.a" REQUIRED)
find_library(_LIBB_PROGRAM_OPTIONS "libboost_program_options.a" REQUIRED)
find_library(_LIBB_SYSTEM "libboost_system.a" REQUIRED)
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
find_library(_LIBDL "libdl.a" REQUIRED) 
find_library(_LIBC "libc.a" REQUIRED)
find_library(_LIBM "libm.a" REQUIRED)
find_library(_LIBRT "librt.a" REQUIRED)
#find_library(_LIBEH "libgcc_eh.a" REQUIRED)
#find_library(_LIBUNWIND "libunwind.a" REQUIRED)

# zstd/build/cmake/lib/libzstd.a  --> ${libzstd_static} 
# set by child project of ${DWARFS_TARGET}

add_executable(tebako-0 src/tebako-0.cpp)
add_dependencies(tebako-0 ${DWARFS_TARGET})
#target_link_libraries(tebako-0 ${DWARFS_NAME})
target_link_libraries(tebako-0 
  ${_LIBFMT}          
  ${_LIBACL}          ${_LIBDC}           ${_LIBGFLAGS}      ${_LIBEVENT}
  ${_LIBCRYPTO}       ${_LIBIBERTY}       ${_LIBXML2}        ${_LIBLZMA}
  ${_LIBLZ4}          ${_LIBZ}            ${_LIBJEMALLOC}    ${_LIBPTHREAD}
  ${_LIBDL}           ${_LIBC}            ${_LIBM}           ${_LIBRT}
  ${_LIBARCH}         
  ${_LIBB_CONTEXT}    ${_LIBB_REGEX}      ${_LIBB_THREAD}    ${_LIBB_DATE_TIME}
  ${_LIBB_FILESYSTEM} ${_LIBB_SYSTEM}     ${_LIBB_PROGRAM_OPTIONS} 
  ${libzstd_static}
  )



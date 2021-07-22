cmake_minimum_required (VERSION 3.12)
include(ExternalProject)

project(tebako)

if(NOT ${DEPS})
   set(DEPS ${CMAKE_CURRENT_SOURCE_DIR}/deps)
endif()

set(DEPS_INCLUDE_DIR ${DEPS}/include)
set(DEPS_LIB_DIR ${DEPS}/lib)


set (DWARFS_NAME dwarfs)
if(NOT ${DWARFS_VERSION})
   set (DWARFS_VERSION 0.5.6)
endif()

set (DWARFS_TARGET _${DWARFS_NAME})

ExternalProject_Add(${DWARFS_TARGET}
   PREFIX ${DEPS}
   URL https://github.com/mhx/dwarfs/releases/download/v${DWARFS_VERSION}/dwarfs-${DWARFS_VERSION}.tar.xz
   DOWNLOAD_NO_PROGRESS true
   CMAKE_ARGS -DCMAKE_INSTALL_PREFIX=${DEPS} 
)


add_executable(tebako-0 src/tebako-0.cpp)
#add_dependencies(tebako-0 ${DWARFS_TARGET})
#target_link_libraries(tebako-0 ${DWARFS_NAME})




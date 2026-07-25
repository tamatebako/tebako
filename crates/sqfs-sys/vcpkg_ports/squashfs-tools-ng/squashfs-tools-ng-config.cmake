get_filename_component(PACKAGE_PREFIX_DIR "${CMAKE_CURRENT_LIST_DIR}/../../" ABSOLUTE)

include(CMakeFindDependencyMacro)

find_dependency(Threads REQUIRED)
find_dependency(ZLIB REQUIRED)
find_dependency(lz4 CONFIG REQUIRED)
find_dependency(zstd CONFIG REQUIRED)
find_dependency(liblzma CONFIG REQUIRED)

if(NOT TARGET squashfs-tools-ng::squashfs)
    add_library(squashfs-tools-ng::squashfs STATIC IMPORTED)
    set_target_properties(squashfs-tools-ng::squashfs PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${PACKAGE_PREFIX_DIR}/include"
        INTERFACE_LINK_LIBRARIES "ZLIB::ZLIB;LZ4::lz4_static;zstd::libzstd_static;liblzma::liblzma;Threads::Threads"
    )
    if(EXISTS "${PACKAGE_PREFIX_DIR}/lib/libsquashfs.a")
        set_property(TARGET squashfs-tools-ng::squashfs APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
        set_target_properties(squashfs-tools-ng::squashfs PROPERTIES
            IMPORTED_LOCATION_RELEASE "${PACKAGE_PREFIX_DIR}/lib/libsquashfs.a")
    endif()
    if(EXISTS "${PACKAGE_PREFIX_DIR}/debug/lib/libsquashfs.a")
        set_property(TARGET squashfs-tools-ng::squashfs APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
        set_target_properties(squashfs-tools-ng::squashfs PROPERTIES
            IMPORTED_LOCATION_DEBUG "${PACKAGE_PREFIX_DIR}/debug/lib/libsquashfs.a")
    endif()
endif()

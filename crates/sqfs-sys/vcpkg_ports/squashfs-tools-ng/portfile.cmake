# vcpkg portfile for squashfs-tools-ng (libsquashfs)
# SquashFS reader/writer library used by the libtfs SquashFS backend.
#
# Upstream (AgentD/squashfs-tools-ng) is autotools-based; the official release
# tarballs at infraroot.at ship a pre-generated configure script. Only the
# LGPL-3.0-or-later libsquashfs library is built (--without-tools); the GPL
# command-line tools are not installed.

vcpkg_check_linkage(ONLY_STATIC_LIBRARY)

vcpkg_download_distfile(ARCHIVE
    URLS "https://infraroot.at/pub/squashfs/squashfs-tools-ng-${VERSION}.tar.xz"
    FILENAME "squashfs-tools-ng-${VERSION}.tar.xz"
    SHA512 6f14d90875ef3f3d36d3cd38932ade6b0da7284922c914bd655e3f0a4abb17ba75d4bc052a1d4244c1b4ff96b3798d83d5a7b3263b8e1b96c112f4cdf967400f
)

vcpkg_extract_source_archive(
    SOURCE_PATH
    ARCHIVE "${ARCHIVE}"
)

# macOS has no strchrnul, and the x86_64 runners' configure misdetects
# it as present — dir_reader.o then references a symbol ld64 cannot
# resolve (the factory's macos-x86_64 miniruby link). Pin the autoconf
# verdict so every Apple build takes the !HAVE_STRCHRNUL path, the one
# the arm64 leg already proves end to end.
if(VCPKG_TARGET_IS_OSX)
    set(ENV{ac_cv_func_strchrnul} no)
endif()

vcpkg_configure_make(
    SOURCE_PATH "${SOURCE_PATH}"
    OPTIONS
        --without-tools
        --without-selinux
        --without-lzo
        --without-bzip2
        --with-gzip
        --with-lz4
        --with-xz
        --with-zstd
)

vcpkg_install_make()
vcpkg_fixup_pkgconfig()

file(REMOVE_RECURSE "${CURRENT_PACKAGES_DIR}/debug/include")
file(REMOVE "${CURRENT_PACKAGES_DIR}/lib/libsquashfs.la"
            "${CURRENT_PACKAGES_DIR}/debug/lib/libsquashfs.la")

# Upstream installs no CMake package config; provide one exporting the
# squashfs-tools-ng::squashfs target (see usage).
file(MAKE_DIRECTORY "${CURRENT_PACKAGES_DIR}/share/${PORT}")
configure_file("${CMAKE_CURRENT_LIST_DIR}/squashfs-tools-ng-config.cmake"
               "${CURRENT_PACKAGES_DIR}/share/${PORT}/squashfs-tools-ng-config.cmake" COPYONLY)

vcpkg_install_copyright(FILE_LIST "${SOURCE_PATH}/COPYING.md")

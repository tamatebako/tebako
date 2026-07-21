# Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
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

# ...................................................................
# libtfs provisioning (included from the top-level CMakeLists.txt)
#
# Two consumption modes for the DwarFS-based tebako filesystem library:
#   DWARFS_PRELOAD=ON  (default) -- download the prebuilt libtfs package
#       (libtfs-<ver>-<platform>.tar.gz) and the prebuilt mkdwarfs binary
#       from the libtfs GitHub release, verify both against the release
#       SHA256SUMS, and deploy them into ${DEPS}. The tarball is NOT
#       self-contained (its libtfsConfig.cmake does find_dependency(dwarfs)
#       and find_dependency(libzip), and no third-party static libs ship in
#       it), so the transitive static dependencies are resolved by installing
#       libtfs' own vcpkg manifest (pinned tag, overlay ports included) into
#       ${DEPS}/vcpkg_installed. vcpkg's per-package archive cache makes
#       repeated installs fast; set VCPKG_BINARY_SOURCES /
#       VCPKG_DEFAULT_BINARY_CACHE to control the cache location (mirrors
#       libtfs CI, which uses a files backend cache with readwrite access).
#   DWARFS_PRELOAD=OFF -- build libtfs from source (ExternalProject,
#       tamatebako/libtfs.git @ ${DWARFS_WR_TAG}) with the vcpkg toolchain.
#       mkdwarfs still comes from the prebuilt release set (the press
#       pipeline runs ${DEPS}/bin/mkdwarfs) and is passed to the libtfs
#       build via -DMKDWARFS (which also selects TEBAKO_BUILD_SCOPE=LIB).
#
# Expected on entry (set by the top-level CMakeLists.txt):
#   DEPS, DEPS_INCLUDE_DIR, DEPS_LIB_DIR, DEPS_BIN_DIR, DEPS_SRC_DIR
#   IS_GNU, IS_MUSL, IS_MSYS, IS_DARWIN
#   DWARFS_PRELOAD (option), DWARFS_WR_TAG (e.g. "v0.12.0")
# Sets for the rest of the build:
#   LIBTFS_VERSION, LIBTFS_PLATFORM
#   LIBTFS_INCLUDE_DIR        -- libtfs headers (deps/include)
#   LIBTFS_LIB_DIR            -- libtfs.a + helper (deps/lib)
#   LIBTFS_MKDWARFS           -- mkdwarfs executable (deps/bin/mkdwarfs[.exe])
#   LIBTFS_VCPKG_INSTALLED_DIR / LIBTFS_VCPKG_TRIPLET_DIR
#                             -- transitive static libs/headers (vcpkg)
#   VCPKG_ROOT                -- vcpkg checkout (env, cache, or bootstrapped)

# Pinned to the default-registry baseline of libtfs v0.12.0
# vcpkg-configuration.json. Only used when tebako bootstraps vcpkg itself
# (i.e. VCPKG_ROOT is not provided by the environment).
set(LIBTFS_VCPKG_BASELINE "11bbc873e00e9e58d4e9dffb30b7a5493a030e0b")

string(REGEX REPLACE "^v" "" LIBTFS_VERSION "${DWARFS_WR_TAG}")
set(LIBTFS_RELEASE_URL "https://github.com/tamatebako/libtfs/releases/download/v${LIBTFS_VERSION}")
set(LIBTFS_DOWNLOAD_DIR "${DEPS}/downloads/libtfs-${LIBTFS_VERSION}")

# ...................................................................
# Map (OSTYPE, host processor) to the libtfs release platform id

string(TOLOWER "${CMAKE_HOST_SYSTEM_PROCESSOR}" __LIBTFS_ARCH)
if(__LIBTFS_ARCH MATCHES "^(x86_64|amd64)$")
  set(__LIBTFS_ARCH "x86_64")
elseif(__LIBTFS_ARCH MATCHES "^(aarch64|arm64)$")
  set(__LIBTFS_ARCH "arm64")
else()
  message(FATAL_ERROR "Unsupported host processor '${CMAKE_HOST_SYSTEM_PROCESSOR}' for prebuilt libtfs")
endif()

if(IS_GNU)
  set(LIBTFS_PLATFORM "linux-gnu-${__LIBTFS_ARCH}")
elseif(IS_MUSL)
  if(NOT __LIBTFS_ARCH STREQUAL "x86_64")
    message(FATAL_ERROR "No prebuilt libtfs package for linux-musl-${__LIBTFS_ARCH}; use -DDWARFS_PRELOAD=OFF (source build)")
  endif()
  set(LIBTFS_PLATFORM "linux-musl-${__LIBTFS_ARCH}")
elseif(IS_DARWIN)
  set(LIBTFS_PLATFORM "macos-${__LIBTFS_ARCH}")
elseif(IS_MSYS)
  set(LIBTFS_PLATFORM "windows-ucrt64")
else()
  message(FATAL_ERROR "Unsupported platform for prebuilt libtfs (OSTYPE: '${OSTYPE_TXT}')")
endif()

if(IS_MSYS)
  set(LIBTFS_PKG_NAME "libtfs-${LIBTFS_VERSION}-${LIBTFS_PLATFORM}.zip")
  set(LIBTFS_MKDWARFS_NAME "mkdwarfs-${LIBTFS_PLATFORM}.exe")
  set(LIBTFS_MKDWARFS "${DEPS_BIN_DIR}/mkdwarfs.exe")
else()
  set(LIBTFS_PKG_NAME "libtfs-${LIBTFS_VERSION}-${LIBTFS_PLATFORM}.tar.gz")
  set(LIBTFS_MKDWARFS_NAME "mkdwarfs-${LIBTFS_PLATFORM}")
  set(LIBTFS_MKDWARFS "${DEPS_BIN_DIR}/mkdwarfs")
endif()

set(LIBTFS_INCLUDE_DIR "${DEPS_INCLUDE_DIR}")
set(LIBTFS_LIB_DIR "${DEPS_LIB_DIR}")
set(LIBTFS_VCPKG_INSTALLED_DIR "${DEPS}/vcpkg_installed")

# ...................................................................
# Download helper: fetch url to dest, verify against the expected SHA256,
# fail the configure on any error. Skips the download when dest already
# matches the expected hash (file(DOWNLOAD EXPECTED_HASH) semantics).

function(libtfs_download URL DEST EXPECTED_SHA256 DESCRIPTION)
  message(STATUS "libtfs: fetching ${DESCRIPTION}")
  message(STATUS "libtfs:   ${URL}")
  file(DOWNLOAD "${URL}" "${DEST}"
       EXPECTED_HASH "SHA256=${EXPECTED_SHA256}"
       STATUS __DL_STATUS
       SHOW_PROGRESS)
  list(GET __DL_STATUS 0 __DL_CODE)
  list(GET __DL_STATUS 1 __DL_MSG)
  if(NOT __DL_CODE EQUAL 0)
    file(REMOVE "${DEST}")
    message(FATAL_ERROR "libtfs: failed to download ${DESCRIPTION}: ${__DL_MSG}")
  endif()
  message(STATUS "libtfs: SHA256 verified (${EXPECTED_SHA256})")
endfunction()

# ...................................................................
# Fetch the release SHA256SUMS manifest and pick out the hashes of the
# two assets this platform needs (libtfs package + mkdwarfs binary)

file(MAKE_DIRECTORY "${LIBTFS_DOWNLOAD_DIR}")
set(LIBTFS_SUMS_FILE "${LIBTFS_DOWNLOAD_DIR}/SHA256SUMS")
message(STATUS "libtfs: fetching SHA256SUMS for v${LIBTFS_VERSION}")
file(DOWNLOAD "${LIBTFS_RELEASE_URL}/SHA256SUMS" "${LIBTFS_SUMS_FILE}"
     STATUS __SUMS_STATUS)
list(GET __SUMS_STATUS 0 __SUMS_CODE)
list(GET __SUMS_STATUS 1 __SUMS_MSG)
if(NOT __SUMS_CODE EQUAL 0)
  message(FATAL_ERROR "libtfs: failed to download SHA256SUMS: ${__SUMS_MSG}")
endif()

set(LIBTFS_PKG_HASH "")
set(LIBTFS_MKDWARFS_HASH "")
file(STRINGS "${LIBTFS_SUMS_FILE}" __SUMS_LINES)
foreach(__LINE IN LISTS __SUMS_LINES)
  # Note: CMake's regex engine mishandles {64} interval repetition, so the
  # hash is matched with + and its length checked explicitly.
  if(__LINE MATCHES "^([0-9a-f]+)  (.+)$")
    string(LENGTH "${CMAKE_MATCH_1}" __HASH_LEN)
    if(__HASH_LEN EQUAL 64 AND CMAKE_MATCH_2 STREQUAL LIBTFS_PKG_NAME)
      set(LIBTFS_PKG_HASH "${CMAKE_MATCH_1}")
    elseif(__HASH_LEN EQUAL 64 AND CMAKE_MATCH_2 STREQUAL LIBTFS_MKDWARFS_NAME)
      set(LIBTFS_MKDWARFS_HASH "${CMAKE_MATCH_1}")
    endif()
  endif()
endforeach()

if(NOT LIBTFS_PKG_HASH)
  message(FATAL_ERROR "libtfs: ${LIBTFS_PKG_NAME} not found in the release SHA256SUMS")
endif()
if(NOT LIBTFS_MKDWARFS_HASH)
  message(FATAL_ERROR "libtfs: ${LIBTFS_MKDWARFS_NAME} not found in the release SHA256SUMS")
endif()

# ...................................................................
# mkdwarfs comes from the prebuilt release set in BOTH modes
# (the press pipeline runs ${DEPS}/bin/mkdwarfs)

file(MAKE_DIRECTORY "${DEPS_BIN_DIR}")
libtfs_download("${LIBTFS_RELEASE_URL}/${LIBTFS_MKDWARFS_NAME}"
                "${LIBTFS_MKDWARFS}" "${LIBTFS_MKDWARFS_HASH}"
                "prebuilt mkdwarfs (${LIBTFS_MKDWARFS_NAME})")
if(NOT IS_MSYS)
  file(CHMOD "${LIBTFS_MKDWARFS}"
       PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE
                   GROUP_READ GROUP_EXECUTE
                   WORLD_READ WORLD_EXECUTE)
endif()

# ...................................................................
# vcpkg bootstrap (both modes)
#   - VCPKG_ROOT from the environment wins;
#   - otherwise clone microsoft/vcpkg at the pinned baseline into
#     ${DEPS}/vcpkg and bootstrap it.

if(DEFINED ENV{VCPKG_ROOT} AND EXISTS "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake")
  set(VCPKG_ROOT "$ENV{VCPKG_ROOT}")
  message(STATUS "libtfs: using VCPKG_ROOT from environment: ${VCPKG_ROOT}")
else()
  set(VCPKG_ROOT "${DEPS}/vcpkg")
  if(NOT EXISTS "${VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake")
    message(STATUS "libtfs: bootstrapping vcpkg at ${VCPKG_ROOT} (baseline ${LIBTFS_VCPKG_BASELINE})")
    find_package(Git REQUIRED)
    file(MAKE_DIRECTORY "${VCPKG_ROOT}")
    # NB: multiple COMMANDs in one execute_process run concurrently as a
    # pipeline, so the git steps run as separate sequential calls.
    execute_process(
      COMMAND ${GIT_EXECUTABLE} init
      WORKING_DIRECTORY "${VCPKG_ROOT}"
      RESULT_VARIABLE __VCPKG_CLONE_RES
    )
    if(__VCPKG_CLONE_RES EQUAL 0)
      execute_process(
        COMMAND ${GIT_EXECUTABLE} remote add origin https://github.com/microsoft/vcpkg.git
        WORKING_DIRECTORY "${VCPKG_ROOT}"
        RESULT_VARIABLE __VCPKG_CLONE_RES
      )
    endif()
    if(__VCPKG_CLONE_RES EQUAL 0)
      execute_process(
        COMMAND ${GIT_EXECUTABLE} fetch --depth 1 origin ${LIBTFS_VCPKG_BASELINE}
        WORKING_DIRECTORY "${VCPKG_ROOT}"
        RESULT_VARIABLE __VCPKG_CLONE_RES
      )
    endif()
    if(__VCPKG_CLONE_RES EQUAL 0)
      execute_process(
        COMMAND ${GIT_EXECUTABLE} checkout -q FETCH_HEAD
        WORKING_DIRECTORY "${VCPKG_ROOT}"
        RESULT_VARIABLE __VCPKG_CLONE_RES
      )
    endif()
    if(NOT __VCPKG_CLONE_RES EQUAL 0)
      file(REMOVE_RECURSE "${VCPKG_ROOT}")
      message(FATAL_ERROR "libtfs: failed to clone vcpkg at baseline ${LIBTFS_VCPKG_BASELINE}")
    endif()
  endif()
endif()

if(WIN32)
  set(__VCPKG_EXE "${VCPKG_ROOT}/vcpkg.exe")
  set(__VCPKG_BOOTSTRAP cmd /c "${VCPKG_ROOT}/bootstrap-vcpkg.bat" -disableMetrics)
else()
  set(__VCPKG_EXE "${VCPKG_ROOT}/vcpkg")
  set(__VCPKG_BOOTSTRAP "${VCPKG_ROOT}/bootstrap-vcpkg.sh" -disableMetrics)
endif()

if(NOT EXISTS "${__VCPKG_EXE}")
  message(STATUS "libtfs: running vcpkg bootstrap")
  execute_process(
    COMMAND ${__VCPKG_BOOTSTRAP}
    WORKING_DIRECTORY "${VCPKG_ROOT}"
    RESULT_VARIABLE __VCPKG_BOOT_RES
  )
  if(NOT __VCPKG_BOOT_RES EQUAL 0 OR NOT EXISTS "${__VCPKG_EXE}")
    message(FATAL_ERROR "libtfs: vcpkg bootstrap failed")
  endif()
endif()

if(IS_MSYS)
  set(LIBTFS_VCPKG_TRIPLET "x64-mingw-static")
  set(__LIBTFS_VCPKG_TRIPLET_ARGS "--triplet" "x64-mingw-static")
  set(LIBTFS_VCPKG_EP_TRIPLET_ARG "-DVCPKG_TARGET_TRIPLET=x64-mingw-static")
else()
  # vcpkg default-host-triplet detection (matches the libtfs release builds)
  set(LIBTFS_VCPKG_TRIPLET "")
  set(__LIBTFS_VCPKG_TRIPLET_ARGS "")
  set(LIBTFS_VCPKG_EP_TRIPLET_ARG "")
endif()

# ...................................................................
if(DWARFS_PRELOAD)
# ...................................................................
# Prebuilt mode: download, verify and deploy the libtfs package

  if(IS_MSYS)
    message(WARNING "libtfs: the prebuilt ${LIBTFS_PLATFORM} package is modern-API-only; "
                    "the legacy tebako API that tebako links requires -DDWARFS_PRELOAD=OFF (source build)")
  endif()

  libtfs_download("${LIBTFS_RELEASE_URL}/${LIBTFS_PKG_NAME}"
                  "${LIBTFS_DOWNLOAD_DIR}/${LIBTFS_PKG_NAME}" "${LIBTFS_PKG_HASH}"
                  "prebuilt libtfs package (${LIBTFS_PKG_NAME})")

  set(__LIBTFS_EXTRACT_DIR "${LIBTFS_DOWNLOAD_DIR}/extract-${LIBTFS_PLATFORM}")
  file(REMOVE_RECURSE "${__LIBTFS_EXTRACT_DIR}")
  file(MAKE_DIRECTORY "${__LIBTFS_EXTRACT_DIR}")
  file(ARCHIVE_EXTRACT INPUT "${LIBTFS_DOWNLOAD_DIR}/${LIBTFS_PKG_NAME}"
       DESTINATION "${__LIBTFS_EXTRACT_DIR}")

  # Deploy headers and static libraries into deps/. The bundled CMake package
  # config (lib/cmake/libtfs) is intentionally NOT deployed: it does
  # find_dependency(dwarfs) + find_dependency(libzip) and is only usable with
  # a vcpkg prefix; tebako consumes the artifacts via DEPS_INCLUDE_DIR /
  # DEPS_LIB_DIR and its own static link lists.
  file(COPY "${__LIBTFS_EXTRACT_DIR}/include/" DESTINATION "${LIBTFS_INCLUDE_DIR}")
  file(GLOB __LIBTFS_LIBS "${__LIBTFS_EXTRACT_DIR}/lib/*.a" "${__LIBTFS_EXTRACT_DIR}/lib/*.lib")
  if(NOT __LIBTFS_LIBS)
    message(FATAL_ERROR "libtfs: prebuilt package contains no static libraries")
  endif()
  file(COPY ${__LIBTFS_LIBS} DESTINATION "${LIBTFS_LIB_DIR}")
  foreach(__LIB IN LISTS __LIBTFS_LIBS)
    message(STATUS "libtfs: deployed ${__LIB}")
  endforeach()

# ...................................................................
# Transitive dependencies (prebuilt mode)
# The libtfs package ships no third-party static libs, so resolve them with
# vcpkg using libtfs' own manifest at the pinned tag (vcpkg.json +
# vcpkg-configuration.json + vcpkg-overlay ports, notably the dwarfs fork
# tebako-v0.14.1-10). The installed tree lands in
# ${DEPS}/vcpkg_installed/<triplet>; its lib/ dir is the transitive
# static-lib list location for tebako's link step.

  set(LIBTFS_MANIFEST_DIR "${DEPS_SRC_DIR}/libtfs-manifest")
  find_package(Git REQUIRED)
  if(EXISTS "${LIBTFS_MANIFEST_DIR}/.git")
    execute_process(
      COMMAND ${GIT_EXECUTABLE} describe --tags --exact-match HEAD
      WORKING_DIRECTORY "${LIBTFS_MANIFEST_DIR}"
      RESULT_VARIABLE __MANIFEST_TAG_RES
      OUTPUT_VARIABLE __MANIFEST_TAG
      OUTPUT_STRIP_TRAILING_WHITESPACE
      ERROR_QUIET
    )
    if(NOT __MANIFEST_TAG_RES EQUAL 0 OR NOT "${__MANIFEST_TAG}" STREQUAL "${DWARFS_WR_TAG}")
      message(STATUS "libtfs: manifest checkout is at '${__MANIFEST_TAG}', need '${DWARFS_WR_TAG}'; refreshing")
      file(REMOVE_RECURSE "${LIBTFS_MANIFEST_DIR}")
    endif()
  endif()
  if(NOT EXISTS "${LIBTFS_MANIFEST_DIR}/.git")
    message(STATUS "libtfs: fetching vcpkg manifest from tamatebako/libtfs @ ${DWARFS_WR_TAG}")
    execute_process(
      COMMAND ${GIT_EXECUTABLE} clone --depth 1 --branch ${DWARFS_WR_TAG}
              https://github.com/tamatebako/libtfs.git "${LIBTFS_MANIFEST_DIR}"
      RESULT_VARIABLE __MANIFEST_CLONE_RES
    )
    if(NOT __MANIFEST_CLONE_RES EQUAL 0)
      message(FATAL_ERROR "libtfs: failed to clone tamatebako/libtfs @ ${DWARFS_WR_TAG} (vcpkg manifest source)")
    endif()
  endif()

  message(STATUS "libtfs: resolving transitive dependencies with vcpkg (this can take a while on a cold cache)")
  execute_process(
    COMMAND ${CMAKE_COMMAND} -E env VCPKG_DISABLE_METRICS=1
            "${__VCPKG_EXE}" install
            --x-manifest-root=${LIBTFS_MANIFEST_DIR}
            --x-install-root=${LIBTFS_VCPKG_INSTALLED_DIR}
            ${__LIBTFS_VCPKG_TRIPLET_ARGS}
    RESULT_VARIABLE __VCPKG_INSTALL_RES
  )
  if(NOT __VCPKG_INSTALL_RES EQUAL 0)
    message(FATAL_ERROR "libtfs: vcpkg install of transitive dependencies failed")
  endif()

endif(DWARFS_PRELOAD)

# ...................................................................
# Detect the vcpkg triplet directory (transitive static-lib location).
# There is exactly one triplet dir per tebako build; 'vcpkg' is vcpkg's
# own bookkeeping subdirectory of the install root.

if(NOT LIBTFS_VCPKG_TRIPLET)
  file(GLOB __TRIPLET_DIRS LIST_DIRECTORIES true "${LIBTFS_VCPKG_INSTALLED_DIR}/*")
  foreach(__DIR IN LISTS __TRIPLET_DIRS)
    get_filename_component(__NAME "${__DIR}" NAME)
    if(NOT __NAME STREQUAL "vcpkg")
      set(LIBTFS_VCPKG_TRIPLET "${__NAME}")
    endif()
  endforeach()
endif()
if(NOT LIBTFS_VCPKG_TRIPLET)
  # Source build on a cold tree: the triplet dir only appears when vcpkg
  # installs during the libtfs build, so compute vcpkg's default host
  # triplet the way the libtfs release builds do.
  if(IS_DARWIN)
    set(LIBTFS_VCPKG_TRIPLET "${__LIBTFS_ARCH}-osx")
  elseif(IS_GNU OR IS_MUSL)
    set(LIBTFS_VCPKG_TRIPLET "${__LIBTFS_ARCH}-linux")
  endif()
endif()
if(LIBTFS_VCPKG_TRIPLET)
  set(LIBTFS_VCPKG_TRIPLET_DIR "${LIBTFS_VCPKG_INSTALLED_DIR}/${LIBTFS_VCPKG_TRIPLET}")
endif()

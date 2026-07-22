/**
 *
 * Copyright (c) 2021-2025 [Ribose Inc](https://www.ribose.com).
 * All rights reserved.
 * This file is a part of the Tebako project.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 *
 */

#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <memory.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdint.h>
#include <limits.h>

#include <string>
#include <cstdint>
#include <vector>
#include <stdexcept>
#include <tuple>

#ifdef _WIN32
#include <winsock2.h>
#include <windows.h>
#include <io.h>
#endif

#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif

#include <tebako/tebako-config.h>
#include <tebako/tebako-io.h>

#include <tebako/tebako-version.h>
#include <tebako/tebako-main.h>
#include <tebako/tebako-fs.h>
#include <tebako/tebako-cmdline.h>

/*
 * tpkg manifest trailer reader (Stage 3A).
 * Vendored copy of libtfs include/tebako/tpkg.h @ 37166d3 (PR #155) at
 * include/tebako/tpkg.h in this repo -- keep in sync with upstream.
 * TPKG_IMPLEMENTATION is defined in exactly this translation unit.
 */
#define TPKG_IMPLEMENTATION
#include <tebako/tpkg.h>

static int running_miniruby = 0;
static tebako::cmdline_args* args = nullptr;
static std::vector<char> package;
/* Owns the self-executable bytes when a tpkg trailer mount replaces the
   incbin image; process-lifetime like the incbin section itself. */
static std::vector<char> self_image;

namespace {

/* Path of the running executable, UTF-8 (best effort; empty on failure) */
std::string self_executable_path()
{
#if defined(_WIN32)
  wchar_t wbuf[32768];
  DWORD n = GetModuleFileNameW(nullptr, wbuf, static_cast<DWORD>(sizeof(wbuf) / sizeof(wbuf[0])));
  if (n == 0 || n >= sizeof(wbuf) / sizeof(wbuf[0])) {
    return std::string();
  }
  int len = WideCharToMultiByte(CP_UTF8, 0, wbuf, n, nullptr, 0, nullptr, nullptr);
  if (len <= 0) {
    return std::string();
  }
  std::string out(static_cast<size_t>(len), '\0');
  WideCharToMultiByte(CP_UTF8, 0, wbuf, n, out.data(), len, nullptr, nullptr);
  return out;
#elif defined(__APPLE__)
  uint32_t bufsize = 0;
  _NSGetExecutablePath(nullptr, &bufsize);
  std::vector<char> buf(bufsize);
  if (_NSGetExecutablePath(buf.data(), &bufsize) != 0) {
    return std::string();
  }
  char resolved[PATH_MAX];
  if (realpath(buf.data(), resolved) != nullptr) {
    return std::string(resolved);
  }
  return std::string(buf.data());
#else
  char buf[PATH_MAX];
  ssize_t n = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
  if (n <= 0) {
    return std::string();
  }
  buf[n] = '\0';
  return std::string(buf);
#endif
}

/* Open the running executable read-only; returns fd or -1 */
int open_self_executable(const std::string& path)
{
#if defined(_WIN32)
  int wlen = MultiByteToWideChar(CP_UTF8, 0, path.c_str(), -1, nullptr, 0);
  if (wlen <= 0) {
    return -1;
  }
  std::vector<wchar_t> wpath(static_cast<size_t>(wlen));
  MultiByteToWideChar(CP_UTF8, 0, path.c_str(), -1, wpath.data(), wlen);
  return _wopen(wpath.data(), _O_RDONLY | _O_BINARY);
#else
  return open(path.c_str(), O_RDONLY);
#endif
}

/*
 * Probe the own executable for a tpkg manifest trailer.
 *
 * Returns 1 if a valid trailer was read into `manifest`, 0 if no trailer is
 * present (classic incbin bundle -- caller falls back), and -1 if a corrupt
 * trailer was detected (caller must fail startup; spec §6: never a partial
 * mount). On return values other than 0 a message naming the binary is
 * printed for the corrupt case.
 */
int read_self_manifest(tpkg_manifest* manifest)
{
  std::string self = self_executable_path();
  if (self.empty()) {
    return 0;  // cannot locate self -- treat as classic bundle
  }

  int fd = open_self_executable(self);
  if (fd < 0) {
    return 0;
  }

  int rc = tpkg_read_fd(fd, manifest);
  if (rc == 0) {
    // Validate slot geometry against the real file size; a CRC-valid
    // manifest pointing outside the file is still corrupt for us.
    off_t fsize = lseek(fd, 0, SEEK_END);
    close(fd);
    if (fsize < 0) {
      return 0;
    }
    for (uint32_t i = 0; i < manifest->slot_count; ++i) {
      const tpkg_slot& s = manifest->slots[i];
      if (s.offset > static_cast<uint64_t>(fsize) || s.size > static_cast<uint64_t>(fsize) - s.offset) {
        printf(
            "Tebako: package manifest trailer in '%s' is corrupt (slot %u outside file bounds). "
            "Re-stitch the package to repair the manifest.\n",
            self.c_str(), i);
        return -1;
      }
    }
    return 1;
  }

  close(fd);
  int err = tpkg_errno();
  if (err == TPKG_ERR_NO_TRAILER) {
    return 0;  // classic bundle, not an error
  }

  printf(
      "Tebako: package manifest trailer in '%s' is corrupt (%s). "
      "Re-stitch the package to repair the manifest.\n",
      self.c_str(), tpkg_strerror(err));
  return -1;
}

/* Read the whole file behind `fd` into self_image; 0 on success */
int read_self_image(int fd)
{
  off_t fsize = lseek(fd, 0, SEEK_END);
  if (fsize <= 0 || lseek(fd, 0, SEEK_SET) != 0) {
    return -1;
  }
  self_image.resize(static_cast<size_t>(fsize));
  size_t got = 0;
  while (got < static_cast<size_t>(fsize)) {
    ssize_t r = read(fd, self_image.data() + got, static_cast<size_t>(fsize) - got);
    if (r <= 0) {
      self_image.clear();
      return -1;
    }
    got += static_cast<size_t>(r);
  }
  return 0;
}

}  // namespace

static void tebako_clean(void)
{
  unmount_root_memfs();
  if (args) {
    delete args;
    args = nullptr;
  }
}

extern "C" int tebako_main(int* argc, char*** argv)
{
  int ret = -1, fsret = -1;
  char** new_argv = nullptr;
  char* argv_memory = nullptr;

  if (strstr((*argv)[0], "miniruby") != nullptr) {
    // Ruby build script is designed in such a way that this patch is also applied towards miniruby
    // Just pass through in such case
    ret = 0;
    running_miniruby = -1;
  }
  else {
    std::string mount_point = tebako::fs_mount_point;
    std::string entry_point = tebako::fs_entry_point;
    std::optional<std::string> cwd;
    if (tebako::package_cwd != nullptr) {
      cwd = tebako::package_cwd;
    }
    const void* data = &gfsData[0];
    size_t size = gfsSize;
    std::string root_image_offset = "auto";
    /* Non-root tpkg slots: (recorded mount point, image offset, image size) */
    struct extra_slot {
      std::string mount_point;
      uint64_t offset;
      uint64_t size;
    };
    std::vector<extra_slot> extra_slots;
    bool trailer_corrupt = false;

    try {
      args = new tebako::cmdline_args(*argc, (const char**)*argv);
      args->parse_arguments();
      if (args->with_application()) {
        args->process_package();
        auto descriptor = args->get_descriptor();
        package = std::move(args->get_package());
        if (descriptor.has_value()) {
          mount_point = descriptor->get_mount_point().c_str();
          entry_point = descriptor->get_entry_point().c_str();
          cwd = descriptor->get_cwd();
          data = package.data();
          size = package.size();
        }
      }

      if (data == &gfsData[0]) {
        /*
         * Incbin bundle startup: probe the own executable for a tpkg
         * manifest trailer (Stage 3A, spec §4.3).
         *  - trailer present: mount each slot at its recorded mount point via
         *    the legacy memfs path with the slot's explicit offset
         *  - no trailer: classic incbin mount below, unchanged
         *  - corrupt trailer: clean startup error (spec §6), no mount at all
         */
        tpkg_manifest manifest;
        int probe = read_self_manifest(&manifest);
        if (probe < 0) {
          trailer_corrupt = true;
        }
        else if (probe > 0) {
          std::string self = self_executable_path();
          int fd = open_self_executable(self);
          if (fd >= 0 && read_self_image(fd) == 0) {
            close(fd);
            /* Root slot: the one recorded at the package mount point;
               fall back to slot 0 (mounted at the compiled-in root). */
            uint32_t root_slot = 0;
            for (uint32_t i = 0; i < manifest.slot_count; ++i) {
              if (mount_point == manifest.slots[i].mount_point) {
                root_slot = i;
                break;
              }
            }
            /*
             * Mount each slot through a memory window covering exactly the
             * slot's image: the legacy memfs API takes an image offset but
             * no image size, and DwarFS parses up to the end of the view --
             * a whole-file view would let it run into the trailer. The
             * slot's explicit file offset selects the window; the image
             * sits at offset 0 within it.
             */
            data = self_image.data() + manifest.slots[root_slot].offset;
            size = static_cast<size_t>(manifest.slots[root_slot].size);
            root_image_offset = "0";
            for (uint32_t i = 0; i < manifest.slot_count; ++i) {
              if (i != root_slot) {
                extra_slots.push_back(
                    {std::string(manifest.slots[i].mount_point), manifest.slots[i].offset, manifest.slots[i].size});
              }
            }
          }
          else {
            if (fd >= 0) {
              close(fd);
            }
            printf("Tebako: failed to read package image from '%s': %s\n", self.c_str(), strerror(errno));
            trailer_corrupt = true;
          }
        }
      }

      if (!trailer_corrupt) {
        fsret = mount_root_memfs(data, size, tebako::fs_log_level, nullptr, nullptr, nullptr, nullptr,
                                 root_image_offset.c_str());
        if (fsret == 0) {
          for (const auto& slot : extra_slots) {
            // mount_memfs_at_root returns the memfs table index on success,
            // -1 on failure
            if (mount_memfs_at_root(self_image.data() + slot.offset, static_cast<unsigned int>(slot.size), "0",
                                    slot.mount_point.c_str()) == -1) {
              printf("Tebako: failed to mount package slot at '%s'\n", slot.mount_point.c_str());
              unmount_root_memfs();  // spec §6: never a partial mount
              fsret = -1;
              break;
            }
          }
        }
        if (fsret == 0) {
          args->process_mountpoints();
          args->build_arguments(mount_point.c_str(), entry_point.c_str());
          *argc = args->get_argc();
          *argv = args->get_argv();
          ret = 0;
          atexit(tebako_clean);
        }
      }
    }
    catch (std::exception e) {
      printf("Failed to process command line: %s\n", e.what());
    }

    if (getcwd(tebako::original_cwd, sizeof(tebako::original_cwd)) == nullptr) {
      printf("Failed to get current directory: %s\n", strerror(errno));
      ret = -1;
    }

    if (cwd.has_value()) {
      if (tebako_chdir(cwd->c_str()) != 0) {
        printf("Failed to chdir to '%s' : %s\n", cwd->c_str(), strerror(errno));
        ret = -1;
      }
    }
  }

  if (ret != 0) {
    try {
      printf("Tebako initialization failed\n");
      tebako_clean();
    }
    catch (...) {
      // Nested error, no recovery :(
    }
  }
  return ret;
}

extern "C" const char* tebako_mount_point(void)
{
  return tebako::fs_mount_point;
}

extern "C" const char* tebako_original_pwd(void)
{
  return tebako::original_cwd;
}

extern "C" int tebako_is_running_miniruby(void)
{
  return running_miniruby;
}

#ifdef RB_W32_PRE_33
extern "C" ssize_t rb_w32_pread(int /* fd */, void* /* buf */, size_t /* size */, size_t /* offset */)
{
  return -1;
}
#endif

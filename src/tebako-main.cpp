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

/*
 * Launcher ABI v1 (Stage 3B) -- the bootstrap -> runtime handoff contract.
 * A lean package's bootstrap execs the runtime as
 *   <runtime> --tebako-image <file>:<slot>:<mount-point> ...
 *             --tebako-entry <argv0> <user args...>
 * and the runtime mounts the referenced image slots directly out of the
 * bootstrap's file (spec 4.4). The Ruby-side constants and the full contract
 * documentation live in lib/tebako/launcher_abi.rb -- keep the two in sync.
 */
#define TEBAKO_LAUNCHER_ABI_VERSION 1u
#define TEBAKO_LAUNCHER_IMAGE_ARG "--tebako-image"
#define TEBAKO_LAUNCHER_ENTRY_ARG "--tebako-entry"
#define TEBAKO_LAUNCHER_VERSION_ARG "--tebako-launcher-abi"

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

/* Open a file read-only from a UTF-8 path (the running executable or a
   --tebako-image package file); returns fd or -1 */
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

/*
 * Launcher ABI v1 (Stage 3B) -- runtime side of the bootstrap handoff.
 */

/* One --tebako-image reference, resolved against the file's tpkg trailer */
struct launcher_image {
  std::string file;
  uint32_t slot = 0; /* index into the file's tpkg slot table */
  std::string mount_point;
  uint64_t size = 0;              /* image length in bytes, from the trailer */
  const char* window = nullptr;   /* slot bytes, owned by launcher_image_store */
};

struct launcher_handoff {
  bool present = false;           /* any launcher ABI option seen at all */
  bool version_seen = false;      /* --tebako-launcher-abi given */
  uint32_t version = 0;
  std::string entry;              /* package argv[0]; empty when --tebako-entry is absent */
  std::vector<launcher_image> images;
  std::vector<std::string> user_args; /* everything after --tebako-entry <argv0> */
  std::string error;              /* non-empty: malformed handoff */
};

/* Owns the image slot bytes read out of --tebako-image files;
   process-lifetime like the incbin section itself. Appending moves the
   outer vector but never the inner buffers, so window pointers stay valid. */
static std::vector<std::vector<char>> launcher_image_store;

bool parse_uint32(const std::string& s, uint32_t* out)
{
  if (s.empty() || s.size() > 10) {
    return false;
  }
  uint64_t v = 0;
  for (char c : s) {
    if (c < '0' || c > '9') {
      return false;
    }
    v = v * 10 + static_cast<uint64_t>(c - '0');
    if (v > UINT32_MAX) {
      return false;
    }
  }
  *out = static_cast<uint32_t>(v);
  return true;
}

/* Split "<file>:<slot>:<mount-point>" on the last two colons; the file
   component may itself contain colons (e.g. Windows drive prefixes). */
bool split_image_spec(const std::string& spec, std::string* file, uint32_t* slot, std::string* mount_point)
{
  size_t last = spec.rfind(':');
  if (last == std::string::npos || last == 0) {
    return false;
  }
  size_t prev = spec.rfind(':', last - 1);
  if (prev == std::string::npos) {
    return false;
  }
  *file = spec.substr(0, prev);
  *mount_point = spec.substr(last + 1);
  if (file->empty() || mount_point->empty()) {
    return false;
  }
  return parse_uint32(spec.substr(prev + 1, last - prev - 1), slot);
}

/* Match one launcher ABI option, inline ("--opt=value") or bare ("--opt").
   Returns 1 for --tebako-image, 2 for --tebako-entry, 3 for
   --tebako-launcher-abi, 0 for anything else. For a bare match *had_inline
   is false and *value is untouched; the caller consumes the next argument. */
int match_launcher_arg(const std::string& arg, bool* had_inline, std::string* value)
{
  static const std::string keys[] = {TEBAKO_LAUNCHER_IMAGE_ARG, TEBAKO_LAUNCHER_ENTRY_ARG, TEBAKO_LAUNCHER_VERSION_ARG};
  for (size_t k = 0; k < 3; ++k) {
    if (arg == keys[k]) {
      *had_inline = false;
      return static_cast<int>(k) + 1;
    }
    if (arg.size() > keys[k].size() && arg.compare(0, keys[k].size(), keys[k]) == 0 && arg[keys[k].size()] == '=') {
      *had_inline = true;
      *value = arg.substr(keys[k].size() + 1);
      return static_cast<int>(k) + 1;
    }
  }
  return 0;
}

/*
 * Scan argv for the launcher ABI handoff. The three ABI options are
 * recognized until --tebako-entry is consumed; everything after its value is
 * application argv. Non-ABI arguments preceding --tebako-entry are ignored
 * (the bootstrap emits none). When no ABI option is present at all the
 * returned handoff is !present and the caller runs the classic flow
 * unchanged.
 */
launcher_handoff parse_launcher_handoff(int argc, char** argv)
{
  launcher_handoff h;
  for (int i = 1; i < argc; ++i) {
    bool had_inline = false;
    std::string value;
    int kind = match_launcher_arg(argv[i], &had_inline, &value);
    if (kind == 0) {
      continue;
    }
    h.present = true;
    if (!had_inline) {
      if (i + 1 >= argc) {
        h.error = std::string("Error: ") + argv[i] +
                  (kind == 1 ? " shall be followed by <file>:<slot>:<mount-point>"
                             : (kind == 2 ? " shall be followed by the package argv[0]"
                                          : " shall be followed by an ABI version number"));
        return h;
      }
      value = argv[++i];
    }
    if (kind == 1) {
      launcher_image img;
      if (!split_image_spec(value, &img.file, &img.slot, &img.mount_point)) {
        h.error = "Error: malformed --tebako-image value '" + value + "' -- expected <file>:<slot>:<mount-point>";
        return h;
      }
      h.images.push_back(std::move(img));
    }
    else if (kind == 2) {
      if (value.empty()) {
        h.error = "Error: --tebako-entry shall be followed by the package argv[0]";
        return h;
      }
      h.entry = value;
      for (++i; i < argc; ++i) {
        h.user_args.emplace_back(argv[i]);
      }
      break;
    }
    else {
      if (!parse_uint32(value, &h.version)) {
        h.error = "Error: malformed --tebako-launcher-abi value '" + value + "' -- expected an ABI version number";
        return h;
      }
      h.version_seen = true;
    }
  }
  return h;
}

/* 64-bit file positioning for image slot reads (slot offsets can exceed 2GB) */
int seek_fd(int fd, uint64_t offset)
{
#if defined(_WIN32)
  return _lseeki64(fd, static_cast<__int64>(offset), SEEK_SET) == -1 ? -1 : 0;
#else
  return lseek(fd, static_cast<off_t>(offset), SEEK_SET) == static_cast<off_t>(-1) ? -1 : 0;
#endif
}

uint64_t file_size_fd(int fd)
{
#if defined(_WIN32)
  __int64 end = _lseeki64(fd, 0, SEEK_END);
  return end < 0 ? 0 : static_cast<uint64_t>(end);
#else
  off_t end = lseek(fd, 0, SEEK_END);
  return end == static_cast<off_t>(-1) ? 0 : static_cast<uint64_t>(end);
#endif
}

/* Read exactly n bytes from fd at absolute offset `offset` into dest */
int read_region(int fd, uint64_t offset, char* dest, size_t n)
{
  if (seek_fd(fd, offset) != 0) {
    return -1;
  }
  size_t got = 0;
  while (got < n) {
    ssize_t r = read(fd, dest + got, n - got);
    if (r <= 0) {
      return -1;
    }
    got += static_cast<size_t>(r);
  }
  return 0;
}

/*
 * Resolve every --tebako-image reference against its file's tpkg manifest
 * trailer and read the slot's bytes out of the file (no extraction, no temp
 * copies; spec 4.4). On failure prints a named startup error and returns
 * false; the caller must fail startup (spec 6: never a partial mount, one
 * bad slot aborts with the slot index).
 */
bool resolve_launcher_images(launcher_handoff* h)
{
  for (auto& img : h->images) {
    int fd = open_self_executable(img.file);
    if (fd < 0) {
      printf("Tebako: cannot open image file '%s': %s\n", img.file.c_str(), strerror(errno));
      return false;
    }
    tpkg_manifest m;
    if (tpkg_read_fd(fd, &m) != 0) {
      int err = tpkg_errno();
      close(fd);
      if (err == TPKG_ERR_NO_TRAILER) {
        printf("Tebako: image file '%s' carries no tpkg manifest trailer --\n"
               "  --tebako-image expects a three-part package file (bootstrap + image slots + trailer)\n",
               img.file.c_str());
      }
      else {
        printf("Tebako: package manifest trailer in '%s' is corrupt (%s). "
               "Re-stitch the package to repair the manifest.\n",
               img.file.c_str(), tpkg_strerror(err));
      }
      return false;
    }
    if (img.slot >= m.slot_count) {
      printf("Tebako: --tebako-image slot %u is out of range for '%s' (%u slot(s) in its manifest)\n", img.slot,
             img.file.c_str(), m.slot_count);
      close(fd);
      return false;
    }
    const tpkg_slot& s = m.slots[img.slot];
    uint64_t fsize = file_size_fd(fd);
    if (s.offset > fsize || s.size > fsize - s.offset) {
      printf("Tebako: package manifest trailer in '%s' is corrupt (slot %u outside file bounds). "
             "Re-stitch the package to repair the manifest.\n",
             img.file.c_str(), img.slot);
      close(fd);
      return false;
    }
    launcher_image_store.emplace_back(static_cast<size_t>(s.size));
    std::vector<char>& buf = launcher_image_store.back();
    if (read_region(fd, s.offset, buf.data(), buf.size()) != 0) {
      printf("Tebako: failed to read image slot %u from '%s': %s\n", img.slot, img.file.c_str(), strerror(errno));
      launcher_image_store.pop_back();
      close(fd);
      return false;
    }
    close(fd);
    img.size = s.size;
    img.window = buf.data();
  }
  return true;
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
    /* Non-root image slots: (recorded mount point, image bytes, image size).
       window points at the slot's image -- into self_image for the trailer
       path, into launcher_image_store for the launcher ABI path. */
    struct extra_slot {
      std::string mount_point;
      const char* window;
      uint64_t size;
    };
    std::vector<extra_slot> extra_slots;
    bool trailer_corrupt = false;
    bool handoff_failed = false;

    try {
      /*
       * Launcher ABI v1 (Stage 3B): a lean package's bootstrap execs the
       * runtime with --tebako-image/--tebako-entry arguments. When present,
       * they are consumed here -- before the classic cmdline/trailer/incbin
       * logic -- and the interpreter is handed a synthetic argv of the
       * package argv[0] and the application arguments.
       */
      launcher_handoff handoff = parse_launcher_handoff(*argc, *argv);
      if (!handoff.error.empty()) {
        printf("Tebako: %s\n", handoff.error.c_str());
        handoff_failed = true;
      }

      int effective_argc = *argc;
      char** effective_argv = *argv;
      std::vector<const char*> synthetic_argv;
      if (handoff.present && !handoff_failed) {
        synthetic_argv.push_back(handoff.entry.empty() ? (*argv)[0] : handoff.entry.c_str());
        for (const auto& a : handoff.user_args) {
          synthetic_argv.push_back(a.c_str());
        }
        effective_argc = static_cast<int>(synthetic_argv.size());
        effective_argv = const_cast<char**>(synthetic_argv.data());
      }

      args = new tebako::cmdline_args(effective_argc, (const char**)effective_argv);
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

      if (handoff.present && !handoff_failed) {
        /*
         * Launcher ABI handoff (Stage 3B): mount the images the bootstrap
         * named, directly out of its package file(s) -- no extraction.
         */
        if (handoff.version_seen && handoff.version > TEBAKO_LAUNCHER_ABI_VERSION) {
          printf(
              "Tebako: launcher ABI mismatch -- the bootstrap speaks ABI %u but this runtime supports ABI %u.\n"
              "  Refresh the runtime via tebako cache, or re-bundle with a matching tebako-bootstrap.\n",
              handoff.version, TEBAKO_LAUNCHER_ABI_VERSION);
          handoff_failed = true;
        }
        else if (handoff.images.empty()) {
          printf("Tebako: launcher ABI handoff without --tebako-image -- nothing to mount\n");
          handoff_failed = true;
        }
        else if (!resolve_launcher_images(&handoff)) {
          handoff_failed = true;  // resolve_launcher_images printed the reason
        }
        else if (data == &gfsData[0]) {
          /* Root image: the one handed over for the package mount point;
             fall back to the first image (mounted at the compiled-in root). */
          size_t root_image = 0;
          for (size_t i = 0; i < handoff.images.size(); ++i) {
            if (mount_point == handoff.images[i].mount_point) {
              root_image = i;
              break;
            }
          }
          data = handoff.images[root_image].window;
          size = static_cast<size_t>(handoff.images[root_image].size);
          root_image_offset = "0";
          for (size_t i = 0; i < handoff.images.size(); ++i) {
            if (i != root_image) {
              extra_slots.push_back({handoff.images[i].mount_point, handoff.images[i].window, handoff.images[i].size});
            }
          }
        }
        else {
          /* --tebako-run owns the root; every handed-over image mounts extra */
          for (const auto& img : handoff.images) {
            extra_slots.push_back({img.mount_point, img.window, img.size});
          }
        }
      }
      else if (data == &gfsData[0]) {
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
                extra_slots.push_back({std::string(manifest.slots[i].mount_point),
                                       self_image.data() + manifest.slots[i].offset, manifest.slots[i].size});
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

      if (!trailer_corrupt && !handoff_failed) {
        fsret = mount_root_memfs(data, size, tebako::fs_log_level, nullptr, nullptr, nullptr, nullptr,
                                 root_image_offset.c_str());
        if (fsret == 0) {
          for (const auto& slot : extra_slots) {
            // mount_memfs_at_root returns the memfs table index on success,
            // -1 on failure
            if (mount_memfs_at_root(slot.window, static_cast<unsigned int>(slot.size), "0",
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

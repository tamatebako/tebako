/**
 *
 * Copyright (c) 2021-2026 [Ribose Inc](https://www.ribose.com).
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

/*
 * Tebako runtime entry driver.
 *
 * Mounts the package filesystem image via the modern libtfs C API
 * (<tebako/fs/c_api.h>, the tebako_fs_* calls) exclusively -- the legacy
 * tebako C/C++ API (tebako-io.h, tebako-cmdline.h, the fd/kfd/memfs tables)
 * is not referenced anywhere in this translation unit.
 *
 * The package contract is unchanged:
 *  - the tpkg manifest trailer (vendored <tebako/tpkg.h>, ABI v1) probed with
 *    read_self_manifest()
 *  - the launcher ABI v1 handoff (--tebako-image/--tebako-entry/
 *    --tebako-launcher-abi) a tebako-bootstrap execs lean packages with
 *  - the classic options --tebako-run and --tebako-extract
 *  - the entry dispatch: after mounting, the interpreter is handed
 *    [<argv0>, <mount point><entry point>, <application args...>] and runs
 *    the packaged dispatcher (stub.rb -> application entry)
 *
 * Two limitations of the modern API shape the flow below (both fail startup
 * cleanly rather than run a partial mount, spec 6):
 *  - the C API mounts a single filesystem at a time, so packages carrying
 *    extra image slots (press-time --image, or several --tebako-image
 *    handoffs) cannot be served by this runtime;
 *  - the C API has no chdir, so a compiled-in/descriptor package working
 *    directory cannot be entered from here; the packaged entry dispatcher
 *    (stub.rb) performs Dir.chdir where the package provides one.
 */

#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <memory.h>
#include <errno.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <stdint.h>
#include <limits.h>

#include <string>
#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <stdexcept>

#ifdef _WIN32
#include <winsock2.h>
#include <windows.h>
#include <io.h>
#endif

#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif

/* The only engine interface of this driver: the modern libtfs C API */
#include <tebako/fs/c_api.h>

#include <tebako/tebako-main.h>
#include <tebako/tebako-fs.h>

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
/* Owns the dispatch argv handed to the interpreter; process-lifetime like
   the incbin section itself (ruby reads argv until process exit). */
static std::vector<std::string> dispatch_args;
static std::vector<char*> dispatch_argv;

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

/*
 * Launcher ABI v1 (Stage 3B) -- runtime side of the bootstrap handoff.
 */

/* One --tebako-image reference, resolved against the file's tpkg trailer */
struct launcher_image {
  std::string file;
  uint32_t slot = 0;   /* index into the file's tpkg slot table */
  std::string mount_point;
  uint64_t offset = 0; /* image start, absolute file offset, from the trailer */
  uint64_t size = 0;   /* image length in bytes, from the trailer */
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

/*
 * Resolve every --tebako-image reference against its file's tpkg manifest
 * trailer and record the slot's file region (offset/size); the images are
 * mounted directly out of the package file(s) by
 * tebako_fs_init_from_file_at() -- no extraction, no temp copies (spec 4.4).
 * On failure prints a named startup error and returns false; the caller must
 * fail startup (spec 6: never a partial mount, one bad slot aborts with the
 * slot index).
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
    if (s.format_id == TPKG_FORMAT_RUNTIME) {
      /* Payload slots are installed into the shared cache by the bootstrap,
         never mounted (lib/tebako/launcher_abi.rb) */
      printf("Tebako: --tebako-image slot %u of '%s' is a runtime payload slot -- payload slots are never mounted\n",
             img.slot, img.file.c_str());
      close(fd);
      return false;
    }
    uint64_t fsize = file_size_fd(fd);
    close(fd);
    if (s.offset > fsize || s.size > fsize - s.offset) {
      printf("Tebako: package manifest trailer in '%s' is corrupt (slot %u outside file bounds). "
             "Re-stitch the package to repair the manifest.\n",
             img.file.c_str(), img.slot);
      return false;
    }
    img.offset = s.offset;
    img.size = s.size;
  }
  return true;
}

/*
 * Classic tebako command line -- the packaged interpreter's own options.
 * Replaces the legacy tebako::cmdline_args (libtfs tebako-cmdline.h): same
 * options, same error texts, same stop-parsing-at-extract rule.
 *   --tebako-run <image>      mount and run an application image file
 *   --tebako-extract [folder] extract the mounted image to disk and exit
 *   --tebako-mount <rule>     rejected -- the modern TFS engine mounts a
 *                             single filesystem image; bind mounts and extra
 *                             image mounts are gone with the memfs tables
 * Malformed options throw std::invalid_argument exactly like the legacy
 * parser; the caller reports through the legacy catch path.
 */
struct classic_args {
  bool run = false;               /* --tebako-run given */
  std::string app_image;          /* its image file name */
  bool extract = false;           /* --tebako-extract given */
  std::string extract_folder;     /* its destination folder */
  std::vector<std::string> other_args; /* non-tebako arguments, argv[0] first */
};

classic_args parse_classic_args(int argc, char** argv)
{
  const std::string error_msg =
      "Error: --tebako-mount shall be followed by a rule (e.g., --tebako-mount <mount point>:<target>)";
  const std::string error_msg_mount =
      "Error: --tebako-mount is not supported by this runtime -- the modern TFS engine mounts a single "
      "filesystem image";
  const std::string error_msg_run =
      "Error: --tebako-run shall be followed by the application image file name (e.g., --tebako-run=<image file "
      "name>)";
  const std::string error_msg_run_nodup = "Error: --tebako-run option can be provided only once";

  const std::string run_key = "--tebako-run";
  const std::string run_key_ex = run_key + "=";
  const std::string mount_key = "--tebako-mount";
  const std::string mount_key_ex = mount_key + "=";
  const std::string extract_key = "--tebako-extract";
  const std::string extract_key_ex = extract_key + "=";
  const std::string extract_dest = "source_filesystem";

  classic_args args;
  for (int i = 0; i < argc; i++) {
    std::string arg = argv[i];

    // Handle "--tebako-run=value" case
    if (arg.rfind(run_key_ex, 0) == 0) {
      std::string value = arg.substr(run_key_ex.size());
      if (!value.empty()) {
        if (args.run) {
          throw std::invalid_argument(error_msg_run_nodup);
        }
        args.run = true;
        args.app_image = std::move(value);
        continue;
      }
      throw std::invalid_argument(error_msg_run);
    }

    // Handle "--tebako-mount=value" case
    if (arg.rfind(mount_key_ex, 0) == 0) {
      std::string value = arg.substr(mount_key_ex.size());
      if (!value.empty()) {
        throw std::invalid_argument(error_msg_mount);
      }
      throw std::invalid_argument(error_msg);
    }

    // Handle "--tebako-extract=value" case
    if (arg.rfind(extract_key_ex, 0) == 0) {
      args.extract = true;
      std::string value = arg.substr(extract_key_ex.size());
      args.extract_folder = value.empty() ? extract_dest : std::move(value);
      return args;
    }

    // Handle "--tebako-run" without '='
    if (arg == run_key) {
      if (args.run) {
        throw std::invalid_argument(error_msg_run_nodup);
      }
      args.run = true;
      // Ensure there is a next argument
      if (i + 1 < argc) {
        std::string next_arg = argv[i + 1];

        // Check if the next argument is valid
        if (next_arg[0] != '-') {  // It's not a flag
          args.app_image = std::move(next_arg);
          i += 1;  // Skip the next argument as it is the rule
          continue;
        }
        throw std::invalid_argument(error_msg_run);
      }
      // If "--tebako-run" is at the end of args without a rule, raise an error
      throw std::invalid_argument(error_msg_run);
    }

    // Handle "--tebako-mount" without '='
    if (arg == mount_key) {
      // Ensure there is a next argument
      if (i + 1 < argc) {
        std::string next_arg = argv[i + 1];

        // Check if the next argument is valid
        if (next_arg[0] != '-') {  // It's not a flag
          throw std::invalid_argument(error_msg_mount);
        }
        throw std::invalid_argument(error_msg);
      }
      // If "--tebako-mount" is at the end of args without a rule, raise an error
      throw std::invalid_argument(error_msg);
    }

    // Handle "--tebako-extract" without '='
    if (arg.rfind(extract_key, 0) == 0) {
      args.extract = true;
      if (i + 1 < argc) {
        std::string next_arg = argv[i + 1];

        // Check if the next argument is valid
        if (next_arg[0] != '-') {  // It's not a flag
          args.extract_folder = std::move(next_arg);
          i += 1;
        }
        else {
          args.extract_folder = extract_dest;
        }
      }
      else {
        args.extract_folder = extract_dest;
      }
      return args;
    }

    // Add other arguments as they are
    args.other_args.push_back(std::move(arg));
  }
  return args;
}

/*
 * Application image descriptor (--tebako-run) -- the "TAMATEBAKO" header
 * mkdwarfs(1) prepends to an application image at press time. Wire format
 * (lib/tebako/package_descriptor.rb writes it -- keep the two in sync):
 * the 10-byte signature, six little-endian u16 version fields (ruby/tebako
 * x.y.z; carried for compatibility, unused by this driver), the u16-length-
 * prefixed mount point and entry point strings, and an optional
 * u16-length-prefixed working directory. Replaces the legacy
 * tebako::package_descriptor (libtfs tebako-package-descriptor.h).
 */
struct app_descriptor {
  std::string mount_point;
  std::string entry_point;
  std::optional<std::string> cwd;
  uint64_t size = 0; /* descriptor byte length == image offset in the file */
};

app_descriptor read_app_descriptor(const std::string& path)
{
  static const char signature[] = "TAMATEBAKO";
  const size_t signature_len = sizeof(signature) - 1;
  /* The descriptor is length-prefixed throughout, so its size is bounded by
     three u16-length strings; one head read covers every legal value. */
  const uint64_t head_cap = signature_len + 12 + 3 * (2 + 0xFFFFu) + 1 + 2;

  int fd = open_self_executable(path);
  if (fd < 0) {
    throw std::invalid_argument("Path " + path + " does not exist");
  }

  uint64_t fsize = file_size_fd(fd);  // leaves the file offset at EOF -- rewind
  size_t want = static_cast<size_t>(fsize < head_cap ? fsize : head_cap);
#if defined(_WIN32)
  _lseeki64(fd, 0, SEEK_SET);
#else
  lseek(fd, 0, SEEK_SET);
#endif
  std::vector<char> buffer(want);
  size_t got = 0;
  while (got < want) {
    ssize_t r = read(fd, buffer.data() + got, want - got);
    if (r <= 0) {
      close(fd);
      throw std::invalid_argument("Failed to load filesystem image from " + path);
    }
    got += static_cast<size_t>(r);
  }
  close(fd);

  size_t offset = 0;
  auto read_from_buffer = [&buffer, &offset](void* data, size_t size) {
    if (offset + size > buffer.size()) {
      throw std::out_of_range("Buffer too short for deserialization");
    }
    std::memcpy(data, buffer.data() + offset, size);
    offset += size;
  };

  if (buffer.size() < signature_len || std::memcmp(buffer.data(), signature, signature_len) != 0) {
    throw std::invalid_argument("Invalid or missing signature");
  }
  offset = signature_len;

  /* ruby/tebako version fields -- skip (the runtime does not gate on them) */
  char versions[12];
  read_from_buffer(versions, sizeof(versions));

  auto read_string = [&read_from_buffer]() -> std::string {
    char len_bytes[2];
    read_from_buffer(len_bytes, sizeof(len_bytes));
    uint16_t len =
        static_cast<uint16_t>(static_cast<uint8_t>(len_bytes[0]) | (static_cast<uint16_t>(len_bytes[1]) << 8));
    std::string value(len, '\0');
    read_from_buffer(value.data(), len);
    return value;
  };

  app_descriptor descriptor;
  descriptor.mount_point = read_string();
  descriptor.entry_point = read_string();
  char cwd_present = 0;
  read_from_buffer(&cwd_present, 1);
  if (cwd_present) {
    descriptor.cwd = read_string();
  }
  descriptor.size = offset;
  return descriptor;
}

/*
 * Mount one image region of a package file as the (single) root filesystem;
 * the modern C API auto-detects the image format at the region start.
 * Returns 0 on success; on failure prints a named error and returns -1.
 */
int mount_image_region(const std::string& file, uint64_t offset, uint64_t size, const std::string& mount_point)
{
  if (tebako_fs_init_from_file_at(file.c_str(), offset, size, mount_point.c_str()) != 0) {
    printf("Tebako: failed to mount the package filesystem image from '%s': %s\n", file.c_str(),
           tebako_strerror(tebako_get_errno()));
    return -1;
  }
  return 0;
}

/*
 * The modern C API mounts a single filesystem at a time (a second mount
 * fails with EEXIST), so a package carrying extra image slots -- press-time
 * --image entries or several handed-over --tebako-image slots -- cannot be
 * served by this runtime. Fail startup rather than run a partial mount
 * (spec 6); prints the reason and returns true for the caller's failed
 * flag.
 */
bool reject_extra_slot(const char* mount_point)
{
  printf("Tebako: package carries an extra image slot recorded at '%s' but this runtime mounts a single\n"
         "  filesystem image -- multi-image packages need a multi-mount TFS. Refusing a partial mount.\n",
         mount_point);
  return true;
}

/*
 * Entry dispatch: hand the interpreter
 *   [<argv0>, <mount point><entry point>, <application args...>]
 * -- the same argv the legacy cmdline_args built (build_arguments_for_run).
 */
void build_dispatch_argv(const classic_args& cargs, const std::string& mount_point, const std::string& entry_point)
{
  if (mount_point.empty() || entry_point.empty()) {
    throw std::invalid_argument("Internal error: fs_mount_point and fs_entry_point must be non-null and non-empty");
  }

  dispatch_args.clear();
  dispatch_args.reserve(cargs.other_args.size() + 1);
  /* other_args[0] is the package argv[0] -- always present (parse starts at
     argv[0]) */
  dispatch_args.push_back(cargs.other_args[0]);
  dispatch_args.push_back(mount_point + entry_point);
  for (size_t i = 1; i < cargs.other_args.size(); ++i) {
    dispatch_args.push_back(cargs.other_args[i]);
  }

  dispatch_argv.clear();
  dispatch_argv.reserve(dispatch_args.size());
  for (auto& arg : dispatch_args) {
    dispatch_argv.push_back(arg.data());
  }
}

}  // namespace

static void tebako_clean(void)
{
  tebako_fs_unmount();  // safe to call multiple times
}

extern "C" int tebako_main(int* argc, char*** argv)
{
  int ret = -1;

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
    bool mounted = false;
    bool startup_failed = false;

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
        startup_failed = true;
      }

      int effective_argc = *argc;
      char** effective_argv = *argv;
      std::vector<const char*> synthetic_argv;
      if (handoff.present && !startup_failed) {
        synthetic_argv.push_back(handoff.entry.empty() ? (*argv)[0] : handoff.entry.c_str());
        for (const auto& a : handoff.user_args) {
          synthetic_argv.push_back(a.c_str());
        }
        effective_argc = static_cast<int>(synthetic_argv.size());
        effective_argv = const_cast<char**>(synthetic_argv.data());
      }

      classic_args cargs = parse_classic_args(effective_argc, effective_argv);

      if (handoff.present && !startup_failed) {
        /*
         * Launcher ABI handoff (Stage 3B): mount the image the bootstrap
         * named, directly out of its package file -- no extraction.
         */
        if (handoff.version_seen && handoff.version > TEBAKO_LAUNCHER_ABI_VERSION) {
          printf(
              "Tebako: launcher ABI mismatch -- the bootstrap speaks ABI %u but this runtime supports ABI %u.\n"
              "  Refresh the runtime via tebako cache, or re-bundle with a matching tebako-bootstrap.\n",
              handoff.version, TEBAKO_LAUNCHER_ABI_VERSION);
          startup_failed = true;
        }
        else if (handoff.images.empty()) {
          printf("Tebako: launcher ABI handoff without --tebako-image -- nothing to mount\n");
          startup_failed = true;
        }
        else if (!resolve_launcher_images(&handoff)) {
          startup_failed = true;  // resolve_launcher_images printed the reason
        }
        else if (cargs.run) {
          /* --tebako-run owns the root; every handed-over image would mount
             extra, which the single-mount TFS cannot serve */
          startup_failed = reject_extra_slot(handoff.images[0].mount_point.c_str());
        }
        else {
          /* Root image: the one handed over for the package mount point;
             fall back to the first image (mounted at the compiled-in root). */
          size_t root_image = 0;
          for (size_t i = 0; i < handoff.images.size(); ++i) {
            if (mount_point == handoff.images[i].mount_point) {
              root_image = i;
              break;
            }
          }
          for (size_t i = 0; i < handoff.images.size() && !startup_failed; ++i) {
            if (i != root_image) {
              startup_failed = reject_extra_slot(handoff.images[i].mount_point.c_str());
            }
          }
          if (!startup_failed) {
            const launcher_image& img = handoff.images[root_image];
            mounted = mount_image_region(img.file, img.offset, img.size, mount_point) == 0;
            startup_failed = !mounted;
          }
        }
      }
      else if (cargs.run && !startup_failed) {
        /*
         * --tebako-run: the application image carries its own TAMATEBAKO
         * descriptor header (mount point, entry point, cwd); the DwarFS
         * image starts right behind it, so the descriptor length selects
         * the mounted file region.
         */
        app_descriptor descriptor = read_app_descriptor(cargs.app_image);
        mount_point = descriptor.mount_point;
        entry_point = descriptor.entry_point;
        cwd = descriptor.cwd;
        mounted = mount_image_region(cargs.app_image, descriptor.size, 0, mount_point) == 0;
        startup_failed = !mounted;
      }
      else if (!startup_failed) {
        /*
         * Incbin bundle startup: probe the own executable for a tpkg
         * manifest trailer (Stage 3A, spec §4.3).
         *  - trailer present: mount the root slot's region of the own
         *    executable via the modern C API
         *  - no trailer: classic incbin mount from memory, unchanged
         *  - corrupt trailer: clean startup error (spec §6), no mount at all
         */
        tpkg_manifest manifest;
        int probe = read_self_manifest(&manifest);
        if (probe < 0) {
          startup_failed = true;  // read_self_manifest printed the reason
        }
        else if (probe > 0) {
          /* Root slot: the one recorded at the package mount point;
             fall back to slot 0 (mounted at the compiled-in root). */
          uint32_t root_slot = 0;
          for (uint32_t i = 0; i < manifest.slot_count; ++i) {
            if (mount_point == manifest.slots[i].mount_point) {
              root_slot = i;
              break;
            }
          }
          if (manifest.slots[root_slot].format_id == TPKG_FORMAT_RUNTIME ||
              manifest.slots[root_slot].mount_point[0] == '\0') {
            printf("Tebako: package manifest trailer in '%s' records no mountable root image slot.\n"
                   "  Re-stitch the package to repair the manifest.\n",
                   self_executable_path().c_str());
            startup_failed = true;
          }
          else {
            /* Runtime payload slots (fat packages) are never mounted -- the
               bootstrap installs them into the shared cache at first run;
               any further image slot is beyond the single-mount TFS. */
            for (uint32_t i = 0; i < manifest.slot_count && !startup_failed; ++i) {
              if (i != root_slot && manifest.slots[i].format_id != TPKG_FORMAT_RUNTIME &&
                  manifest.slots[i].mount_point[0] != '\0') {
                startup_failed = reject_extra_slot(manifest.slots[i].mount_point);
              }
            }
            if (!startup_failed) {
              mounted = mount_image_region(self_executable_path(), manifest.slots[root_slot].offset,
                                           manifest.slots[root_slot].size, mount_point) == 0;
              startup_failed = !mounted;
            }
          }
        }
        else {
          /* Classic incbin bundle: mount the embedded image from memory */
          if (tebako_fs_init(&gfsData[0], gfsSize, mount_point.c_str()) != 0) {
            printf("Tebako: failed to mount the embedded filesystem image: %s\n",
                   tebako_strerror(tebako_get_errno()));
            startup_failed = true;
          }
          else {
            mounted = true;
          }
        }
      }

      if (mounted) {
        if (cargs.extract) {
          /*
           * --tebako-extract: the mounted image is extracted natively by
           * libtfs (the legacy flow ran a ruby FileUtils.copy_entry snippet
           * against the memfs). Nothing is dispatched to the interpreter.
           */
          printf("Extracting tebako image to '%s' \n", cargs.extract_folder.c_str());
          if (tebako_fs_extract_all(cargs.extract_folder.c_str()) != 0) {
            printf("Tebako: failed to extract the package filesystem image to '%s': %s\n",
                   cargs.extract_folder.c_str(), tebako_strerror(tebako_get_errno()));
            tebako_fs_unmount();
            exit(1);
          }
          tebako_fs_unmount();
          exit(0);
        }

        build_dispatch_argv(cargs, mount_point, entry_point);
        *argc = static_cast<int>(dispatch_argv.size());
        *argv = dispatch_argv.data();
        ret = 0;
        atexit(tebako_clean);
      }
    }
    catch (const std::exception& e) {
      printf("Failed to process command line: %s\n", e.what());
    }

    if (getcwd(tebako::original_cwd, sizeof(tebako::original_cwd)) == nullptr) {
      printf("Failed to get current directory: %s\n", strerror(errno));
      ret = -1;
    }

    if (ret == 0 && cwd.has_value()) {
      /*
       * The legacy driver entered the package working directory through the
       * memfs chdir call; the modern C API has no chdir, so the process stays
       * in the launch directory. The packaged entry dispatcher (stub.rb)
       * performs Dir.chdir where the package provides one -- bundle-mode
       * packages pressed with --cwd are the one case that relied on the
       * driver-level chdir, hence the warning.
       */
      printf("Tebako: warning: package working directory '%s' is not honored by this runtime --\n"
             "  the modern TFS API has no chdir; the application starts in '%s'.\n",
             cwd->c_str(), tebako::original_cwd);
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

/**
 *
 * Copyright (c) 2021-2024 [Ribose Inc](https://www.ribose.com).
 * All rights reserved.
 * This file is a part of tebako
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

#include <string>
#include <cstdint>
#include <vector>
#include <stdexcept>
#include <tuple>

#ifdef _WIN32
#include <winsock2.h>
#include <windows.h>
#endif

#include <tebako/tebako-config.h>
#include <tebako/tebako-io.h>

#include <tebako/tebako-version.h>
#include <tebako/tebako-main.h>
#include <tebako/tebako-fs.h>
#include <tebako/tebako-cmdline-helpers.h>

static int running_miniruby = 0;

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
    try {
      fsret = mount_root_memfs(
        &gfsData[0],
        gfsSize,
        tebako::fs_log_level,
        nullptr /* cachesize */,
        nullptr /* workers */,
        nullptr /* mlock */,
        nullptr /* decompress_ratio*/,
        nullptr /* image_offset */
      );

      if (fsret == 0) {
        if ((*argc > 1) && strcmp((*argv)[1], "--tebako-extract") == 0) {
          ret = tebako::build_arguments_for_extract(argc, argv, tebako::fs_mount_point);
        }
        else {
          auto [mountpoints, parsed_argv] = tebako::parse_arguments(*argc, *argv);
          // for (auto& mp : mountpoints) {
          // printf("Mountpoint: %s\n", mp.c_str());
          // }
          tebako::process_mountpoints(mountpoints);
          std::tie(*argc, *argv) = tebako::build_arguments(parsed_argv, tebako::fs_mount_point, tebako::fs_entry_point);
          ret = 0;
        }
      }
      atexit(unmount_root_memfs);
    }

    catch (std::exception e) {
      printf("Failed to process command line: %s\n", e.what());
    }

    if (getcwd(tebako::original_cwd, sizeof(tebako::original_cwd)) == nullptr) {
      printf("Failed to get current directory: %s\n", strerror(errno));
      ret = -1;
    }

    if (tebako::needs_cwd) {
      if (tebako_chdir(tebako::package_cwd) != 0) {
        printf("Failed to chdir to '%s' : %s\n", tebako::package_cwd, strerror(errno));
        ret = -1;
      }
    }
  }

  if (ret != 0) {
    try {
      printf("Tebako initialization failed\n");
      if (new_argv) {
        delete new_argv;
        new_argv = nullptr;
      }
      if (argv_memory) {
        delete argv_memory;
        argv_memory = nullptr;
      }
      if (fsret == 0) {
        unmount_root_memfs();
      }
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

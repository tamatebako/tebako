/**
 *
 * Copyright (c) 2021-2022 [Ribose Inc](https://www.ribose.com).
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

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <memory.h>

#include <string>

#include <tebako/tebako-io.h>

#include <tebako-version.h>
#include <tebako-main.h>
#include <tebako-fs.h>

extern "C" int tebako_main(int* argc, char*** argv) {
	int ret = -1, fsret = -1;
	char** new_argv = NULL;
	char* argv_memory = NULL;

	if (strstr((*argv)[0], "miniruby") != NULL) {
// Ruby build script is designed in such a way that this patch is also applied towards miniruby
// Just pass through in such case
		ret = 0;
	}
	else {
		try {
			fsret = load_fs(&gfsData[0],
				gfsSize,
				tebako::fs_log_level,
				NULL	/* cachesize*/,
				NULL	/* workers */,
				NULL	/* mlock */,
				NULL	/* decompress_ratio*/,
				NULL    /* image_offset */
			);

			if (fsret == 0) {
				if ((*argc > 1) && strcmp((*argv)[1], "--tebako-extract")==0) {
				// ruby -e "require 'fileutils'; FileUtils.copy_entry '<tebako::fs_mount_point>',<argv[2] || 'source_filesystem'"
					std::string dest = std::string(((*argc) < 3 ? "source_filesystem" : (*argv)[2]));
					std::string cmd = std::string("require 'fileutils'; FileUtils.copy_entry '") + (tebako::fs_mount_point) + "', '" + dest + "'";
					printf("Extracting tebako image to '%s' \n", dest.c_str());
					size_t new_argv_size = 3 + cmd.size() + 1 + strlen((*argv)[0]) + 1;
					char** new_argv = new char* [3];
					char* argv_memory = new char[new_argv_size];
					if (new_argv != NULL && argv_memory != NULL) {
						strcpy(argv_memory, (*argv)[0]);
						new_argv[0] = argv_memory;
						argv_memory += (strlen((*argv)[0]) + 1);
						strcpy(argv_memory, "-e");
						new_argv[1] = argv_memory;
						argv_memory += 3;
						strcpy(argv_memory, cmd.c_str());
						new_argv[2] = argv_memory;
						ret = 0;
						*argv = new_argv;
						(*argc) = 3;
					}
				}
				else {
					size_t new_argv_size = strlen(tebako::fs_mount_point) + strlen(tebako::fs_entry_point) + 1;
					for (int i = 0; i < (*argc); i++) {
						new_argv_size += (strlen((*argv)[i]) + 1);
					}
					/* argv memory should be adjacent */
					char** new_argv = new char* [(*argc) + 1];
					char* argv_memory = new char[new_argv_size];
					if (new_argv != NULL && argv_memory != NULL) {
						memcpy(argv_memory, (*argv)[0], strlen((*argv)[0]) + 1);
						new_argv[0] = argv_memory;
						argv_memory += (strlen((*argv)[0]) + 1);
						memcpy(argv_memory, tebako::fs_mount_point, strlen(tebako::fs_mount_point));
						new_argv[1] = argv_memory;
						argv_memory += strlen(tebako::fs_mount_point);
						memcpy(argv_memory, tebako::fs_entry_point, strlen(tebako::fs_entry_point) + 1);
						argv_memory += (strlen(tebako::fs_entry_point) + 1);
						for (int i = 1; i < (*argc); i++) {
							memcpy(argv_memory, (*argv)[i], strlen((*argv)[i]) + 1);
							new_argv[i+1] = argv_memory;
							argv_memory += (strlen((*argv)[i]) + 1);
						}
						*argv = new_argv;
						(*argc) += 1;
						ret = 0;
					}
				}
			    atexit(drop_fs);
			}
		}
		catch (...) {

		}

		if (ret != 0) {
			try {
				printf("Tebako initialization failed\n");
				if (new_argv) delete new_argv;
				if (argv_memory) delete argv_memory;
				if (fsret == 0) {
					drop_fs();
				}
			}
			catch (...) {
				// Nested error, no recovery :(
			}
		}
	}
	return ret;
}

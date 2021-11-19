/**
 *
 * Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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
#include <memory.h>

#include <tebako/tebako-io.h>

#include <version.h>
#include <tebako-main.h>
#include <tebako-fs.h>

extern "C" int tebako_main(int* argc, char*** argv) {
	int ret = -1, fsret = -1;
	char** new_argv = NULL;
	char* argv_memory = NULL;

	try {
		fsret = load_fs(&gfsData[0],
			gfsSize,
			"debug" /*debuglevel*/,
			NULL	/* cachesize*/,
			NULL	/* workers */,
			NULL	/* mlock */,
			NULL	/* decompress_ratio*/,
			NULL    /* image_offset */
		);

		if (fsret == 0) {
			size_t new_argv_size = strlen(tebako::fs_entry_point);
			for (int i = 0; i < (*argc); i++) {
				new_argv_size += strlen((*argv)[i]);
			}
			/* argv memory should be adjacent */
			char** new_argv = new char* [(*argc) + 1];
			char* argv_memory = new char[new_argv_size];
			if (new_argv != NULL && argv_memory != NULL) {
				memcpy(argv_memory, (*argv)[0], strlen((*argv)[0]) + 1);
				new_argv[0] = argv_memory;
				argv_memory += strlen((*argv)[0]) + 1;
				memcpy(argv_memory, (*argv)[1], strlen(tebako::fs_entry_point) + 1);
				new_argv[1] = argv_memory;
				argv_memory += strlen(tebako::fs_entry_point) + 1;
				for (int i = 2; i < (*argc); i++) {
					memcpy(argv_memory, new_argv[i], strlen(new_argv[i]) + 1);
					new_argv[i] = argv_memory;
					argv_memory += strlen(new_argv[i]) + 1;
				}
				*argv = new_argv;
				(*argc) += 1;
				ret = 0;
			}
		}
	}
	catch (...)
	{

	}

	if (ret != 0) {
		try {
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
	return ret;
}
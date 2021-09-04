/**
 *
 * Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
 * All rights reserved.
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

#include "sys/mount.h"
#include <atomic>
#include <thread>
#include <chrono>
#include <iostream>
#if FUSE_USE_VERSION >= 30
#include <fuse3/fuse_lowlevel.h>
#else
#include <fuse.h>
#include <fuse/fuse_lowlevel.h>
#endif

#include "version.h"
#include "tebako-fs.h"
#include "tebako-dfs.h"


// TODO:
// This is just a sketch
// Shall merge run_dwarfs into dwarfs_starter 

const int max_wait_cycles = 10;
std::atomic_int ret = 0;

void dwarfs_starter(void* args) {
    ret = dwarfs::run_dwarfs(reinterpret_cast<struct fuse_args*>(args));
}

int main(int argc, char** argv) {
    return dwarfs::safe_main([&]
        {
            std::cerr << "Starting " << PRJ_NAME << " version " << PRJ_VERSION_STRING << "..." << std::endl;


            using namespace std::chrono_literals;
            int wait_cycle = 0;
            char* _argv[2];

            // TODO:
            // use dealloc provided by fuse lib ?
            _argv[0] = argv[0];
            _argv[1] = strdup(tebako::fs_mount_point);
            struct fuse_args args = FUSE_ARGS_INIT(2, _argv);

            std::thread dfs(dwarfs_starter, &args);

            while (ret==0 &&
                   !dwarfs::is_fuse_session_ready() &&
                   wait_cycle++ < max_wait_cycles) 
                   std::this_thread::sleep_for(100ms);

            if (ret ==0 && !dwarfs::is_fuse_session_ready()) {
                std::cerr << "Exceeded startup time. Exiting ..." << std::endl;
                // No attempts to free memory. It is a crash state already
                // free(_argv[1]);
                ::exit(-1);
            }

            if (ret == 0) {
//                std::string cmd("ls -l ");
//                cmd += tebako::fs_mount_point;
//                std::cerr << "Running " << cmd << std::endl;
//                system(cmd.c_str());

//                cmd += "/local";
//                std::cerr << "Running " << cmd << std::endl;
//                system(cmd.c_str());

//                std::string cmd = tebako::fs_mount_point;
//                cmd += "/tests/test-0.sh";
//                system(cmd.c_str());
                
//                cmd = std::string("ls -l ") + tebako::fs_mount_point + "/bin";
//                std::cerr << "Running " << cmd << std::endl;
//                ret = system(cmd.c_str());

                    
                std::string cmd = std::string(tebako::fs_mount_point) + tebako::fs_entry_point;
                std::cerr << "Running " << cmd << std::endl;
                ret = system(cmd.c_str());

                if (ret) {
                    std::cerr << "Packaged program exited with error code " << ret << std::endl;
                } 

                dwarfs::stop_fuse_session();
                dfs.join();

            }
            else {
                std::cerr << "dwarFS startup failed. Exiting ..." << std::endl;
            }
            free(_argv[1]);

            return (int)ret;
        });
}

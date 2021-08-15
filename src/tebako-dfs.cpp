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

#include <array>
#include <iostream>
#include <stdexcept>

#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <filesystem>

#include <folly/Conv.h>
#include <folly/experimental/symbolizer/SignalHandler.h>

#if FUSE_USE_VERSION >= 30
#include <fuse3/fuse_lowlevel.h>
#else
#include <fuse.h>
#include <fuse/fuse_lowlevel.h>
#endif

#include "version.h"
#include "tebako-fs.h"
#include "tebako-mfs.h"
#include "tebako-dfs.h"

namespace dwarfs {
 
#define DWARFS_OPT(t, p, v)                                                    \
  { t, offsetof(struct options, p), v }

    constexpr struct ::fuse_opt dwarfs_opts[] = {
        // TODO: user, group, atime, mtime, ctime for those fs who don't have it?
        DWARFS_OPT("cachesize=%s", cachesize_str, 0),
        DWARFS_OPT("debuglevel=%s", debuglevel_str, 0),
        DWARFS_OPT("workers=%s", workers_str, 0),
        DWARFS_OPT("mlock=%s", mlock_str, 0),
        DWARFS_OPT("decratio=%s", decompress_ratio_str, 0),
        DWARFS_OPT("offset=%s", image_offset_str, 0),
        DWARFS_OPT("enable_nlink", enable_nlink, 1),
        DWARFS_OPT("readonly", readonly, 1),
        DWARFS_OPT("cache_image", cache_image, 1),
        DWARFS_OPT("no_cache_image", cache_image, 0),
        DWARFS_OPT("cache_files", cache_files, 1),
        DWARFS_OPT("no_cache_files", cache_files, 0),
        FUSE_OPT_END };

    void usage(const char* progname) {
        std::cerr
            << "usage: " << progname << " mountpoint [options]\n\n"
            << "DWARFS options:\n"
            << "    -o cachesize=SIZE      set size of block cache (512M)\n"
            << "    -o workers=NUM         number of worker threads (2)\n"
            << "    -o mlock=NAME          mlock mode: (none), try, must\n"
            << "    -o decratio=NUM        ratio for full decompression (0.8)\n"
            << "    -o offset=NUM|auto     filesystem image offset in bytes (0)\n"
            << "    -o enable_nlink        show correct hardlink numbers\n"
            << "    -o readonly            show read-only file system\n"
            << "    -o (no_)cache_image    (don't) keep image in kernel cache\n"
            << "    -o (no_)cache_files    (don't) keep files in kernel cache\n"
            << "    -o debuglevel=NAME     error, warn, (info), debug, trace\n"
            << std::endl;

#if FUSE_USE_VERSION >= 30
        fuse_cmdline_help();
#else
        struct fuse_args args = FUSE_ARGS_INIT(0, nullptr);
        fuse_opt_add_arg(&args, progname);
        fuse_opt_add_arg(&args, "-ho");
        struct fuse_operations fsops;
        ::memset(&fsops, 0, sizeof(fsops));
        fuse_main(args.argc, args.argv, &fsops, nullptr);
        fuse_opt_free_args(&args);
#endif

        ::exit(1);
    }

    int option_hdl(void* data, const char* arg, int key,
        struct fuse_args* /*outargs*/) {
        auto* opts = reinterpret_cast<options*>(data);

        switch (key) {
        case FUSE_OPT_KEY_NONOPT:
            if (opts->seen_mountpoint) {
                return -1;
            }
            opts->seen_mountpoint = 1;
            break;

        case FUSE_OPT_KEY_OPT:
            if (::strncmp(arg, "-h", 2) == 0 || ::strncmp(arg, "--help", 6) == 0) {
                usage(opts->progname);
            }
            break;

        default:
            break;
        }

        return 1;
    }


#if FUSE_USE_VERSION > 30

//  TODO:
//  This set of fuse operations shall be wrapped in a class ???
//  or  something like it

    std::atomic<fuse_session*> session = NULL;
    std::atomic_bool session_ready = false;


    void stop_fuse_session(void) {
        if (session) {
            fuse_session_exit(session);
            fuse_session_unmount(session);
            session_ready = false;
        }
    }

    bool is_fuse_session_ready(void) {
        return session != NULL && !fuse_session_exited(session) && session_ready;
    }


    int run_fuse(struct fuse_args& args, struct fuse_cmdline_opts const& fuse_opts,
        dwarfs_userdata& userdata) {
        struct fuse_lowlevel_ops fsops;

        tebako::init_fuse_ops(fsops, userdata.opts.debuglevel);
        int err = 1;

        if (session =
            fuse_session_new(&args, &fsops, sizeof(fsops), &userdata)) {
            if (fuse_set_signal_handlers(session) == 0) {
                if (fuse_session_mount(session, fuse_opts.mountpoint) == 0) {
                    session_ready = true;
                    if (fuse_daemonize(fuse_opts.foreground) == 0) {
                        if (fuse_opts.singlethread) {
                            err = fuse_session_loop(session);
                        }
                        else {
                            struct fuse_loop_config config;
                            config.clone_fd = fuse_opts.clone_fd;
                            config.max_idle_threads = fuse_opts.max_idle_threads;
                            err = fuse_session_loop_mt(session, &config);
                        }
                    }
                    session_ready = false;
                    fuse_session_unmount(session);
                }
                fuse_remove_signal_handlers(session);
            }
            fuse_session_destroy(session);
            session = NULL;
        }

        ::free(fuse_opts.mountpoint);
        fuse_opt_free_args(&args);

        return err;
    }

#else

    int run_fuse(struct fuse_args& args, char* mountpoint, int mt, int fg,
        dwarfs_userdata& userdata) {
        struct fuse_lowlevel_ops fsops;

        ::memset(&fsops, 0, sizeof(fsops));

        if (userdata.opts.debuglevel >= logger::DEBUG) {
            tebako::init_fuse_ops<debug_logger_policy>(fsops);
        }
        else {
            tebako::init_fuse_ops<prod_logger_policy>(fsops);
        }

        int err = 1;

        if (auto ch = fuse_mount(mountpoint, &args)) {
            if (auto se = fuse_lowlevel_new(&args, &fsops, sizeof(fsops), &userdata)) {
                if (fuse_daemonize(fg) != -1) {
                    if (fuse_set_signal_handlers(se) != -1) {
                        fuse_session_add_chan(se, ch);
                        err = mt ? fuse_session_loop_mt(se) : fuse_session_loop(se);
                        fuse_remove_signal_handlers(se);
                        fuse_session_remove_chan(ch);
                    }
                }
                fuse_session_destroy(se);
            }
            fuse_unmount(mountpoint, ch);
        }

        ::free(mountpoint);
        fuse_opt_free_args(&args);

        return err;
    }

#endif

    template <typename LoggerPolicy>
    void load_filesystem(dwarfs_userdata& userdata) {
        LOG_PROXY(LoggerPolicy, userdata.lgr);
        auto ti = LOG_TIMED_INFO;
        auto& opts = userdata.opts;

        filesystem_options fsopts;
        fsopts.lock_mode = opts.lock_mode;
        fsopts.block_cache.max_bytes = opts.cachesize;
        fsopts.block_cache.num_workers = opts.workers;
        fsopts.block_cache.decompress_ratio = opts.decompress_ratio;
        fsopts.block_cache.mm_release = !opts.cache_image;
        fsopts.block_cache.init_workers = false;
        fsopts.metadata.enable_nlink = bool(opts.enable_nlink);
        fsopts.metadata.readonly = bool(opts.readonly);

        if (opts.image_offset_str) {
            std::string image_offset{ opts.image_offset_str };

            try {
                fsopts.image_offset = image_offset == "auto"
                    ? filesystem_options::IMAGE_OFFSET_AUTO
                    : folly::to<off_t>(image_offset);
            }
            catch (...) {
                DWARFS_THROW(runtime_error, "failed to parse offset: " + image_offset);
            }
        }

        userdata.fs = filesystem_v2(
            userdata.lgr, std::make_shared<tebako::mfs>(&tebako::gfsData, tebako::gfsSize), fsopts, FUSE_ROOT_ID);


        ti << "file system initialized";
    }

    int run_dwarfs(struct fuse_args* args) {
        dwarfs_userdata userdata(std::cerr);
        auto& opts = userdata.opts;

        opts.progname = PRJ_NAME;
        opts.cache_image = 0;
        opts.cache_files = 1;

        fuse_opt_parse(args, &opts, dwarfs_opts, option_hdl);

#if FUSE_USE_VERSION >= 30
        struct fuse_cmdline_opts fuse_opts;

        if (fuse_parse_cmdline(args, &fuse_opts) == -1 || !fuse_opts.mountpoint) {
            usage(opts.progname);
        }

        fuse_opts.foreground = true;

        if (fuse_opts.foreground) {
            folly::symbolizer::installFatalSignalHandler();
        }
#else
        char* mountpoint = nullptr;
        int mt, fg;

        if (fuse_parse_cmdline(args, &mountpoint, &mt, &fg) == -1 || !mountpoint) {
            usage(opts.progname);
        }

        if (fg) {
            folly::symbolizer::installFatalSignalHandler();
        }
#endif

        try {
            // TODO: foreground mode, stderr vs. syslog?

            opts.debuglevel = opts.debuglevel_str
                ? logger::parse_level(opts.debuglevel_str)
                : logger::INFO;

            userdata.lgr.set_threshold(opts.debuglevel);
            userdata.lgr.set_with_context(opts.debuglevel >= logger::DEBUG);

            opts.cachesize = opts.cachesize_str
                ? parse_size_with_unit(opts.cachesize_str)
                : (static_cast<size_t>(512) << 20);
            opts.workers = opts.workers_str ? folly::to<size_t>(opts.workers_str) : 2;
            opts.lock_mode =
                opts.mlock_str ? parse_mlock_mode(opts.mlock_str) : mlock_mode::NONE;
            opts.decompress_ratio = opts.decompress_ratio_str
                ? folly::to<double>(opts.decompress_ratio_str)
                : 0.8;
        }
        catch (runtime_error const& e) {
            std::cerr << "error: " << e.what() << std::endl;
            return 1;
        }
        catch (std::filesystem::filesystem_error const& e) {
            std::cerr << e.what() << std::endl;
            return 1;
        }

        if (opts.decompress_ratio < 0.0 || opts.decompress_ratio > 1.0) {
            std::cerr << "error: decratio must be between 0.0 and 1.0" << std::endl;
            return 1;
        }

        if (!opts.seen_mountpoint) {
            usage(opts.progname);
        }

        LOG_PROXY(debug_logger_policy, userdata.lgr);

        LOG_INFO << PRJ_NAME
            << " version " << PRJ_VERSION_STRING
            << ", fuse version " << FUSE_USE_VERSION;

        try {
            if (userdata.opts.debuglevel >= logger::DEBUG) {
                load_filesystem<debug_logger_policy>(userdata);
            }
            else {
                load_filesystem<prod_logger_policy>(userdata);
            }
        }
        catch (std::exception const& e) {
            LOG_ERROR << "error initializing file system: " << e.what();
            return 1;
        }

#if FUSE_USE_VERSION >= 30
        return run_fuse(*args, fuse_opts, userdata);
#else
        return run_fuse(*args, mountpoint, mt, fg, userdata);
#endif
    }

} // namespace dwarfs


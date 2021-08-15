/**
 *
 * Copyright (c) 2021 The Unknown Fathers
 * This file is a part of tebako
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

#pragma once

#include "dwarfs/error.h"
#include "dwarfs/filesystem_v2.h"
#include "dwarfs/fstypes.h"
#include "dwarfs/logger.h"
#include "dwarfs/metadata_v2.h"
#include "dwarfs/mmap.h"
#include "dwarfs/options.h"
#include "dwarfs/util.h"
#include "dwarfs/version.h"

namespace dwarfs {

    struct options {
        const char* progname{ nullptr };
        std::string fsimage;
        int seen_mountpoint{ 0 };
        const char* cachesize_str{ nullptr };        // TODO: const?? -> use string?
        const char* debuglevel_str{ nullptr };       // TODO: const?? -> use string?
        const char* workers_str{ nullptr };          // TODO: const?? -> use string?
        const char* mlock_str{ nullptr };            // TODO: const?? -> use string?
        const char* decompress_ratio_str{ nullptr }; // TODO: const?? -> use string?
        const char* image_offset_str{ nullptr };     // TODO: const?? -> use string?
        int enable_nlink{ 0 };
        int readonly{ 0 };
        int cache_image{ 0 };
        int cache_files{ 0 };
        size_t cachesize{ 0 };
        size_t workers{ 0 };
        mlock_mode lock_mode{ mlock_mode::NONE };
        double decompress_ratio{ 0.0 };
        logger::level_type debuglevel{ logger::level_type::ERROR };
    };

    struct dwarfs_userdata {
        dwarfs_userdata(std::ostream& os)
            : lgr{ os } {}

        options opts;
        stream_logger lgr;
        filesystem_v2 fs;
    };

    int run_dwarfs(struct fuse_args* args);
    void stop_fuse_session(void);
    bool is_fuse_session_ready(void);

}
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

#include <iostream>
#include <stdexcept>

#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <filesystem>

#if FUSE_USE_VERSION >= 30
#include <fuse3/fuse_lowlevel.h>
#else
#include <fuse.h>
#include <fuse/fuse_lowlevel.h>
#endif

#include "tebako-mfs.h"
#include "tebako-dfs.h"

namespace tebako {

    template <typename LoggerPolicy>
    static void _init(void* data, struct fuse_conn_info* /*conn*/) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(data);
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;
        userdata->fs.set_num_workers(userdata->opts.workers);
    }

    template <typename LoggerPolicy>
    static void _lookup(fuse_req_t req, fuse_ino_t parent, const char* name) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__ << "(" << parent << ", " << name << ")";

        int err = ENOENT;
        try {
            auto entry = userdata->fs.find(parent, name);

            if (entry) {
                struct ::fuse_entry_param e;
                err = userdata->fs.getattr(*entry, &e.attr);

                if (err == 0) {
                    e.generation = 1;
                    e.ino = e.attr.st_ino;
                    e.attr_timeout = std::numeric_limits<double>::max();
                    e.entry_timeout = std::numeric_limits<double>::max();

                    fuse_reply_entry(req, &e);

                    return;
                }
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _getattr(fuse_req_t req, fuse_ino_t ino, struct fuse_file_info*) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__ << "(" << ino << ")";

        int err = ENOENT;
        try {
            auto entry = userdata->fs.find(ino);

            if (entry) {
                struct ::stat stbuf;
                err = userdata->fs.getattr(*entry, &stbuf);

                if (err == 0) {
                    fuse_reply_attr(req, &stbuf, std::numeric_limits<double>::max());

                    return;
                }
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _access(fuse_req_t req, fuse_ino_t ino, int mode) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = ENOENT;

        try {
            auto entry = userdata->fs.find(ino);

            if (entry) {
                auto ctx = fuse_req_ctx(req);
                err = userdata->fs.access(*entry, mode, ctx->uid, ctx->gid);
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _readlink(fuse_req_t req, fuse_ino_t ino) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = ENOENT;

        try {
            auto entry = userdata->fs.find(ino);

            if (entry) {
                std::string str;
                err = userdata->fs.readlink(*entry, &str);

                if (err == 0) {
                    fuse_reply_readlink(req, str.c_str());

                    return;
                }
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _open(fuse_req_t req, fuse_ino_t ino, struct fuse_file_info* fi) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = ENOENT;

        try {
            auto entry = userdata->fs.find(ino);

            if (entry) {
                if (S_ISDIR(entry->mode())) {
                    err = EISDIR;
                }
                else if (fi->flags & (O_APPEND | O_CREAT | O_TRUNC)) {
                    err = EACCES;
                }
                else {
                    fi->fh = FUSE_ROOT_ID + entry->inode_num();
                    fi->direct_io = !userdata->opts.cache_files;
                    fi->keep_cache = userdata->opts.cache_files;
                    fuse_reply_open(req, fi);
                    return;
                }
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _read(fuse_req_t req, fuse_ino_t ino, size_t size, off_t off, struct fuse_file_info* fi) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = ENOENT;

        try {
            if (fi->fh == ino) {
                dwarfs::iovec_read_buf buf;
                ssize_t rv = userdata->fs.readv(ino, buf, size, off);

                if (rv >= 0) {
                    fuse_reply_iov(req, buf.buf.empty() ? nullptr : &buf.buf[0],
                        buf.buf.size());

                    return;
                }

                err = -rv;
            }
            else {
                err = EIO;
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _readdir(fuse_req_t req, fuse_ino_t ino, size_t size, off_t off, struct fuse_file_info* /*fi*/) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = ENOENT;

        try {
            auto dirent = userdata->fs.find(ino);

            if (dirent) {
                auto dir = userdata->fs.opendir(*dirent);

                if (dir) {
                    off_t lastoff = userdata->fs.dirsize(*dir);
                    struct stat stbuf;
                    std::vector<char> buf(size);
                    size_t written = 0;

                    while (off < lastoff) {
                        auto res = userdata->fs.readdir(*dir, off);
                        assert(res);

                        auto [entry, name_view] = *res;
                        std::string name(name_view);

                        userdata->fs.getattr(entry, &stbuf);

                        size_t needed =
                            fuse_add_direntry(req, &buf[written], buf.size() - written,
                                name.c_str(), &stbuf, off + 1);

                        if (written + needed > size) {
                            break;
                        }

                        written += needed;
                        ++off;
                    }

                    fuse_reply_buf(req, written > 0 ? &buf[0] : nullptr, written);

                    return;
                }

                err = ENOTDIR;
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _statfs(fuse_req_t req, fuse_ino_t /*ino*/) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__;

        int err = EIO;

        try {
            struct ::statvfs buf;
            err = userdata->fs.statvfs(&buf);

            if (err == 0) {
                fuse_reply_statfs(req, &buf);
                return;
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }

    template <typename LoggerPolicy>
    static void _getxattr(fuse_req_t req, fuse_ino_t ino, char const* name, size_t size) {
        auto userdata = reinterpret_cast<dwarfs::dwarfs_userdata*>(fuse_req_userdata(req));
        LOG_PROXY(LoggerPolicy, userdata->lgr);
        LOG_DEBUG << __func__ << "(" << ino << ", " << name << ", " << size << ")";

        static constexpr std::string_view pid_xattr{ "user.dwarfs.driver.pid" };
        int err = ENODATA;

        try {
            if (ino == FUSE_ROOT_ID && name == pid_xattr) {
                auto pidstr = std::to_string(::getpid());
                if (size > 0) {
                    fuse_reply_buf(req, pidstr.data(), pidstr.size());
                }
                else {
                    fuse_reply_xattr(req, pidstr.size());
                }
                return;
            }
        }
        catch (dwarfs::system_error const& e) {
            LOG_ERROR << e.what();
            err = e.get_errno();
        }
        catch (std::exception const& e) {
            LOG_ERROR << e.what();
            err = EIO;
        }

        fuse_reply_err(req, err);
    }


    template <typename LoggerPolicy>
    static void _init_fuse_ops(struct fuse_lowlevel_ops& ops) {
        ops.init = &_init<LoggerPolicy>;
        ops.lookup = &_lookup<LoggerPolicy>;
        ops.getattr = &_getattr<LoggerPolicy>;
        ops.access = &_access<LoggerPolicy>;
        ops.readlink = &_readlink<LoggerPolicy>;
        ops.open = &_open<LoggerPolicy>;
        ops.read = &_read<LoggerPolicy>;
        ops.readdir = &_readdir<LoggerPolicy>;
        ops.statfs = &_statfs<LoggerPolicy>;
        ops.getxattr = &_getxattr<LoggerPolicy>;
    }

    void init_fuse_ops(struct fuse_lowlevel_ops& ops, dwarfs::logger::level_type debuglevel) {
        ::memset(&ops, 0, sizeof(ops));

        if (debuglevel >= dwarfs::logger::DEBUG) {
            _init_fuse_ops<dwarfs::prod_logger_policy>(ops);
        }
        else {
            _init_fuse_ops<dwarfs::prod_logger_policy>(ops);
        }

    }

}  // namespace tebako
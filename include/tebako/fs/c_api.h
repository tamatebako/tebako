/**
 *
 * Copyright (c) 2024-2025 [Ribose Inc](https://www.ribose.com).
 * All rights reserved.
 * This file is a part of the Tebako project (libtfs).
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
 * PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

#ifndef TEBAKO_FS_C_API_H
#define TEBAKO_FS_C_API_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <tebako/fs/platform.h>

/* Directory entry type constants (from POSIX dirent.h) */
#ifndef DT_REG
#define DT_REG 8 /**< Regular file */
#endif
#ifndef DT_DIR
#define DT_DIR 4 /**< Directory */
#endif

/* ============================================================
 * FD Namespace Separation
 * ============================================================ */

/**
 * @brief Flag bit to distinguish libtfs FDs from host OS FDs
 *
 * This bit is set on all file descriptors returned by tebako_fs_open()
 * to ensure they never collide with host OS file descriptors.
 */
#define TEBAKO_FD_FLAG 0x40000000

/**
 * @brief Maximum internal FD value
 */
#define TEBAKO_FD_MAX 0x0FFFFFFF

/* ============================================================
 * Lifecycle Management
 * ============================================================ */

/**
 * @brief Initialize libtfs from file path
 *
 * Opens and mounts an archive file at the specified mount point.
 * The archive format is auto-detected (ZIP, SquashFS, etc.).
 *
 * @param archive_path Path to archive file on disk
 * @param mount_point Virtual mount point (e.g., "/__tebako__")
 * @return 0 on success, -1 on error (check errno via tebako_get_errno())
 *
 * @note Only one filesystem can be mounted at a time
 * @note Calling this while already mounted returns -1 with errno=EEXIST
 * @note The mount point should be an absolute path
 *
 * @example
 * @code
 * if (tebako_fs_init_from_file("/app/data.zip", "/__tebako__") == 0) {
 *     // Filesystem ready for use
 * }
 * @endcode
 */
int tebako_fs_init_from_file(const char* archive_path, const char* mount_point);

/**
 * @brief Initialize libtfs from a region of a file
 *
 * Mounts `length` bytes starting at byte `offset` of the archive file at the
 * specified mount point. The archive format is auto-detected from the region
 * content (DwarFS, ZIP, SquashFS).
 *
 * This allows mounting an image embedded inside a larger file (e.g., appended
 * to an executable or stored alongside a manifest trailer) without copying
 * the surrounding data.
 *
 * @param archive_path Path to the file containing the archive
 * @param offset Byte offset of the archive start within the file
 * @param length Length of the archive in bytes; 0 means "to end of file"
 * @param mount_point Virtual mount point (e.g., "/__tebako__")
 * @return 0 on success, -1 on error (check errno via tebako_get_errno())
 *
 * @note Only one filesystem can be mounted at a time
 * @note Calling this while already mounted returns -1 with errno=EEXIST
 * @note offset == 0 && length == 0 mounts the whole file directly (zero-copy);
 *       any other region is read into memory owned by libtfs until
 *       tebako_fs_unmount()
 * @note Returns -1 with errno=ENOENT if the file does not exist, errno=EINVAL
 *       if offset is past end of file or offset+length exceeds the file size
 *
 * @example
 * @code
 * // Mount a DwarFS image stored at offset 4096 of a package file
 * if (tebako_fs_init_from_file_at("/app/pkg.bin", 4096, 0, "/__tebako__") == 0) {
 *     // Filesystem ready for use
 * }
 * @endcode
 */
int tebako_fs_init_from_file_at(const char* archive_path, uint64_t offset, uint64_t length, const char* mount_point);

/**
 * @brief Initialize libtfs from memory-embedded image
 *
 * Mounts an archive from memory (typically embedded in executable).
 * The archive format is auto-detected.
 *
 * @param data Pointer to archive data in memory
 * @param size Size of archive in bytes
 * @param mount_point Virtual mount point
 * @return 0 on success, -1 on error
 *
 * @note The memory buffer must remain valid until tebako_fs_unmount()
 * @note Only one filesystem can be mounted at a time
 *
 * @example
 * @code
 * extern const uint8_t embedded_archive[];
 * extern const size_t embedded_archive_size;
 *
 * if (tebako_fs_init(embedded_archive, embedded_archive_size,
 *                    "/__tebako__") == 0) {
 *     // Filesystem ready
 * }
 * @endcode
 */
int tebako_fs_init(const void* data, size_t size, const char* mount_point);

/**
 * @brief Unmount and cleanup libtfs
 *
 * Closes all open file handles and releases all resources.
 * After unmount, all file descriptors and directory handles become invalid.
 *
 * @note Safe to call multiple times
 * @note Does nothing if not currently mounted
 */
void tebako_fs_unmount(void);

/**
 * @brief Check if libtfs is initialized
 *
 * @return 1 if mounted and ready, 0 otherwise
 */
int tebako_is_initialized(void);

/* ============================================================
 * Multi-Mount Management
 * ============================================================ */

/**
 * @brief Opaque mount handle
 *
 * Identifies one mounted archive in the libtfs mount table.
 * Handles are small increasing integers (>= 0) and are never reused
 * within a process run.
 */
typedef int tebako_mount_t;

/**
 * @brief Mount an archive file, returning a mount handle
 *
 * Multi-mount variant of tebako_fs_init_from_file(): mounts the archive at
 * the specified mount point without disturbing any existing mounts.
 * The archive format is auto-detected (ZIP, SquashFS, etc.).
 *
 * @param archive_path Path to archive file on disk
 * @param mount_point Virtual mount point (e.g., "/__tebako_data__");
 *                    must be non-empty and not already mounted
 * @param out_handle Receives the mount handle on success
 * @return 0 on success, -1 on error (check errno via tebako_get_errno())
 *
 * @note Returns -1 with errno=EEXIST if mount_point is already mounted
 * @note Returns -1 with errno=EINVAL for bad arguments (NULL archive_path,
 *       NULL or empty mount_point, NULL out_handle)
 * @note Any number of archives can be mounted concurrently; paths are
 *       dispatched to the owning mount by longest mount-point prefix match
 *
 * @example
 * @code
 * tebako_mount_t h;
 * if (tebako_fs_mount_from_file("/app/data.zip", "/__tebako_data__", &h) == 0) {
 *     // ... use ...
 *     tebako_fs_unmount_handle(h);
 * }
 * @endcode
 */
int tebako_fs_mount_from_file(const char* archive_path, const char* mount_point, tebako_mount_t* out_handle);

/**
 * @brief Mount a region of a file, returning a mount handle
 *
 * Multi-mount variant of tebako_fs_init_from_file_at(): mounts `length`
 * bytes starting at byte `offset` of the archive file at the specified
 * mount point. The archive format is auto-detected from the region content
 * (DwarFS, ZIP, SquashFS).
 *
 * @param archive_path Path to the file containing the archive
 * @param offset Byte offset of the archive start within the file
 * @param length Length of the archive in bytes; 0 means "to end of file"
 * @param mount_point Virtual mount point; must be non-empty and not
 *                    already mounted
 * @param out_handle Receives the mount handle on success
 * @return 0 on success, -1 on error (check errno via tebako_get_errno())
 *
 * @note Returns -1 with errno=EEXIST if mount_point is already mounted
 * @note Returns -1 with errno=EINVAL for bad arguments (NULL archive_path,
 *       NULL or empty mount_point, NULL out_handle, offset past end of
 *       file, offset+length exceeding the file size)
 * @note Returns -1 with errno=ENOENT if the file does not exist
 * @note offset == 0 && length == 0 mounts the whole file directly
 *       (zero-copy); any other region is read into memory owned by libtfs
 *       until the mount is unmounted
 */
int tebako_fs_mount_from_file_at(const char* archive_path, uint64_t offset, uint64_t length, const char* mount_point,
                                 tebako_mount_t* out_handle);

/**
 * @brief Mount an archive from memory, returning a mount handle
 *
 * Multi-mount variant of tebako_fs_init(): mounts an archive residing in
 * memory (typically embedded in an executable). The archive format is
 * auto-detected.
 *
 * @param data Pointer to archive data in memory
 * @param size Size of archive in bytes
 * @param mount_point Virtual mount point; must be non-empty and not
 *                    already mounted
 * @param out_handle Receives the mount handle on success
 * @return 0 on success, -1 on error
 *
 * @note The memory buffer must remain valid until the mount is unmounted
 * @note Returns -1 with errno=EEXIST if mount_point is already mounted
 * @note Returns -1 with errno=EINVAL for bad arguments (NULL data, zero
 *       size, NULL or empty mount_point, NULL out_handle)
 */
int tebako_fs_mount_from_memory(const void* data, size_t size, const char* mount_point, tebako_mount_t* out_handle);

/**
 * @brief Unmount a single mount by handle
 *
 * Force-closes all file descriptors and directory handles owned by this
 * mount (subsequent operations on them fail with EBADF), destroys the
 * filesystem, and releases the mount point. Other mounts are unaffected.
 *
 * @param handle Mount handle returned by a tebako_fs_mount_* call
 * @return 0 on success, -1 with errno=ENODEV if the handle is unknown
 *
 * @note tebako_fs_unmount() still unmounts ALL mounts at once
 */
int tebako_fs_unmount_handle(tebako_mount_t handle);

/* ============================================================
 * File Operations
 * ============================================================ */

/**
 * @brief Open a file from embedded filesystem
 *
 * Behaves like POSIX open(2). Returns a file descriptor with
 * TEBAKO_FD_FLAG set to distinguish from host OS file descriptors.
 *
 * @param path Absolute path within mount point
 * @param flags Open flags (O_RDONLY, etc.) - write operations not supported
 * @return File descriptor on success (>0 with TEBAKO_FD_FLAG set), -1 on error
 *
 * @note Only read-only access is supported
 * @note The returned FD must be closed with tebako_fs_close()
 * @note Do not use with standard close() - use tebako_fs_close() instead
 *
 * @example
 * @code
 * int fd = tebako_fs_open("/__tebako__/config.txt", O_RDONLY);
 * if (fd > 0) {
 *     // Read file...
 *     tebako_fs_close(fd);
 * }
 * @endcode
 */
int tebako_fs_open(const char* path, int flags);

/**
 * @brief Read from embedded file
 *
 * Reads up to count bytes from the file into buffer.
 * Behaves like POSIX read(2).
 *
 * @param fd File descriptor from tebako_fs_open()
 * @param buf Buffer to read into
 * @param count Maximum number of bytes to read
 * @return Number of bytes read (may be less than count), 0 on EOF, -1 on error
 *
 * @note Returns -1 with errno=EBADF if fd is not a valid libtfs FD
 */
ssize_t tebako_fs_read(int fd, void* buf, size_t count);

/**
 * @brief Read from embedded file at a given offset
 *
 * Reads up to nbyte bytes from the file at byte `offset` into buffer.
 * Behaves like POSIX pread(2): the file position of `fd` is NOT modified
 * by this call, and concurrent reads through the same fd are unaffected.
 *
 * @param fd File descriptor from tebako_fs_open()
 * @param buf Buffer to read into
 * @param nbyte Maximum number of bytes to read
 * @param offset Byte offset from the beginning of the file
 * @return Number of bytes read (may be less than nbyte), 0 at EOF, -1 on error
 *
 * @note Returns -1 with errno=EBADF if fd is not a valid libtfs FD
 * @note Returns -1 with errno=EINVAL if offset is negative
 */
ssize_t tebako_fs_pread(int fd, void* buf, size_t nbyte, off_t offset);

/**
 * @brief Seek within embedded file
 *
 * Changes the file position. Behaves like POSIX lseek(2).
 *
 * @param fd File descriptor
 * @param offset Offset value
 * @param whence Position reference:
 *               - SEEK_SET: offset from beginning
 *               - SEEK_CUR: offset from current position
 *               - SEEK_END: offset from end of file
 * @return New file position from beginning, or -1 on error
 *
 * @example
 * @code
 * // Seek to byte 100
 * off_t pos = tebako_fs_lseek(fd, 100, SEEK_SET);
 *
 * // Get file size
 * off_t size = tebako_fs_lseek(fd, 0, SEEK_END);
 * tebako_fs_lseek(fd, 0, SEEK_SET);  // Reset to start
 * @endcode
 */
off_t tebako_fs_lseek(int fd, off_t offset, int whence);

/**
 * @brief Close embedded file
 *
 * Releases resources associated with the file descriptor.
 *
 * @param fd File descriptor from tebako_fs_open()
 * @return 0 on success, -1 on error
 *
 * @note After close, the FD becomes invalid
 * @note Safe to call multiple times with the same FD
 */
int tebako_fs_close(int fd);

/* ============================================================
 * Directory Operations
 * ============================================================ */

/**
 * @brief Directory handle type (opaque)
 *
 * This is an opaque pointer to internal directory iteration state.
 * Do not dereference or modify directly.
 */
typedef void* tebako_dir_t;

/**
 * @brief Directory entry structure
 *
 * Compatible with POSIX struct dirent for easier integration.
 */
struct tebako_c_dirent {
  char d_name[256];     /**< Entry name (null-terminated) */
  unsigned char d_type; /**< Entry type: DT_REG (file) or DT_DIR (directory) */
};

/**
 * @brief Open directory from embedded filesystem
 *
 * @param path Directory path
 * @return Directory handle on success, NULL on error
 *
 * @note The returned handle must be closed with tebako_fs_closedir()
 *
 * @example
 * @code
 * tebako_dir_t dir = tebako_fs_opendir("/__tebako__/config");
 * if (dir != NULL) {
 *     struct tebako_c_dirent* entry;
 *     while ((entry = tebako_fs_readdir(dir)) != NULL) {
 *         printf("%s\n", entry->d_name);
 *     }
 *     tebako_fs_closedir(dir);
 * }
 * @endcode
 */
tebako_dir_t tebako_fs_opendir(const char* path);

/**
 * @brief Read next directory entry
 *
 * Returns the next entry in the directory. Returns NULL when
 * no more entries or on error.
 *
 * @param dir Directory handle from tebako_fs_opendir()
 * @return Pointer to entry structure, or NULL at end/error
 *
 * @note The returned pointer is valid until next call to tebako_fs_readdir()
 *       or tebako_fs_closedir()
 * @note Entries "." and ".." are excluded
 */
struct tebako_c_dirent* tebako_fs_readdir(tebako_dir_t dir);

/**
 * @brief Close directory handle
 *
 * Releases resources associated with the directory handle.
 *
 * @param dir Directory handle from tebako_fs_opendir()
 * @return 0 on success, -1 on error
 */
int tebako_fs_closedir(tebako_dir_t dir);

/**
 * @brief Check if directory handle is from libtfs
 *
 * Registry-membership test for directory handles, the dir-handle
 * counterpart of tebako_fd_is_embedded(). Used to dispatch
 * readdir/closedir/etc. between the embedded filesystem and the host.
 *
 * @param dir Directory handle to check
 * @return 1 if dir is a live handle from tebako_fs_opendir(), 0 otherwise
 *         (NULL, unknown, or already closed/unmounted handle)
 */
int tebako_fs_dir_is_embedded(tebako_dir_t dir);

/**
 * @brief Reset directory stream to the beginning
 *
 * Behaves like POSIX rewinddir(3): the next tebako_fs_readdir() call
 * returns the first entry again. Equivalent to tebako_fs_seekdir(dir, 0).
 *
 * @param dir Directory handle from tebako_fs_opendir()
 *
 * @note On an invalid handle the call is a no-op with errno=EBADF
 *       (check via tebako_get_errno())
 */
void tebako_fs_rewinddir(tebako_dir_t dir);

/**
 * @brief Current location in a directory stream
 *
 * Behaves like POSIX telldir(3). Cookies are index-based: the returned
 * value is the ordinal (0-based) of the entry the next tebako_fs_readdir()
 * call will return. A cookie obtained from this function remains valid
 * for the lifetime of the directory handle (the entry order of a mounted
 * directory does not change; the filesystem is read-only).
 *
 * @param dir Directory handle from tebako_fs_opendir()
 * @return Position cookie, or -1 with errno=EBADF for an invalid handle
 */
long tebako_fs_telldir(tebako_dir_t dir);

/**
 * @brief Set the location of a directory stream
 *
 * Behaves like POSIX seekdir(3) with index-based cookies (see
 * tebako_fs_telldir()): after the call, the next tebako_fs_readdir()
 * returns entry #pos. pos == 0 rewinds. Seeking backwards resets the
 * underlying iterator and advances; seeking past the end leaves the
 * stream at end-of-directory.
 *
 * @param dir Directory handle from tebako_fs_opendir()
 * @param pos Position cookie (ordinal of the next entry to return)
 *
 * @note On an invalid handle the call is a no-op with errno=EBADF;
 *       a negative pos is rejected with errno=EINVAL (check via
 *       tebako_get_errno())
 */
void tebako_fs_seekdir(tebako_dir_t dir, long pos);

/* ============================================================
 * Metadata Operations
 * ============================================================ */

/**
 * @brief Get file status
 *
 * Fills in a stat structure with file metadata. Behaves like POSIX stat(2).
 *
 * @param path File path
 * @param st Pointer to stat structure to fill
 * @return 0 on success, -1 on error
 *
 * @note Populates: st_mode, st_size, st_mtime
 * @note Other fields may be zero or undefined
 *
 * @example
 * @code
 * struct stat st;
 * if (tebako_fs_stat("/__tebako__/file.txt", &st) == 0) {
 *     printf("Size: %lld bytes\n", (long long)st.st_size);
 *     if (S_ISREG(st.st_mode)) {
 *         printf("Regular file\n");
 *     }
 * }
 * @endcode
 */
int tebako_fs_stat(const char* path, struct stat* st);

/**
 * @brief Get file status via file descriptor
 *
 * Like tebako_fs_stat() but takes a file descriptor.
 *
 * @param fd File descriptor from tebako_fs_open()
 * @param st Pointer to stat structure to fill
 * @return 0 on success, -1 on error
 */
int tebako_fs_fstat(int fd, struct stat* st);

/* ============================================================
 * Path Detection
 * ============================================================ */

/**
 * @brief Check if path is within embedded filesystem
 *
 * Determines if a path would be handled by libtfs based on
 * the current mount point.
 *
 * @param path Path to check
 * @return 1 if path is within mounted filesystem, 0 otherwise
 *
 * @example
 * @code
 * if (tebako_path_is_embedded("/__tebako__/file.txt")) {
 *     // Use tebako_fs_open()
 * } else {
 *     // Use regular open()
 * }
 * @endcode
 */
int tebako_path_is_embedded(const char* path);

/**
 * @brief Check if file descriptor is from libtfs
 *
 * Tests if a file descriptor was returned by tebako_fs_open().
 *
 * @param fd File descriptor to check
 * @return 1 if FD is from libtfs, 0 otherwise
 *
 * @note Checks for TEBAKO_FD_FLAG bit
 */
int tebako_fd_is_embedded(int fd);

/* ============================================================
 * Error Handling
 * ============================================================ */

/**
 * @brief Get last error code
 *
 * Returns the error code from the last failed operation.
 * Error codes are standard errno values (ENOENT, EBADF, etc.).
 *
 * @return errno-style error code
 *
 * @note Error codes are thread-local
 */
int tebako_get_errno(void);

/**
 * @brief Get error message string
 *
 * Converts an error code to a human-readable string.
 *
 * @param err Error code from tebako_get_errno()
 * @return Error message (do not free)
 *
 * @note The returned string is static and must not be freed
 */
const char* tebako_strerror(int err);

/* ============================================================
 * Extraction (for --tebako-extract flag)
 * ============================================================ */

/**
 * @brief Extract all files to disk
 *
 * Recursively extracts all files from the embedded filesystem
 * to the specified destination directory.
 *
 * @param dest_path Destination directory (must exist)
 * @return 0 on success, -1 on error
 *
 * @note Creates subdirectories as needed
 * @note Preserves file permissions and timestamps
 * @note Used to implement --tebako-extract functionality
 */
int tebako_fs_extract_all(const char* dest_path);

/* ============================================================
 * Dynamic Loading Support
 * ============================================================ */

/**
 * @brief Extract a memfs file to a host filesystem path for dlopen
 *
 * Modern entry point of the legacy tebako_dlmap2file() mechanism with the
 * same extraction/cache/lifetime semantics: the file at `path` (dispatched
 * to its owning mount by longest mount-point prefix) is streamed out to a
 * per-process temporary directory and the host path is returned, so that
 * native code (e.g. dlopen of a packaged extension) can load it. Repeated
 * calls for the same path return the cached host file; extracted files are
 * owned by libtfs and removed at process teardown.
 *
 * @param path Absolute path within a mounted filesystem
 * @return Newly allocated host path string on success, NULL on error
 *         (check errno via tebako_get_errno())
 *
 * @note Ownership: the RETURNED STRING is caller-owned and must be
 *       released with free(). The host file it points to is owned by
 *       libtfs — do not unlink or modify it.
 * @note Returns NULL with errno=ENOENT if the path is not within any
 *       mounted filesystem or does not exist in its mount
 *
 * @example
 * @code
 * char* host_path = tebako_fs_dlmap2file("/__tebako__/lib/native/ext.so");
 * if (host_path != NULL) {
 *     void* h = dlopen(host_path, RTLD_LAZY);
 *     free(host_path);
 *     // ...
 * }
 * @endcode
 */
char* tebako_fs_dlmap2file(const char* path);

/**
 * @brief Serialize the mount table in the TEBAKO_TFS_MOUNTS grammar
 * ("image:mount,image:mount,…").
 *
 * A spawned child re-establishes the namespace from this: the spawn hook
 * writes it into the child's environment (the preload shim parses the
 * same grammar). Only file-backed mounts serialize; memory mounts are
 * skipped (a child cannot remount them).
 *
 * @return Heap-allocated string (free with libc free()); NULL when
 *         nothing file-backed is mounted.
 *
 * @code
 * char* mounts = tebako_fs_mounts();
 * if (mounts != NULL) {
 *     setenv("TEBAKO_TFS_MOUNTS", mounts, 1);
 *     free(mounts);
 * }
 * @endcode
 */
char* tebako_fs_mounts(void);

/* ============================================================
 * Utility Functions
 * ============================================================ */

/**
 * @brief Get current mount point
 *
 * Returns the path where the filesystem is currently mounted.
 *
 * @return Mount point string, or NULL if not mounted
 *
 * @note The returned string is valid until tebako_fs_unmount()
 */
const char* tebako_get_mount_point(void);

/**
 * @brief Get archive path
 *
 * Returns the path to the mounted archive file (if mounted from file).
 *
 * @return Archive path string, or NULL if mounted from memory or not mounted
 */
const char* tebako_get_archive_path(void);

/**
 * @brief Get backend name
 *
 * Returns the name of the currently mounted backend (e.g., "ZIP", "SquashFS").
 *
 * @return Backend name string, or NULL if not mounted
 */
const char* tebako_get_backend_name(void);

#ifdef __cplusplus
}
#endif

#endif /* TEBAKO_FS_C_API_H */
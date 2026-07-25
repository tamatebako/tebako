/**
 * @file mount_read.c
 * @brief Minimal C consumer of the Rust libtfs C ABI (contract smoke).
 *
 * Mounts a ZIP image passed as argv[1], reads the file argv[2] through
 * tebako_fs_open/tebako_fs_read/tebako_fs_close and prints it to stdout.
 * Exercises the C ABI exactly like a real consumer: no Rust headers, no
 * C++ — only the exported tebako_fs_* symbols.
 *
 * Declarations mirror include/tebako/fs/c_api.h (libtfs) byte-for-byte.
 */

#include <stdio.h>
#include <string.h>
#include <stddef.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>

/* ---- tebako_fs_* C ABI (declarations mirror c_api.h) ---- */
extern int tebako_fs_init_from_file(const char* archive_path, const char* mount_point);
extern int tebako_is_initialized(void);
extern int tebako_fs_open(const char* path, int flags);
extern ssize_t tebako_fs_read(int fd, void* buf, size_t count);
extern off_t tebako_fs_lseek(int fd, off_t offset, int whence);
extern int tebako_fs_close(int fd);
extern int tebako_fs_stat(const char* path, struct stat* st);
extern void tebako_fs_unmount(void);
extern int tebako_get_errno(void);
extern const char* tebako_strerror(int err);
extern const char* tebako_get_mount_point(void);
extern const char* tebako_get_backend_name(void);

int main(int argc, char** argv)
{
  char buf[256];
  ssize_t n;
  int fd;
  struct stat st;

  if (argc != 3) {
    fprintf(stderr, "usage: %s <archive> <in-image-path>\n", argv[0]);
    return 2;
  }

  if (tebako_fs_init_from_file(argv[1], "/__tebako_test__") != 0) {
    fprintf(stderr, "init failed: %s\n", tebako_strerror(tebako_get_errno()));
    return 1;
  }
  if (!tebako_is_initialized()) {
    fprintf(stderr, "not initialized after init\n");
    return 1;
  }
  printf("mounted at %s (backend %s)\n", tebako_get_mount_point(), tebako_get_backend_name());

  if (tebako_fs_stat(argv[2], &st) != 0 || !S_ISREG(st.st_mode)) {
    fprintf(stderr, "stat failed: %s\n", tebako_strerror(tebako_get_errno()));
    return 1;
  }
  printf("size=%lld mode=%o\n", (long long)st.st_size, (unsigned)(st.st_mode & 0777));

  fd = tebako_fs_open(argv[2], O_RDONLY);
  if (fd < 0) {
    fprintf(stderr, "open failed: %s\n", tebako_strerror(tebako_get_errno()));
    return 1;
  }
  n = tebako_fs_read(fd, buf, sizeof(buf) - 1);
  if (n < 0) {
    fprintf(stderr, "read failed: %s\n", tebako_strerror(tebako_get_errno()));
    return 1;
  }
  buf[n] = '\0';
  printf("content=%s\n", buf);

  if (tebako_fs_close(fd) != 0) {
    fprintf(stderr, "close failed: %s\n", tebako_strerror(tebako_get_errno()));
    return 1;
  }

  tebako_fs_unmount();
  if (tebako_is_initialized()) {
    fprintf(stderr, "still initialized after unmount\n");
    return 1;
  }
  return 0;
}

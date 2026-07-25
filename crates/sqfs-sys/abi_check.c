/*
 * ABI cross-check for the hand-written Rust FFI declarations in
 * sqfs-sys/src/lib.rs against the real libsquashfs headers (same
 * discipline as dwarfs-t-sys's abi_check.c): every struct size, field
 * offset and constant the Rust side relies on is asserted here.
 */

#include <stddef.h>

#include <sqfs/super.h>
#include <sqfs/inode.h>
#include <sqfs/dir.h>
#include <sqfs/io.h>
#include <sqfs/error.h>
#include <sqfs/compressor.h>
#include <sqfs/predef.h>

/* ---- constants ---------------------------------------------------------- */
_Static_assert(SQFS_FILE_OPEN_READ_ONLY == 0x01, "SQFS_FILE_OPEN_READ_ONLY");
_Static_assert(SQFS_COMP_FLAG_UNCOMPRESS == 0x8000, "SQFS_COMP_FLAG_UNCOMPRESS");
_Static_assert(SQFS_INODE_DIR == 1, "SQFS_INODE_DIR");
_Static_assert(SQFS_INODE_FILE == 2, "SQFS_INODE_FILE");
_Static_assert(SQFS_INODE_SLINK == 3, "SQFS_INODE_SLINK");
_Static_assert(SQFS_INODE_EXT_DIR == 8, "SQFS_INODE_EXT_DIR");
_Static_assert(SQFS_INODE_EXT_FILE == 9, "SQFS_INODE_EXT_FILE");
_Static_assert(SQFS_INODE_EXT_SLINK == 10, "SQFS_INODE_EXT_SLINK");
_Static_assert(SQFS_ERROR_IO == -2, "SQFS_ERROR_IO");
_Static_assert(SQFS_ERROR_NOT_FILE == -15, "SQFS_ERROR_NOT_FILE");
_Static_assert(SQFS_ERROR_UNSUPPORTED == -6, "SQFS_ERROR_UNSUPPORTED");

/* ---- sqfs_super_t (96 bytes) --------------------------------------------- */
_Static_assert(sizeof(sqfs_super_t) == 96, "sqfs_super_t size");
_Static_assert(offsetof(sqfs_super_t, magic) == 0, "super.magic");
_Static_assert(offsetof(sqfs_super_t, block_size) == 12, "super.block_size");
_Static_assert(offsetof(sqfs_super_t, compression_id) == 20, "super.compression_id");
_Static_assert(offsetof(sqfs_super_t, root_inode_ref) == 32, "super.root_inode_ref");
_Static_assert(offsetof(sqfs_super_t, export_table_start) == 88, "super.export_table_start");

/* ---- sqfs_inode_t (16 bytes; leading member of sqfs_inode_generic_t) ----- */
_Static_assert(sizeof(sqfs_inode_t) == 16, "sqfs_inode_t size");
_Static_assert(offsetof(sqfs_inode_t, type) == 0, "inode.type");
_Static_assert(offsetof(sqfs_inode_t, mode) == 2, "inode.mode");
_Static_assert(offsetof(sqfs_inode_t, mod_time) == 8, "inode.mod_time");
_Static_assert(offsetof(sqfs_inode_generic_t, base) == 0, "generic.base must be first");

/* ---- sqfs_dir_entry_t (8-byte header + inline name) ---------------------- */
_Static_assert(offsetof(sqfs_dir_entry_t, type) == 4, "dirent.type");
_Static_assert(offsetof(sqfs_dir_entry_t, size) == 6, "dirent.size");
_Static_assert(offsetof(sqfs_dir_entry_t, name) == 8, "dirent.name");

/* ---- sqfs_file_t vtable ---------------------------------------------------- */
_Static_assert(offsetof(sqfs_object_t, destroy) == 0, "object.destroy");
_Static_assert(offsetof(sqfs_file_t, base) == 0, "file.base must be first");
_Static_assert(offsetof(sqfs_file_t, read_at) == sizeof(sqfs_object_t), "file.read_at");

int sqfs_sys_abi_check(void)
{
  return 0;
}

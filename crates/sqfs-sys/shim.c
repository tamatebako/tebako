/*
 * shim.c — small C helpers for sqfs-sys.
 *
 * sqfs_compressor_config_t has a variable, version-dependent layout that we
 * deliberately keep entirely on the C side: the shim initializes it and
 * creates the decompressor in one call.
 */

#include <sqfs/super.h>
#include <sqfs/compressor.h>

int sqfs_shim_compressor_create(sqfs_u16 compression_id, sqfs_u32 block_size,
                                sqfs_compressor_t** out)
{
  sqfs_compressor_config_t cfg;
  if (sqfs_compressor_config_init(&cfg, (SQFS_COMPRESSOR)compression_id,
                                  block_size, SQFS_COMP_FLAG_UNCOMPRESS) != 0) {
    return -1;
  }
  return sqfs_compressor_create(&cfg, out);
}

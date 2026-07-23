/**
 * @file tpkg.h
 * @brief tebako package manifest (tpkg) — single-header C99 mini-lib
 *
 * Copyright (c) 2026 [Ribose Inc](https://www.ribose.com).
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
 * ============================================================================
 * Manifest trailer v1 — wire layout (all integers little-endian):
 *
 *   [payload][slot 0 .. slot n-1 records][trailer header — fixed size, at EOF]
 *
 *   trailer header (TPKG_HEADER_SIZE = 166 bytes):
 *     offset  size  field
 *        0    10    magic "TEBAKOTFS\0" (10 bytes, NUL-terminated)
 *       10     4    u32 version (TPKG_VERSION = 1)
 *       14     4    u32 package_flags (bit 0: TPKG_FLAG_LEAN — bootstrap+images
 *                    only, runtime resolved at run time)
 *       18     4    u32 slot_count (1..TPKG_MAX_SLOTS)
 *       22     8    u64 slot_table_offset (absolute file offset of slot 0 record)
 *       30   128    char runtime_ref[128] (UTF-8, NUL-padded; empty = classic bundle)
 *      158     4    u32 launcher_abi
 *      162     4    u32 header_crc32 — tpkg_crc32() over header bytes [0, 162)
 *
 *   slot record (TPKG_SLOT_SIZE = 280 bytes):
 *     offset  size  field
 *        0     8    u64 offset (image start, absolute file offset)
 *        8     8    u64 size   (image length in bytes)
 *       16     4    u32 format_id (0=auto(magic), 1=dwarfs, 2=squashfs, 3=zip, 4=runtime payload)
 *       20     4    u32 flags
 *       24   256    char mount_point[256] (UTF-8, NUL-padded)
 *
 * Reader algorithm: read the fixed-size header at EOF - TPKG_HEADER_SIZE,
 * check the magic, verify header_crc32, then read slot_count slot records at
 * slot_table_offset. The table (and therefore the payload) may sit at any —
 * including odd — file offset.
 *
 * Absent vs. corrupt: a file whose last-166-byte window does not start with
 * the 4-byte prefix "TEBA" is reported as TPKG_ERR_NO_TRAILER (a classic
 * bundle without a manifest — not an error condition per se; callers fall
 * back to offset auto-detection). A matching prefix with a mismatching full
 * magic is TPKG_ERR_MAGIC (corrupt trailer); a magic-valid header with a bad
 * crc is TPKG_ERR_CRC.
 *
 * Error handling: all tpkg_* functions (except tpkg_crc32, tpkg_errno and
 * tpkg_strerror) return 0 on success and -1 on failure, with a TPKG_ERR_*
 * code available from tpkg_errno(). The error state is a process-global
 * (ISO C99 has no portable TLS); it is reset at the entry of every public
 * function and is NOT thread-safe.
 *
 * Usage: in exactly one translation unit, #define TPKG_IMPLEMENTATION before
 * #include <tebako/tpkg.h>; every other TU includes it plain. On POSIX the
 * implementation section defines _POSIX_C_SOURCE 200809L if it is not
 * defined yet, so include tpkg.h before any system header in that TU.
 * No dependencies beyond libc.
 * ============================================================================
 */

#ifndef TEBAKO_TPKG_H
#define TEBAKO_TPKG_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- format constants ---------------------------------------------------- */

#define TPKG_VERSION 1u
#define TPKG_MAX_SLOTS 8u
#define TPKG_HEADER_SIZE 166u
#define TPKG_SLOT_SIZE 280u
#define TPKG_MOUNT_POINT_LEN 256u
#define TPKG_RUNTIME_REF_LEN 128u
#define TPKG_MAGIC "TEBAKOTFS"
#define TPKG_MAGIC_LEN 10u       /* including the terminating NUL */
#define TPKG_MAGIC_PREFIX_LEN 4u /* "TEBA": absent-vs-corrupt discriminator */

/* package_flags */
#define TPKG_FLAG_LEAN 0x1u

/* format_id */
#define TPKG_FORMAT_AUTO 0u
#define TPKG_FORMAT_DWARFS 1u
#define TPKG_FORMAT_SQUASHFS 2u
#define TPKG_FORMAT_ZIP 3u
/* A runtime payload slot (fat packages): the compressed language-runtime
 * package the bootstrap installs into the shared cache on first run. */
#define TPKG_FORMAT_RUNTIME 4u

/* ---- error codes (returned by tpkg_errno) -------------------------------- */

enum {
  TPKG_OK = 0,             /* success */
  TPKG_ERR_NO_TRAILER = 1, /* no manifest trailer present (absent — not an error per se) */
  TPKG_ERR_MAGIC = 2,      /* magic prefix present but full magic mismatch (corrupt) */
  TPKG_ERR_CRC = 3,        /* header_crc32 mismatch (corrupt) */
  TPKG_ERR_IO = 4,         /* underlying i/o failure */
  TPKG_ERR_BOUNDS = 5,     /* slot table outside file bounds */
  TPKG_ERR_SLOTS = 6,      /* slot_count == 0 or > TPKG_MAX_SLOTS */
  TPKG_ERR_INVALID = 7,    /* structural validation failure */
  TPKG_ERR_ARG = 8,        /* invalid argument (NULL, bad fd) */
  TPKG_ERR_VERSION = 9     /* unsupported manifest version */
};

/* ---- manifest structures -------------------------------------------------- */

typedef struct tpkg_slot {
  uint64_t offset;
  uint64_t size;
  uint32_t format_id;
  uint32_t flags;
  char mount_point[TPKG_MOUNT_POINT_LEN];
} tpkg_slot;

typedef struct tpkg_manifest {
  uint32_t version;
  uint32_t package_flags;
  uint32_t slot_count;
  uint32_t launcher_abi;
  char runtime_ref[TPKG_RUNTIME_REF_LEN];
  tpkg_slot slots[TPKG_MAX_SLOTS];
} tpkg_manifest;

/* ---- API ------------------------------------------------------------------ */

/* Read the manifest trailer of an open binary (seeks to EOF; the file offset
 * is left unspecified). */
int tpkg_read_fd(int fd, tpkg_manifest* out);

/* Read the manifest trailer from an in-memory image of the binary. */
int tpkg_read_mem(const void* data, size_t size, tpkg_manifest* out);

/* Append the slot table + trailer header to an open binary (at current EOF).
 * The manifest is validated first; a rejected manifest appends nothing. */
int tpkg_write_fd(int fd, const tpkg_manifest* m);

/* Magic-independent structural checks: version supported, slot_count in
 * 1..TPKG_MAX_SLOTS, offset+size non-overflowing, format_id <= TPKG_FORMAT_RUNTIME,
 * runtime_ref and mount_points NUL-terminated within their fixed fields. */
int tpkg_validate(const tpkg_manifest* m);

/* CRC-32 (zlib polynomial 0xEDB88320, init/xorout 0xFFFFFFFF) — exposed for
 * header_crc32 computation and verification. Pure; does not touch tpkg_errno. */
uint32_t tpkg_crc32(const void* data, size_t n);

/* Code of the last failed (or succeeded) tpkg operation on this process. */
int tpkg_errno(void);

/* Static string for a TPKG_ERR_* code (never NULL). */
const char* tpkg_strerror(int err);

#ifdef __cplusplus
}
#endif

#endif /* TEBAKO_TPKG_H */

/* ~~~~~~~~~~~~~~~~~~~~~~~~~ implementation ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ */

#if defined(TPKG_IMPLEMENTATION) && !defined(TPKG_IMPLEMENTATION_DONE)
#define TPKG_IMPLEMENTATION_DONE

#include <stdio.h> /* SEEK_SET / SEEK_END */
#include <string.h>

#if defined(_WIN32)
#include <io.h>
typedef __int64 tpkg__off_t;
typedef int tpkg__ssize_t;
typedef unsigned int tpkg__io_count_t;
#define tpkg__lseek _lseeki64
#define tpkg__read _read
#define tpkg__write _write
#else
#if !defined(_POSIX_C_SOURCE)
#define _POSIX_C_SOURCE 200809L
#endif
#include <sys/types.h>
#include <unistd.h>
typedef off_t tpkg__off_t;
typedef ssize_t tpkg__ssize_t;
typedef size_t tpkg__io_count_t;
#define tpkg__lseek lseek
#define tpkg__read read
#define tpkg__write write
#endif

/* header field offsets */
enum {
  TPKG__OFF_MAGIC = 0,
  TPKG__OFF_VERSION = 10,
  TPKG__OFF_PACKAGE_FLAGS = 14,
  TPKG__OFF_SLOT_COUNT = 18,
  TPKG__OFF_TABLE = 22,
  TPKG__OFF_RUNTIME_REF = 30,
  TPKG__OFF_LAUNCHER_ABI = 158,
  TPKG__OFF_CRC32 = 162
};

/* slot record field offsets */
enum { TPKG__REC_OFFSET = 0, TPKG__REC_SIZE = 8, TPKG__REC_FORMAT = 16, TPKG__REC_FLAGS = 20, TPKG__REC_MOUNT = 24 };

/* wire sizes are fixed by the format; catch exotic padding at compile time */
typedef char tpkg__assert_slot_size[(sizeof(tpkg_slot) == TPKG_SLOT_SIZE) ? 1 : -1];
typedef char tpkg__assert_manifest_size[(sizeof(tpkg_manifest) == 4 * sizeof(uint32_t) + TPKG_RUNTIME_REF_LEN +
                                                                      TPKG_MAX_SLOTS * sizeof(tpkg_slot))
                                            ? 1
                                            : -1];

/* ---- error state ---------------------------------------------------------- */

static int tpkg__last_err = TPKG_OK;

int tpkg_errno(void)
{
  return tpkg__last_err;
}

static int tpkg__fail(int code)
{
  tpkg__last_err = code;
  return -1;
}

const char* tpkg_strerror(int err)
{
  switch (err) {
    case TPKG_OK:
      return "success";
    case TPKG_ERR_NO_TRAILER:
      return "no tpkg manifest trailer present";
    case TPKG_ERR_MAGIC:
      return "corrupt tpkg trailer magic";
    case TPKG_ERR_CRC:
      return "tpkg trailer header crc32 mismatch";
    case TPKG_ERR_IO:
      return "tpkg i/o error";
    case TPKG_ERR_BOUNDS:
      return "tpkg slot table out of file bounds";
    case TPKG_ERR_SLOTS:
      return "tpkg slot count out of range (1..TPKG_MAX_SLOTS)";
    case TPKG_ERR_INVALID:
      return "invalid tpkg manifest structure";
    case TPKG_ERR_ARG:
      return "invalid tpkg argument";
    case TPKG_ERR_VERSION:
      return "unsupported tpkg manifest version";
    default:
      return "unknown tpkg error";
  }
}

/* ---- CRC-32 (zlib polynomial) -------------------------------------------- */

uint32_t tpkg_crc32(const void* data, size_t n)
{
  const uint8_t* p = (const uint8_t*)data;
  uint32_t crc = 0xFFFFFFFFu;
  size_t i;
  int k;
  for (i = 0; i < n; i++) {
    crc ^= p[i];
    for (k = 0; k < 8; k++) {
      crc = (crc >> 1) ^ (0xEDB88320u & (0u - (crc & 1u)));
    }
  }
  return crc ^ 0xFFFFFFFFu;
}

/* ---- little-endian codec -------------------------------------------------- */

static void tpkg__put32(uint8_t* p, uint32_t v)
{
  p[0] = (uint8_t)(v & 0xFFu);
  p[1] = (uint8_t)((v >> 8) & 0xFFu);
  p[2] = (uint8_t)((v >> 16) & 0xFFu);
  p[3] = (uint8_t)((v >> 24) & 0xFFu);
}

static void tpkg__put64(uint8_t* p, uint64_t v)
{
  int i;
  for (i = 0; i < 8; i++) {
    p[i] = (uint8_t)((v >> (8 * i)) & 0xFFu);
  }
}

static uint32_t tpkg__get32(const uint8_t* p)
{
  return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static uint64_t tpkg__get64(const uint8_t* p)
{
  uint64_t v = 0;
  int i;
  for (i = 0; i < 8; i++) {
    v |= (uint64_t)p[i] << (8 * i);
  }
  return v;
}

/* ---- helpers -------------------------------------------------------------- */

static size_t tpkg__strnlen(const char* s, size_t max)
{
  const char* nul = (const char*)memchr(s, '\0', max);
  return nul ? (size_t)(nul - s) : max;
}

/* copy a C string into a fixed-width field, zero-padding the remainder;
 * callers must have validated NUL-termination within width */
static void tpkg__put_str(uint8_t* p, const char* s, size_t width)
{
  size_t len = tpkg__strnlen(s, width);
  memcpy(p, s, len);
  memset(p + len, 0, width - len);
}

static int tpkg__read_full(int fd, void* buf, size_t n)
{
  uint8_t* p = (uint8_t*)buf;
  while (n > 0) {
    tpkg__ssize_t r = tpkg__read(fd, p, (tpkg__io_count_t)n);
    if (r <= 0) {
      return -1;
    }
    p += (size_t)r;
    n -= (size_t)r;
  }
  return 0;
}

static int tpkg__write_full(int fd, const void* buf, size_t n)
{
  const uint8_t* p = (const uint8_t*)buf;
  while (n > 0) {
    tpkg__ssize_t r = tpkg__write(fd, p, (tpkg__io_count_t)n);
    if (r <= 0) {
      return -1;
    }
    p += (size_t)r;
    n -= (size_t)r;
  }
  return 0;
}

/* ---- validation ------------------------------------------------------------ */

int tpkg_validate(const tpkg_manifest* m)
{
  uint32_t i;
  tpkg__last_err = TPKG_OK;
  if (m == NULL) {
    return tpkg__fail(TPKG_ERR_ARG);
  }
  if (m->version != TPKG_VERSION) {
    return tpkg__fail(TPKG_ERR_VERSION);
  }
  if (m->slot_count == 0 || m->slot_count > TPKG_MAX_SLOTS) {
    return tpkg__fail(TPKG_ERR_SLOTS);
  }
  if (tpkg__strnlen(m->runtime_ref, TPKG_RUNTIME_REF_LEN) == TPKG_RUNTIME_REF_LEN) {
    return tpkg__fail(TPKG_ERR_INVALID);
  }
  for (i = 0; i < m->slot_count; i++) {
    const tpkg_slot* s = &m->slots[i];
    if (s->size > UINT64_MAX - s->offset) {
      return tpkg__fail(TPKG_ERR_INVALID);
    }
    if (s->format_id > TPKG_FORMAT_RUNTIME) {
      return tpkg__fail(TPKG_ERR_INVALID);
    }
    if (tpkg__strnlen(s->mount_point, TPKG_MOUNT_POINT_LEN) == TPKG_MOUNT_POINT_LEN) {
      return tpkg__fail(TPKG_ERR_INVALID);
    }
  }
  return 0;
}

/* ---- reader core ------------------------------------------------------------ */

/* read exactly n bytes at absolute offset off (off+n guaranteed in bounds) */
typedef int (*tpkg__readat_fn)(void* ctx, uint64_t off, void* buf, size_t n);

static int tpkg__mem_readat(void* ctx, uint64_t off, void* buf, size_t n)
{
  memcpy(buf, (const uint8_t*)ctx + off, n);
  return 0;
}

static int tpkg__fd_readat(void* ctx, uint64_t off, void* buf, size_t n)
{
  int fd = *(int*)ctx;
  if (tpkg__lseek(fd, (tpkg__off_t)off, SEEK_SET) == (tpkg__off_t)-1) {
    return -1;
  }
  return tpkg__read_full(fd, buf, n);
}

static int tpkg__read_core(tpkg__readat_fn readat, void* ctx, uint64_t size, tpkg_manifest* out)
{
  uint8_t hdr[TPKG_HEADER_SIZE];
  uint8_t table[TPKG_MAX_SLOTS * TPKG_SLOT_SIZE];
  uint64_t table_off;
  uint64_t avail;
  uint32_t version;
  uint32_t slot_count;
  uint32_t i;

  if (size < TPKG_HEADER_SIZE) {
    return tpkg__fail(TPKG_ERR_NO_TRAILER);
  }
  if (readat(ctx, size - TPKG_HEADER_SIZE, hdr, TPKG_HEADER_SIZE) != 0) {
    return tpkg__fail(TPKG_ERR_IO);
  }

  /* absent vs corrupt: no "TEBA" prefix -> classic bundle, no trailer */
  if (memcmp(hdr + TPKG__OFF_MAGIC, TPKG_MAGIC, TPKG_MAGIC_PREFIX_LEN) != 0) {
    return tpkg__fail(TPKG_ERR_NO_TRAILER);
  }
  if (memcmp(hdr + TPKG__OFF_MAGIC, TPKG_MAGIC, TPKG_MAGIC_LEN) != 0) {
    return tpkg__fail(TPKG_ERR_MAGIC);
  }
  if (tpkg_crc32(hdr, TPKG__OFF_CRC32) != tpkg__get32(hdr + TPKG__OFF_CRC32)) {
    return tpkg__fail(TPKG_ERR_CRC);
  }

  version = tpkg__get32(hdr + TPKG__OFF_VERSION);
  if (version != TPKG_VERSION) {
    return tpkg__fail(TPKG_ERR_VERSION);
  }
  slot_count = tpkg__get32(hdr + TPKG__OFF_SLOT_COUNT);
  if (slot_count == 0 || slot_count > TPKG_MAX_SLOTS) {
    return tpkg__fail(TPKG_ERR_SLOTS);
  }

  table_off = tpkg__get64(hdr + TPKG__OFF_TABLE);
  avail = size - TPKG_HEADER_SIZE; /* bytes preceding the header */
  /* overflow-free: table must fit entirely before the header */
  if (table_off > avail || (uint64_t)slot_count > (avail - table_off) / TPKG_SLOT_SIZE) {
    return tpkg__fail(TPKG_ERR_BOUNDS);
  }
  if (readat(ctx, table_off, table, (size_t)slot_count * TPKG_SLOT_SIZE) != 0) {
    return tpkg__fail(TPKG_ERR_IO);
  }

  memset(out, 0, sizeof *out);
  out->version = version;
  out->package_flags = tpkg__get32(hdr + TPKG__OFF_PACKAGE_FLAGS);
  out->slot_count = slot_count;
  out->launcher_abi = tpkg__get32(hdr + TPKG__OFF_LAUNCHER_ABI);
  memcpy(out->runtime_ref, hdr + TPKG__OFF_RUNTIME_REF, TPKG_RUNTIME_REF_LEN);
  for (i = 0; i < slot_count; i++) {
    const uint8_t* rec = table + (size_t)i * TPKG_SLOT_SIZE;
    tpkg_slot* s = &out->slots[i];
    s->offset = tpkg__get64(rec + TPKG__REC_OFFSET);
    s->size = tpkg__get64(rec + TPKG__REC_SIZE);
    s->format_id = tpkg__get32(rec + TPKG__REC_FORMAT);
    s->flags = tpkg__get32(rec + TPKG__REC_FLAGS);
    memcpy(s->mount_point, rec + TPKG__REC_MOUNT, TPKG_MOUNT_POINT_LEN);
  }

  /* structural sanity of the parsed manifest (a valid crc does not imply
   * well-formed fields when the trailer was not written by us) */
  return tpkg_validate(out);
}

int tpkg_read_fd(int fd, tpkg_manifest* out)
{
  tpkg__off_t end;
  tpkg__last_err = TPKG_OK;
  if (fd < 0 || out == NULL) {
    return tpkg__fail(TPKG_ERR_ARG);
  }
  end = tpkg__lseek(fd, 0, SEEK_END);
  if (end == (tpkg__off_t)-1) {
    return tpkg__fail(TPKG_ERR_IO);
  }
  return tpkg__read_core(tpkg__fd_readat, &fd, (uint64_t)end, out);
}

int tpkg_read_mem(const void* data, size_t size, tpkg_manifest* out)
{
  tpkg__last_err = TPKG_OK;
  if (data == NULL || out == NULL) {
    return tpkg__fail(TPKG_ERR_ARG);
  }
  return tpkg__read_core(tpkg__mem_readat, (void*)data, (uint64_t)size, out);
}

/* ---- writer ----------------------------------------------------------------- */

int tpkg_write_fd(int fd, const tpkg_manifest* m)
{
  uint8_t buf[TPKG_MAX_SLOTS * TPKG_SLOT_SIZE + TPKG_HEADER_SIZE];
  uint8_t* p = buf;
  uint8_t* hdr;
  tpkg__off_t end;
  size_t total;
  uint32_t i;

  tpkg__last_err = TPKG_OK;
  if (fd < 0 || m == NULL) {
    return tpkg__fail(TPKG_ERR_ARG);
  }
  if (tpkg_validate(m) != 0) {
    return -1; /* tpkg_errno set by tpkg_validate; nothing appended */
  }

  end = tpkg__lseek(fd, 0, SEEK_END);
  if (end == (tpkg__off_t)-1) {
    return tpkg__fail(TPKG_ERR_IO);
  }

  /* slot table */
  for (i = 0; i < m->slot_count; i++) {
    const tpkg_slot* s = &m->slots[i];
    tpkg__put64(p + TPKG__REC_OFFSET, s->offset);
    tpkg__put64(p + TPKG__REC_SIZE, s->size);
    tpkg__put32(p + TPKG__REC_FORMAT, s->format_id);
    tpkg__put32(p + TPKG__REC_FLAGS, s->flags);
    tpkg__put_str(p + TPKG__REC_MOUNT, s->mount_point, TPKG_MOUNT_POINT_LEN);
    p += TPKG_SLOT_SIZE;
  }

  /* trailer header */
  hdr = p;
  memcpy(hdr + TPKG__OFF_MAGIC, TPKG_MAGIC, TPKG_MAGIC_LEN);
  tpkg__put32(hdr + TPKG__OFF_VERSION, m->version);
  tpkg__put32(hdr + TPKG__OFF_PACKAGE_FLAGS, m->package_flags);
  tpkg__put32(hdr + TPKG__OFF_SLOT_COUNT, m->slot_count);
  tpkg__put64(hdr + TPKG__OFF_TABLE, (uint64_t)end);
  tpkg__put_str(hdr + TPKG__OFF_RUNTIME_REF, m->runtime_ref, TPKG_RUNTIME_REF_LEN);
  tpkg__put32(hdr + TPKG__OFF_LAUNCHER_ABI, m->launcher_abi);
  tpkg__put32(hdr + TPKG__OFF_CRC32, tpkg_crc32(hdr, TPKG__OFF_CRC32));
  p += TPKG_HEADER_SIZE;

  total = (size_t)(p - buf);
  if (tpkg__write_full(fd, buf, total) != 0) {
    return tpkg__fail(TPKG_ERR_IO);
  }
  return 0;
}

#endif /* TPKG_IMPLEMENTATION */

/*
 * ci/windows-gnu-link-wrap.c — the release-link policy for the shipped
 * windows-gnu binaries: the MinGW C/C++ runtime chain is ALWAYS linked
 * statically, no matter which crate in the dependency tree emits the
 * link directive.
 *
 * Why this exists (the exit-127 class): a windows-gnu exe that imports
 * libstdc++-6.dll / libwinpthread-1.dll dies before main() on any stock
 * Windows (STATUS_DLL_NOT_FOUND — surfaces as exit 127) unless the
 * ucrt64 toolchain's bin dir happens to sit on PATH. Audience law: a
 * user running tebako packages needs NO toolchain libraries. The tebako
 * 0.1.1 windows-ucrt64 exes shipped exactly that failure:
 * RUSTFLAGS="-C link-arg=-static-libstdc++ ..." looks like the fix but
 * is not — rustc emits a build script's (dylib) `stdc++` / `pthread`
 * as `-Wl,-Bdynamic` + `-lstdc++` / `-lpthread` at its own position,
 * BEFORE the trailing `-C link-arg`s, so the driver-level static flags
 * never govern them (and rnp-rs — external — emits an explicit
 * `dylib=stdc++`, which no flag of ours can rewrite).
 *
 * What it does: every reference to the mingw runtime libs is rewritten
 * to the `-l:<file>` exact-archive form, which is IMMUNE to the
 * surrounding -Bstatic / -Bdynamic mode:
 *
 *   -lstdc++      -> -l:libstdc++.a
 *   -lpthread     -> -l:libwinpthread.a
 *   -lwinpthread  -> -l:libwinpthread.a
 *
 * in three shapes: standalone argv entries, members of -Wl,<a>,<b>,...
 * comma lists, and tokens inside @response files (rustc moves long link
 * lines into one — the rewritten file rides next to the original as
 * <name>.wrapped). The gcc driver's own library search dirs resolve the
 * archives; system import libraries (kernel32 & co.) are untouched.
 *
 * The enforcement half lives in ci/windows-gnu-import-gate.sh: every
 * shipped exe's import table is audited against the inbox-DLL allowlist
 * in the same CI leg, so a future emission shape this wrapper misses
 * fails the build loudly instead of shipping a broken exe.
 *
 * Compiled once per CI leg by the windows-gnu scripts:
 *   gcc -O2 -o "$RUNNER_TEMP/tebako-link-wrap.exe" ci/windows-gnu-link-wrap.c
 * and engaged via
 *   CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=<that exe>
 * (cargo spawns the linker with CreateProcess — the wrapper must be a
 * real exe, not a shell script).
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#if defined(_WIN32)
#include <process.h> /* execvp lives here on MinGW; unistd.h elsewhere */
#endif

static const char *rewrite_lib(const char *name)
{
  if (strcmp(name, "stdc++") == 0)
    return "-l:libstdc++.a";
  if (strcmp(name, "pthread") == 0 || strcmp(name, "winpthread") == 0)
    return "-l:libwinpthread.a";
  return NULL;
}

/* If arg is exactly -l<name> with a static rewrite, return the
 * replacement; otherwise NULL. */
static const char *rewrite_arg(const char *arg)
{
  if (strncmp(arg, "-l", 2) != 0 || strncmp(arg, "-l:", 3) == 0)
    return NULL;
  return rewrite_lib(arg + 2);
}

/* ---------------- token rewriting ---------------- */

typedef struct {
  char *data;
  size_t len, cap;
} buf;

static void buf_putc(buf *b, char c)
{
  if (b->len + 1 >= b->cap) {
    b->cap = b->cap ? b->cap * 2 : 1 << 16;
    b->data = realloc(b->data, b->cap);
    if (!b->data) {
      fprintf(stderr, "tebako-link-wrap: out of memory\n");
      exit(70);
    }
  }
  b->data[b->len++] = c;
}

static void buf_puts(buf *b, const char *s)
{
  while (*s)
    buf_putc(b, *s++);
}

static void *xstrndup(const char *s, size_t len)
{
  char *m = malloc(len + 1);
  if (!m) {
    fprintf(stderr, "tebako-link-wrap: out of memory\n");
    exit(70);
  }
  memcpy(m, s, len);
  m[len] = '\0';
  return m;
}

/* The full token rewrite, shared by the argv path and the @response-file
 * path: an exact -l<name>, or any -l<name> member of a -Wl,<a>,<b>,...
 * comma list (member order and framing preserved). Returns a malloc'd
 * replacement, or NULL when the token needs none. */
static char *rewrite_token_dup(const char *tok)
{
  const char *r = rewrite_arg(tok);
  if (r)
    return xstrndup(r, strlen(r));
  if (strncmp(tok, "-Wl,", 4) != 0)
    return NULL;
  buf out = {0};
  buf_puts(&out, "-Wl,");
  const char *p = tok + 4;
  int first = 1, changed = 0;
  while (1) {
    const char *comma = strchr(p, ',');
    size_t len = comma ? (size_t)(comma - p) : strlen(p);
    char *member = xstrndup(p, len);
    const char *mr = rewrite_arg(member);
    if (mr)
      changed = 1;
    if (!first)
      buf_putc(&out, ',');
    buf_puts(&out, mr ? mr : member);
    free(member);
    first = 0;
    if (!comma)
      break;
    p = comma + 1;
  }
  buf_putc(&out, '\0');
  if (!changed) {
    free(out.data);
    return NULL;
  }
  return out.data;
}

/* ---------------- @response files ---------------- */
/* gcc response file grammar: whitespace-separated tokens, single- or
 * double-quoted groups, backslash escapes the next character. The
 * rewrite is token-exact, so re-emitting tokens bare (quoting only when
 * a token genuinely needs it) preserves the file's semantics. */

/* Append `tok` to `out`, applying the rewrite; quote on emit only when
 * the (possibly rewritten) token carries response-file specials. */
static void emit_token(buf *out, const char *tok)
{
  char *rt = rewrite_token_dup(tok);
  const char *s = rt ? rt : tok;
  int need_quote = (s[0] == '\0');
  for (const char *p = s; *p && !need_quote; p++)
    if (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r' || *p == '"' || *p == '\'' || *p == '\\')
      need_quote = 1;
  if (!need_quote) {
    buf_puts(out, s);
    free(rt);
    return;
  }
  buf_putc(out, '"');
  for (const char *p = s; *p; p++) {
    if (*p == '"' || *p == '\\')
      buf_putc(out, '\\');
    buf_putc(out, *p);
  }
  buf_putc(out, '"');
  free(rt);
}

/* Rewrite the response file at `path` (token-exact) into
 * "<path>.wrapped" and return the new @-argument. Falls through to the
 * original argument untouched when the file cannot be read (the link
 * will then succeed or fail exactly as unwrapped). */
static char *rewrite_response_file(const char *arg)
{
  const char *path = arg + 1;
  FILE *f = fopen(path, "rb");
  if (!f)
    return (char *)arg;
  buf in = {0}, out = {0};
  int c;
  while ((c = fgetc(f)) != EOF)
    buf_putc(&in, (char)c);
  fclose(f);
  buf_putc(&in, '\0');

  /* tokenize */
  size_t i = 0;
  int first = 1;
  while (i < in.len - 1) {
    while (i < in.len - 1 && (in.data[i] == ' ' || in.data[i] == '\t' || in.data[i] == '\n' || in.data[i] == '\r'))
      i++;
    if (i >= in.len - 1)
      break;
    buf tok = {0};
    while (i < in.len - 1 && in.data[i] != ' ' && in.data[i] != '\t' && in.data[i] != '\n' && in.data[i] != '\r') {
      if (in.data[i] == '\\' && i + 1 < in.len - 1) {
        /* Conservative backslash rule: it escapes only a recognized
         * special (quote, backslash, whitespace); before anything else
         * it is kept verbatim — Windows paths with single backslashes
         * survive the round trip either way. */
        char nxt = in.data[i + 1];
        if (nxt == '"' || nxt == '\'' || nxt == '\\' || nxt == ' ' || nxt == '\t' || nxt == '\n' || nxt == '\r') {
          buf_putc(&tok, nxt);
          i += 2;
          continue;
        }
        buf_putc(&tok, in.data[i++]);
        continue;
      }
      if (in.data[i] == '"' || in.data[i] == '\'') {
        char q = in.data[i++];
        while (i < in.len - 1 && in.data[i] != q) {
          if (q == '"' && in.data[i] == '\\' && i + 1 < in.len - 1) {
            char nxt = in.data[i + 1];
            if (nxt == '"' || nxt == '\\') {
              buf_putc(&tok, nxt);
              i += 2;
              continue;
            }
            /* keep a non-escape backslash verbatim */
            buf_putc(&tok, in.data[i++]);
            continue;
          }
          buf_putc(&tok, in.data[i++]);
        }
        if (i < in.len - 1)
          i++; /* closing quote */
        continue;
      }
      buf_putc(&tok, in.data[i++]);
    }
    buf_putc(&tok, '\0');
    if (!first)
      buf_putc(&out, ' ');
    first = 0;
    emit_token(&out, tok.data ? tok.data : "");
    free(tok.data);
  }
  buf_putc(&out, '\0');

  static char wrapped[4096];
  snprintf(wrapped, sizeof(wrapped), "%s.wrapped", path);
  FILE *wf = fopen(wrapped, "wb");
  if (!wf) {
    free(in.data);
    free(out.data);
    return (char *)arg;
  }
  fwrite(out.data, 1, out.len - 1, wf);
  fclose(wf);
  free(in.data);
  free(out.data);

  static char newarg[4160];
  snprintf(newarg, sizeof(newarg), "@%s", wrapped);
  return newarg;
}

int main(int argc, char **argv)
{
  static char *out[8192];
  int n = 0;
  out[n++] = (char *)"gcc";

  for (int i = 1; i < argc; i++) {
    const char *a = argv[i];

    if (a[0] == '@' && a[1] != '\0') {
      out[n++] = rewrite_response_file(a);
      continue;
    }

    char *rt = rewrite_token_dup(a);
    if (rt) {
      out[n++] = rt;
      continue;
    }

    out[n++] = argv[i];
  }
  out[n] = NULL;

  execvp("gcc", out);
  /* execvp only returns on failure; make the failure diagnosable. */
  perror("tebako-link-wrap: cannot exec gcc");
  return 127;
}

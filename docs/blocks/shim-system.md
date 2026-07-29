# The shim system

The shim system is how a packaged command becomes an ordinary command
on the user's PATH. After installing an application with tebako, the
user types its name — `metanorma`, for example — and it runs, with the
right version and the right runtime chosen automatically.

## How it works

There is one directory on PATH: `~/.tebako/shims`. Every installed
command is a small file in it:

- on Linux and macOS, a symlink;
- on Windows, a copy named `<command>.exe` (symlinks need elevation
  there, a copy does not).

Both point at one compiled dispatcher binary, `tebako-shim`. When the
command runs, the dispatcher reads its own invocation name, looks the
command up, and launches it. This is the same arrangement busybox and
rustup use, and it is chosen over shell scripts for concrete reasons:
arguments pass through without quoting problems in four different shell
dialects, there is no extra process layer swallowing signals, and the
command's own flags can never be confused with tebako's.

## What the dispatcher does on each invocation

1. Reads the command name from how it was invoked.
2. Picks a version: an environment override first, then a pin file in
   the current project, then the user's default, then the registry's
   default. Two projects can pin different versions of the same tool
   and both just work.
3. Resolves the runtime the command needs — newest compatible one
   already cached, or a download.
4. Mounts the payload and its declared dependencies, applies the jail
   policy, and starts the command. On Linux and macOS the dispatcher
   replaces itself with the runtime process; on Windows it spawns,
   waits, and passes the exit code back.

## PATH setup

`tebako-shim install-shell` performs the one-time setup:

- On Linux and macOS it adds a small managed block to the shell's
  startup file (`.bashrc`, `.zshrc`, fish's config, `.cshrc`), marked
  so `uninstall-shell` can remove exactly that block later.
- On Windows it prepends the shim directory to the user's PATH in the
  registry and broadcasts the change so new terminals see it. The
  current terminal needs to be reopened; the tool says so.

After that, installing or removing applications only adds or removes
files in the shim directory — PATH itself is never edited again. The
`doctor` command checks the whole arrangement (PATH entry, links,
payload integrity) and names any problem it finds.

## Explicitness

Installing an application from a registry registers its commands
because that is what installing means. Everything else is explicit
only: running a package creates no links, and installing a local
package links only with an explicit `--shims`. A casual run never
silently claims a name on the user's PATH.

## Implementation

`crates/tebako-shim` — the dispatcher, the version manager
(list/enable/disable/which/doctor), and the shell integration. The
full setup walkthrough, including per-shell instructions, is in
[docs/shell-setup.md](../shell-setup.md).

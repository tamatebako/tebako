# Setting up tebako shims — every shell, every OS

One-time setup. After it, every payload you install just works — new
commands appear without ever touching PATH again.

## The one-liner answer

```
tebako-shim install-shell
```

It detects your shell, inserts a small managed block into the right
startup file (idempotent — run it twice, nothing changes), and prints
what it did. Open a NEW terminal (or re-source the file) and you're
done. Verify with:

```
tebako-shim doctor
```

`uninstall-shell` removes exactly that block and nothing else.

## What it does, per shell (and how to do it by hand)

The whole mechanism is ONE directory on PATH: `~/.tebako/shims`. The
managed block is one line wrapped in markers:

```
# >>> tebako shims >>>
export PATH="$HOME/.tebako/shims:$PATH"
# <<< tebako shims <<<
```

| shell | startup file | the line |
|---|---|---|
| bash | `~/.bashrc` | `export PATH="$HOME/.tebako/shims:$PATH"` |
| zsh | `~/.zshrc` | `export PATH="$HOME/.tebako/shims:$PATH"` |
| fish | `~/.config/fish/config.fish` | `set -gx PATH "$HOME/.tebako/shims" $PATH` |
| csh / tcsh | `~/.cshrc` | `setenv PATH "$HOME/.tebako/shims:$PATH"` |

`install-shell --shell bash|zsh|fish|csh` overrides detection (it reads
`$SHELL` otherwise).

**Other shells** — no managed block yet; add the equivalent yourself:

- **nushell** (`~/.config/nushell/config.nu`):
  `$env.PATH = ($env.PATH | prepend $"($env.HOME)/.tebako/shims")`
- **elvish** (`~/.config/elvish/rc.elv`):
  `set paths = [~/.tebako/shims $@paths]`
- **xonsh** (`~/.xonshrc`):
  `$PATH.insert(0, "~/.tebako/shims")`
- **anything POSIX** (`~/.profile`): the bash line above.

The rule that always works, in any shell, forever: *prepend
`~/.tebako/shims` to PATH once.*

## Windows

```
tebako-shim install-shell
```

No startup files exist here — the command prepends the shim directory
(`%LOCALAPPDATA%\tebako\shims`) to your **user PATH in the registry**
(`HKCU\Environment`), then broadcasts the change so new windows notice.
Open a NEW terminal (running consoles keep the old PATH — cmd,
PowerShell, and Windows Terminal all behave this way).

By hand instead: *System Properties → Environment Variables → User
variables → Path → Edit → New* → `%LOCALAPPDATA%\tebako\shims`. Or in
PowerShell (exactly what the tool does):

```powershell
$p = [Environment]::GetEnvironmentVariable("Path", "User")
[Environment]::SetEnvironmentVariable("Path", "$env:LOCALAPPDATA\tebako\shims;$p", "User")
```

Never `setx PATH` — it truncates PATH at 1024 characters.

## How a shim actually works (why it's not a script)

Each command is a **link to one compiled dispatcher binary**, not a
shell script:

- `~/.tebako/shims/metanorma` → symlink to `tebako-shim` (Linux/macOS)
- `…\shims\metanorma.exe` → a copy of `tebako-shim.exe` (Windows —
  symlinks need privilege there, a copy doesn't)

When you type `metanorma`, the dispatcher reads **argv[0]** to learn
which tool it is — the busybox/rustup pattern — then resolves version
(project pin → user default → registry default), mounts the payload and
its runtime through TFS, and hands off. On Linux/macOS it `execve`s —
the shim process *becomes* the runtime, so signals and exit codes are
exactly the program's own. On Windows it spawns, waits, and exits with
the child's code (Windows has no `execve`).

Why not shell scripts calling `tebako run <name> -- …`?

- **Four dialects, one bug each.** POSIX sh, cmd `.bat`, PowerShell, and
  Git Bash all quote and forward arguments differently; a `.bat` isn't
  even executable by `CreateProcess` from a non-shell parent.
- **A process layer you can feel.** Script → tebako → runtime means
  Ctrl-C lands on the script's shell, not your program, and signal
  forwarding through it is lossy.
- **Flag ambiguity.** `tebako run metanorma --version` — whose
  `--version` is that? argv[0] dispatch never makes you ask.

`tebako run <name> -- <args>` still exists for when you want the
explicit, no-shim path — the shims are sugar over the same dispatch
chain.

## If it doesn't work

`tebako-shim doctor` checks: shim dir on PATH, links resolve to installed
payloads, payload images intact, trust anchors present. It names the
problem and the fix, every time.

//! Shell integration (spec 07 §3): ONE directory on PATH —
//! `~/.tebako/shims`. `install-shell` inserts a managed BEGIN/END block
//! into the right startup file; idempotent; `uninstall-shell` removes
//! exactly its block and never touches lines outside its markers. No
//! eval-init hook (the mise model, not the rbenv model).

use std::path::{Path, PathBuf};

use crate::{fail, ShimError, EX_TEBAKO_IO, EX_TEBAKO_MANIFEST, EX_USAGE};

pub const BEGIN_MARKER: &str = "# >>> tebako shims >>>";
pub const END_MARKER: &str = "# <<< tebako shims <<<";

/// Where the managed block puts the shims dir on PATH (tebako#1004 §4):
/// prepend wins over everything already there (the default — a shim
/// must answer); append defers to existing resolutions (version
/// managers like mise/rbenv keep their lead, tebako answers only for
/// names nothing else provides).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Prepend,
    Append,
}

impl Position {
    pub fn parse(name: &str) -> Result<Position, ShimError> {
        match name {
            "prepend" => Ok(Position::Prepend),
            "append" => Ok(Position::Append),
            _ => fail(
                EX_USAGE,
                format!("unsupported position \"{name}\" — supported: prepend, append"),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Csh,
}

impl Shell {
    pub fn parse(name: &str) -> Result<Shell, ShimError> {
        match name {
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            "fish" => Ok(Shell::Fish),
            "csh" | "tcsh" => Ok(Shell::Csh),
            _ => fail(
                EX_USAGE,
                format!("unsupported shell \"{name}\" — supported: bash, zsh, fish, csh"),
            ),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Csh => "csh",
        }
    }

    /// Detect from `$SHELL` (basename).
    pub fn detect(shell_env: Option<&str>) -> Result<Shell, ShimError> {
        let base = shell_env
            .and_then(|s| Path::new(s).file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Shell::parse(&base).map_err(|_| {
            ShimError::new(
                EX_USAGE,
                format!(
                    "cannot detect the shell from SHELL={} — pass --shell bash|zsh|fish|csh",
                    shell_env.unwrap_or("<unset>")
                ),
            )
        })
    }
}

/// The startup file for a shell (`user_home` is the OS home, not the
/// tebako home): `.bashrc` / `.zshrc` / `.config/fish/config.fish` /
/// `.cshrc`.
pub fn startup_file(shell: Shell, user_home: &Path) -> PathBuf {
    match shell {
        Shell::Bash => user_home.join(".bashrc"),
        Shell::Zsh => user_home.join(".zshrc"),
        Shell::Fish => user_home.join(".config").join("fish").join("config.fish"),
        Shell::Csh => user_home.join(".cshrc"),
    }
}

/// The managed block, BEGIN/END markers included.
pub fn block_text(shell: Shell, position: Position) -> String {
    let line = match (shell, position) {
        (Shell::Bash | Shell::Zsh, Position::Prepend) => {
            "export PATH=\"$HOME/.tebako/shims:$PATH\""
        }
        (Shell::Bash | Shell::Zsh, Position::Append) => "export PATH=\"$PATH:$HOME/.tebako/shims\"",
        (Shell::Fish, Position::Prepend) => "set -gx PATH \"$HOME/.tebako/shims\" $PATH",
        (Shell::Fish, Position::Append) => "set -gx PATH $PATH \"$HOME/.tebako/shims\"",
        (Shell::Csh, Position::Prepend) => "setenv PATH \"$HOME/.tebako/shims:$PATH\"",
        (Shell::Csh, Position::Append) => "setenv PATH \"$PATH:$HOME/.tebako/shims\"",
    };
    format!("{BEGIN_MARKER}\n{line}\n{END_MARKER}\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Installed,
    AlreadyPresent,
    Removed,
    NotPresent,
}

fn write_atomic(path: &Path, text: &str) -> Result<(), ShimError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ShimError::new(
                EX_TEBAKO_IO,
                format!("cannot create {}: {e}", parent.display()),
            )
        })?;
    }
    let tmp = path.with_extension(format!(
        "{}.tebako-tmp",
        path.extension().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(&tmp, text).map_err(|e| {
        ShimError::new(EX_TEBAKO_IO, format!("cannot write {}: {e}", tmp.display()))
    })?;
    std::fs::rename(&tmp, path).map_err(|e| {
        ShimError::new(
            EX_TEBAKO_IO,
            format!("cannot install {}: {e}", path.display()),
        )
    })
}

/// Insert the managed block. Idempotent: a file already carrying the
/// BEGIN marker at the requested position is left byte-identical; a
/// block present at the OTHER position is rewritten in place (a
/// position change is an install action, tebako#1004 §4), lines outside
/// the markers never touched.
pub fn install(shell: Shell, file: &Path, position: Position) -> Result<Change, ShimError> {
    let existing = std::fs::read_to_string(file).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == BEGIN_MARKER) {
        let wanted = block_text(shell, position);
        // The block is well-formed when a file carried it (install only
        // ever writes both markers); an unterminated one is the
        // uninstall path's named error — mirror it here, never guess.
        let lines: Vec<&str> = existing.lines().collect();
        let begin = lines.iter().position(|l| l.trim() == BEGIN_MARKER).unwrap();
        let Some(end_rel) = lines[begin..].iter().position(|l| l.trim() == END_MARKER) else {
            return fail(
                EX_TEBAKO_MANIFEST,
                format!(
                    "unterminated tebako block in {} (\"{BEGIN_MARKER}\" without \"{END_MARKER}\") — remove it manually; refusing to guess",
                    file.display()
                ),
            );
        };
        let end = begin + end_rel;
        let current = format!(
            "{}\n",
            lines[begin..=end]
                .iter()
                .map(|l| l.trim_end_matches('\r'))
                .collect::<Vec<_>>()
                .join("\n")
        );
        if current == wanted {
            return Ok(Change::AlreadyPresent);
        }
        let mut out: Vec<&str> = Vec::with_capacity(lines.len());
        out.extend_from_slice(&lines[..begin]);
        out.extend(wanted.trim_end_matches('\n').split('\n'));
        out.extend_from_slice(&lines[end + 1..]);
        let mut text = out.join("\n");
        if existing.ends_with('\n') && !text.ends_with('\n') {
            text.push('\n');
        }
        write_atomic(file, &text)?;
        return Ok(Change::Installed);
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&block_text(shell, position));
    write_atomic(file, &text)?;
    Ok(Change::Installed)
}

/// Remove exactly the managed block. Idempotent: no BEGIN marker →
/// no-op. An unterminated block is a named error and the file is left
/// untouched (lines outside the markers are never rewritten).
pub fn uninstall(file: &Path) -> Result<Change, ShimError> {
    let existing = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Change::NotPresent),
        Err(e) => return fail(EX_TEBAKO_IO, format!("cannot read {}: {e}", file.display())),
    };
    let lines: Vec<&str> = existing.lines().collect();
    let begin = lines.iter().position(|l| l.trim() == BEGIN_MARKER);
    let Some(begin) = begin else {
        return Ok(Change::NotPresent);
    };
    let end_rel = lines[begin..].iter().position(|l| l.trim() == END_MARKER);
    let Some(end_rel) = end_rel else {
        return fail(
            EX_TEBAKO_MANIFEST,
            format!(
                "unterminated tebako block in {} (\"{BEGIN_MARKER}\" without \"{END_MARKER}\") — remove it manually; refusing to guess",
                file.display()
            ),
        );
    };
    let end = begin + end_rel;
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    out.extend_from_slice(&lines[..begin]);
    out.extend_from_slice(&lines[end + 1..]);
    let mut text = out.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    write_atomic(file, &text)?;
    Ok(Change::Removed)
}

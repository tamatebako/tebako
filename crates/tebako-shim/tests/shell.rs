//! Shell block idempotency (spec 07 §3): install inserts a managed
//! BEGIN/END block once; uninstall removes exactly its block; lines
//! outside the markers are never touched.

mod common;

use common::TempDir;
use tebako_shim::shell::{self, Change, Position, Shell};

#[test]
fn install_is_idempotent_and_preserves_user_lines() {
    let tmp = TempDir::new("shell-install");
    let rc = tmp.path().join(".bashrc");
    let original = "# my own aliases\nalias ll='ls -l'\nexport EDITOR=vim\n";
    std::fs::write(&rc, original).unwrap();

    assert_eq!(
        shell::install(Shell::Bash, &rc, Position::Prepend).unwrap(),
        Change::Installed
    );
    let after_first = std::fs::read_to_string(&rc).unwrap();
    assert!(after_first.starts_with(original), "user lines untouched");
    assert!(after_first.contains(shell::BEGIN_MARKER));
    assert!(after_first.contains("export PATH=\"$HOME/.tebako/shims:$PATH\""));
    assert!(after_first.contains(shell::END_MARKER));

    assert_eq!(
        shell::install(Shell::Bash, &rc, Position::Prepend).unwrap(),
        Change::AlreadyPresent
    );
    assert_eq!(
        std::fs::read_to_string(&rc).unwrap(),
        after_first,
        "idempotent"
    );
}

#[test]
fn uninstall_removes_exactly_the_block_and_is_idempotent() {
    let tmp = TempDir::new("shell-uninstall");
    let rc = tmp.path().join(".zshrc");
    let original = "# zsh config\nsetopt autocd\n";
    std::fs::write(&rc, original).unwrap();

    shell::install(Shell::Zsh, &rc, Position::Prepend).unwrap();
    assert_eq!(shell::uninstall(&rc).unwrap(), Change::Removed);
    assert_eq!(
        std::fs::read_to_string(&rc).unwrap(),
        original,
        "uninstall restores the original bytes"
    );
    assert_eq!(shell::uninstall(&rc).unwrap(), Change::NotPresent);
}

#[test]
fn uninstall_never_touches_lines_outside_the_markers() {
    let tmp = TempDir::new("shell-strict");
    let rc = tmp.path().join(".bashrc");
    let original = format!(
        "before\n{}\nexport PATH=\"$HOME/.tebako/shims:$PATH\"\n{}\nafter\n",
        shell::BEGIN_MARKER,
        shell::END_MARKER
    );
    std::fs::write(&rc, &original).unwrap();
    shell::uninstall(&rc).unwrap();
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), "before\nafter\n");
}

#[test]
fn unterminated_block_is_a_named_error_and_the_file_is_untouched() {
    let tmp = TempDir::new("shell-unterminated");
    let rc = tmp.path().join(".bashrc");
    let original = format!("before\n{}\nexport PATH=x\n", shell::BEGIN_MARKER);
    std::fs::write(&rc, &original).unwrap();
    let err = shell::uninstall(&rc).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("unterminated"), "{}", err.message);
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), original);
}

#[test]
fn per_shell_syntax_and_startup_files() {
    assert!(shell::block_text(Shell::Fish, Position::Prepend)
        .contains("set -gx PATH \"$HOME/.tebako/shims\" $PATH"));
    assert!(shell::block_text(Shell::Csh, Position::Prepend)
        .contains("setenv PATH \"$HOME/.tebako/shims:$PATH\""));
    assert!(shell::block_text(Shell::Zsh, Position::Prepend)
        .contains("export PATH=\"$HOME/.tebako/shims:$PATH\""));

    let home = std::path::Path::new("/home/u");
    assert_eq!(shell::startup_file(Shell::Bash, home), home.join(".bashrc"));
    assert_eq!(shell::startup_file(Shell::Zsh, home), home.join(".zshrc"));
    assert_eq!(
        shell::startup_file(Shell::Fish, home),
        home.join(".config").join("fish").join("config.fish")
    );
    assert_eq!(shell::startup_file(Shell::Csh, home), home.join(".cshrc"));
}

#[test]
fn shell_detection_and_unknown_shells() {
    assert_eq!(Shell::detect(Some("/bin/zsh")).unwrap(), Shell::Zsh);
    assert_eq!(
        Shell::detect(Some("/usr/local/bin/fish")).unwrap(),
        Shell::Fish
    );
    assert_eq!(Shell::detect(Some("/bin/tcsh")).unwrap(), Shell::Csh);
    assert!(Shell::detect(Some("/bin/ion")).is_err());
    assert!(Shell::detect(None).is_err());
    assert!(Shell::parse("powershell").is_err());
}

#[test]
fn install_creates_a_missing_startup_file() {
    let tmp = TempDir::new("shell-create");
    let rc = tmp.path().join(".config").join("fish").join("config.fish");
    assert_eq!(
        shell::install(Shell::Fish, &rc, Position::Prepend).unwrap(),
        Change::Installed
    );
    let text = std::fs::read_to_string(&rc).unwrap();
    assert_eq!(text, shell::block_text(Shell::Fish, Position::Prepend));
}

#[test]
fn append_position_renders_the_shims_dir_last() {
    assert!(shell::block_text(Shell::Bash, Position::Append)
        .contains("export PATH=\"$PATH:$HOME/.tebako/shims\""));
    assert!(shell::block_text(Shell::Fish, Position::Append)
        .contains("set -gx PATH $PATH \"$HOME/.tebako/shims\""));
    assert!(shell::block_text(Shell::Csh, Position::Append)
        .contains("setenv PATH \"$PATH:$HOME/.tebako/shims\""));
    assert!(shell::Position::parse("prepend").unwrap() == Position::Prepend);
    assert!(shell::Position::parse("append").unwrap() == Position::Append);
    assert!(shell::Position::parse("middle").is_err());
}

#[test]
fn a_position_change_rewrites_the_block_in_place() {
    let tmp = TempDir::new("shell-position");
    let rc = tmp.path().join(".bashrc");
    let original = "# my own lines\nexport EDITOR=vim\n";
    std::fs::write(&rc, original).unwrap();

    shell::install(Shell::Bash, &rc, Position::Prepend).unwrap();
    let prepended = std::fs::read_to_string(&rc).unwrap();
    assert!(prepended.contains("export PATH=\"$HOME/.tebako/shims:$PATH\""));

    // Asking for the other position swaps the line, keeps the user's
    // lines, keeps ONE block.
    assert_eq!(
        shell::install(Shell::Bash, &rc, Position::Append).unwrap(),
        Change::Installed
    );
    let appended = std::fs::read_to_string(&rc).unwrap();
    assert!(appended.starts_with(original), "user lines untouched");
    assert!(appended.contains("export PATH=\"$PATH:$HOME/.tebako/shims\""));
    assert!(!appended.contains("$HOME/.tebako/shims:$PATH"));
    assert_eq!(appended.matches(shell::BEGIN_MARKER).count(), 1);

    // Same position again: byte-identical.
    assert_eq!(
        shell::install(Shell::Bash, &rc, Position::Append).unwrap(),
        Change::AlreadyPresent
    );
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), appended);
}

#[test]
fn install_with_position_refuses_an_unterminated_block() {
    let tmp = TempDir::new("shell-unterminated-install");
    let rc = tmp.path().join(".bashrc");
    let original = format!("before\n{}\nexport PATH=x\n", shell::BEGIN_MARKER);
    std::fs::write(&rc, &original).unwrap();
    let err = shell::install(Shell::Bash, &rc, Position::Append).unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST);
    assert!(err.message.contains("unterminated"), "{}", err.message);
    assert_eq!(std::fs::read_to_string(&rc).unwrap(), original);
}

use std::process::Command;

#[test]
fn help_and_version_work_outside_a_repository() {
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["commit", "--help"],
        vec!["submit", "--help"],
        vec!["changelog", "--help"],
        vec!["check-commit", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"))
            .args(args)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn removed_interfaces_and_ambiguous_arguments_fail_before_execution() {
    for args in [
        vec![
            "check-ci",
            "--event-name",
            "pull_request",
            "--event-file",
            "event.json",
        ],
        vec!["__verify-commit"],
        vec!["changelog", "--base", "main", "--pr", "42"],
        vec!["changelog", "--from", "main"],
        vec!["changelog", "--from", "a", "--to", "b", "--check"],
        vec!["commit", "--yes", "--yes"],
        vec!["commit", "--timeout", "0"],
        vec!["submit", "--revision"],
        vec!["submit", "--revision", "a", "--revision", "b"],
        vec![
            "check-commit",
            "--message-file",
            "x",
            "--from",
            "main",
            "--to",
            "HEAD",
        ],
        vec![
            "changelog",
            "--from",
            "a",
            "--to",
            "b",
            "--entry-file",
            "x",
            "--codex",
            "codex",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"))
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}

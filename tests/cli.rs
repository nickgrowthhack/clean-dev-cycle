use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"))
        .args(arguments)
        .current_dir(std::env::temp_dir())
        .output()
        .expect("o binário deve executar sem depender do diretório do projeto")
}

#[test]
fn help_is_available_without_a_repository() {
    for arguments in [&[][..], &["--help"][..], &["-h"][..]] {
        let output = invoke(arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).expect("ajuda em UTF-8");
        assert!(help.contains("Uso: clean-dev-cycle"));
        assert!(help.contains("Opções:"));
        assert!(help.contains("--version"));
    }
}

#[test]
fn version_matches_the_installable_package() {
    for argument in ["--version", "-V"] {
        let output = invoke(&[argument]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout)
                .expect("versão em UTF-8")
                .trim(),
            format!("clean-dev-cycle {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn invalid_invocations_cannot_report_success() {
    for arguments in [
        &["--inexistente"][..],
        &["check-ci"][..],
        &[
            "check-ci",
            "--event-name",
            "push",
            "--event-file",
            "event.json",
        ][..],
        &["check-ci", "--event-name", "pull_request"][..],
        &["check-ci", "--event-file", "event.json"][..],
        &[
            "check-ci",
            "--event-name",
            "pull_request",
            "--event-name",
            "pull_request",
            "--event-file",
            "event.json",
        ][..],
        &[
            "check-ci",
            "--event-name",
            "pull_request",
            "--event-file",
            "event.json",
            "--config",
            "policy.toml",
        ][..],
        &["check-commit"][..],
        &["check-commit", "--from", "main"][..],
        &[
            "check-commit",
            "--message-file",
            "message",
            "--config",
            "policy.toml",
        ][..],
        &[
            "check-commit",
            "--message-file",
            "message",
            "--from",
            "main",
            "--to",
            "HEAD",
        ][..],
        &[
            "check-commit",
            "--from",
            "main",
            "--to",
            "HEAD",
            "--to",
            "main",
        ][..],
        &["check-commit", "--message-file", "message", "--yes"][..],
        &["ação-desconhecida"][..],
        &["--version", "extra"][..],
        &["--help", "--version"][..],
        &["changelog"][..],
        &["changelog", "--base", "main", "--pr", "1", "--entry-file"][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--entry-file",
            "note.md",
            "--check",
        ][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--entry-file",
            "note.md",
            "--model",
            "model",
        ][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--entry-file",
            "note.md",
            "--codex",
            "codex",
        ][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--entry-file",
            "note.md",
            "--timeout",
            "1",
        ][..],
        &["changelog", "--base", "main"][..],
        &["changelog", "--base", "--pr", "1"][..],
        &["changelog", "--base", "main", "--pr", "0"][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--check",
            "--dry-run",
        ][..],
        &[
            "changelog",
            "--base",
            "main",
            "--pr",
            "1",
            "--check",
            "--yes",
        ][..],
        &["changelog", "--base", "main", "--pr", "1", "--pr", "2"][..],
        &["changelog", "--base", "main", "--pr", "1", "--timeout"][..],
    ] {
        let output = invoke(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).expect("erro em UTF-8");
        assert!(error.contains("Erro:"));
        assert!(error.contains("--help"));
    }
}

#[test]
fn check_commit_help_describes_the_fixed_read_only_interface() {
    let output = invoke(&["check-commit", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "--message-file",
        "--from",
        "--to",
        "Não chama IA",
        "0",
        "1",
        "2",
    ] {
        assert!(help.contains(expected), "{expected}");
    }
    assert!(!help.contains("--config"));
}

#[test]
fn ci_help_describes_pr_validation_without_a_repository() {
    let output = invoke(&["check-ci", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "pull_request",
        "--event-file",
        "título do PR é livre",
        "feat, fix, perf",
    ] {
        assert!(help.contains(expected));
    }
    assert!(!help.contains("push"));
}

#[test]
fn changelog_help_explains_pr_review_without_a_repository() {
    let output = invoke(&["changelog", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "--base",
        "--pr",
        "--head",
        "--dry-run",
        "--check",
        "--context-file",
        "Codex",
    ] {
        assert!(help.contains(expected));
    }
}

mod common;
use common::{Repo, failure, success};
use std::{
    fs,
    process::{Command, Output},
};

const BIN: &str = env!("CARGO_BIN_EXE_clean-dev-cycle");

fn commit(repo: &Repo, message: &str) -> String {
    repo.git(&["commit", "--allow-empty", "-m", message]);
    repo.git(&["rev-parse", "HEAD"]).trim().to_owned()
}

fn check(repo: &Repo, args: &[&str]) -> Output {
    repo.cli(&["check-commit"]).args(args).output().unwrap()
}

#[test]
fn message_files_are_checked_without_a_repository_and_never_rewritten() {
    let repo = Repo::new();
    let file = repo.directory.path().join("message.txt");
    let original = "fix: corrigir seleção\r\n\r\nRefs: #12\r\n";
    fs::write(&file, original).unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.directory.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    success(&result);
    assert_eq!(fs::read_to_string(&file).unwrap(), original);
    fs::write(&file, "feat: Adicionar recurso.").unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.directory.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    success(&result);
    fs::write(&file, [0xff, 0xfe]).unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.directory.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    failure(&result, "UTF-8");
}

#[test]
fn message_validation_needs_neither_git_nor_project_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("message.txt");
    fs::write(&file, "feat: Adicionar recurso.\n").unwrap();
    fs::write(dir.path().join("clean-dev-cycle.toml"), "invalid TOML").unwrap();
    let output = Command::new(BIN)
        .current_dir(dir.path())
        .env("PATH", dir.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    success(&output);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "feat: Adicionar recurso.\n"
    );
}

#[test]
fn ranges_check_every_commit_without_touching_head_index_or_worktree() {
    let repo = Repo::new();
    let base = commit(&repo, "chore: iniciar");
    let invalid = commit(&repo, "mensagem inválida");
    let last = commit(&repo, "fix: corrigir recurso");
    fs::write(repo.root.join("partial.txt"), "staged\n").unwrap();
    repo.git(&["add", "partial.txt"]);
    fs::write(repo.root.join("partial.txt"), "unstaged\n").unwrap();
    let index = repo.git(&["ls-files", "--stage"]);
    let status = repo.git(&["status", "--porcelain=v1"]);
    let result = check(&repo, &["--from", &base, "--to", "HEAD"]);
    failure(&result, &invalid);
    assert!(String::from_utf8_lossy(&result.stderr).contains("1 de 2"));
    success(&check(&repo, &["--from", &invalid, "--to", "HEAD"]));
    let empty = check(&repo, &["--from", &last, "--to", &last]);
    success(&empty);
    assert!(String::from_utf8_lossy(&empty.stdout).contains("0 commit(s)"));
    assert_eq!(repo.git(&["rev-parse", "HEAD"]).trim(), last);
    assert_eq!(repo.git(&["ls-files", "--stage"]), index);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), status);
    assert_eq!(
        fs::read_to_string(repo.root.join("partial.txt")).unwrap(),
        "unstaged\n"
    );
    failure(
        &check(&repo, &["--from", "missing", "--to", "HEAD"]),
        "referência",
    );
    failure(
        &check(&repo, &["--from", &base, "--to", "HEAD^{tree}"]),
        "referência",
    );
}

#[test]
fn two_dot_ranges_include_merged_branches_and_do_not_ignore_merge_messages() {
    let repo = Repo::new();
    let base = commit(&repo, "chore: iniciar");
    repo.git(&["checkout", "-b", "feature"]);
    let bad = commit(&repo, "mensagem inválida");
    repo.git(&["checkout", "main"]);
    commit(&repo, "fix: corrigir outro recurso");
    repo.git(&[
        "merge",
        "--no-ff",
        "feature",
        "-m",
        "Merge branch 'feature'",
    ]);
    let merge = repo.git(&["rev-parse", "HEAD"]).trim().to_owned();
    let result = check(&repo, &["--from", &base, "--to", "HEAD"]);
    failure(&result, &bad);
    assert!(String::from_utf8_lossy(&result.stderr).contains(&merge));
    assert!(String::from_utf8_lossy(&result.stderr).contains("2 de 3"));
}

#[test]
fn shallow_history_cannot_report_partial_validation_as_success() {
    let repo = Repo::new();
    commit(&repo, "chore: iniciar");
    commit(&repo, "fix: corrigir");
    let shallow = repo.directory.path().join("shallow");
    let url = format!("file://{}", repo.root.to_string_lossy().replace('\\', "/"));
    success(
        &repo
            .command("git")
            .args(["clone", "--depth=1", &url])
            .arg(&shallow)
            .output()
            .unwrap(),
    );
    let result = repo
        .command(BIN)
        .current_dir(&shallow)
        .args(["check-commit", "--from", "HEAD", "--to", "HEAD"])
        .output()
        .unwrap();
    failure(&result, "histórico completo");
}

#[test]
fn manual_commits_use_the_same_validator_through_an_existing_hook() {
    let repo = Repo::new();
    let base = commit(&repo, "chore: iniciar");
    let hooks = repo.root.join("hooks d'ação");
    fs::create_dir(&hooks).unwrap();
    let hook = hooks.join("commit-msg");
    let binary = BIN.replace('\\', "/").replace('\'', "'\\''");
    fs::write(&hook, format!("#!/bin/sh\nprintf 'executado\\n' >> hook-log\n'{binary}' check-commit --message-file \"$1\"\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    }
    repo.git(&["config", "core.hooksPath", "hooks d'ação"]);
    fs::write(repo.root.join("file.txt"), "staged\n").unwrap();
    repo.git(&["add", "file.txt"]);
    let index = repo.git(&["ls-files", "--stage"]);
    let result = repo
        .command("git")
        .args(["commit", "-m", "mensagem inválida"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(repo.git(&["rev-parse", "HEAD"]).trim(), base);
    assert_eq!(repo.git(&["ls-files", "--stage"]), index);
    repo.git(&["commit", "-m", "fix: corrigir arquivo"]);
    assert_eq!(
        repo.git(&["config", "core.hooksPath"]).trim(),
        "hooks d'ação"
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("hook-log"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_clean-dev-cycle");

struct Repo {
    dir: TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::write(dir.path().join("gitconfig"), "").unwrap();
        let repo = Self { dir, root };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.name", "Teste"]);
        repo.git(&["config", "user.email", "teste@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo
    }

    fn command(&self, program: &str) -> Command {
        let mut cmd = Command::new(program);
        cmd.current_dir(&self.root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"));
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            cmd.env_remove(name);
        }
        cmd
    }

    fn git(&self, args: &[&str]) -> String {
        let result = self.command("git").args(args).output().unwrap();
        success(&result);
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    }

    fn commit(&self, message: &str) -> String {
        self.git(&["commit", "--allow-empty", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn check(&self, args: &[&str]) -> Output {
        self.command(BIN)
            .arg("check-commit")
            .args(args)
            .output()
            .unwrap()
    }
}

fn success(result: &Output) {
    assert!(
        result.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn failure(result: &Output, expected: &str) {
    assert_eq!(result.status.code(), Some(1), "{result:?}");
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(expected),
        "{result:?}"
    );
}

#[test]
fn message_files_are_checked_without_a_repository_and_never_rewritten() {
    let repo = Repo::new();
    let file = repo.dir.path().join("message.txt");
    let original = "fix: corrigir seleção\r\n\r\nRefs: #12\r\n";
    fs::write(&file, original).unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.dir.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    success(&result);
    assert_eq!(fs::read_to_string(&file).unwrap(), original);
    fs::write(&file, "feat: Adicionar recurso.").unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.dir.path())
        .args(["check-commit", "--message-file"])
        .arg(&file)
        .output()
        .unwrap();
    success(&result);
    fs::write(&file, [0xff, 0xfe]).unwrap();
    let result = repo
        .command(BIN)
        .current_dir(repo.dir.path())
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
    let base = repo.commit("chore: iniciar");
    let invalid = repo.commit("mensagem inválida");
    let last = repo.commit("fix: corrigir recurso");
    fs::write(repo.root.join("partial.txt"), "staged\n").unwrap();
    repo.git(&["add", "partial.txt"]);
    fs::write(repo.root.join("partial.txt"), "unstaged\n").unwrap();
    let index = repo.git(&["ls-files", "--stage"]);
    let status = repo.git(&["status", "--porcelain=v1"]);
    let result = repo.check(&["--from", &base, "--to", "HEAD"]);
    failure(&result, &invalid);
    assert!(String::from_utf8_lossy(&result.stderr).contains("1 de 2"));
    success(&repo.check(&["--from", &invalid, "--to", "HEAD"]));
    let empty = repo.check(&["--from", &last, "--to", &last]);
    success(&empty);
    assert!(String::from_utf8_lossy(&empty.stdout).contains("0 commit(s)"));
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), last);
    assert_eq!(repo.git(&["ls-files", "--stage"]), index);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), status);
    assert_eq!(
        fs::read_to_string(repo.root.join("partial.txt")).unwrap(),
        "unstaged\n"
    );
    failure(
        &repo.check(&["--from", "missing", "--to", "HEAD"]),
        "referência",
    );
    failure(
        &repo.check(&["--from", &base, "--to", "HEAD^{tree}"]),
        "referência",
    );
}

#[test]
fn two_dot_ranges_include_merged_branches_and_do_not_ignore_merge_messages() {
    let repo = Repo::new();
    let base = repo.commit("chore: iniciar");
    repo.git(&["checkout", "-b", "feature"]);
    let bad = repo.commit("mensagem inválida");
    repo.git(&["checkout", "main"]);
    repo.commit("fix: corrigir outro recurso");
    repo.git(&[
        "merge",
        "--no-ff",
        "feature",
        "-m",
        "Merge branch 'feature'",
    ]);
    let merge = repo.git(&["rev-parse", "HEAD"]);
    let result = repo.check(&["--from", &base, "--to", "HEAD"]);
    failure(&result, &bad);
    assert!(String::from_utf8_lossy(&result.stderr).contains(&merge));
    assert!(String::from_utf8_lossy(&result.stderr).contains("2 de 3"));
}

#[test]
fn shallow_history_cannot_report_partial_validation_as_success() {
    let repo = Repo::new();
    repo.commit("chore: iniciar");
    repo.commit("fix: corrigir");
    let shallow = repo.dir.path().join("shallow");
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
    let base = repo.commit("chore: iniciar");
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
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), base);
    assert_eq!(repo.git(&["ls-files", "--stage"]), index);
    repo.git(&["commit", "-m", "fix: corrigir arquivo"]);
    assert_eq!(repo.git(&["config", "core.hooksPath"]), "hooks d'ação");
    assert_eq!(
        fs::read_to_string(repo.root.join("hook-log"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

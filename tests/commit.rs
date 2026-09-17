use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};
use tempfile::TempDir;

const MESSAGE: &str = "feat(cli): implementar geração de commits";

struct Repo {
    directory: TempDir,
    root: PathBuf,
}

fn fake_codex() -> PathBuf {
    static DIRECTORY: OnceLock<TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let directory = tempfile::Builder::new()
                .prefix("codex simulado ")
                .tempdir()
                .unwrap();
            let executable = directory
                .path()
                .join(format!("codex{}", std::env::consts::EXE_SUFFIX));
            let output = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "fake_codex"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_codex.rs"))
                .arg("-o")
                .arg(executable)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            directory
        })
        .path()
        .join(format!("codex{}", std::env::consts::EXE_SUFFIX))
}

impl Repo {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("ciclo d'ação ")
            .tempdir()
            .unwrap();
        let root = directory.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::create_dir(directory.path().join("codex-home")).unwrap();
        fs::write(
            directory.path().join("codex-home/config.toml"),
            "model = 'modelo-configurado'\nmodel_reasoning_effort = 'low'\n",
        )
        .unwrap();
        fs::write(directory.path().join("gitconfig"), "").unwrap();
        let repo = Self { directory, root };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.name", "Teste"]);
        repo.git(&["config", "user.email", "teste@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo
    }
    fn isolate(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.directory.path().join("gitconfig"));
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            command.env_remove(name);
        }
    }
    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        command.current_dir(&self.root).args(args);
        self.isolate(&mut command);
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn stage(&self, name: &str, content: impl AsRef<[u8]>) {
        fs::write(self.root.join(name), content).unwrap();
        self.git(&["add", "--", name]);
    }
    fn command(&self, mode: &str, args: &[&str]) -> Command {
        let mut command = self.default_command(mode);
        command.arg("--codex").arg(fake_codex()).args(args);
        command
    }
    fn default_command(&self, mode: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"));
        command
            .current_dir(&self.root)
            .arg("commit")
            .env("CODEX_HOME", self.directory.path().join("codex-home"))
            .env("FAKE_MODE", mode)
            .env("FAKE_REPO", &self.root)
            .env("FAKE_LOG", self.directory.path().join("calls"));
        self.isolate(&mut command);
        command
    }
    fn invoke(&self, mode: &str, args: &[&str]) -> Output {
        self.command(mode, args).output().unwrap()
    }
    fn calls(&self) -> usize {
        fs::read_to_string(self.directory.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
    fn unchanged(&self) {
        assert!(self.git(&["ls-files", "--stage"]).contains("arquivo.txt"));
        assert!(!self.root.join(".git/refs/heads/main").exists());
    }
    fn hook(&self, directory: &str, name: &str, body: &str) {
        let directory = self.root.join(directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn failure(output: &Output, expected: &str) {
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn dry_run_uses_only_stage_and_preserves_partial_selection() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "conteúdo selecionado\n");
    fs::write(repo.root.join("arquivo.txt"), "SEGREDO_NAO_SELECIONADO\n").unwrap();
    let before = repo.git(&["status", "--porcelain=v1"]);
    let output = repo.invoke("valid", &["--dry-run"]);
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains(MESSAGE));
    assert_eq!(before, repo.git(&["status", "--porcelain=v1"]));
    repo.unchanged();
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("conteúdo selecionado"));
    assert!(!prompt.contains("SEGREDO_NAO_SELECIONADO"));
    let args = fs::read_to_string(repo.directory.path().join("calls.args")).unwrap();
    assert!(args.contains("modelo-configurado"));
}

#[test]
fn commits_initial_stage_and_keeps_unstaged_changes() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "selecionado\n");
    fs::write(repo.root.join("arquivo.txt"), "alteração posterior\n").unwrap();
    success(&repo.invoke("valid", &["--yes"]));
    assert_eq!(repo.git(&["show", "HEAD:arquivo.txt"]), "selecionado\n");
    assert_eq!(repo.git(&["log", "-1", "--format=%B"]).trim(), MESSAGE);
    assert!(repo.git(&["diff", "--cached"]).is_empty());
    assert!(repo.git(&["diff"]).contains("alteração posterior"));
}

#[test]
fn existing_history_subdirectories_and_relative_context_are_supported() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "antes\n");
    repo.git(&["commit", "-m", "chore: iniciar"]);
    repo.stage("arquivo.txt", "depois\n");
    fs::create_dir(repo.root.join("subdir")).unwrap();
    fs::write(
        repo.root.join("subdir/contexto.txt"),
        "Intenção explicitamente fornecida",
    )
    .unwrap();
    success(
        &repo
            .command(
                "valid",
                &[
                    "--yes",
                    "--context-file",
                    "contexto.txt",
                    "--model",
                    "modelo-explicito",
                ],
            )
            .current_dir(repo.root.join("subdir"))
            .output()
            .unwrap(),
    );
    assert_eq!(repo.git(&["rev-list", "--count", "HEAD"]).trim(), "2");
    assert!(
        fs::read_to_string(repo.directory.path().join("calls.prompt"))
            .unwrap()
            .contains("Intenção explicitamente fornecida")
    );
    assert!(
        fs::read_to_string(repo.directory.path().join("calls.args"))
            .unwrap()
            .contains("modelo-explicito")
    );
}

#[test]
fn existing_hooks_and_custom_hook_path_are_preserved() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    let hooks = "hooks d'ação";
    repo.git(&["config", "core.hooksPath", hooks]);
    repo.hook(hooks, "pre-commit", "printf 'pre\\n' >> .git/hook-log");
    repo.hook(
        hooks,
        "prepare-commit-msg",
        "printf 'prepare\\n' >> .git/hook-log",
    );
    repo.hook(hooks, "commit-msg", "printf 'message\\n' >> .git/hook-log\nprintf '\\nSigned-off-by: Teste <teste@example.com>\\n' >> \"$1\"");
    repo.hook(hooks, "post-commit", "printf 'post\\n' >> .git/hook-log");
    success(&repo.invoke("valid", &["--yes"]));
    assert_eq!(
        fs::read_to_string(repo.root.join(".git/hook-log")).unwrap(),
        "pre\nprepare\nmessage\npost\n"
    );
    assert_eq!(repo.git(&["config", "core.hooksPath"]).trim(), hooks);
    assert!(
        repo.git(&["log", "-1", "--format=%B"])
            .contains("Signed-off-by")
    );
}

#[test]
fn hook_changes_to_stage_invalidate_the_message_before_commit() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    repo.hook(
        ".git/hooks",
        "pre-commit",
        "printf 'mudança do hook\\n' > outra.txt\ngit add outra.txt",
    );
    failure(&repo.invoke("valid", &["--yes"]), "mudou durante a revisão");
    repo.unchanged();
    assert!(repo.git(&["ls-files"]).contains("outra.txt"));
}

#[test]
fn failed_hooks_and_invalid_final_messages_block_commit() {
    for (hook, script, expected) in [
        (
            "pre-commit",
            "echo 'hook recusou' >&2\nexit 1",
            "hook recusou",
        ),
        (
            "commit-msg",
            "printf 'mensagem inválida\\n' > \"$1\"",
            "Conventional Commits",
        ),
    ] {
        let repo = Repo::new();
        repo.stage("arquivo.txt", "novo\n");
        repo.hook(".git/hooks", hook, script);
        failure(&repo.invoke("valid", &["--yes"]), expected);
        repo.unchanged();
    }
}

#[test]
fn stage_changes_during_generation_block_commit() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    failure(
        &repo.invoke("mutate", &["--yes"]),
        "mudou durante a revisão",
    );
    repo.unchanged();
}

#[test]
fn invalid_messages_are_corrected_at_most_once() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    success(&repo.invoke("retry", &["--dry-run"]));
    assert_eq!(repo.calls(), 2);
    for mode in ["invalid", "malformed"] {
        let repo = Repo::new();
        repo.stage("arquivo.txt", "novo\n");
        failure(&repo.invoke(mode, &["--yes"]), "após duas tentativas");
        assert_eq!(repo.calls(), 2);
        repo.unchanged();
    }
}

#[test]
fn context_requests_and_provider_failures_never_commit() {
    for (mode, expected) in [
        ("needs-context", "precisa de mais contexto"),
        ("failure", "falha simulada"),
        ("tool", "evento não permitido (command_execution)"),
        ("timeout", "tempo limite excedido"),
        ("overflow", "excedeu o limite"),
    ] {
        let repo = Repo::new();
        repo.stage("arquivo.txt", "novo\n");
        failure(&repo.invoke(mode, &["--yes", "--timeout", "1"]), expected);
        repo.unchanged();
        assert_eq!(repo.calls(), 1);
    }
}

#[test]
fn no_stage_noninteractive_input_and_missing_context_fail_before_codex() {
    let repo = Repo::new();
    failure(&repo.invoke("valid", &["--dry-run"]), "não há alterações");
    repo.stage("arquivo.txt", "novo\n");
    failure(&repo.invoke("valid", &[]), "exige um terminal");
    failure(
        &repo.invoke("valid", &["--yes", "--context-file", "inexistente.txt"]),
        "não foi possível ler",
    );
    assert_eq!(repo.calls(), 0);
}

#[test]
fn sensitive_binary_non_utf8_and_large_diffs_are_not_sent() {
    for (name, content, expected) in [
        (".env", b"TOKEN=exemplo".to_vec(), "potencialmente sensível"),
        ("binario", vec![0, 1, 2, 0], "binário"),
        ("texto", vec![0xff, 0xff, b'\n'], "UTF-8"),
        ("grande", vec![b'x'; 130 * 1024], "128 KiB"),
    ] {
        let repo = Repo::new();
        repo.stage(name, content);
        failure(&repo.invoke("valid", &["--dry-run"]), expected);
        assert_eq!(repo.calls(), 0);
    }
}

#[test]
fn merge_in_progress_is_not_committed_by_accident() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    fs::write(
        repo.root.join(".git/MERGE_HEAD"),
        "0000000000000000000000000000000000000000\n",
    )
    .unwrap();
    failure(&repo.invoke("valid", &["--yes"]), "conclua a operação");
    assert_eq!(repo.calls(), 0);
}

#[test]
fn invalid_options_fail_without_running_codex() {
    let repo = Repo::new();
    for args in [
        &["--timeout", "0"][..],
        &["--timeout", "banana"][..],
        &["--context-file"][..],
        &["--yes", "--dry-run"][..],
        &["--yes", "--yes"][..],
        &["--inexistente"][..],
    ] {
        let output = repo.invoke("valid", args);
        assert_eq!(output.status.code(), Some(2));
    }
    assert_eq!(repo.calls(), 0);
}

#[test]
fn conflicts_are_detected_before_generation() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "base\n");
    repo.git(&["commit", "-m", "chore: iniciar"]);
    repo.git(&["checkout", "-b", "outra"]);
    repo.stage("arquivo.txt", "outra\n");
    repo.git(&["commit", "-m", "feat: mudar"]);
    repo.git(&["checkout", "main"]);
    repo.stage("arquivo.txt", "main\n");
    repo.git(&["commit", "-m", "fix: corrigir"]);
    let mut command = Command::new("git");
    command.current_dir(&repo.root).args(["merge", "outra"]);
    repo.isolate(&mut command);
    assert!(!command.output().unwrap().status.success());
    failure(
        &repo.invoke("valid", &["--yes"]),
        "conflitos não resolvidos",
    );
    assert_eq!(repo.calls(), 0);
}

#[test]
fn a_linked_worktree_commits_its_own_index() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "base\n");
    repo.git(&["commit", "-m", "chore: iniciar"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let worktree = repo.directory.path().join("outra árvore");
    repo.git(&["worktree", "add", "--detach", worktree.to_str().unwrap()]);
    fs::write(worktree.join("arquivo.txt"), "alterado\n").unwrap();
    repo.git(&["-C", worktree.to_str().unwrap(), "add", "arquivo.txt"]);
    success(
        &repo
            .command("valid", &["--yes"])
            .current_dir(&worktree)
            .output()
            .unwrap(),
    );
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(
        repo.git(&["-C", worktree.to_str().unwrap(), "log", "-1", "--format=%s"])
            .trim(),
        MESSAGE
    );
}

#[test]
fn a_missing_codex_executable_does_not_change_the_stage() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    let mut command = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"));
    command
        .current_dir(&repo.root)
        .args(["commit", "--yes", "--codex"])
        .arg(repo.directory.path().join("codex-inexistente.exe"))
        .env("CODEX_HOME", repo.directory.path().join("codex-home"));
    repo.isolate(&mut command);
    failure(&command.output().unwrap(), "não foi possível executar");
    repo.unchanged();
}

#[test]
#[ignore = "usa o Codex CLI autenticado e consome cota; executar explicitamente"]
fn real_codex_generates_a_message_without_committing() {
    let repo = Repo::new();
    repo.stage(
        "arquivo.txt",
        "# Projeto de exemplo\n\nExecute cargo test para verificar o projeto.\n",
    );
    let before = repo.git(&["status", "--porcelain=v1"]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"));
    command
        .current_dir(&repo.root)
        .args(["commit", "--dry-run"]);
    #[cfg(windows)]
    command.env("PATH", path_without_codex());
    repo.isolate(&mut command);
    let output = command.output().unwrap();
    success(&output);
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Mensagem proposta:"));
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), before);
    repo.unchanged();
}

fn path_without_codex() -> std::ffi::OsString {
    let paths = std::env::var_os("PATH").unwrap();
    std::env::join_paths(std::env::split_paths(&paths).filter(|directory| {
        ![
            "codex",
            "codex.exe",
            "codex.com",
            "codex.cmd",
            "codex.bat",
            "codex.ps1",
        ]
        .iter()
        .any(|name| directory.join(name).is_file())
    }))
    .unwrap()
}

#[test]
fn default_discovery_reports_an_actionable_error_when_codex_is_absent() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    let output = repo
        .default_command("valid")
        .arg("--dry-run")
        .env("PATH", path_without_codex())
        .env("LOCALAPPDATA", repo.directory.path().join("sem aplicativo"))
        .output()
        .unwrap();
    failure(
        &output,
        "Instale o Codex CLI no PATH ou informe --codex CAMINHO",
    );
    assert_eq!(repo.calls(), 0);
    repo.unchanged();
}

#[cfg(windows)]
#[test]
fn desktop_codex_is_discovered_outside_the_app_environment() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    let local = repo.directory.path().join("Local AppData");
    let bundle = local.join("OpenAI/Codex/bin/identificador-do-pacote");
    fs::create_dir_all(&bundle).unwrap();
    fs::copy(fake_codex(), bundle.join("codex.exe")).unwrap();
    let before = repo.git(&["status", "--porcelain=v1"]);
    let output = repo
        .default_command("valid")
        .arg("--dry-run")
        .env("PATH", path_without_codex())
        .env("LOCALAPPDATA", &local)
        .output()
        .unwrap();
    success(&output);
    assert_eq!(repo.calls(), 1);
    assert_eq!(before, repo.git(&["status", "--porcelain=v1"]));
    repo.unchanged();
}

#[cfg(windows)]
#[test]
fn windows_command_shims_in_path_preserve_arguments() {
    let repo = Repo::new();
    repo.stage("arquivo.txt", "novo\n");
    let tools = repo.directory.path().join("ferramentas");
    fs::create_dir(&tools).unwrap();
    fs::write(
        tools.join("codex.cmd"),
        format!("@echo off\r\n\"{}\" %*\r\n", fake_codex().display()),
    )
    .unwrap();
    let path = std::env::join_paths(
        std::iter::once(tools).chain(std::env::split_paths(&path_without_codex())),
    )
    .unwrap();
    let output = repo
        .default_command("valid")
        .args(["--dry-run", "--model", "modelo com espaço & literal"])
        .env("PATH", path)
        .output()
        .unwrap();
    success(&output);
    let args = fs::read_to_string(repo.directory.path().join("calls.args")).unwrap();
    assert!(args.contains("modelo com espaço & literal"));
    assert_eq!(repo.calls(), 1);
    repo.unchanged();
}

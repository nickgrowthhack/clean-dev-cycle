use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};
use tempfile::TempDir;

fn fake_codex() -> PathBuf {
    static DIRECTORY: OnceLock<TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let output = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "fake_codex"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_codex.rs"))
                .arg("-o")
                .arg(
                    directory
                        .path()
                        .join(format!("codex{}", std::env::consts::EXE_SUFFIX)),
                )
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

struct Repo {
    directory: TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("changelog d'ação ")
            .tempdir()
            .unwrap();
        let root = directory.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::create_dir(directory.path().join("codex-home")).unwrap();
        fs::write(directory.path().join("gitconfig"), "").unwrap();
        let repo = Self { directory, root };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.name", "Teste"]);
        repo.git(&["config", "user.email", "teste@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo.commit("app.txt", "base\n", "início");
        repo.git(&["checkout", "-b", "feature"]);
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
        success(&output);
        String::from_utf8(output.stdout).unwrap()
    }

    fn commit(&self, path: &str, content: &str, message: &str) {
        fs::write(self.root.join(path), content).unwrap();
        self.git(&["add", "--", path]);
        self.git(&["commit", "-m", message]);
    }

    fn command(&self, mode: &str, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"));
        command
            .current_dir(&self.root)
            .args(["changelog", "--base", "main", "--pr", "42", "--codex"])
            .arg(fake_codex())
            .args(args)
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

    fn without_ai(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"));
        command
            .current_dir(&self.root)
            .args(["changelog", "--base", "main", "--pr", "42"])
            .args(args)
            // A regular file cannot be used as a Codex configuration directory.
            .env("CODEX_HOME", self.directory.path().join("gitconfig"));
        self.isolate(&mut command);
        command.output().unwrap()
    }

    fn note_file(&self, text: &str) -> PathBuf {
        let path = self.directory.path().join("nota.md");
        fs::write(&path, text).unwrap();
        path
    }

    fn document(&self) -> String {
        fs::read_to_string(self.root.join("CHANGELOG.md")).unwrap()
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.directory.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
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
fn reviews_the_whole_pr_from_the_merge_base_without_stage_or_intermediate_changes() {
    let repo = Repo::new();
    repo.commit("app.txt", "INTERMEDIARIO\n", "primeiro passo");
    repo.commit("app.txt", "RESULTADO_FINAL\n", "segundo passo");
    repo.commit("segundo.txt", "OUTRA_PARTE_DA_ENTREGA\n", "terceiro passo");
    repo.git(&["checkout", "main"]);
    repo.commit(
        "base.txt",
        "SOMENTE_NA_BASE_ATUAL\n",
        "mudança independente na base",
    );
    repo.git(&["checkout", "feature"]);
    fs::write(repo.root.join("app.txt"), "SOMENTE_NO_STAGE\n").unwrap();
    repo.git(&["add", "app.txt"]);
    fs::write(repo.root.join("app.txt"), "SOMENTE_NO_WORKTREE\n").unwrap();
    let before = repo.git(&["status", "--porcelain=v1"]);
    let output = repo.invoke("valid", &["--dry-run"]);
    success(&output);
    assert!(!repo.root.join("CHANGELOG.md").exists());
    assert_eq!(before, repo.git(&["status", "--porcelain=v1"]));
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("RESULTADO_FINAL"));
    assert!(prompt.contains("OUTRA_PARTE_DA_ENTREGA"));
    for excluded in [
        "INTERMEDIARIO",
        "SOMENTE_NA_BASE_ATUAL",
        "SOMENTE_NO_STAGE",
        "SOMENTE_NO_WORKTREE",
    ] {
        assert!(!prompt.contains(excluded), "{excluded}");
    }
}

#[test]
fn one_entry_per_pr_stays_current_after_committing_the_changelog() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "entrega em vários commits");
    failure(
        &repo.invoke("valid", &["--check"]),
        "ausente ou desatualizada",
    );
    assert_eq!(repo.calls(), 0);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let index = repo.git(&["ls-files", "--stage"]);
    success(&repo.invoke("valid", &["--yes"]));
    assert_eq!(head, repo.git(&["rev-parse", "HEAD"]));
    assert_eq!(index, repo.git(&["ls-files", "--stage"]));
    let first = repo.document();
    assert!(first.contains("### Revisão completa de entregas (#42)"));
    assert!(!first.contains("entrega em vários commits"));
    repo.git(&["add", "CHANGELOG.md"]);
    repo.git(&["commit", "-m", "docs: registrar a entrega"]);
    success(&repo.invoke("failure", &["--check"]));
    success(&repo.invoke("failure", &["--yes"]));
    assert_eq!(repo.calls(), 1);
    assert_eq!(first, repo.document());
    repo.commit("app.txt", "entrega ampliada\n", "ajuste posterior");
    failure(
        &repo.invoke("failure", &["--check"]),
        "ausente ou desatualizada",
    );
    assert_eq!(first, repo.document());
    success(&repo.invoke("valid", &["--yes"]));
    assert_eq!(repo.calls(), 2);
    assert_eq!(repo.document().matches("(#42)").count(), 1);
    assert_ne!(first, repo.document());
}

#[test]
fn preserves_manual_notes_and_uses_context_from_the_calling_directory() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "entrega");
    fs::write(
        repo.root.join("CHANGELOG.md"),
        "# Changelog\r\n\r\nIntrodução manual.\r\n\r\n## [0.1.0]\r\n\r\nHistórico mantido.\r\n",
    )
    .unwrap();
    let subdir = repo.root.join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("contexto.txt"), "INTENCAO_DO_PR").unwrap();
    success(
        &repo
            .command("valid", &["--yes", "--context-file", "contexto.txt"])
            .current_dir(&subdir)
            .output()
            .unwrap(),
    );
    let first = repo.document();
    assert!(first.starts_with("# Changelog\r\n\r\nIntrodução manual."));
    assert!(first.ends_with("## [0.1.0]\r\n\r\nHistórico mantido.\r\n"));
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("INTENCAO_DO_PR"));
    failure(
        &repo.invoke("failure", &["--check"]),
        "ausente ou desatualizada",
    );
    success(
        &repo
            .command("failure", &["--check", "--context-file", "contexto.txt"])
            .current_dir(&subdir)
            .output()
            .unwrap(),
    );
    assert_eq!(repo.calls(), 1);
}

#[test]
fn provider_errors_and_concurrent_edits_do_not_overwrite_the_changelog() {
    for (mode, expected) in [
        ("failure", "Codex CLI falhou"),
        ("invalid", "após duas tentativas"),
        ("needs-context", "precisa de mais contexto"),
        ("tool", "evento não permitido"),
        ("mutate", "mudou durante a revisão"),
        ("mutate-base", "referência do PR mudou"),
    ] {
        let repo = Repo::new();
        repo.commit("app.txt", "entrega\n", "entrega");
        fs::write(repo.root.join("CHANGELOG.md"), "# Notas manuais\n").unwrap();
        failure(&repo.invoke(mode, &["--yes"]), expected);
        assert_eq!(repo.document(), "# Notas manuais\n");
    }
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "entrega");
    failure(
        &repo.invoke("mutate-changelog", &["--yes"]),
        "CHANGELOG.md mudou",
    );
    assert_eq!(repo.document(), "alteração manual concorrente\n");
}

#[test]
fn invalid_ranges_and_incomplete_diffs_fail_before_using_ai() {
    let repo = Repo::new();
    failure(
        &repo.invoke("valid", &["--yes"]),
        "não há alterações no intervalo",
    );
    repo.commit("CHANGELOG.md", "# Histórico\n", "só o changelog");
    failure(
        &repo.invoke("valid", &["--yes"]),
        "não há alterações no intervalo",
    );
    repo.commit(".env", "TOKEN=exemplo\n", "arquivo sensível");
    failure(&repo.invoke("valid", &["--yes"]), "potencialmente sensível");
    assert_eq!(repo.calls(), 0);
    for (content, expected) in [
        ("\0binário".into(), "binário"),
        ("x".repeat(130 * 1024), "128 KiB"),
    ] {
        let repo = Repo::new();
        repo.commit("app.txt", &content, "arquivo");
        failure(&repo.invoke("valid", &["--yes"]), expected);
        assert_eq!(repo.calls(), 0);
    }
}

#[test]
fn missing_refs_broken_markers_and_noninteractive_review_are_rejected() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "entrega");
    failure(&repo.invoke("valid", &[]), "exige um terminal");
    failure(
        &repo.invoke("valid", &["--head", "inexistente", "--yes"]),
        "não resolve para um commit local",
    );
    fs::write(
        repo.root.join("CHANGELOG.md"),
        "<!-- clean-dev-cycle:pr:42:start -->\n",
    )
    .unwrap();
    failure(&repo.invoke("valid", &["--yes"]), "marcadores");
    assert_eq!(repo.calls(), 0);
}

#[test]
fn check_rejects_shallow_history_without_writing() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "entrega");
    let shallow = repo.directory.path().join("shallow");
    repo.git(&[
        "clone",
        "--depth=1",
        "--no-local",
        repo.root.to_str().unwrap(),
        shallow.to_str().unwrap(),
    ]);
    failure(
        &repo
            .command("valid", &["--check"])
            .current_dir(&shallow)
            .output()
            .unwrap(),
        "histórico completo",
    );
    assert!(!shallow.join("CHANGELOG.md").exists());
    assert_eq!(repo.calls(), 0);
}

#[test]
fn manual_notes_cover_binary_non_utf8_large_and_sensitive_changes_without_ai() {
    for (name, content) in [
        ("image.bin", b"\0image".to_vec()),
        ("non-utf8.txt", vec![0xff, 0xfe, b'\n']),
        ("large.txt", vec![b'x'; 140 * 1024]),
        (".env", b"EXAMPLE=value\n".to_vec()),
    ] {
        let repo = Repo::new();
        fs::write(repo.root.join(name), &content).unwrap();
        repo.git(&["add", "--", name]);
        repo.git(&["commit", "-m", "feat: incluir recurso"]);
        let note = repo.note_file("### Entrega completa\n\nSíntese revisada da mudança.");
        let file = note.to_str().unwrap();
        let head = repo.git(&["rev-parse", "HEAD"]);
        let index = repo.git(&["ls-files", "--stage"]);
        success(&repo.without_ai(&["--entry-file", file, "--dry-run"]));
        assert!(!repo.root.join("CHANGELOG.md").exists());
        success(&repo.without_ai(&["--entry-file", file, "--yes"]));
        assert_eq!(head, repo.git(&["rev-parse", "HEAD"]));
        assert_eq!(index, repo.git(&["ls-files", "--stage"]));
        assert!(repo.document().contains("fingerprint:v2:"));
        repo.git(&["add", "CHANGELOG.md"]);
        repo.git(&["commit", "-m", "docs: registrar nota"]);
        success(&repo.without_ai(&["--check"]));
        success(&repo.without_ai(&["--yes"]));
        fs::write(
            repo.root.join(name),
            [content, b"changed".to_vec()].concat(),
        )
        .unwrap();
        repo.git(&["add", "--", name]);
        repo.git(&["commit", "-m", "fix: ajustar recurso"]);
        failure(&repo.without_ai(&["--check"]), "desatualizada");
        assert_eq!(repo.calls(), 0);
    }
}

#[test]
fn supplied_note_can_replace_prose_and_rejects_invalid_input_without_writing() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "feat: entregar");
    let note = repo.note_file("### Primeira versão\n\nResultado.");
    let file = note.to_str().unwrap();
    success(&repo.without_ai(&["--entry-file", file, "--yes"]));
    let first = repo.document();
    fs::write(&note, "### Texto revisado\n\nResultado explicado.").unwrap();
    success(&repo.without_ai(&["--entry-file", file, "--yes"]));
    let revised = repo.document();
    assert!(revised.contains("Texto revisado"));
    assert!(!revised.contains("Primeira versão"));
    assert_eq!(
        first.lines().find(|s| s.contains("fingerprint:")),
        revised.lines().find(|s| s.contains("fingerprint:"))
    );
    for invalid in ["nota sem título".to_owned(), "x".repeat(17 * 1024)] {
        fs::write(&note, invalid).unwrap();
        assert!(
            !repo
                .without_ai(&["--entry-file", file, "--yes"])
                .status
                .success()
        );
        assert_eq!(repo.document(), revised);
    }
    assert!(
        !repo
            .without_ai(&["--entry-file", "missing.md", "--yes"])
            .status
            .success()
    );
    assert_eq!(repo.document(), revised);
}

#[test]
fn metadata_fingerprint_tracks_paths_modes_deletions_submodules_and_context() {
    let repo = Repo::new();
    repo.commit("z.txt", "z\n", "feat: adicionar z");
    repo.commit("a.txt", "a\n", "feat: adicionar a");
    let note = repo.note_file("### Entrega\n\nResultado revisado.");
    let file = note.to_str().unwrap();
    let context = repo.directory.path().join("context.txt");
    fs::write(&context, "intenção").unwrap();
    let context_path = context.to_str().unwrap();
    let record = || {
        success(&repo.without_ai(&[
            "--entry-file",
            file,
            "--context-file",
            context_path,
            "--yes",
        ]))
    };
    let check = || repo.without_ai(&["--context-file", context_path, "--check"]);
    record();
    let order = repo.directory.path().join("order.txt");
    fs::write(&order, "z*\na*\n").unwrap();
    repo.git(&["config", "diff.orderFile", order.to_str().unwrap()]);
    success(&check());
    repo.git(&["mv", "a.txt", "renamed.txt"]);
    repo.git(&["commit", "-m", "refactor: renomear"]);
    failure(&check(), "desatualizada");
    record();
    repo.git(&["update-index", "--chmod=+x", "z.txt"]);
    repo.git(&["commit", "-m", "build: marcar executável"]);
    failure(&check(), "desatualizada");
    record();
    repo.git(&["rm", "renamed.txt"]);
    repo.git(&["commit", "-m", "refactor: remover arquivo"]);
    failure(&check(), "desatualizada");
    record();
    let target = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{},module", target.trim()),
    ]);
    repo.git(&["commit", "-m", "feat: incluir submódulo"]);
    failure(&check(), "desatualizada");
    record();
    success(&check());
    fs::write(&context, "outra intenção").unwrap();
    failure(&check(), "desatualizada");
}

#[test]
fn legacy_entries_remain_verifiable_and_only_the_updated_entry_is_converted() {
    let repo = Repo::new();
    repo.commit("app.txt", "entrega\n", "feat: entregar");
    let diff = repo.git(&[
        "diff",
        "--ignore-submodules=none",
        "--no-relative",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--no-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--submodule=short",
        "--unified=3",
        "--full-index",
        "--diff-algorithm=myers",
        "--no-indent-heuristic",
        "--inter-hunk-context=0",
        "main",
        "HEAD",
        "--",
        ".",
        ":(top,exclude)CHANGELOG.md",
    ]);
    let input = serde_json::json!({"format": 1, "diff": diff, "context": ""}).to_string();
    let hash_file = repo.directory.path().join("fingerprint.json");
    fs::write(&hash_file, input).unwrap();
    let hash = repo.git(&["hash-object", "--no-filters", hash_file.to_str().unwrap()]);
    let other = format!(
        "<!-- clean-dev-cycle:pr:7:start -->\n<!-- clean-dev-cycle:fingerprint:{} -->\n### Histórico (#7)\n\nPreservado.\n<!-- clean-dev-cycle:pr:7:end -->\n",
        "a".repeat(40)
    );
    let legacy = format!(
        "# Changelog\n\nIntrodução manual.\n\n## [Não lançado]\n\n<!-- clean-dev-cycle:pr:42:start -->\n<!-- clean-dev-cycle:fingerprint:{} -->\n### Entrega antiga (#42)\n\nResultado.\n<!-- clean-dev-cycle:pr:42:end -->\n\n## [0.1.0]\n\n{other}",
        hash.trim()
    );
    fs::write(repo.root.join("CHANGELOG.md"), &legacy).unwrap();
    success(&repo.without_ai(&["--check"]));
    success(&repo.without_ai(&["--yes"]));
    assert_eq!(repo.document(), legacy);
    let note = repo.note_file("### Entrega atualizada\n\nResultado revisado.");
    success(&repo.without_ai(&["--entry-file", note.to_str().unwrap(), "--yes"]));
    assert!(repo.document().contains("fingerprint:v2:"));
    assert!(repo.document().ends_with(&format!("## [0.1.0]\n\n{other}")));
    success(&repo.without_ai(&["--check"]));
}

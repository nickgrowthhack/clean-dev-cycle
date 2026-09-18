use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_clean-dev-cycle");
const EMPTY_LOG: &str = "# Changelog\n\n## [Não lançado]\n";

struct Repo {
    dir: TempDir,
    root: PathBuf,
    base: String,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::create_dir(dir.path().join("codex-home")).unwrap();
        fs::write(dir.path().join("gitconfig"), "").unwrap();
        let mut repo = Self {
            dir,
            root,
            base: String::new(),
        };
        repo.git(&["init", "--initial-branch=main"]);
        for (key, value) in [
            ("user.name", "Teste"),
            ("user.email", "test@example.com"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            repo.git(&["config", key, value]);
        }
        repo.write("CHANGELOG.md", EMPTY_LOG);
        repo.write("app.txt", "base\n");
        repo.base = repo.commit("chore: iniciar projeto");
        repo.git(&["update-ref", "refs/remotes/origin/main", &repo.base]);
        repo.git(&["checkout", "-b", "feature"]);
        repo
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .env("CODEX_HOME", self.dir.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"));
        for key in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            command.env_remove(key);
        }
        command
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.command("git").args(args).output().unwrap();
        success(&output);
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn write(&self, path: &str, value: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        self.git(&["add", "."]);
        self.git(&["commit", "--allow-empty", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn note(&self, context: Option<&str>) {
        let path = self.dir.path().join("note.md");
        fs::write(&path, "### Entrega\n\nCada PR passa pela revisão.\n").unwrap();
        let mut command = self.command(BIN);
        command
            .args([
                "changelog",
                "--base",
                "main",
                "--pr",
                "42",
                "--yes",
                "--entry-file",
            ])
            .arg(path);
        if let Some(context) = context {
            command.args(["--context-file", context]);
        }
        success(&command.output().unwrap());
        self.commit("docs: registrar entrega");
    }

    fn event(&self) -> Value {
        json!({"number":42,"pull_request":{
            "head":{"sha":self.git(&["rev-parse", "HEAD"])},
            "base":{"sha":self.git(&["rev-parse", "main"])},
            "title":"feat: conferir entrega",
            "body":"Descrição **livre** em Markdown. Não vira mensagem de commit."
        }})
    }

    fn check(&self, name: &str, event: Value) -> Output {
        let path = self.dir.path().join("event.json");
        fs::write(&path, event.to_string()).unwrap();
        self.command(BIN)
            .args(["check-ci", "--event-name", name, "--event-file"])
            .arg(path)
            .output()
            .unwrap()
    }
}

fn success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
}
fn failure(output: &Output, message: &str) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{output:?}"
    );
}

#[test]
fn pr_requires_current_note_and_accepts_free_title_without_ai_or_mutations() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat!: mudar contrato");
    failure(
        &repo.check("pull_request", repo.event()),
        "entrada do PR está ausente",
    );
    repo.note(None);
    let snapshot = repo.git(&["status", "--porcelain=v1"]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    let index = fs::read(repo.root.join(".git/index")).unwrap();
    let note = fs::read(repo.root.join("CHANGELOG.md")).unwrap();
    success(&repo.check("pull_request", repo.event()));
    let mut event = repo.event();
    event["pull_request"]["title"] = json!("Agora o desenvolvimento fica mais simples.");
    success(&repo.check("pull_request", event));
    let mut event = repo.event();
    event["pull_request"]["title"] = json!("feat: válido\n\ncorpo");
    failure(&repo.check("pull_request", event), "única linha");
    assert_eq!(snapshot, repo.git(&["status", "--porcelain=v1"]));
    assert_eq!(head, repo.git(&["rev-parse", "HEAD"]));
    assert_eq!(index, fs::read(repo.root.join(".git/index")).unwrap());
    assert_eq!(note, fs::read(repo.root.join("CHANGELOG.md")).unwrap());
    repo.write("app.txt", "entrega ampliada\n");
    repo.commit("fix: ampliar entrega");
    failure(&repo.check("pull_request", repo.event()), "desatualizada");
    repo.note(None);
    success(&repo.check("pull_request", repo.event()));
}

#[test]
fn invalid_intermediate_commit_cannot_hide_behind_a_valid_head_or_release_label() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("mensagem inválida");
    repo.note(None);
    let mut event = repo.event();
    event["pull_request"]["labels"] = json!([{"name":"autorelease: pending"}]);
    failure(&repo.check("pull_request", event), "commit(s) inválido(s)");
}

#[test]
fn base_updates_require_rebase_and_changed_diff_requires_new_review() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: entregar mudança");
    repo.note(None);
    repo.git(&["checkout", "main"]);
    repo.write("other.txt", "base nova\n");
    repo.commit("fix: atualizar base");
    repo.git(&["checkout", "feature"]);
    failure(&repo.check("pull_request", repo.event()), "Faça rebase");
    repo.git(&["rebase", "main"]);
    success(&repo.check("pull_request", repo.event()));
    repo.write("app.txt", "resolução alterou resultado\n");
    repo.commit("fix: resolver conflito");
    failure(&repo.check("pull_request", repo.event()), "desatualizada");
}

#[test]
fn checkout_of_another_commit_and_merge_commits_are_rejected() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: entregar mudança");
    let event = repo.event();
    repo.git(&["checkout", "-b", "side", "main"]);
    repo.write("other.txt", "outra\n");
    repo.commit("fix: corrigir outra parte");
    repo.git(&["checkout", "feature"]);
    repo.git(&["merge", "--no-ff", "side", "-m", "chore: integrar branch"]);
    failure(&repo.check("pull_request", event), "HEAD real");
    failure(
        &repo.check("pull_request", repo.event()),
        "contém commits de merge",
    );
}

#[test]
fn versioned_context_is_used_and_missing_or_changed_context_does_not_pass() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: entregar mudança");
    fs::write(repo.dir.path().join("external.md"), "intenção").unwrap();
    repo.note(Some(repo.dir.path().join("external.md").to_str().unwrap()));
    failure(
        &repo.check("pull_request", repo.event()),
        "Contexto externo não está disponível",
    );
    repo.write(".changelog-context/42.md", "intenção\n");
    repo.commit("docs: incluir contexto reproduzível");
    repo.note(Some(".changelog-context/42.md"));
    success(&repo.check("pull_request", repo.event()));
    repo.write(".changelog-context/42.md", "intenção diferente\n");
    repo.commit("docs: esclarecer intenção");
    failure(&repo.check("pull_request", repo.event()), "desatualizada");
}

#[test]
fn editorial_fixes_preserve_versions_ids_fingerprints_and_valid_entries() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: entregar mudança");
    repo.note(None);
    let original = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    repo.git(&["branch", "-f", "main", "HEAD"]);
    repo.write(
        "CHANGELOG.md",
        &original.replace("Cada PR passa", "Cada entrega passa"),
    );
    repo.commit("docs: corrigir redação");
    success(&repo.check("pull_request", repo.event()));
    repo.write(
        "CHANGELOG.md",
        &original.replace("## [Não lançado]", "## [0.1.0]"),
    );
    repo.commit("chore: simular release");
    failure(
        &repo.check("pull_request", repo.event()),
        "correção editorial não pode alterar",
    );
    repo.write("CHANGELOG.md", &original.replace("(#42)", "(#43)"));
    repo.commit("docs: mudar identidade indevidamente");
    failure(&repo.check("pull_request", repo.event()), "marcadores");
    repo.write("CHANGELOG.md", EMPTY_LOG);
    repo.commit("docs: remover notas indevidamente");
    failure(
        &repo.check("pull_request", repo.event()),
        "correção editorial não pode alterar",
    );
}

#[test]
fn ordinary_pr_cannot_put_its_note_in_a_released_section_or_use_untracked_changelog() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: entregar mudança");
    repo.note(None);
    let document = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    repo.write(
        "CHANGELOG.md",
        &document.replace("## [Não lançado]", "## [0.1.0]"),
    );
    repo.commit("docs: mover nota antes da release");
    failure(
        &repo.check("pull_request", repo.event()),
        "seção Não lançado",
    );
    repo.git(&["rm", "--cached", "CHANGELOG.md"]);
    repo.git(&["commit", "-m", "docs: retirar changelog do histórico"]);
    failure(&repo.check("pull_request", repo.event()), "ausente no HEAD");
}

#[test]
fn malformed_events_and_shallow_repositories_fail_closed() {
    let repo = Repo::new();
    let mut event = repo.event();
    event["pull_request"]["head"]["sha"] = json!("--help");
    failure(&repo.check("pull_request", event), "SHA inválido");
    failure(&repo.check("pull_request", json!({})), "campo ausente");
    fs::write(repo.root.join(".git/shallow"), format!("{}\n", repo.base)).unwrap();
    failure(
        &repo.check("pull_request", repo.event()),
        "histórico completo",
    );
}

#[test]
fn impact_is_determined_by_every_commit_type_and_breaking_marker() {
    for message in [
        "feat: Entregar.",
        "fix: Corrigir.",
        "perf: Acelerar.",
        "docs!: Mudar contrato.",
        "refactor: Mudar contrato\n\nBREAKING CHANGE: usar o novo caminho.",
        "chore: Mudar contrato\n\nBREAKING-CHANGE: usar o novo caminho.",
    ] {
        let repo = Repo::new();
        repo.write("app.txt", "mudança\n");
        repo.commit(message);
        repo.commit("chore: Concluir revisão.");
        failure(
            &repo.check("pull_request", repo.event()),
            "entrada do PR está ausente",
        );
        repo.note(None);
        success(&repo.check("pull_request", repo.event()));
    }
}

#[test]
fn internal_prs_need_neither_changelog_nor_an_impact_declaration() {
    for kind in [
        "refactor", "docs", "test", "style", "ci", "build", "chore", "revert",
    ] {
        let repo = Repo::new();
        fs::remove_file(repo.root.join("CHANGELOG.md")).unwrap();
        repo.write("app.txt", "mudança interna\n");
        repo.commit(&format!("{kind}: Reorganizar internamente."));
        let mut event = repo.event();
        event["pull_request"]["title"] = json!("Reorganizar o projeto para facilitar a manutenção");
        event["pull_request"]["body"] = Value::Null;
        success(&repo.check("pull_request", event));
    }
}

#[test]
fn optional_notes_must_be_current_and_in_unreleased() {
    let repo = Repo::new();
    repo.write("app.txt", "organização\n");
    repo.commit("refactor: Reorganizar.");
    success(&repo.check("pull_request", repo.event()));
    repo.note(None);
    success(&repo.check("pull_request", repo.event()));
    repo.write("app.txt", "outra organização\n");
    repo.commit("refactor: Reorganizar novamente.");
    failure(&repo.check("pull_request", repo.event()), "desatualizada");
    repo.note(None);
    let document = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    repo.write(
        "CHANGELOG.md",
        &document.replace("## [Não lançado]", "## [0.1.0]"),
    );
    repo.commit("docs: Mover nota.");
    failure(
        &repo.check("pull_request", repo.event()),
        "seção Não lançado",
    );
}

#[test]
fn manual_notes_cover_binary_and_large_changes_without_ai() {
    let repo = Repo::new();
    fs::write(repo.root.join("image.bin"), [0, 255, 1]).unwrap();
    repo.write("large.txt", &"a".repeat(140 * 1024));
    repo.commit("feat: Entregar imagem e dados.");
    repo.note(None);
    success(&repo.check("pull_request", repo.event()));
    fs::write(repo.root.join("image.bin"), [0, 255, 2]).unwrap();
    repo.commit("chore: Atualizar imagem.");
    failure(&repo.check("pull_request", repo.event()), "desatualizada");
}

#[test]
fn editorial_exception_cannot_hide_impact_commits() {
    let repo = Repo::new();
    repo.write("app.txt", "entrega\n");
    repo.commit("feat: Entregar mudança.");
    repo.note(None);
    repo.git(&["branch", "-f", "main", "HEAD"]);
    let original = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    repo.write(
        "CHANGELOG.md",
        &original.replace("Cada PR passa", "Cada entrega passa"),
    );
    repo.commit("fix: Corrigir documentação pública.");
    let mut event = repo.event();
    event["number"] = json!(43);
    failure(
        &repo.check("pull_request", event),
        "entrada do PR está ausente",
    );
}

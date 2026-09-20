mod common;
use common::{Repo, failure, success};
use std::{
    fs,
    path::{Path, PathBuf},
};

const NOTE: &str = "### Entrega revisada\n\nResultado completo com as mudanças verificadas.\n";
const MANIFEST: &str = ".clean-dev-cycle-release.json";

fn project() -> (Repo, PathBuf) {
    let repo = Repo::new();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/example/project.git",
    ]);
    repo.write("clean-dev-cycle.toml", "[release]\nenabled = true\n");
    repo.write("code.txt", "first feature\n");
    let note = repo.directory.path().join("notes.md");
    fs::write(&note, NOTE).unwrap();
    (repo, note)
}

fn commit(repo: &Repo, note: &Path, message: &str) {
    success(
        &repo
            .cli(&[
                "commit",
                "--yes",
                "--message",
                message,
                "--entry-file",
                note.to_str().unwrap(),
            ])
            .output()
            .unwrap(),
    );
}

fn check(repo: &Repo, revision: &str) {
    success(
        &repo
            .cli(&["release", "check", "--revision", revision])
            .output()
            .unwrap(),
    );
}

#[test]
fn first_release_is_atomic_without_package_files_and_finishes_original_change() {
    let (repo, note) = project();
    let change = repo.revision("@", "change_id");
    commit(&repo, &note, "feat: primeira entrega");
    assert_eq!(repo.revision("@-", "change_id"), change);
    assert_eq!(repo.revision("@", "empty"), "true");
    let sha = repo.revision("@-", "commit_id");
    check(&repo, &sha);
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo.root.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(data["version"], "0.1.0");
    let files = repo.git(&["diff", "--name-only", &repo.base, &sha]);
    assert_eq!(
        files.lines().collect::<Vec<_>>(),
        [MANIFEST, "CHANGELOG.md", "clean-dev-cycle.toml", "code.txt"]
    );
    let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    assert!(log.contains("### Exemplo") && log.contains("## 0.1.0") && log.contains(NOTE.trim()));
    assert_eq!(repo.calls(), 0);
}

#[test]
fn preview_and_provider_failure_do_not_write_release_files() {
    let (repo, note) = project();
    let before = repo.revision("@", "commit_id");
    let output = repo
        .cli(&[
            "commit",
            "--dry-run",
            "--message",
            "feat: entrega",
            "--entry-file",
            note.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("0.1.0"));
    assert_eq!(repo.revision("@", "commit_id"), before);
    assert!(!repo.root.join(MANIFEST).exists());
    let output = repo
        .cli(&["commit", "--yes", "--message", "feat: entrega", "--codex"])
        .arg(common::fake_codex())
        .env("FAKE_MODE", "failure")
        .output()
        .unwrap();
    failure(&output, "Codex CLI falhou");
    assert_eq!(repo.revision("@", "commit_id"), before);
}

#[test]
fn pending_stack_versions_advance_but_repreparation_does_not() {
    let (repo, note) = project();
    commit(&repo, &note, "feat: inicial");
    repo.write("code.txt", "second feature\n");
    commit(&repo, &note, "feat: segunda");
    assert!(
        fs::read_to_string(repo.root.join("CHANGELOG.md"))
            .unwrap()
            .contains("## 0.2.0")
    );
    let second = repo.revision("@-", "change_id");
    repo.jj(&["edit", &second]);
    repo.write("code.txt", "second feature improved\n");
    commit(&repo, &note, "feat: segunda");
    let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    assert_eq!(log.matches("## 0.2.0").count(), 1);
    assert!(!log.contains("0.3.0"));
    check(&repo, &repo.revision("@-", "commit_id"));
    repo.write("code.txt", "fix second feature\n");
    commit(&repo, &note, "fix: corrigir");
    assert!(
        fs::read_to_string(repo.root.join("CHANGELOG.md"))
            .unwrap()
            .contains("## 0.2.1")
    );
}

#[test]
fn checks_reject_code_note_version_and_rebase_tampering() {
    for field in ["code", "note", "version", "base"] {
        let (repo, note) = project();
        commit(&repo, &note, "feat: inicial");
        let change = repo.revision("@-", "change_id");
        repo.jj(&["edit", &change]);
        match field {
            "code" => repo.write("code.txt", "unreviewed\n"),
            "note" => {
                let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
                repo.write(
                    "CHANGELOG.md",
                    log.replace("Resultado completo", "Texto diferente"),
                );
            }
            "version" | "base" => {
                let mut metadata: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(repo.root.join(MANIFEST)).unwrap())
                        .unwrap();
                if field == "version" {
                    metadata["version"] = "0.9.0".into();
                } else {
                    metadata["parent"] = "0000000000000000000000000000000000000000".into();
                }
                repo.write(MANIFEST, metadata.to_string());
            }
            _ => unreachable!(),
        }
        let sha = repo.revision("@", "commit_id");
        assert!(
            !repo
                .cli(&["release", "check", "--revision", &sha])
                .output()
                .unwrap()
                .status
                .success(),
            "{field}"
        );
    }
}

#[test]
fn internal_changes_do_not_release_and_manual_eligible_commit_requires_preparation() {
    let (repo, note) = project();
    commit(&repo, &note, "feat: inicial");
    repo.write("docs.txt", "internal\n");
    success(
        &repo
            .cli(&["commit", "--yes", "--message", "docs: orientar"])
            .output()
            .unwrap(),
    );
    check(&repo, &repo.revision("@-", "commit_id"));
    repo.write("code.txt", "unprepared\n");
    repo.jj(&["commit", "-m", "fix: manual"]);
    failure(
        &repo
            .cli(&[
                "release",
                "check",
                "--revision",
                &repo.revision("@-", "commit_id"),
            ])
            .output()
            .unwrap(),
        "release não preparada",
    );
}

#[test]
fn ai_uses_published_range_preserving_dependency_changes_and_excluding_generated_notes() {
    let (repo, note) = project();
    commit(&repo, &note, "feat: inicial");
    let base = repo.revision("@-", "commit_id");
    let list = repo.directory.path().join("releases.json");
    fs::write(
        &list,
        r#"[{"tag_name":"v0.1.0","draft":false,"prerelease":false}]"#,
    )
    .unwrap();
    repo.write("deps.txt", "library = 2\n");
    repo.write("code.txt", "next feature\n");
    let output = repo
        .cli(&[
            "commit",
            "--yes",
            "--message",
            "feat: dependência nova",
            "--codex",
        ])
        .arg(common::fake_codex())
        .env("FAKE_GH_RELEASES", list)
        .env("FAKE_GH_SHA", &base)
        .output()
        .unwrap();
    success(&output);
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("library = 2") && prompt.contains("next feature"));
    assert!(!prompt.contains("Resultado completo") && !prompt.contains("notes_hash"));
    check(&repo, &repo.revision("@-", "commit_id"));
}

#[test]
fn concurrent_edits_during_release_generation_preserve_workspace_and_abort() {
    let (repo, _) = project();
    let output = repo
        .cli(&["commit", "--yes", "--message", "feat: entrega", "--codex"])
        .arg(common::fake_codex())
        .env("FAKE_MODE", "mutate")
        .output()
        .unwrap();
    failure(&output, "mudou durante a revisão");
    assert!(repo.root.join("outra.txt").exists());
    assert!(!repo.root.join(MANIFEST).exists());
    assert_eq!(repo.revision("@", "description"), "");
}

#[test]
fn rebase_requires_and_accepts_fresh_preparation() {
    let (repo, note) = project();
    commit(&repo, &note, "feat: inicial");
    let change = repo.revision("@-", "change_id");
    repo.jj(&["new", &repo.base]);
    repo.write("docs.txt", "nova base\n");
    repo.jj(&["commit", "-m", "docs: nova base"]);
    let new_base = repo.revision("@-", "commit_id");
    repo.jj(&["rebase", "-r", &change, "-o", &new_base]);
    let rebased = repo.revision(&change, "commit_id");
    failure(
        &repo
            .cli(&["release", "check", "--revision", &rebased])
            .output()
            .unwrap(),
        "desatualizada",
    );
    repo.jj(&["edit", &change]);
    commit(&repo, &note, "feat: inicial");
    check(&repo, &repo.revision("@-", "commit_id"));
    assert_eq!(
        fs::read_to_string(repo.root.join("CHANGELOG.md"))
            .unwrap()
            .matches("## 0.1.0")
            .count(),
        1
    );
}

#[test]
fn binary_release_uses_manual_notes_and_publication_requires_ci_context() {
    let (repo, note) = project();
    repo.write("data.bin", [0, 1, 2]);
    commit(&repo, &note, "feat: dados binários");
    let sha = repo.revision("@-", "commit_id");
    check(&repo, &sha);
    failure(
        &repo
            .cli(&["release", "publish", "--revision", &sha])
            .env_remove("GITHUB_ACTIONS")
            .output()
            .unwrap(),
        "GitHub Actions",
    );
    assert_eq!(repo.calls(), 0);
}

#[test]
fn initial_version_starts_the_first_release_and_invalid_settings_are_rejected() {
    let (repo, note) = project();
    repo.write(
        "clean-dev-cycle.toml",
        "[release]\nenabled = true\ninitial_version = \"1.2.0\"\n",
    );
    commit(&repo, &note, "feat: inicial");
    let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    assert!(log.contains("## 1.2.0"));
    check(&repo, &repo.revision("@-", "commit_id"));
    repo.write("code.txt", "second feature\n");
    commit(&repo, &note, "feat: segunda");
    let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    assert!(log.contains("## 1.3.0"));
    check(&repo, &repo.revision("@-", "commit_id"));
    for (config, expected) in [
        (
            "[release]\nenabled = true\ninitial_version = \"1.0.0-rc1\"\n",
            "initial_version",
        ),
        (
            "[release]\nenabled = true\ninitial_version = \"0.0.1\"\n",
            "initial_version",
        ),
        (
            "[release]\nenabled = true\ninital_version = \"1.0.0\"\n",
            "chave desconhecida",
        ),
    ] {
        let (repo, note) = project();
        repo.write("clean-dev-cycle.toml", config);
        let output = repo
            .cli(&[
                "commit",
                "--yes",
                "--message",
                "feat: inválida",
                "--entry-file",
                note.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        failure(&output, expected);
        repo.jj(&["commit", "-m", "feat: manual"]);
        failure(
            &repo
                .cli(&[
                    "release",
                    "check",
                    "--revision",
                    &repo.revision("@-", "commit_id"),
                ])
                .output()
                .unwrap(),
            expected,
        );
    }
}

#[test]
fn published_tags_without_a_manifest_continue_the_version_sequence() {
    let (repo, note) = project();
    let list = repo.directory.path().join("releases.json");
    fs::write(
        &list,
        r#"[{"tag_name":"v0.4.0","draft":false,"prerelease":false}]"#,
    )
    .unwrap();
    let output = repo
        .cli(&[
            "commit",
            "--yes",
            "--message",
            "feat: continuar",
            "--entry-file",
            note.to_str().unwrap(),
        ])
        .env("FAKE_GH_RELEASES", list)
        .env("FAKE_GH_SHA", &repo.base)
        .output()
        .unwrap();
    success(&output);
    let data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(repo.root.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(data["version"], "0.5.0");
    assert_eq!(data["base_tag"], "v0.4.0");
    check(&repo, &repo.revision("@-", "commit_id"));
}

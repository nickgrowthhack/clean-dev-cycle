mod common;
use common::{Repo, failure, success};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};
use tempfile::TempDir;

const NOTE: &str = "### Entrega revisada\n\nResultado completo com as mudanças verificadas.\n";
const MANIFEST: &str = ".clean-dev-cycle-release.json";

fn gh_directory() -> &'static Path {
    static DIRECTORY: OnceLock<TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            success(
                &Command::new("rustc")
                    .args(["--edition=2024", "--crate-name", "fake_gh"])
                    .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_gh.rs"))
                    .arg("-o")
                    .arg(
                        dir.path()
                            .join(format!("gh{}", std::env::consts::EXE_SUFFIX)),
                    )
                    .output()
                    .unwrap(),
            );
            dir
        })
        .path()
}

fn command(repo: &Repo, args: &[&str]) -> Command {
    let mut paths = vec![gh_directory().to_owned()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut cmd = repo.cli(args);
    cmd.env("PATH", std::env::join_paths(paths).unwrap());
    cmd
}

fn project() -> (Repo, PathBuf) {
    let repo = Repo::new();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/example/project.git",
    ]);
    repo.write("Cargo.toml", "# keep this comment\n[package]\nname = 'example'\nversion = '0.1.0' # version comment\n\n[dependencies]\nserde = '1'\n");
    repo.write("Cargo.lock", "# keep lock comment\nversion = 4\n\n[[package]]\nname = 'example'\nversion = '0.1.0'\ndependencies = ['serde']\n\n[[package]]\nname = 'serde'\nversion = '1.0.0'\nsource = 'registry+https://example.com'\n");
    repo.write("clean-dev-cycle.toml", "[release]\nenabled = true\n");
    repo.write("code.txt", "first feature\n");
    let note = repo.directory.path().join("notes.md");
    fs::write(&note, NOTE).unwrap();
    (repo, note)
}

fn commit(repo: &Repo, note: &Path, message: &str) {
    success(
        &command(
            repo,
            &[
                "commit",
                "--yes",
                "--message",
                message,
                "--entry-file",
                note.to_str().unwrap(),
            ],
        )
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
fn first_release_is_atomic_preserves_comments_and_finishes_original_change() {
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
    let cargo = fs::read_to_string(repo.root.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("# keep this comment")
            && cargo.contains("# version comment")
            && cargo.contains("serde = '1'")
    );
    let log = fs::read_to_string(repo.root.join("CHANGELOG.md")).unwrap();
    assert!(log.contains("### Exemplo") && log.contains("## 0.1.0") && log.contains(NOTE.trim()));
    assert_eq!(repo.calls(), 0);
}

#[test]
fn preview_and_provider_failure_do_not_write_release_files() {
    let (repo, note) = project();
    let before = repo.revision("@", "commit_id");
    let cargo = fs::read(repo.root.join("Cargo.toml")).unwrap();
    let output = command(
        &repo,
        &[
            "commit",
            "--dry-run",
            "--message",
            "feat: entrega",
            "--entry-file",
            note.to_str().unwrap(),
        ],
    )
    .output()
    .unwrap();
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("0.1.0"));
    assert_eq!(repo.revision("@", "commit_id"), before);
    assert_eq!(fs::read(repo.root.join("Cargo.toml")).unwrap(), cargo);
    assert!(!repo.root.join(MANIFEST).exists());
    let output = command(
        &repo,
        &["commit", "--yes", "--message", "feat: entrega", "--codex"],
    )
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
            "version" => {
                let cargo = fs::read_to_string(repo.root.join("Cargo.toml")).unwrap();
                repo.write("Cargo.toml", cargo.replace("0.1.0", "0.9.0"));
            }
            "base" => {
                let mut metadata: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(repo.root.join(MANIFEST)).unwrap())
                        .unwrap();
                metadata["parent"] = "0000000000000000000000000000000000000000".into();
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
        &command(&repo, &["commit", "--yes", "--message", "docs: orientar"])
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
    let cargo = fs::read_to_string(repo.root.join("Cargo.toml")).unwrap();
    repo.write("Cargo.toml", cargo.replace("serde = '1'", "serde = '2'"));
    repo.write("code.txt", "next feature\n");
    let output = command(
        &repo,
        &[
            "commit",
            "--yes",
            "--message",
            "feat: dependência nova",
            "--codex",
        ],
    )
    .arg(common::fake_codex())
    .env("FAKE_GH_RELEASES", list)
    .env("FAKE_GH_SHA", &base)
    .output()
    .unwrap();
    success(&output);
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("serde = '2'") && prompt.contains("next feature"));
    assert!(!prompt.contains("Resultado completo") && !prompt.contains("notes_hash"));
    check(&repo, &repo.revision("@-", "commit_id"));
}

#[test]
fn concurrent_edits_during_release_generation_preserve_workspace_and_abort() {
    let (repo, _) = project();
    let output = command(
        &repo,
        &["commit", "--yes", "--message", "feat: entrega", "--codex"],
    )
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

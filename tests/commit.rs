mod common;
use common::{Repo, failure, success};
use std::fs;

const MESSAGE: &str = "feat(cli): implementar geração de commits";

#[test]
fn dry_run_reviews_current_jj_change_without_finishing_it() {
    let repo = Repo::new();
    repo.write("arquivo.txt", "mudança atual\n");
    let before = repo.revision("@", "commit_id");
    success(&repo.ai(&["commit", "--dry-run"], "valid"));
    assert_eq!(repo.revision("@", "commit_id"), before);
    assert_eq!(repo.revision("@", "description"), "");
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("mudança atual"));
    let args = fs::read_to_string(repo.directory.path().join("calls.args")).unwrap();
    assert!(args.contains("modelo-configurado"));
}

#[test]
fn confirmation_finishes_the_reviewed_change_and_opens_an_empty_one() {
    let repo = Repo::new();
    repo.write("arquivo.txt", "conteúdo\n");
    let change = repo.revision("@", "change_id");
    success(&repo.ai(&["commit", "--yes"], "valid"));
    assert_eq!(repo.revision("@-", "change_id"), change);
    assert_eq!(repo.revision("@-", "description").trim(), MESSAGE);
    assert_eq!(repo.revision("@", "empty"), "true");
    assert_eq!(
        repo.git(&[
            "show",
            &format!("{}:arquivo.txt", repo.revision("@-", "commit_id"))
        ]),
        "conteúdo\n"
    );
}

#[test]
fn missing_identity_is_rejected_before_ai_without_finishing_the_change() {
    for variable in ["JJ_USER", "JJ_EMAIL"] {
        let repo = Repo::new();
        repo.write("arquivo.txt", "preservar\n");
        let before = repo.revision("@", "commit_id");
        let output = repo
            .cli(&["commit", "--yes", "--codex"])
            .arg(common::fake_codex())
            .env(variable, "")
            .output()
            .unwrap();
        failure(&output, "identidade do Jujutsu incompleta");
        assert_eq!(repo.calls(), 0);
        assert_eq!(repo.revision("@", "commit_id"), before);
        assert_eq!(
            fs::read_to_string(repo.root.join("arquivo.txt")).unwrap(),
            "preservar\n"
        );
    }
}

#[test]
fn configuring_identity_does_not_silently_reattribute_an_existing_change() {
    let repo = Repo::new();
    repo.write("arquivo.txt", "preservar\n");
    repo.jj(&["--config", "user.name=''", "metaedit", "--update-author"]);
    let before = repo.revision("@", "commit_id");
    failure(&repo.ai(&["commit", "--yes"], "valid"), "autor incompleto");
    assert_eq!(repo.calls(), 0);
    assert_eq!(repo.revision("@", "commit_id"), before);
    repo.jj(&["metaedit", "--update-author"]);
    success(&repo.ai(&["commit", "--yes"], "valid"));
    assert_eq!(repo.revision("@-", "author.name()"), "Teste");
    assert_eq!(
        repo.revision("@-", "committer.email()"),
        "teste@example.com"
    );
}

#[test]
fn jj_split_keeps_other_layers_out_of_the_prompt() {
    let repo = Repo::new();
    repo.write("primeira.txt", "CAMADA_ANTERIOR\n");
    repo.write("segunda.txt", "CAMADA_ATUAL\n");
    repo.jj(&["split", "primeira.txt", "-m", "feat: primeira camada"]);
    success(&repo.ai(&["commit", "--yes"], "valid"));
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("CAMADA_ATUAL"));
    assert!(!prompt.contains("CAMADA_ANTERIOR"));
    assert_eq!(
        repo.revision("@--", "description").trim(),
        "feat: primeira camada"
    );
}

#[test]
fn concurrent_working_copy_edits_abort_without_describing_the_change() {
    let repo = Repo::new();
    repo.write("arquivo.txt", "original\n");
    let change = repo.revision("@", "change_id");
    failure(
        &repo.ai(&["commit", "--yes"], "mutate"),
        "mudou durante a revisão",
    );
    assert_eq!(repo.revision("@", "change_id"), change);
    assert_eq!(repo.revision("@", "description"), "");
    assert_eq!(
        fs::read_to_string(repo.root.join("arquivo.txt")).unwrap(),
        "original\n"
    );
    assert!(repo.root.join("outra.txt").exists());
}

#[test]
fn provider_errors_never_finish_or_discard_the_change() {
    for mode in [
        "failure",
        "needs-context",
        "invalid",
        "malformed",
        "tool",
        "overflow",
        "timeout",
    ] {
        let repo = Repo::new();
        repo.write("arquivo.txt", "preservar\n");
        let change = repo.revision("@", "change_id");
        let output = repo.ai(&["commit", "--yes", "--timeout", "1"], mode);
        assert!(!output.status.success(), "{mode}");
        assert_eq!(repo.revision("@", "change_id"), change);
        assert_eq!(repo.revision("@", "description"), "");
        assert_eq!(
            fs::read_to_string(repo.root.join("arquivo.txt")).unwrap(),
            "preservar\n"
        );
    }
}

#[test]
fn invalid_generation_gets_only_one_correction_attempt() {
    let repo = Repo::new();
    repo.write("arquivo.txt", "novo\n");
    success(&repo.ai(&["commit", "--yes"], "retry"));
    assert_eq!(repo.calls(), 2);
}

#[test]
fn sensitive_binary_non_utf8_and_large_diffs_never_reach_ai() {
    for (name, content) in [
        (".env", b"TOKEN=secret".to_vec()),
        ("data.bin", vec![0, 1, 2]),
        ("texto.txt", vec![0xff, 0xfe]),
        ("grande.txt", vec![b'x'; 130 * 1024]),
    ] {
        let repo = Repo::new();
        repo.write(name, content);
        assert!(!repo.ai(&["commit", "--yes"], "valid").status.success());
        assert_eq!(repo.calls(), 0);
        assert_eq!(repo.revision("@", "description"), "");
    }
}

#[test]
fn empty_change_and_noninteractive_confirmation_fail_without_ai() {
    let repo = Repo::new();
    failure(&repo.ai(&["commit", "--yes"], "valid"), "não há alterações");
    repo.write("arquivo.txt", "novo");
    failure(&repo.ai(&["commit"], "valid"), "terminal");
    assert_eq!(repo.calls(), 0);
}

#[test]
fn jj_conflicts_are_rejected_before_ai() {
    let repo = Repo::new();
    repo.write("file", "base\n");
    repo.jj(&["commit", "-m", "feat: base"]);
    let base = repo.revision("@-", "commit_id");
    repo.write("file", "left\n");
    repo.jj(&["commit", "-m", "feat: left"]);
    let left = repo.revision("@-", "commit_id");
    repo.jj(&["new", &base]);
    repo.write("file", "right\n");
    repo.jj(&["commit", "-m", "feat: right"]);
    let right = repo.revision("@-", "commit_id");
    repo.jj(&["new", &left, &right]);
    failure(&repo.ai(&["commit", "--yes"], "valid"), "conflitos");
    assert_eq!(repo.calls(), 0);
}

#[test]
fn pinned_operation_does_not_capture_late_edits() {
    let repo = Repo::new();
    repo.write("file", "reviewed\n");
    repo.jj(&["status"]);
    let op = repo.jj(&["op", "log", "--limit", "1", "--no-graph", "-T", "id"]);
    repo.write("file", "late edit\n");
    repo.jj(&["--at-operation", &op, "commit", "-m", "feat: reviewed"]);
    repo.jj(&["status"]);
    assert_eq!(repo.jj(&["file", "show", "-r", "@-", "file"]), "reviewed\n");
    assert_eq!(repo.jj(&["file", "show", "-r", "@", "file"]), "late edit\n");
}

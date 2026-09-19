mod common;
use common::{Repo, failure, success};
use std::fs;

#[test]
fn generated_note_uses_the_complete_range_and_only_writes_stdout() {
    let repo = Repo::new();
    let original = fs::read(repo.root.join("CHANGELOG.md")).unwrap();
    for name in ["first", "second"] {
        repo.write(name, name);
        repo.jj(&["commit", "-m", &format!("feat: {name}")]);
    }
    let to = repo.revision("@-", "commit_id");
    let before = repo.revision("@", "commit_id");
    let output = repo.ai(&["changelog", "--from", &repo.base, "--to", &to], "valid");
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("### "));
    let prompt = fs::read_to_string(repo.directory.path().join("calls.prompt")).unwrap();
    assert!(prompt.contains("+first") && prompt.contains("+second"));
    assert_eq!(fs::read(repo.root.join("CHANGELOG.md")).unwrap(), original);
    assert_eq!(repo.revision("@", "commit_id"), before);
}

#[test]
fn manual_note_supports_binary_changes_without_ai() {
    let repo = Repo::new();
    repo.write("data.bin", [0, 1, 2]);
    repo.jj(&["commit", "-m", "feat: binary"]);
    let to = repo.revision("@-", "commit_id");
    let note = repo.directory.path().join("note.md");
    fs::write(&note, "### Dados\n\nNovo formato binário.\n").unwrap();
    let output = repo
        .cli(&[
            "changelog",
            "--from",
            &repo.base,
            "--to",
            &to,
            "--entry-file",
            note.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    success(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "### Dados\n\nNovo formato binário.\n"
    );
    assert_eq!(repo.calls(), 0);
}

#[test]
fn empty_or_reversed_ranges_and_moving_refs_are_rejected() {
    let repo = Repo::new();
    repo.write("file", "first");
    repo.jj(&["commit", "-m", "feat: first"]);
    let to = repo.revision("@-", "commit_id");
    failure(
        &repo.ai(&["changelog", "--from", &to, "--to", &to], "valid"),
        "não há alterações",
    );
    failure(
        &repo.ai(&["changelog", "--from", &to, "--to", &repo.base], "valid"),
        "ancestral",
    );
    failure(
        &repo.ai(&["changelog", "--from", "main", "--to", &to], "mutate-base"),
        "referência mudou",
    );
}

#[test]
fn invalid_manual_note_does_not_modify_history() {
    let repo = Repo::new();
    repo.write("file", "first");
    repo.jj(&["commit", "-m", "feat: first"]);
    let to = repo.revision("@-", "commit_id");
    let note = repo.directory.path().join("note.md");
    fs::write(&note, "invalid note").unwrap();
    assert!(
        !repo
            .cli(&[
                "changelog",
                "--from",
                &repo.base,
                "--to",
                &to,
                "--entry-file",
                note.to_str().unwrap()
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(repo.revision("@-", "commit_id"), to);
}

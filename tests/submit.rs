mod common;
use common::{Repo, failure, success};

#[test]
fn two_layers_land_individually_and_resubmission_is_a_noop() {
    let repo = Repo::new();
    let remote = repo.remote();
    repo.write("first", "first");
    repo.jj(&["commit", "-m", "feat: first"]);
    let first = repo.revision("@-", "commit_id");
    repo.write("second", "second");
    repo.jj(&["commit", "-m", "feat: second"]);
    let second = repo.revision("@-", "commit_id");
    failure(&repo.cli(&["submit"]).output().unwrap(), "primeira camada");
    success(
        &repo
            .cli(&["submit", "--revision", &first])
            .output()
            .unwrap(),
    );
    assert_eq!(repo.remote_ref(&remote, "nick/submit"), first);
    assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
    repo.git(&[
        "--git-dir",
        remote.to_str().unwrap(),
        "update-ref",
        "refs/heads/main",
        &first,
        &repo.base,
    ]);
    success(&repo.cli(&["submit"]).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "nick/submit"), second);
    repo.git(&[
        "--git-dir",
        remote.to_str().unwrap(),
        "update-ref",
        "refs/heads/main",
        &second,
        &first,
    ]);
    let output = repo.cli(&["submit"]).output().unwrap();
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("já integrada"));
    assert_eq!(repo.remote_ref(&remote, "main"), second);
    assert_eq!(repo.revision("@", "empty"), "true");
}

#[test]
fn invalid_message_and_empty_change_cannot_be_submitted() {
    for (message, has_file) in [("invalid", true), ("chore: empty", false)] {
        let repo = Repo::new();
        let remote = repo.remote();
        if has_file {
            repo.write("file", "value");
        }
        repo.jj(&["commit", "-m", message]);
        assert!(!repo.cli(&["submit"]).output().unwrap().status.success());
        assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
        assert!(
            repo.git(&[
                "--git-dir",
                remote.to_str().unwrap(),
                "for-each-ref",
                "refs/heads/nick/submit"
            ])
            .is_empty()
        );
    }
}

#[test]
fn advanced_main_requires_rebase_without_overwriting_remote_work() {
    let repo = Repo::new();
    let remote = repo.remote();
    repo.write("first", "candidate");
    repo.jj(&["commit", "-m", "feat: candidate"]);
    let candidate = repo.revision("@-", "commit_id");
    repo.jj(&["new", &repo.base]);
    repo.write("another", "other work");
    repo.jj(&["commit", "-m", "feat: other work"]);
    let other = repo.revision("@-", "commit_id");
    repo.git(&[
        "--git-dir",
        remote.to_str().unwrap(),
        "fetch",
        repo.root.to_str().unwrap(),
        &other,
    ]);
    repo.git(&[
        "--git-dir",
        remote.to_str().unwrap(),
        "update-ref",
        "refs/heads/main",
        &other,
        &repo.base,
    ]);
    failure(
        &repo
            .cli(&["submit", "--revision", &candidate])
            .output()
            .unwrap(),
        "filha direta",
    );
    assert_eq!(repo.remote_ref(&remote, "main"), other);
}

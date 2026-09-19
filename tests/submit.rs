mod common;
use common::{Repo, failure, fake_checks, success};
use std::{fs, process::Command};

fn configure(repo: &Repo, modes: &[&str]) {
    let program = fake_checks().to_str().unwrap().to_owned();
    let commands: Vec<_> = modes
        .iter()
        .map(|mode| vec![program.clone(), (*mode).into()])
        .collect();
    repo.write(
        "clean-dev-cycle.toml",
        format!(
            "[checks]\ncommands = {}\n",
            serde_json::to_string(&commands).unwrap()
        ),
    );
}

fn checked_repo() -> Repo {
    let mut repo = Repo::new();
    configure(&repo, &["ok"]);
    repo.jj(&["commit", "-m", "chore: configure checks"]);
    repo.base = repo.revision("@-", "commit_id");
    repo.jj(&["bookmark", "set", "main", "-r", "@-"]);
    repo
}

fn submit(repo: &Repo) -> Command {
    let mut command = repo.cli(&["submit"]);
    command.env("CHECK_LOG", repo.directory.path().join("checks.log"));
    command
}

fn check_paths(repo: &Repo) -> Vec<String> {
    fs::read_to_string(repo.directory.path().join("checks.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn two_layers_land_individually_and_resubmission_is_a_noop() {
    let repo = checked_repo();
    let remote = repo.remote();
    repo.write("first", "first");
    repo.jj(&["commit", "-m", "feat: first"]);
    let first = repo.revision("@-", "commit_id");
    repo.write("second", "second");
    repo.jj(&["commit", "-m", "feat: second"]);
    let second = repo.revision("@-", "commit_id");
    failure(&submit(&repo).output().unwrap(), "primeira camada");
    success(&submit(&repo).args(["--revision", &first]).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "main"), first);
    success(&submit(&repo).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "main"), second);
    let output = submit(&repo).output().unwrap();
    success(&output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("já integrada"));
    assert_eq!(check_paths(&repo).len(), 2);
    assert_eq!(
        repo.git(&[
            "--git-dir",
            remote.to_str().unwrap(),
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/"
        ])
        .trim(),
        "refs/heads/main"
    );
    assert_eq!(
        repo.git(&["for-each-ref", "--format=%(refname)", "refs/heads/"])
            .trim(),
        "refs/heads/main"
    );
    assert_eq!(repo.revision("@", "empty"), "true");
}

#[test]
fn invalid_message_and_empty_change_cannot_be_submitted() {
    for (message, has_file) in [("invalid", true), ("chore: empty", false)] {
        let repo = checked_repo();
        let remote = repo.remote();
        if has_file {
            repo.write("file", "value");
        }
        repo.jj(&["commit", "-m", message]);
        assert!(!submit(&repo).output().unwrap().status.success());
        assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
        assert!(check_paths(&repo).is_empty());
    }
}

#[test]
fn missing_committer_is_reported_before_checks_or_moving_main() {
    let repo = checked_repo();
    let remote = repo.remote();
    repo.write("file", "preservar\n");
    repo.jj(&[
        "--config",
        "user.email=''",
        "commit",
        "-m",
        "docs: preservar",
    ]);
    let before = repo.revision("@-", "commit_id");
    failure(&submit(&repo).output().unwrap(), "committer incompleta");
    assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
    assert_eq!(repo.revision("main", "commit_id"), repo.base);
    assert!(check_paths(&repo).is_empty());
    repo.jj(&["metaedit", "-r", "@-", "--force-rewrite"]);
    let repaired = repo.revision("@-", "commit_id");
    assert_eq!(repo.git(&["diff", &before, &repaired]), "");
    success(&submit(&repo).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "main"), repaired);
}

#[test]
fn checks_use_committed_configuration_and_preserve_next_change() {
    let repo = checked_repo();
    let remote = repo.remote();
    repo.write("file", "candidate");
    repo.jj(&["commit", "-m", "feat: candidate"]);
    let candidate = repo.revision("@-", "commit_id");
    repo.write("pending", "not ready");
    repo.write("clean-dev-cycle.toml", "invalid working copy configuration");
    success(&submit(&repo).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "main"), candidate);
    assert_eq!(
        fs::read_to_string(repo.root.join("pending")).unwrap(),
        "not ready"
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("clean-dev-cycle.toml")).unwrap(),
        "invalid working copy configuration"
    );
    assert!(
        check_paths(&repo)
            .iter()
            .all(|p| !std::path::Path::new(p).exists())
    );
}

#[test]
fn failed_or_mutating_checks_stop_before_publication_and_clean_up() {
    for mode in ["fail", "modify", "head"] {
        let repo = checked_repo();
        let remote = repo.remote();
        configure(&repo, &[mode, "ok"]);
        repo.jj(&["commit", "-m", "chore: failing checks"]);
        let output = submit(&repo).output().unwrap();
        failure(
            &output,
            if mode == "fail" {
                "falhou"
            } else {
                "um check alterou"
            },
        );
        assert_eq!(check_paths(&repo).len(), 1);
        assert!(
            check_paths(&repo)
                .iter()
                .all(|p| !std::path::Path::new(p).exists())
        );
        assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
        assert_eq!(repo.revision("main", "commit_id"), repo.base);
    }
}

#[test]
fn absent_invalid_empty_configuration_and_missing_program_prevent_push() {
    for config in [
        None,
        Some("not toml"),
        Some("[checks]\ncommands=[]"),
        Some("[checks]\ncommands=[['clean-dev-cycle-missing-executable']]"),
    ] {
        let repo = checked_repo();
        let remote = repo.remote();
        match config {
            Some(text) => repo.write("clean-dev-cycle.toml", text),
            None => fs::remove_file(repo.root.join("clean-dev-cycle.toml")).unwrap(),
        }
        repo.jj(&["commit", "-m", "chore: configure"]);
        assert!(!submit(&repo).output().unwrap().status.success());
        assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
        assert_eq!(repo.revision("main", "commit_id"), repo.base);
    }
}

#[test]
fn rewritten_revision_during_checks_is_not_published() {
    let repo = checked_repo();
    let remote = repo.remote();
    configure(&repo, &["rewrite"]);
    repo.jj(&["commit", "-m", "feat: candidate"]);
    let change = repo.revision("@-", "change_id");
    failure(
        &submit(&repo)
            .env("CHECK_REVISION", &change)
            .output()
            .unwrap(),
        "reescrita",
    );
    assert_eq!(repo.remote_ref(&remote, "main"), repo.base);
}

#[test]
fn advanced_main_before_or_during_checks_is_not_overwritten() {
    for during in [false, true] {
        let repo = checked_repo();
        let remote = repo.remote();
        configure(&repo, &["advance"]);
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
        if !during {
            repo.git(&[
                "--git-dir",
                remote.to_str().unwrap(),
                "update-ref",
                "refs/heads/main",
                &other,
                &repo.base,
            ]);
        }
        failure(
            &submit(&repo)
                .args(["--revision", &candidate])
                .env("CHECK_REMOTE", &remote)
                .env("CHECK_ADVANCE_SHA", &other)
                .output()
                .unwrap(),
            if during {
                "avançou durante"
            } else {
                "filha direta"
            },
        );
        assert_eq!(repo.remote_ref(&remote, "main"), other);
    }
}

#[test]
fn commands_receive_literal_arguments_without_a_shell() {
    let repo = checked_repo();
    let remote = repo.remote();
    let program = fake_checks().to_str().unwrap().to_owned();
    repo.write(
        "clean-dev-cycle.toml",
        format!(
            "[checks]\ncommands={}\n",
            serde_json::to_string(&vec![vec![
                program.as_str(),
                "arguments",
                "two words",
                "$HOME",
                "",
                "a;b",
                "$(literal)"
            ]])
            .unwrap()
        ),
    );
    repo.jj(&["commit", "-m", "chore: literal arguments"]);
    let candidate = repo.revision("@-", "commit_id");
    success(&submit(&repo).output().unwrap());
    assert_eq!(repo.remote_ref(&remote, "main"), candidate);
}

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct Fixture {
    directory: tempfile::TempDir,
    base: String,
    candidate: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git(root, &["init", "--bare", "remote.git"]);
        git(root, &["init", "--initial-branch=main"]);
        git(root, &["config", "user.name", "Test"]);
        git(root, &["config", "user.email", "test@example.com"]);
        git(root, &["remote", "add", "origin", "remote.git"]);
        fs::write(root.join("file"), "base").unwrap();
        git(root, &["add", "file"]);
        git(root, &["commit", "-m", "feat: base"]);
        let base = git(root, &["rev-parse", "HEAD"]);
        git(root, &["push", "origin", "main"]);
        fs::write(root.join("file"), "candidate").unwrap();
        git(root, &["commit", "-am", "fix: candidate"]);
        let candidate = git(root, &["rev-parse", "HEAD"]);
        git(root, &["push", "origin", "HEAD:refs/heads/nick/submit"]);
        Self {
            directory,
            base,
            candidate,
        }
    }

    fn promote(&self) -> Output {
        self.script("promote.ps1")
    }

    fn script(&self, name: &str) -> Output {
        Command::new("pwsh")
            .current_dir(self.directory.path())
            .args(["-NoProfile", "-File"])
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(".github/scripts")
                    .join(name),
            )
            .args(["-Base", &self.base, "-Candidate", &self.candidate])
            .output()
            .unwrap()
    }

    fn remote_main(&self) -> String {
        git(
            self.directory.path(),
            &["--git-dir=remote.git", "rev-parse", "main"],
        )
    }
}

#[test]
fn promotion_preserves_sha_and_is_idempotent() {
    let fixture = Fixture::new();
    assert!(fixture.script("check-candidate.ps1").status.success());
    for _ in 0..2 {
        let output = fixture.promote();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fixture.remote_main(), fixture.candidate);
    }
}

#[test]
fn replaced_submission_and_advanced_main_are_not_promoted() {
    for reference in ["nick/submit", "main"] {
        let fixture = Fixture::new();
        let root = fixture.directory.path();
        git(root, &["checkout", "--detach", &fixture.base]);
        fs::write(root.join("file"), "another change").unwrap();
        git(root, &["commit", "-am", "fix: another change"]);
        git(
            root,
            &[
                "push",
                "--force",
                "origin",
                &format!("HEAD:refs/heads/{reference}"),
            ],
        );
        let before = fixture.remote_main();
        assert!(!fixture.promote().status.success());
        assert_eq!(fixture.remote_main(), before);
    }
}

#[test]
fn multiple_layers_are_not_promoted() {
    let mut fixture = Fixture::new();
    let root = fixture.directory.path();
    fs::write(root.join("file"), "second layer").unwrap();
    git(root, &["commit", "-am", "fix: second layer"]);
    fixture.candidate = git(root, &["rev-parse", "HEAD"]);
    git(root, &["push", "origin", "HEAD:refs/heads/nick/submit"]);
    assert!(!fixture.promote().status.success());
    assert_eq!(fixture.remote_main(), fixture.base);
}

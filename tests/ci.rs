mod common;
use common::{Repo, failure, success};
use std::{path::Path, process::Command};

#[test]
fn diagnostics_are_utf8_independent_of_console_encoding() {
    for code_page in [850, 1252, 65001] {
        let mut command = Command::new("pwsh");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let output = command
            .args([
                "-NoProfile",
                "-Command",
                "[Console]::OutputEncoding = [Text.Encoding]::GetEncoding([int]$env:TEST_CONSOLE_CODE_PAGE); & $env:TEST_CI_SCRIPT -Binary unused",
            ])
            .env("TEST_CONSOLE_CODE_PAGE", code_page.to_string())
            .env(
                "TEST_CI_SCRIPT",
                Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/scripts/check-messages.ps1"),
            )
            .env("GITHUB_REF", "refs/heads/main")
            .env("GITHUB_SHA", "invalid")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "code page {code_page}");
        let stderr = std::str::from_utf8(&output.stderr)
            .unwrap_or_else(|error| panic!("code page {code_page}: {error}"));
        assert!(stderr.contains("SHA do evento inválido."), "{stderr}");
    }
}

fn run(repo: &Repo, event: &str, before: &str, after: &str) -> std::process::Output {
    let payload = repo.directory.path().join("event.json");
    std::fs::write(
        &payload,
        serde_json::json!({"before": before, "after": after}).to_string(),
    )
    .unwrap();
    repo.command("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/scripts/check-messages.ps1"))
        .arg("-Binary")
        .arg(env!("CARGO_BIN_EXE_clean-dev-cycle"))
        .env("GITHUB_EVENT_NAME", event)
        .env("GITHUB_EVENT_PATH", payload)
        .env("GITHUB_REF", "refs/heads/main")
        .env("GITHUB_SHA", after)
        .output()
        .unwrap()
}

#[test]
fn push_validates_every_message_in_the_event_interval() {
    for middle in ["fix: first", "invalid middle message"] {
        let repo = Repo::new();
        repo.write("first", "first");
        repo.jj(&["commit", "-m", middle]);
        let first = repo.revision("@-", "commit_id");
        if middle.starts_with("fix:") {
            success(&run(&repo, "push", &repo.base, &first));
        }
        repo.write("last", "last");
        repo.jj(&["commit", "-m", "feat: last"]);
        let last = repo.revision("@-", "commit_id");
        let output = run(&repo, "push", &repo.base, &last);
        if middle.starts_with("fix:") {
            success(&output);
        } else {
            failure(&output, "mensagens inválidas");
        }
        // A manual rerun checks its selected commit, not unrelated earlier commits.
        success(&run(&repo, "workflow_dispatch", "", &last));
    }
}

#[test]
fn missing_base_and_backward_push_do_not_silently_skip_validation() {
    let repo = Repo::new();
    repo.write("file", "value");
    repo.jj(&["commit", "-m", "fix: next"]);
    let next = repo.revision("@-", "commit_id");
    failure(
        &run(&repo, "push", &"0".repeat(40), &next),
        "base existente",
    );
    failure(&run(&repo, "push", &next, &repo.base), "não é um avanço");
}

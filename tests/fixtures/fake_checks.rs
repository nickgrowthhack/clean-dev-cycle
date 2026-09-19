use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    process::Command,
};

fn main() {
    let root = env::var_os("FAKE_REPO").unwrap();
    let log = env::var_os("CHECK_LOG").unwrap();
    let mut log = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log)
        .unwrap();
    let cwd = env::current_dir().unwrap();
    writeln!(log, "{}", cwd.display()).unwrap();
    assert_ne!(
        cwd.canonicalize().unwrap(),
        fs::canonicalize(&root).unwrap()
    );
    assert!(
        !cwd.join("pending").exists(),
        "a próxima mudança entrou nos checks"
    );
    let mode = env::args().nth(1).unwrap_or_else(|| "ok".into());
    match mode.as_str() {
        "fail" => std::process::exit(9),
        "modify" => fs::write("CHANGELOG.md", "alterado pelo check").unwrap(),
        "head" => assert!(
            Command::new("git")
                .args(["checkout", "--detach", "HEAD^"])
                .status()
                .unwrap()
                .success()
        ),
        "rewrite" => assert!(
            Command::new("jj")
                .current_dir(&root)
                .args([
                    "describe",
                    "-r",
                    &env::var("CHECK_REVISION").unwrap(),
                    "-m",
                    "fix: rewritten during checks"
                ])
                .status()
                .unwrap()
                .success()
        ),
        "advance" => assert!(
            Command::new("git")
                .args([
                    "--git-dir",
                    &env::var("CHECK_REMOTE").unwrap(),
                    "update-ref",
                    "refs/heads/main",
                    &env::var("CHECK_ADVANCE_SHA").unwrap()
                ])
                .status()
                .unwrap()
                .success()
        ),
        "arguments" => assert_eq!(
            env::args().skip(2).collect::<Vec<_>>(),
            ["two words", "$HOME", "", "a;b", "$(literal)"]
        ),
        "ok" => (),
        _ => panic!("unknown check: {mode}"),
    }
}

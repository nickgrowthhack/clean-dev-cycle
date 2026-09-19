#![allow(dead_code)]
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};
use tempfile::TempDir;

pub fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
pub fn failure(output: &Output, expected: &str) {
    assert!(!output.status.success(), "operação deveria falhar");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
pub fn fake_codex() -> PathBuf {
    static DIRECTORY: OnceLock<TempDir> = OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let output = Command::new("rustc")
                .args(["--edition=2024", "--crate-name", "fake_codex"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_codex.rs"))
                .arg("-o")
                .arg(
                    directory
                        .path()
                        .join(format!("codex{}", std::env::consts::EXE_SUFFIX)),
                )
                .output()
                .unwrap();
            success(&output);
            directory
        })
        .path()
        .join(format!("codex{}", std::env::consts::EXE_SUFFIX))
}

pub struct Repo {
    pub directory: TempDir,
    pub root: PathBuf,
    pub base: String,
}
impl Repo {
    pub fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("ciclo d'ação ")
            .tempdir()
            .unwrap();
        let root = directory.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::write(directory.path().join("gitconfig"), "").unwrap();
        fs::write(
            directory.path().join("jj.toml"),
            "[user]\nname = 'Teste'\nemail = 'teste@example.com'\n",
        )
        .unwrap();
        fs::create_dir(directory.path().join("codex-home")).unwrap();
        fs::write(
            directory.path().join("codex-home/config.toml"),
            "model = 'modelo-configurado'\nmodel_reasoning_effort = 'low'\n",
        )
        .unwrap();
        let mut repo = Self {
            directory,
            root,
            base: String::new(),
        };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.name", "Teste"]);
        repo.git(&["config", "user.email", "teste@example.com"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo.write("CHANGELOG.md", "# Changelog\n\nHistórico preservado.\n");
        repo.git(&["add", "CHANGELOG.md"]);
        repo.git(&["commit", "-m", "docs: base"]);
        repo.base = repo.git(&["rev-parse", "HEAD"]).trim().into();
        repo.jj(&["git", "init", "--colocate"]);
        repo
    }
    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.directory.path().join("gitconfig"))
            .env("JJ_CONFIG", self.directory.path().join("jj.toml"))
            .env("CODEX_HOME", self.directory.path().join("codex-home"));
        for name in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_PARAMETERS",
        ] {
            command.env_remove(name);
        }
        command
    }
    pub fn git(&self, args: &[&str]) -> String {
        let output = self.command("git").args(args).output().unwrap();
        success(&output);
        String::from_utf8(output.stdout).unwrap()
    }
    pub fn jj(&self, args: &[&str]) -> String {
        let output = self
            .command("jj")
            .args(["--no-pager", "--color=never"])
            .args(args)
            .output()
            .unwrap();
        success(&output);
        String::from_utf8(output.stdout).unwrap()
    }
    pub fn write(&self, path: &str, contents: impl AsRef<[u8]>) {
        fs::write(self.root.join(path), contents).unwrap();
    }
    pub fn cli(&self, args: &[&str]) -> Command {
        let mut command = self.command(env!("CARGO_BIN_EXE_clean-dev-cycle"));
        command
            .args(args)
            .env("FAKE_REPO", &self.root)
            .env("FAKE_LOG", self.directory.path().join("calls"))
            .env("FAKE_MODE", "valid");
        command
    }
    pub fn ai(&self, args: &[&str], mode: &str) -> Output {
        self.cli(args)
            .arg("--codex")
            .arg(fake_codex())
            .env("FAKE_MODE", mode)
            .output()
            .unwrap()
    }
    pub fn revision(&self, rev: &str, template: &str) -> String {
        self.jj(&["log", "--no-graph", "-r", rev, "-T", template])
    }
    pub fn calls(&self) -> usize {
        fs::read_to_string(self.directory.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
    pub fn remote(&self) -> PathBuf {
        let remote = self.directory.path().join("remote.git");
        self.git(&["init", "--bare", remote.to_str().unwrap()]);
        self.git(&["remote", "add", "origin", remote.to_str().unwrap()]);
        self.git(&["push", "origin", "main"]);
        self.jj(&["git", "fetch", "--remote", "origin"]);
        remote
    }
    pub fn remote_ref(&self, remote: &Path, name: &str) -> String {
        self.git(&["--git-dir", remote.to_str().unwrap(), "rev-parse", name])
            .trim()
            .into()
    }
}

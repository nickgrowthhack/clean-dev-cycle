use crate::{Result, process};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub const MAX_DIFF_BYTES: usize = 128 * 1024;

pub struct Repository {
    pub root: PathBuf,
}

#[derive(PartialEq, Eq)]
pub struct Snapshot {
    index: Vec<u8>,
    head: Vec<u8>,
    reference: Vec<u8>,
}

impl Repository {
    pub fn discover() -> Result<Self> {
        let current = std::env::current_dir().map_err(|e| e.to_string())?;
        let provisional = Self { root: current };
        let output = provisional.output(&["rev-parse", "--show-toplevel"])?;
        if !output.status.success() {
            return Err(
                "execute este comando dentro de um repositório Git com diretório de trabalho."
                    .into(),
            );
        }
        Ok(Self {
            root: output_path(output.stdout)?,
        })
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0");
        command
    }

    fn output(&self, args: &[&str]) -> Result<Output> {
        process::capture(
            self.command().args(args),
            Vec::new(),
            Duration::from_secs(30),
            4 * 1024 * 1024,
            &AtomicBool::new(false),
        )
    }

    pub fn read(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = self.output(args)?;
        if !output.status.success() {
            return Err(format!(
                "Git não conseguiu ler o repositório: {}",
                process::diagnostic(&output.stderr)
            ));
        }
        Ok(output.stdout)
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        output_path(self.read(&["rev-parse", "--path-format=absolute", "--git-path", name])?)
    }

    pub fn check_state(&self) -> Result<()> {
        if !self.read(&["ls-files", "--unmerged", "-z"])?.is_empty() {
            return Err("há conflitos não resolvidos no stage.".into());
        }
        for name in [
            "MERGE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "rebase-merge",
            "rebase-apply",
            "sequencer",
        ] {
            if self.path(name)?.exists() {
                return Err("conclua a operação de merge, rebase, cherry-pick ou revert antes de usar este comando.".into());
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let head = self.output(&["rev-parse", "--verify", "--quiet", "HEAD"])?;
        if !head.status.success() && head.status.code() != Some(1) {
            return Err("não foi possível consultar HEAD.".into());
        }
        let reference = self.output(&["symbolic-ref", "--quiet", "HEAD"])?;
        if !reference.status.success() && reference.status.code() != Some(1) {
            return Err("não foi possível consultar a branch atual.".into());
        }
        Ok(Snapshot {
            index: self.read(&["ls-files", "--stage", "-z"])?,
            head: head.stdout,
            reference: reference.stdout,
        })
    }

    pub fn ensure_unchanged(&self, expected: &Snapshot) -> Result<()> {
        self.check_state()?;
        if self.snapshot()? != *expected {
            return Err("o stage, HEAD ou a branch mudou durante a revisão. Confira as alterações e execute o comando novamente.".into());
        }
        Ok(())
    }

    pub fn diff(&self) -> Result<String> {
        let names = self.read(&[
            "diff",
            "--cached",
            "--name-only",
            "--no-renames",
            "-z",
            "--",
        ])?;
        if names.is_empty() {
            return Err(
                "não há alterações no stage. Selecione os arquivos com git add antes de continuar."
                    .into(),
            );
        }
        for name in names.split(|&b| b == 0).filter(|s| !s.is_empty()) {
            let name = std::str::from_utf8(name)
                .map_err(|_| "há um nome de arquivo que não está em UTF-8.")?;
            if name.chars().any(char::is_control) {
                return Err("há um nome de arquivo com caracteres de controle.".into());
            }
            if sensitive_path(name) {
                return Err(format!(
                    "arquivo potencialmente sensível no stage: {name}. Revise a seleção antes de enviar o diff ao Codex."
                ));
            }
        }
        let stats = self.read(&["diff", "--cached", "--numstat", "--no-renames", "-z", "--"])?;
        if stats
            .split(|&b| b == 0)
            .any(|entry| entry.starts_with(b"-\t-\t"))
        {
            return Err(
                "o stage contém arquivo binário; esta versão exige um diff textual completo."
                    .into(),
            );
        }
        let output = process::capture(
            self.command().args([
                "diff",
                "--cached",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--no-renames",
                "--src-prefix=a/",
                "--dst-prefix=b/",
                "--submodule=short",
                "--unified=3",
                "--",
            ]),
            Vec::new(),
            Duration::from_secs(30),
            MAX_DIFF_BYTES,
            &AtomicBool::new(false),
        )
        .map_err(|e| format!("não foi possível obter o diff completo (limite de 128 KiB): {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "não foi possível obter o diff: {}",
                process::diagnostic(&output.stderr)
            ));
        }
        let diff = String::from_utf8(output.stdout)
            .map_err(|_| "o diff contém dados que não estão em UTF-8.")?;
        if diff.lines().any(|line| {
            line.starts_with("+Subproject commit ") || line.starts_with("-Subproject commit ")
        }) {
            return Err("esta versão não gera mensagens para alterações em submódulos.".into());
        }
        Ok(diff)
    }
}

impl Snapshot {
    pub fn save(&self, directory: &Path) -> Result<()> {
        for (name, bytes) in [
            ("index", &self.index),
            ("head", &self.head),
            ("reference", &self.reference),
        ] {
            fs::write(directory.join(name), bytes)
                .map_err(|e| format!("não foi possível guardar a seleção: {e}."))?;
        }
        Ok(())
    }

    pub fn load(directory: &Path) -> Result<Self> {
        let read = |name| {
            fs::read(directory.join(name))
                .map_err(|e| format!("não foi possível conferir a seleção: {e}."))
        };
        Ok(Self {
            index: read("index")?,
            head: read("head")?,
            reference: read("reference")?,
        })
    }
}

fn sensitive_path(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let sample = [".example", ".sample", ".template"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    (!sample && (name == ".env" || name.starts_with(".env.")))
        || ["id_rsa", "id_ed25519", "credentials.json"].contains(&name.as_str())
        || [".pem", ".key", ".p12", ".pfx"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

fn output_path(mut bytes: Vec<u8>) -> Result<PathBuf> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes).into())
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| "o caminho do repositório não está em UTF-8.".into())
    }
}

pub fn shell_path(path: &OsStr) -> Result<String> {
    let value = path
        .to_str()
        .ok_or("o caminho do hook não está em UTF-8.")?;
    #[cfg(windows)]
    let value = value
        .strip_prefix("\\\\?\\")
        .unwrap_or(value)
        .replace('\\', "/");
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

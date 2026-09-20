use crate::{Result, process};
use std::{ffi::OsStr, path::PathBuf, process::Command, sync::atomic::AtomicBool, time::Duration};

const MAX_DIFF_BYTES: usize = 128 * 1024;

// Every Git invocation shares one bound on time and output size.
pub fn run(command: &mut Command, input: Vec<u8>, cancelled: &AtomicBool) -> Result<Vec<u8>> {
    let output = process::capture(
        command,
        input,
        Duration::from_secs(120),
        4 * 1024 * 1024,
        cancelled,
    )?;
    if !output.status.success() {
        return Err(format!("Git: {}", process::diagnostic(&output.stderr)));
    }
    Ok(output.stdout)
}

pub struct Repository {
    pub root: PathBuf,
}

impl Repository {
    pub fn discover() -> Result<Self> {
        let current = std::env::current_dir().map_err(|e| e.to_string())?;
        let provisional = Self { root: current };
        let root = provisional
            .read(&["rev-parse", "--show-toplevel"])
            .map_err(
                |_| "execute este comando dentro de um repositório Git com diretório de trabalho.",
            )?;
        Ok(Self {
            root: output_path(root)?,
        })
    }

    pub fn resolve_commit(&self, reference: &OsStr) -> Result<String> {
        if reference.is_empty() || reference.to_string_lossy().starts_with('-') {
            return Err("referência de commit inválida.".into());
        }
        let mut revision = reference.to_owned();
        revision.push("^{commit}");
        let output = run(
            self.command()
                .args(["rev-parse", "--verify", "--end-of-options"])
                .arg(revision),
            Vec::new(),
            &process::NONE,
        )
        .map_err(|_| {
            format!(
                "a referência {} não resolve para um commit local.",
                reference.to_string_lossy()
            )
        })?;
        String::from_utf8(output)
            .map(|s| s.trim().to_owned())
            .map_err(|_| "identificador de commit inválido.".into())
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(&self.root)
            .env("GIT_OPTIONAL_LOCKS", "0");
        command
    }

    pub fn read(&self, args: &[&str]) -> Result<Vec<u8>> {
        run(self.command().args(args), Vec::new(), &process::NONE)
    }

    pub fn ensure_full_history(&self) -> Result<()> {
        if self.read(&["rev-parse", "--is-shallow-repository"])? != b"false\n" {
            return Err("esta operação exige histórico completo.".into());
        }
        Ok(())
    }

    pub fn has_changes(&self, base: &str, head: &str) -> Result<bool> {
        Ok(!self
            .read(&["diff", "--name-only", base, head, "--"])?
            .is_empty())
    }

    pub fn diff_between(&self, base: &str, head: &str) -> Result<String> {
        let read = |flags: &[&str]| self.read(&diff_arguments(base, head, flags));
        let names = read(&["--name-only", "--no-renames", "-z"])?;
        if names.is_empty() {
            return Err("não há alterações no intervalo selecionado.".into());
        }
        for name in names.split(|&b| b == 0).filter(|s| !s.is_empty()) {
            let name = std::str::from_utf8(name)
                .map_err(|_| "há um nome de arquivo que não está em UTF-8.")?;
            if name.chars().any(char::is_control) {
                return Err("há um nome de arquivo com caracteres de controle.".into());
            }
            if sensitive_path(name) {
                return Err(format!(
                    "arquivo potencialmente sensível na seleção: {name}. Revise a seleção antes de enviar o diff ao Codex."
                ));
            }
        }
        let stats = read(&[
            "--numstat",
            "--no-renames",
            "-z",
            "--no-ext-diff",
            "--no-textconv",
        ])?;
        if stats
            .split(|&b| b == 0)
            .any(|entry| entry.starts_with(b"-\t-\t"))
        {
            return Err(
                "a seleção contém arquivo binário; esta versão exige um diff textual completo."
                    .into(),
            );
        }
        let output = process::capture(
            self.command().args(diff_arguments(
                base,
                head,
                &[
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--no-renames",
                    "--src-prefix=a/",
                    "--dst-prefix=b/",
                    "--submodule=short",
                    "--unified=3",
                    "--full-index",
                    "--diff-algorithm=myers",
                    "--no-indent-heuristic",
                    "--inter-hunk-context=0",
                ],
            )),
            Vec::new(),
            Duration::from_secs(30),
            MAX_DIFF_BYTES,
            &process::NONE,
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

fn diff_arguments<'a>(base: &'a str, head: &'a str, flags: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["diff", "--ignore-submodules=none", "--no-relative"];
    args.extend(flags);
    args.extend([base, head, "--"]);
    args
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

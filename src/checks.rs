use crate::{
    Result,
    git::{self, Repository},
    process,
};
use std::{ffi::OsStr, path::Path, process::Command, sync::atomic::AtomicBool, time::Duration};

const CONFIG: &str = "clean-dev-cycle.toml";

fn commands(text: &str) -> Result<Vec<Vec<String>>> {
    let config = text
        .parse::<toml::Table>()
        .map_err(|e| format!("{CONFIG} inválido: {e}"))?;
    let entries = config
        .get("checks")
        .and_then(|v| v.get("commands"))
        .and_then(toml::Value::as_array)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            format!("{CONFIG} deve definir checks.commands como uma lista não vazia.")
        })?;
    entries
        .iter()
        .map(|entry| {
            let args = entry
                .as_array()
                .filter(|v| !v.is_empty())
                .ok_or("cada check deve ser uma lista com programa e argumentos.")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .filter(|s| !s.contains('\0'))
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            "programa e argumentos dos checks devem ser textos sem NUL.".into()
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            if args[0].trim().is_empty() {
                return Err("o programa de um check não pode estar vazio.".into());
            }
            Ok(args)
        })
        .collect()
}

pub fn run(repo: &Repository, revision: &str, cancelled: &AtomicBool) -> Result<()> {
    let config = repo
        .read(&["show", &format!("{revision}:{CONFIG}")])
        .map_err(|e| format!("a mudança selecionada precisa conter {CONFIG}. {e}"))?;
    let config =
        std::str::from_utf8(&config).map_err(|_| format!("{CONFIG} deve estar em UTF-8."))?;
    let commands = commands(config)?;
    let directory = tempfile::Builder::new()
        .prefix("clean-dev-cycle-checks-")
        .tempdir()
        .map_err(|e| format!("não foi possível criar a cópia de verificação: {e}."))?;
    let root = directory.path().join("repo");
    eprintln!("Verificando o commit {revision} em uma cópia temporária...");
    git::run(
        Command::new("git")
            .args(["clone", "--no-hardlinks", "--no-checkout", "--"])
            .arg(&repo.root)
            .arg(&root),
        Vec::new(),
        cancelled,
    )?;
    let isolated = Repository { root };
    git::run(
        isolated.command().args(["checkout", "--detach", revision]),
        Vec::new(),
        cancelled,
    )?;
    for (index, args) in commands.iter().enumerate() {
        eprintln!("Check {}/{}: {:?}", index + 1, commands.len(), args);
        // Resolve relative executable paths against the isolated checkout on every platform.
        let program = Path::new(&args[0]);
        let program = if program.components().count() > 1 && program.is_relative() {
            isolated.root.join(program)
        } else {
            program.to_owned()
        };
        process::stream(
            Command::new(program)
                .args(&args[1..])
                .current_dir(&isolated.root),
            Duration::from_secs(15 * 60),
            cancelled,
        )?;
        if isolated.resolve_commit(OsStr::new("HEAD"))? != revision
            || !isolated
                .read(&["status", "--porcelain", "--untracked-files=no"])?
                .is_empty()
        {
            return Err("um check alterou arquivos versionados, o índice ou o commit da cópia. Use verificações que não reescrevam arquivos.".into());
        }
    }
    directory
        .close()
        .map_err(|e| format!("não foi possível remover a cópia de verificação: {e}."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_requires_nonempty_commands_and_string_arguments() {
        for invalid in [
            "",
            "[checks]",
            "[checks]\ncommands=[]",
            "[checks]\ncommands=['cargo test']",
            "[checks]\ncommands=[[]]",
            "[checks]\ncommands=[['']]",
            "[checks]\ncommands=[['cargo', 1]]",
        ] {
            assert!(commands(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            commands("[checks]\ncommands=[['tool', 'two words', '', '$HOME']]").unwrap(),
            vec![vec!["tool", "two words", "", "$HOME"]]
        );
    }
}

use crate::{
    Result,
    cli::Options,
    git::{self, Repository, Snapshot},
    message, provider,
};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, IsTerminal, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub fn run(options: Options) -> Result<()> {
    let repository = Repository::discover()?;
    repository.check_state()?;
    let snapshot = repository.snapshot()?;
    let diff = repository.diff()?;
    repository.ensure_unchanged(&snapshot)?;
    if !options.dry_run && !options.yes && !io::stdin().is_terminal() {
        return Err("a confirmação exige um terminal. Use --dry-run para revisar ou --yes para confirmar explicitamente.".into());
    }
    let context = options
        .context_file
        .as_ref()
        .map(|path| provider::read_text(path, 16 * 1024))
        .transpose()?
        .unwrap_or_default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .map_err(|e| format!("não foi possível preparar o cancelamento: {e}."))?;
    eprintln!("Gerando mensagem com Codex CLI a partir do stage...");
    let generated = provider::generate(&options, &diff, &context, &cancelled)?;
    repository.ensure_unchanged(&snapshot)?;
    println!("\nMensagem proposta:\n\n{generated}\n");
    if options.dry_run {
        println!("Simulação concluída. Nenhum commit foi criado.");
        return Ok(());
    }
    let message = if options.yes {
        Some(generated)
    } else {
        review(
            generated,
            &mut io::stdin().lock(),
            &mut io::stdout().lock(),
            &cancelled,
            message::validate,
        )?
    };
    let Some(message) = message else {
        println!("Commit cancelado.");
        return Ok(());
    };
    if cancelled.load(Ordering::Relaxed) {
        return Err("operação cancelada.".into());
    }
    repository.ensure_unchanged(&snapshot)?;
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    snapshot.save(directory.path())?;
    let message_path = directory.path().join("message.txt");
    fs::write(&message_path, format!("{}\n", message::validate(&message)?))
        .map_err(|e| e.to_string())?;
    let hooks = directory.path().join("hooks");
    prepare_hooks(&repository, &hooks, directory.path())?;
    repository.ensure_unchanged(&snapshot)?;
    let status = repository
        .command()
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks.display()))
        .args(["commit", "--cleanup=verbatim", "--file"])
        .arg(message_path)
        .status()
        .map_err(|e| format!("não foi possível executar git commit: {e}."))?;
    if !status.success() {
        return Err("git commit falhou. Confira a mensagem do Git ou dos hooks acima e revise o stage antes de tentar novamente.".into());
    }
    println!("Commit criado com sucesso.");
    Ok(())
}

pub(crate) fn review(
    mut message: String,
    input: &mut impl BufRead,
    output: &mut impl Write,
    cancelled: &AtomicBool,
    validate: fn(&str) -> Result<String>,
) -> Result<Option<String>> {
    loop {
        write!(output, "[Enter/s] confirmar, [e] editar, [n] cancelar: ")
            .map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        let mut choice = String::new();
        if input.read_line(&mut choice).map_err(|e| e.to_string())? == 0
            || cancelled.load(Ordering::Relaxed)
        {
            return Ok(None);
        }
        match choice.trim().to_lowercase().as_str() {
            "" | "s" | "sim" => return Ok(Some(message)),
            "n" | "não" | "nao" => return Ok(None),
            "e" => {
                writeln!(output, "Digite a mensagem completa. Termine com uma linha contendo apenas um ponto (.).").map_err(|e| e.to_string())?;
                output.flush().map_err(|e| e.to_string())?;
                let mut edited = String::new();
                loop {
                    let mut line = String::new();
                    if input.read_line(&mut line).map_err(|e| e.to_string())? == 0
                        || cancelled.load(Ordering::Relaxed)
                    {
                        return Ok(None);
                    }
                    if line.trim_end_matches(['\r', '\n']) == "." {
                        break;
                    }
                    edited.push_str(&line);
                    if edited.len() > message::MAX_MESSAGE_BYTES {
                        return Err("a mensagem editada excede 16 KiB.".into());
                    }
                }
                match validate(&edited) {
                    Ok(valid) => {
                        message = valid;
                        writeln!(output, "\nMensagem revisada:\n\n{message}\n")
                            .map_err(|e| e.to_string())?;
                    }
                    Err(error) => {
                        writeln!(
                            output,
                            "Mensagem inválida: {error}\nA mensagem anterior foi mantida."
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
            _ => {
                writeln!(output, "Escolha s, e ou n.").map_err(|e| e.to_string())?;
            }
        }
    }
}

fn prepare_hooks(repository: &Repository, directory: &Path, snapshot: &Path) -> Result<()> {
    fs::create_dir(directory).map_err(|e| e.to_string())?;
    let original = repository.path("hooks")?;
    let entries = match fs::read_dir(&original) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "não foi possível ler os hooks existentes: {error}."
            ));
        }
    };
    if let Some(entries) = entries {
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == "commit-msg" || !is_executable(&entry.path())? {
                continue;
            }
            let script = format!(
                "#!/bin/sh\nexec {} \"$@\"\n",
                git::shell_path(entry.path().as_os_str())?
            );
            write_hook(&directory.join(entry.file_name()), &script)?;
        }
    }
    let mut script = String::from("#!/bin/sh\n");
    let original_message_hook = original.join("commit-msg");
    if is_executable(&original_message_hook)? {
        script.push_str(&format!(
            "{} \"$@\" || exit $?\n",
            git::shell_path(original_message_hook.as_os_str())?
        ));
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    script.push_str(&format!(
        "exec {} __verify-commit \"$1\" {}\n",
        git::shell_path(executable.as_os_str())?,
        git::shell_path(snapshot.as_os_str())?
    ));
    write_hook(&directory.join("commit-msg"), &script)
}

fn is_executable(path: &Path) -> Result<bool> {
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "não foi possível inspecionar o hook {}: {error}.",
                path.display()
            ));
        }
    };
    if !metadata.is_file() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(true)
    }
}

fn write_hook(path: &Path, script: &str) -> Result<()> {
    fs::write(path, script).map_err(|e| format!("não foi possível preparar o hook: {e}."))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn verify_hook(arguments: &[OsString]) -> Result<()> {
    if arguments.len() != 2 {
        return Err("invocação interna de validação inválida.".into());
    }
    let repository = Repository::discover()?;
    let snapshot = Snapshot::load(Path::new(&arguments[1]))?;
    repository.ensure_unchanged(&snapshot)?;
    let path = Path::new(&arguments[0]);
    let text = provider::read_text(path, message::MAX_MESSAGE_BYTES + 2)?;
    let valid = message::validate(&text)?;
    fs::write(path, format!("{valid}\n"))
        .map_err(|e| format!("não foi possível finalizar a mensagem: {e}."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_requires_validation_and_another_confirmation() {
        let input = b"e\nmensagem invalida\n.\ne\nfix: corrigir selecao\n.\ns\n";
        let mut output = Vec::new();
        assert_eq!(
            review(
                "feat: adicionar fluxo".into(),
                &mut &input[..],
                &mut output,
                &AtomicBool::new(false),
                message::validate,
            )
            .unwrap(),
            Some("fix: corrigir selecao".into())
        );
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Mensagem inválida")
        );
    }
    #[test]
    fn cancellation_and_closed_input_do_not_confirm() {
        for input in ["n\n", "", "e\nfix: incompleta\n"] {
            assert!(
                review(
                    "feat: adicionar fluxo".into(),
                    &mut input.as_bytes(),
                    &mut Vec::new(),
                    &AtomicBool::new(false),
                    message::validate,
                )
                .unwrap()
                .is_none()
            );
        }
    }
}

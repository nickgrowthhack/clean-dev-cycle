use crate::{Result, cli::Options, jj::Jujutsu, message, provider};
use std::{
    io::{self, BufRead, IsTerminal, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub fn run(options: Options) -> Result<()> {
    let repository = Jujutsu::discover()?;
    if !options.dry_run && !options.yes && !io::stdin().is_terminal() {
        return Err("a confirmação exige um terminal. Use --dry-run ou --yes.".into());
    }
    repository.ensure_configured_identity()?;
    let snapshot = repository.snapshot()?;
    repository.ensure_author(&snapshot.revision)?;
    let diff = repository.diff(&snapshot.revision)?;
    let context = options
        .context_file
        .as_ref()
        .map(|p| provider::read_text(p, 16 * 1024))
        .transpose()?
        .unwrap_or_default();
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(|e| e.to_string())?;
    eprintln!(
        "Gerando descrição da mudança {} com Codex...",
        snapshot.revision.change
    );
    let generated = provider::generate(&options, &diff, &context, &cancelled)?;
    repository.ensure_unchanged(&snapshot)?;
    println!("\nMensagem proposta:\n\n{generated}\n");
    if options.dry_run {
        return Ok(());
    }
    let reviewed = if options.yes {
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
    let Some(reviewed) = reviewed else {
        println!("Commit cancelado.");
        return Ok(());
    };
    if cancelled.load(Ordering::Relaxed) {
        return Err("operação cancelada.".into());
    }
    repository.finish(&snapshot, &message::validate(&reviewed)?)?;
    println!("Mudança concluída. A próxima mudança está aberta em @. Use submit para enviar.");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn cancellation_and_eof_do_not_confirm_a_message() {
        for input in ["n\n", ""] {
            assert!(
                review(
                    "feat: original".into(),
                    &mut Cursor::new(input),
                    &mut Vec::new(),
                    &AtomicBool::new(false),
                    message::validate
                )
                .unwrap()
                .is_none()
            );
        }
    }

    #[test]
    fn edited_message_is_validated_and_requires_confirmation() {
        let mut input = Cursor::new("e\ninvalid\n.\ne\nfix: corrigir\n.\ns\n");
        let mut output = Vec::new();
        let result = review(
            "feat: original".into(),
            &mut input,
            &mut output,
            &AtomicBool::new(false),
            message::validate,
        )
        .unwrap();
        assert_eq!(result.as_deref(), Some("fix: corrigir"));
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("Mensagem inválida")
        );
    }
}

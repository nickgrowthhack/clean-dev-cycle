use crate::Result;
use std::{ffi::OsString, path::PathBuf, time::Duration};

const HELP: &str = "Uso: clean-dev-cycle <COMANDO> [OPÇÕES]

Comandos:
  commit          Gerar e revisar uma mensagem para o que está no stage.

Opções:
  -h, --help      Exibir esta ajuda.
  -V, --version   Exibir a versão.

Use clean-dev-cycle commit --help para consultar o fluxo de commit.
";

const COMMIT_HELP: &str = "Uso: clean-dev-cycle commit [OPÇÕES]

Gera uma mensagem em português do Brasil usando Codex CLI e o diff do stage.
Exibe a mensagem para confirmar, editar ou cancelar antes de executar git commit.

Opções:
  --dry-run              Gerar e exibir a mensagem sem criar um commit (usa IA).
  --yes                  Confirmar a mensagem válida sem interação.
  --context-file CAMINHO Acrescentar um arquivo UTF-8 com a intenção da mudança.
  --model MODELO         Escolher o modelo; por padrão, usar a escolha do Codex.
  --codex CAMINHO        Executável do Codex (padrão: detecção automática).
  --timeout SEGUNDOS     Prazo por chamada ao Codex (padrão: 120).
  -h, --help             Exibir esta ajuda.

Selecione as alterações com git add antes de executar. Os hooks do Git são mantidos.
";

pub struct Options {
    pub dry_run: bool,
    pub yes: bool,
    pub context_file: Option<PathBuf>,
    pub model: Option<OsString>,
    pub codex: OsString,
    pub timeout: Duration,
}

pub enum Action {
    Help(&'static str),
    Version,
    Commit(Options),
}

pub fn parse(arguments: Vec<OsString>) -> Result<Action> {
    if arguments.is_empty() {
        return Ok(Action::Help(HELP));
    }
    if arguments.len() == 1 {
        match arguments[0].to_str() {
            Some("-h" | "--help") => return Ok(Action::Help(HELP)),
            Some("-V" | "--version") => return Ok(Action::Version),
            _ => {}
        }
    }
    if arguments[0] != "commit" {
        return Err(format!(
            "argumentos não reconhecidos: {}.",
            arguments[0].to_string_lossy()
        ));
    }
    if arguments.len() == 2 && matches!(arguments[1].to_str(), Some("-h" | "--help")) {
        return Ok(Action::Help(COMMIT_HELP));
    }
    let mut options = Options {
        dry_run: false,
        yes: false,
        context_file: None,
        model: None,
        codex: "codex".into(),
        timeout: Duration::from_secs(120),
    };
    let mut seen = std::collections::HashSet::new();
    let mut arguments = arguments.into_iter().skip(1);
    while let Some(argument) = arguments.next() {
        if !seen.insert(argument.clone()) {
            return Err(format!("opção repetida: {}.", argument.to_string_lossy()));
        }
        match argument.to_str() {
            Some("--dry-run") => options.dry_run = true,
            Some("--yes") => options.yes = true,
            Some("--context-file" | "--model" | "--codex" | "--timeout") => {
                let value = arguments
                    .next()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        format!("informe um valor para {}.", argument.to_string_lossy())
                    })?;
                match argument.to_str().unwrap() {
                    "--context-file" => options.context_file = Some(value.into()),
                    "--model" => options.model = Some(value),
                    "--codex" => options.codex = value,
                    _ => {
                        let seconds = value
                            .to_str()
                            .and_then(|s| s.parse::<u64>().ok())
                            .filter(|&n| (1..=3600).contains(&n))
                            .ok_or("--timeout deve ser um inteiro entre 1 e 3600.")?;
                        options.timeout = Duration::from_secs(seconds);
                    }
                }
            }
            _ => {
                return Err(format!(
                    "opção não reconhecida: {}.",
                    argument.to_string_lossy()
                ));
            }
        }
    }
    if options.dry_run && options.yes {
        return Err("use --dry-run ou --yes, separadamente.".into());
    }
    Ok(Action::Commit(options))
}

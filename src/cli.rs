use crate::Result;
use std::{ffi::OsString, path::PathBuf, time::Duration};

const HELP: &str = "Uso: clean-dev-cycle <COMANDO> [OPÇÕES]

Comandos:
  commit          Gerar e revisar uma mensagem para o que está no stage.
  check-commit    Validar mensagens de commit sem IA ou escrita.
  changelog       Revisar a entrega de um PR e atualizar CHANGELOG.md.

Opções:
  -h, --help      Exibir esta ajuda.
  -V, --version   Exibir a versão.

Use clean-dev-cycle <COMANDO> --help para consultar cada fluxo.
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

const CHECK_COMMIT_HELP: &str = "Uso: clean-dev-cycle check-commit --message-file CAMINHO
     clean-dev-cycle check-commit --from REF --to REF

Valida um arquivo UTF-8 ou todos os commits alcançáveis por TO, excluindo FROM
e seus ancestrais (FROM..TO). As referências são resolvidas antes da leitura.
Não chama IA, modifica arquivos, altera o stage ou cria commits.

Opções:
  --message-file CAMINHO Arquivo com a mensagem, inclusive o argumento de commit-msg.
  --from REF             Referência inicial, exclusiva.
  --to REF               Referência final, inclusiva.
  -h, --help             Exibir esta ajuda.

Usa o mesmo perfil fixo da geração de commits, sem configuração por projeto.
Intervalos exigem histórico completo. Um intervalo vazio é válido e informa zero.
Mensagens automáticas de merge, revert, fixup e squash não são ignoradas.
Saída: 0 para mensagens válidas, 1 para falha de validação/leitura, 2 para uso inválido.
";

pub enum CheckCommitInput {
    MessageFile(PathBuf),
    Range { from: OsString, to: OsString },
}

pub struct CheckCommitOptions {
    pub input: CheckCommitInput,
}

const CHANGELOG_HELP: &str = "Uso: clean-dev-cycle changelog --base REF --pr NÚMERO [OPÇÕES]

Usa o Codex para revisar o diff completo desde a base comum até HEAD,
ou recebe uma nota revisada por --entry-file, sem chamar IA.
Propõe uma síntese da entrega em português, para confirmar, editar ou cancelar.
Grava uma entrada por PR em CHANGELOG.md na raiz, preservando as demais notas.

Opções:
  --base REF            Branch ou referência local de destino do PR (obrigatória).
  --pr NÚMERO           Número do PR que identifica a entrada (obrigatório).
  --head REF            Referência local da entrega (padrão: HEAD).
  --entry-file CAMINHO  Fornecer a nota em Markdown, sem IA.
  --dry-run             Exibir a entrada sem gravar (usa IA sem --entry-file).
  --check               Conferir se a entrada corresponde ao diff, sem IA ou escrita.
  --yes                 Confirmar a entrada válida sem interação.
  --context-file CAMINHO Acrescentar intenção e contexto do PR em UTF-8.
  --model MODELO        Escolher o modelo; por padrão, usar a escolha do Codex.
  --codex CAMINHO       Executável do Codex (padrão: detecção automática).
  --timeout SEGUNDOS    Prazo por chamada ao Codex (padrão: 120).
  -h, --help            Exibir esta ajuda.

Execute antes do merge, com as referências locais atualizadas e histórico completo.
O número identifica o PR; a CLI não consulta o GitHub. Stage e arquivos ainda não
commitados não entram na revisão. CHANGELOG.md é excluído do diff.
";

pub struct ChangelogOptions {
    pub base: OsString,
    pub head: OsString,
    pub pr: u64,
    pub check: bool,
    pub entry_file: Option<PathBuf>,
    pub generation: Options,
}

pub enum Action {
    Help(&'static str),
    Version,
    Commit(Options),
    CheckCommit(CheckCommitOptions),
    Changelog(ChangelogOptions),
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
    if arguments[0] == "changelog" {
        return parse_changelog(arguments.into_iter().skip(1).collect());
    }
    if arguments[0] == "check-commit" {
        return parse_check_commit(arguments.into_iter().skip(1).collect());
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
    Ok(Action::Commit(parse_options(
        arguments.into_iter().skip(1),
    )?))
}

fn parse_options(mut arguments: impl Iterator<Item = OsString>) -> Result<Options> {
    let mut options = Options {
        dry_run: false,
        yes: false,
        context_file: None,
        model: None,
        codex: "codex".into(),
        timeout: Duration::from_secs(120),
    };
    let mut seen = std::collections::HashSet::new();
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
    Ok(options)
}

fn parse_changelog(arguments: Vec<OsString>) -> Result<Action> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("-h" | "--help")) {
        return Ok(Action::Help(CHANGELOG_HELP));
    }
    let mut base = None;
    let mut head = None;
    let mut pr = None;
    let mut check = false;
    let mut entry_file = None;
    let mut generation = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if !seen.insert(argument.clone()) {
            return Err(format!("opção repetida: {}.", argument.to_string_lossy()));
        }
        match argument.to_str() {
            Some("--check") => check = true,
            Some("--base" | "--head" | "--pr" | "--entry-file") => {
                let value = arguments
                    .next()
                    .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with('-'))
                    .ok_or_else(|| {
                        format!("informe um valor para {}.", argument.to_string_lossy())
                    })?;
                match argument.to_str().unwrap() {
                    "--base" => base = Some(value),
                    "--head" => head = Some(value),
                    "--entry-file" => entry_file = Some(PathBuf::from(value)),
                    _ => {
                        pr = Some(
                            value
                                .to_str()
                                .and_then(|v| v.parse::<u64>().ok())
                                .filter(|&n| n > 0)
                                .ok_or("--pr deve ser um número inteiro positivo.")?,
                        )
                    }
                }
            }
            Some("--model" | "--codex" | "--timeout" | "--context-file") => {
                let value = arguments
                    .next()
                    .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with("--"))
                    .ok_or_else(|| {
                        format!("informe um valor para {}.", argument.to_string_lossy())
                    })?;
                generation.extend([argument, value]);
            }
            _ => generation.push(argument),
        }
    }
    if entry_file.is_some()
        && (check
            || ["--model", "--codex", "--timeout"]
                .iter()
                .any(|option| seen.contains(&OsString::from(option))))
    {
        return Err(
            "use --entry-file separadamente de --check e das opções --model, --codex e --timeout."
                .into(),
        );
    }
    let generation = parse_options(generation.into_iter())?;
    if check && (generation.dry_run || generation.yes) {
        return Err("use --check separadamente de --dry-run e --yes.".into());
    }
    Ok(Action::Changelog(ChangelogOptions {
        base: base.ok_or("informe a referência de destino com --base.")?,
        head: head.unwrap_or_else(|| "HEAD".into()),
        pr: pr.ok_or("informe o número do PR com --pr.")?,
        check,
        entry_file,
        generation,
    }))
}

fn parse_check_commit(arguments: Vec<OsString>) -> Result<Action> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("-h" | "--help")) {
        return Ok(Action::Help(CHECK_COMMIT_HELP));
    }
    let (mut file, mut from, mut to) = (None, None, None);
    let mut seen = std::collections::HashSet::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if !seen.insert(argument.clone()) {
            return Err(format!("opção repetida: {}.", argument.to_string_lossy()));
        }
        if !matches!(
            argument.to_str(),
            Some("--message-file" | "--from" | "--to")
        ) {
            return Err(format!(
                "opção não reconhecida: {}.",
                argument.to_string_lossy()
            ));
        }
        let value = arguments
            .next()
            .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with('-'))
            .ok_or_else(|| format!("informe um valor para {}.", argument.to_string_lossy()))?;
        match argument.to_str().unwrap() {
            "--message-file" => file = Some(PathBuf::from(value)),
            "--from" => from = Some(value),
            "--to" => to = Some(value),
            _ => unreachable!(),
        }
    }
    let input = match (file, from, to) {
        (Some(path), None, None) => CheckCommitInput::MessageFile(path),
        (None, Some(from), Some(to)) => CheckCommitInput::Range { from, to },
        _ => {
            return Err("use --message-file CAMINHO ou --from REF --to REF, separadamente.".into());
        }
    };
    Ok(Action::CheckCommit(CheckCommitOptions { input }))
}

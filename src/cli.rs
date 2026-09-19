use crate::Result;
use std::{collections::HashSet, ffi::OsString, path::PathBuf, time::Duration};

const HELP: &str = "Uso: clean-dev-cycle <COMANDO> [OPÇÕES]

Comandos:
  commit          Revisar e concluir a mudança atual do Jujutsu (@).
  submit          Verificar e publicar uma mudança diretamente na main.
  check-commit    Validar mensagens de commit sem IA.
  changelog       Sintetizar um intervalo em Markdown, sem alterar arquivos.

Opções:
  -h, --help      Exibir ajuda.
  -V, --version   Exibir a versão.

Use clean-dev-cycle <COMANDO> --help para consultar cada fluxo.
";
const COMMIT_HELP: &str = "Uso: clean-dev-cycle commit [OPÇÕES]

Gera e revisa a descrição de @ no Jujutsu. Confirmar conclui a mudança
e abre a próxima, como jj commit. Use jj split para separar alterações.
A geração considera apenas o diff da mudança atual.

Opções:
  --dry-run              Exibir a proposta sem descrever ou concluir @ (usa IA).
  --yes                  Confirmar sem interação.
  --context-file CAMINHO Contexto UTF-8, até 16 KiB.
  --model MODELO         Modelo do Codex.
  --codex CAMINHO        Executável do Codex.
  --timeout SEGUNDOS     Prazo por chamada, padrão 120.
  -h, --help             Exibir ajuda.

Requer jj 0.45.1 no PATH e workspace Git colocated. A leitura do jj pode
salvar snapshots locais. O caminho sem IA é jj commit -m MENSAGEM.
";
const SUBMIT_HELP: &str = "Uso: clean-dev-cycle submit [--revision REV]

Atualiza origin e publica uma mudança concluída (padrão @-) na main.
A mudança deve ser filha direta de main@origin, sem conflitos ou merges.
Executa os checks de clean-dev-cycle.toml do commit em uma cópia temporária,
com limite de 15 minutos por comando. Falhas impedem o envio. O CI roda após o push.
Camadas anteriores devem ser enviadas primeiro. Não faz rebase automático.
Reenviar uma mudança já integrada é uma operação sem efeito.
";
const CHECK_HELP: &str = "Uso: clean-dev-cycle check-commit --message-file CAMINHO
     clean-dev-cycle check-commit --from REF --to REF

Valida o arquivo UTF-8 ou todos os commits em FROM..TO com o perfil fixo.
Não chama IA nem modifica o repositório. O modo arquivo funciona sem Git.
Intervalos exigem histórico completo. Um intervalo vazio é válido.
Saída: 0 para sucesso, 1 para falha, 2 para uso inválido.
";
const CHANGELOG_HELP: &str = "Uso: clean-dev-cycle changelog --from REF --to REF [OPÇÕES]

Sintetiza o diff completo entre duas referências Git (FROM ancestral de TO).
Emite somente Markdown em stdout, sem alterar arquivos.
Use SHAs Git de mudanças jj concluídas. O intervalo exige histórico completo.

Opções:
  --entry-file CAMINHO   Fornecer síntese revisada sem IA, até 16 KiB.
  --context-file CAMINHO Contexto adicional UTF-8, até 16 KiB.
  --model MODELO         Modelo do Codex.
  --codex CAMINHO        Executável do Codex.
  --timeout SEGUNDOS     Prazo por chamada, padrão 120.
  -h, --help             Exibir ajuda.
";

pub struct Options {
    pub dry_run: bool,
    pub yes: bool,
    pub context_file: Option<PathBuf>,
    pub model: Option<OsString>,
    pub codex: OsString,
    pub timeout: Duration,
}
pub enum CheckCommitInput {
    MessageFile(PathBuf),
    Range { from: OsString, to: OsString },
}
pub struct CheckCommitOptions {
    pub input: CheckCommitInput,
}
pub struct ChangelogOptions {
    pub from: OsString,
    pub to: OsString,
    pub entry_file: Option<PathBuf>,
    pub generation: Options,
}
pub enum Action {
    Help(&'static str),
    Version,
    Commit(Options),
    Submit(OsString),
    CheckCommit(CheckCommitOptions),
    Changelog(ChangelogOptions),
}

pub fn parse(mut arguments: Vec<OsString>) -> Result<Action> {
    if arguments.is_empty() {
        return Ok(Action::Help(HELP));
    }
    let command = arguments.remove(0);
    if arguments.is_empty() {
        match command.to_str() {
            Some("-h" | "--help") => return Ok(Action::Help(HELP)),
            Some("-V" | "--version") => return Ok(Action::Version),
            _ => {}
        }
    }
    let help = match command.to_str() {
        Some("commit") => COMMIT_HELP,
        Some("submit") => SUBMIT_HELP,
        Some("check-commit") => CHECK_HELP,
        Some("changelog") => CHANGELOG_HELP,
        _ => {
            return Err(format!(
                "comando não reconhecido: {}.",
                command.to_string_lossy()
            ));
        }
    };
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("-h" | "--help")) {
        return Ok(Action::Help(help));
    }
    let mut generation = Options {
        dry_run: false,
        yes: false,
        context_file: None,
        model: None,
        codex: "codex".into(),
        timeout: Duration::from_secs(120),
    };
    let (mut from, mut to, mut file, mut entry_file, mut revision) = (None, None, None, None, None);
    let mut seen = HashSet::new();
    let mut args = arguments.into_iter();
    while let Some(option) = args.next() {
        if !seen.insert(option.clone()) {
            return Err(format!("opção repetida: {}.", option.to_string_lossy()));
        }
        let name = option.to_str().ok_or("opção inválida.")?;
        let allowed = match command.to_str().unwrap() {
            "commit" => matches!(
                name,
                "--dry-run" | "--yes" | "--context-file" | "--model" | "--codex" | "--timeout"
            ),
            "submit" => name == "--revision",
            "check-commit" => matches!(name, "--from" | "--to" | "--message-file"),
            "changelog" => matches!(
                name,
                "--from"
                    | "--to"
                    | "--entry-file"
                    | "--context-file"
                    | "--model"
                    | "--codex"
                    | "--timeout"
            ),
            _ => false,
        };
        if !allowed {
            return Err(format!("opção não reconhecida: {name}."));
        }
        match name {
            "--dry-run" => {
                generation.dry_run = true;
                continue;
            }
            "--yes" => {
                generation.yes = true;
                continue;
            }
            _ => {}
        }
        let value = args
            .next()
            .filter(|v| !v.is_empty() && !v.to_string_lossy().starts_with('-'))
            .ok_or_else(|| format!("informe um valor para {name}."))?;
        match name {
            "--from" => from = Some(value),
            "--to" => to = Some(value),
            "--message-file" => file = Some(PathBuf::from(value)),
            "--entry-file" => entry_file = Some(PathBuf::from(value)),
            "--revision" => revision = Some(value),
            "--context-file" => generation.context_file = Some(value.into()),
            "--model" => generation.model = Some(value),
            "--codex" => generation.codex = value,
            "--timeout" => {
                let seconds = value
                    .to_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|n| (1..=3600).contains(n))
                    .ok_or("--timeout deve ser um inteiro entre 1 e 3600.")?;
                generation.timeout = Duration::from_secs(seconds);
            }
            _ => unreachable!(),
        }
    }
    match command.to_str().unwrap() {
        "commit" => Ok(Action::Commit(generation)),
        "submit" => Ok(Action::Submit(revision.unwrap_or_else(|| "@-".into()))),
        "check-commit" => {
            let input = match (file, from, to) {
                (Some(path), None, None) => CheckCommitInput::MessageFile(path),
                (None, Some(from), Some(to)) => CheckCommitInput::Range { from, to },
                _ => {
                    return Err(
                        "use --message-file CAMINHO ou --from REF --to REF, separadamente.".into(),
                    );
                }
            };
            Ok(Action::CheckCommit(CheckCommitOptions { input }))
        }
        "changelog" => {
            if entry_file.is_some()
                && ["--model", "--codex", "--timeout"]
                    .iter()
                    .any(|s| seen.contains(&OsString::from(s)))
            {
                return Err("use --entry-file separadamente das opções de IA.".into());
            }
            Ok(Action::Changelog(ChangelogOptions {
                from: from.ok_or("informe --from REF.")?,
                to: to.ok_or("informe --to REF.")?,
                entry_file,
                generation,
            }))
        }
        _ => unreachable!(),
    }
}

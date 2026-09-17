use crate::Result;

pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

pub fn validate(input: &str) -> Result<String> {
    let message = input.replace("\r\n", "\n").trim().to_owned();
    if message.is_empty() || message.len() > MAX_MESSAGE_BYTES {
        return Err("a mensagem deve ter entre 1 byte e 16 KiB.".into());
    }
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("a mensagem contém caracteres de controle inválidos.".into());
    }
    let header = message.lines().next().unwrap();
    if header.chars().count() > 100 {
        return Err("o título deve ter no máximo 100 caracteres.".into());
    }
    let parsed = git_conventional::Commit::parse(&message)
        .map_err(|_| "use Conventional Commits: tipo(escopo opcional): descrição; separe o corpo com uma linha em branco.")?;
    let kind = parsed.type_().to_string();
    if ![
        "feat", "fix", "perf", "refactor", "docs", "test", "style", "ci", "build", "chore",
        "revert",
    ]
    .contains(&kind.as_str())
    {
        return Err(format!("tipo de commit não permitido: {kind}."));
    }
    if parsed.description().trim().is_empty() {
        return Err("a descrição do commit está vazia.".into());
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_breaking_changes_and_git_trailers() {
        for text in [
            "feat(cli): permitir revisão da mensagem",
            "feat!: remover formato antigo\n\nBREAKING CHANGE: usar o novo formato.\n\nRefs: #12",
            "fix: corrigir seleção\n\nSigned-off-by: Pessoa <pessoa@example.com>",
        ] {
            assert_eq!(validate(text).unwrap(), text);
        }
    }
    #[test]
    fn rejects_unstructured_and_unbounded_messages() {
        for text in [
            "",
            "mensagem solta",
            "feat: ",
            "wip: continuar amanhã",
            "```\nfeat: título\n```",
            "feat: título\u{1b}",
            "feat: título\ncorpo sem separação",
        ] {
            assert!(validate(text).is_err(), "{text:?}");
        }
        assert!(validate(&format!("feat: {}", "á".repeat(100))).is_err());
    }
}

use crate::{
    Result,
    cli::{CheckCommitInput, CheckCommitOptions},
    git::Repository,
    message::{self, MAX_MESSAGE_BYTES},
    provider,
};

pub fn run(options: CheckCommitOptions) -> Result<()> {
    match options.input {
        CheckCommitInput::MessageFile(path) => {
            let text = provider::read_text(&path, MAX_MESSAGE_BYTES + 2)?;
            message::validate(&text)?;
            println!("Mensagem de commit válida.");
            Ok(())
        }
        CheckCommitInput::Range { from, to } => {
            let repository = Repository::discover()?;
            if repository.read(&["rev-parse", "--is-shallow-repository"])? != b"false\n" {
                return Err("a validação de intervalos exige histórico completo.".into());
            }
            let from = repository.resolve_commit(&from)?;
            let to = repository.resolve_commit(&to)?;
            validate_range(&repository, &from, &to).map(|_| ())
        }
    }
}

pub fn validate_range(repository: &Repository, from: &str, to: &str) -> Result<bool> {
    let range = format!("{from}..{to}");
    let ids = repository.read(&["rev-list", "--reverse", &range, "--"])?;
    let ids = std::str::from_utf8(&ids).map_err(|_| "lista de commits inválida.")?;
    let mut failures = Vec::new();
    let mut count = 0;
    let mut requires_note = false;
    for id in ids.lines() {
        count += 1;
        match validate_commit(repository, id) {
            Ok(impact) => requires_note |= impact,
            Err(error) => failures.push(format!("{id}: {error}")),
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "{} de {count} commit(s) inválido(s):\n{}",
            failures.len(),
            failures.join("\n")
        ));
    }
    println!("{count} commit(s) válido(s) no intervalo {range}.");
    Ok(requires_note)
}

fn validate_commit(repository: &Repository, id: &str) -> Result<bool> {
    // Read the object itself: separators in messages, Git notes and pretty-format settings
    // cannot hide additional commits or manufacture message boundaries.
    let object = repository.read(&["cat-file", "commit", id])?;
    let start = object
        .windows(2)
        .position(|w| w == b"\n\n")
        .ok_or("objeto de commit sem mensagem.")?
        + 2;
    let text =
        std::str::from_utf8(&object[start..]).map_err(|_| "a mensagem precisa estar em UTF-8.")?;
    let message = message::validate(text)?;
    let parsed = git_conventional::Commit::parse(&message).map_err(|e| e.to_string())?;
    Ok(parsed.breaking() || ["feat", "fix", "perf"].contains(&parsed.type_().as_str()))
}

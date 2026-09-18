use crate::{
    Result, changelog, check_commit,
    cli::{ChangelogOptions, CiOptions, Options},
    git::Repository,
    provider,
};
use serde_json::Value;
use std::{ffi::OsStr, fs, path::PathBuf, time::Duration};

pub fn run(options: CiOptions) -> Result<()> {
    let event: Value =
        serde_json::from_str(&provider::read_text(&options.event_file, 1024 * 1024)?)
            .map_err(|e| format!("evento JSON inválido: {e}"))?;
    let repository = Repository::discover()?;
    repository.check_state()?;
    if repository.read(&["rev-parse", "--is-shallow-repository"])? != b"false\n" {
        return Err("check-ci exige histórico completo (fetch-depth: 0).".into());
    }
    repository.read(&["diff", "--exit-code", "HEAD", "--"])?;
    check_pr(&repository, &event)
}

fn text<'a>(event: &'a Value, pointer: &str) -> Result<&'a str> {
    event
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("campo ausente ou inválido no evento: {pointer}."))
}

fn sha<'a>(event: &'a Value, pointer: &str) -> Result<&'a str> {
    let value = text(event, pointer)?;
    if ![40, 64].contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("SHA inválido em {pointer}."));
    }
    Ok(value)
}

fn checkout(repository: &Repository, head: &str) -> Result<()> {
    if repository.resolve_commit(OsStr::new("HEAD"))? != head {
        return Err(
            "faça checkout do HEAD real do evento, não do merge sintético do GitHub.".into(),
        );
    }
    Ok(())
}

fn check_pr(repository: &Repository, event: &Value) -> Result<()> {
    let head = sha(event, "/pull_request/head/sha")?;
    let base = sha(event, "/pull_request/base/sha")?;
    let number = event
        .get("number")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or("número do PR inválido.")?;
    checkout(repository, head)?;
    repository.resolve_commit(OsStr::new(base))?;
    let common = repository.read(&["merge-base", "--all", base, head])?;
    if common != format!("{base}\n").as_bytes() {
        return Err("a branch do PR precisa estar atualizada com a base. Faça rebase e execute os checks novamente.".into());
    }
    let range = format!("{base}..{head}");
    if !repository
        .read(&["rev-list", "--merges", &range, "--"])?
        .is_empty()
    {
        return Err(
            "o PR contém commits de merge. Use rebase para preservar um histórico linear.".into(),
        );
    }
    let requires_note = check_commit::validate_range(repository, base, head)?;
    let title = text(event, "/pull_request/title")?;
    if title.contains(['\r', '\n']) {
        return Err("o título do PR deve ocupar uma única linha.".into());
    }
    let files = repository.read(&[
        "diff",
        "--name-only",
        "--no-renames",
        "-z",
        base,
        head,
        "--",
    ])?;
    if files.is_empty() {
        return Err("o PR não contém alterações de arquivos.".into());
    }
    let document = tracked_text(repository, head, "CHANGELOG.md")?;
    if files == b"CHANGELOG.md\0" && !requires_note {
        let previous = tracked_text(repository, base, "CHANGELOG.md")?
            .ok_or("a correção editorial exige um changelog existente na base.")?;
        changelog::check_editorial_change(
            &previous,
            document
                .as_deref()
                .ok_or("a correção editorial não pode remover o changelog.")?,
        )?;
        println!(
            "Correção editorial: somente CHANGELOG.md mudou, com versões e metadados preservados."
        );
        return Ok(());
    }
    let has_note = document
        .as_deref()
        .map(|text| changelog::has_entry(text, number))
        .transpose()?
        .unwrap_or(false);
    if !requires_note && !has_note {
        println!("PR interno: nota de changelog opcional e não fornecida.");
        return Ok(());
    }
    let document = document
        .ok_or("CHANGELOG.md está ausente no HEAD do PR. Gere e inclua a entrada na branch.")?;
    changelog::check_unreleased_entry(&document, number)?;
    let context_name = format!(".changelog-context/{number}.md");
    let context_file = if tracked_text(repository, head, &context_name)?.is_some() {
        Some(repository.root.join(&context_name))
    } else {
        None
    };
    changelog::run(ChangelogOptions {
        base: base.into(), head: head.into(), pr: number, check: true, entry_file: None,
        generation: Options {
            dry_run: false, yes: false, context_file, model: None,
            codex: "codex".into(), timeout: Duration::from_secs(120),
        },
    }).map_err(|e| format!("{e}\nNo CI, o contexto adicional deve estar versionado em {context_name} e ser usado na geração com --context-file. Contexto externo não está disponível."))
}

fn tracked_text(repository: &Repository, revision: &str, path: &str) -> Result<Option<String>> {
    let tree = repository.read(&["ls-tree", "-z", revision, "--", path])?;
    if tree.is_empty() {
        return Ok(None);
    }
    if !tree.starts_with(b"100644 blob ") && !tree.starts_with(b"100755 blob ") {
        return Err(format!("{path} precisa ser um arquivo regular versionado."));
    }
    let object = repository.read(&["show", &format!("{revision}:{path}")])?;
    if object.len() > 1024 * 1024 {
        return Err(format!("{path} excede 1 MiB."));
    }
    let value = String::from_utf8(object).map_err(|_| format!("{path} deve ser UTF-8."))?;
    if repository.resolve_commit(OsStr::new("HEAD"))? == revision {
        let local = repository.root.join(PathBuf::from(path));
        if fs::symlink_metadata(&local)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err(format!("{path} não pode ser um link."));
        }
        let content = provider::read_text(&local, 1024 * 1024)?;
        if content != value {
            return Err(format!(
                "{path} no checkout difere do conteúdo versionado. Use LF e confira os filtros Git."
            ));
        }
    }
    Ok(Some(value))
}

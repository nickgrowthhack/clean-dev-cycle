use crate::{Result, check_commit, checks, jj::Jujutsu, message};
use std::{
    ffi::OsStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const BOOKMARK: &str = "main";

pub fn run(reference: &OsStr) -> Result<()> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancelled);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(|e| e.to_string())?;
    let repo = Jujutsu::discover()?;
    repo.run(&["status"])?;
    // Resolve once before fetch: never silently select another change when refs move.
    let revision = repo.revision(reference, None)?;
    repo.run(&["git", "fetch", "--remote", "origin"])?;
    repo.run(&["bookmark", "track", "main@origin"])?;
    let main = repo.revision(OsStr::new("main@origin"), None)?;
    let common = repo.git.read(&["merge-base", &revision.id, &main.id])?;
    if common == format!("{}\n", revision.id).as_bytes() {
        println!("Mudança já integrada na main: {}.", revision.id);
        return Ok(());
    }
    if repo.revision(OsStr::new("@"), None)?.change == revision.change {
        return Err("conclua @ com jj commit antes de enviar a mudança.".into());
    }
    repo.ensure_no_conflicts(&revision)?;
    repo.ensure_publishable_identity(&revision)?;
    if revision.parents != [main.id.clone()] {
        let first = repo.git.read(&[
            "rev-list",
            "--reverse",
            "--ancestry-path",
            &format!("{}..{}", main.id, revision.id),
        ])?;
        let first = String::from_utf8_lossy(&first);
        let hint = first
            .lines()
            .next()
            .map(|id| format!(" Envie a primeira camada com submit --revision {id}."))
            .unwrap_or_default();
        return Err(format!(
            "a mudança precisa ser filha direta de main@origin. Atualize a base com jj rebase.{hint}"
        ));
    }
    message::validate(&revision.description)?;
    check_commit::validate_range(&repo.git, &main.id, &revision.id)?;
    if repo
        .git
        .read(&["diff", "--name-only", &main.id, &revision.id, "--"])?
        .is_empty()
    {
        return Err("não há alterações de arquivos para enviar.".into());
    }
    checks::run(&repo.git, &revision.id, &cancelled)?;
    repo.run(&["git", "fetch", "--remote", "origin"])?;
    if repo.revision(OsStr::new("main@origin"), None)?.id != main.id {
        return Err("a main avançou durante os checks. Atualize a base, revise e reenvie.".into());
    }
    let current = repo.revision(OsStr::new(&revision.change), None)?;
    if current != revision {
        return Err("a mudança foi reescrita durante o envio. Revise e reenvie.".into());
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err("operação cancelada.".into());
    }
    repo.run(&["bookmark", "set", BOOKMARK, "--revision", &revision.id])?;
    let operation = repo.run(&[
        "--ignore-working-copy",
        "op",
        "log",
        "--limit",
        "1",
        "--no-graph",
        "-T",
        "id",
    ])?;
    if repo.revision(OsStr::new(BOOKMARK), Some(&operation))? != revision
        || repo
            .revision(OsStr::new("main@origin"), Some(&operation))?
            .id
            != main.id
        || cancelled.load(Ordering::Relaxed)
    {
        return Err("o envio foi cancelado ou as referências mudaram. Revise e reenvie.".into());
    }
    repo.run(&[
        "--at-operation",
        &operation,
        "git",
        "push",
        "--remote",
        "origin",
        "--bookmark",
        BOOKMARK,
    ])?;
    println!(
        "Publicado {} diretamente na main. Checks locais aprovados. Acompanhe o CI no GitHub.",
        revision.id
    );
    Ok(())
}

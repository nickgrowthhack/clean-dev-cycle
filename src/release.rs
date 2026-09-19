use crate::{
    Result, changelog, cli::CommitOptions, git::Repository, github, message, process, provider,
};
use next_version::VersionUpdater;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, sync::atomic::AtomicBool, time::Duration};
use toml_edit::{DocumentMut, value};

pub const MANIFEST: &str = ".clean-dev-cycle-release.json";
pub const FILES: [&str; 4] = ["Cargo.toml", "Cargo.lock", "CHANGELOG.md", MANIFEST];
const START: &str = "<!-- clean-dev-cycle:release:start -->\n";
const END: &str = "<!-- clean-dev-cycle:release:end -->\n\n";
const HEADER: &str = "# Changelog\n\n";
const REPAIR: &str =
    "Reabra a mudança com jj edit e conclua com clean-dev-cycle commit para preparar a release.";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub parent: String,
    pub base: Option<String>,
    pub base_tag: Option<String>,
    pub fingerprint: String,
    pub message: String,
    pub notes_hash: String,
}

pub struct Prepared {
    pub tree: String,
    pub version: String,
    pub notes: String,
}

pub fn file(repo: &Repository, revision: &str, path: &str) -> Result<Option<String>> {
    let entries = repo.read(&["ls-tree", revision, "--", path])?;
    if entries.is_empty() {
        return Ok(None);
    }
    if !entries.starts_with(b"100644 blob ") && !entries.starts_with(b"100755 blob ") {
        return Err(format!("{path} deve ser um arquivo regular."));
    }
    let bytes = repo.read(&["show", &format!("{revision}:{path}")])?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| format!("{path} precisa estar em UTF-8."))
}

pub fn enabled(repo: &Repository, revision: &str) -> Result<bool> {
    let Some(config) = file(repo, revision, "clean-dev-cycle.toml")? else {
        return Ok(false);
    };
    let config: toml::Table = config
        .parse()
        .map_err(|e| format!("configuração inválida: {e}"))?;
    let Some(release) = config.get("release") else {
        return Ok(false);
    };
    release
        .get("enabled")
        .and_then(toml::Value::as_bool)
        .ok_or("release.enabled deve ser booleano.".into())
}

pub fn eligible(description: &str) -> Result<bool> {
    let description = message::validate(description)?;
    let commit = git_conventional::Commit::parse(&description).map_err(|e| e.to_string())?;
    Ok(commit.breaking() || matches!(commit.type_().as_str(), "feat" | "fix" | "perf"))
}

fn version(text: &str) -> Result<(String, Version)> {
    let doc = document(text)?;
    if doc.contains_key("workspace") || doc["package"]["version"].as_str().is_none() {
        return Err(
            "a primeira versão suporta um pacote Rust na raiz, sem workspace ou versão herdada."
                .into(),
        );
    }
    let name = doc["package"]["name"]
        .as_str()
        .ok_or("Cargo.toml sem package.name.")?;
    let version =
        Version::parse(doc["package"]["version"].as_str().unwrap()).map_err(|e| e.to_string())?;
    if !version.pre.is_empty() || !version.build.is_empty() || version < Version::new(0, 1, 0) {
        return Err("use uma versão estável a partir de 0.1.0, sem metadados de build.".into());
    }
    Ok((name.into(), version))
}

fn document(text: &str) -> Result<DocumentMut> {
    text.parse().map_err(|e| format!("TOML inválido: {e}"))
}

fn set_version(text: &str, name: &str, next: &str, lock: bool) -> Result<String> {
    let mut doc = document(text)?;
    let item = if lock {
        let packages = doc["package"]
            .as_array_of_tables_mut()
            .ok_or("Cargo.lock sem pacotes.")?;
        let indices: Vec<_> = packages
            .iter()
            .enumerate()
            .filter(|(_, p)| p["name"].as_str() == Some(name) && !p.contains_key("source"))
            .map(|(i, _)| i)
            .collect();
        if indices.len() != 1 {
            return Err(
                "Cargo.lock precisa conter exatamente um pacote raiz correspondente.".into(),
            );
        }
        &mut packages.get_mut(indices[0]).unwrap()["version"]
    } else {
        &mut doc["package"]["version"]
    };
    let decor = item.as_value().ok_or("versão inválida.")?.decor().clone();
    *item = value(next);
    *item.as_value_mut().unwrap().decor_mut() = decor;
    Ok(doc.to_string())
}

fn lock_version(text: &str, name: &str) -> Result<String> {
    let doc = document(text)?;
    let packages = doc["package"]
        .as_array_of_tables()
        .ok_or("Cargo.lock sem pacotes.")?;
    let values: Vec<_> = packages
        .iter()
        .filter(|p| p["name"].as_str() == Some(name) && !p.contains_key("source"))
        .collect();
    if values.len() != 1 {
        return Err("Cargo.lock sem pacote raiz único.".into());
    }
    values[0]["version"]
        .as_str()
        .map(str::to_owned)
        .ok_or("versão inválida no Cargo.lock.".into())
}

fn parent(repo: &Repository, revision: &str) -> Result<String> {
    let line = String::from_utf8(repo.read(&["rev-list", "--parents", "-n", "1", revision])?)
        .map_err(|e| e.to_string())?;
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() != 2 {
        return Err("a preparação exige um commit com um único pai.".into());
    }
    Ok(parts[1].into())
}

fn hash(repo: &Repository, bytes: &[u8], write: bool) -> Result<String> {
    let mut command = repo.command();
    command.args(["hash-object", "--stdin"]);
    if write {
        command.arg("-w");
    }
    capture(&mut command, bytes.to_vec())
}

fn capture(command: &mut std::process::Command, input: Vec<u8>) -> Result<String> {
    let output = process::capture(
        command,
        input,
        Duration::from_secs(120),
        4 * 1024 * 1024,
        &AtomicBool::new(false),
    )?;
    if !output.status.success() {
        return Err(process::diagnostic(&output.stderr));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().into())
        .map_err(|e| e.to_string())
}

// A separate index provides a full tree copy without touching the user's index or files.
fn tree(repo: &Repository, revision: &str, edits: &[(&str, Option<String>)]) -> Result<String> {
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let index = directory.path().join("index");
    let command = || {
        let mut c = repo.command();
        c.env("GIT_INDEX_FILE", &index);
        c
    };
    capture(command().args(["read-tree", revision]), Vec::new())?;
    for (path, content) in edits {
        if let Some(content) = content {
            let id = hash(repo, content.as_bytes(), true)?;
            let entry = repo.read(&["ls-tree", revision, "--", path])?;
            let mode = if entry.starts_with(b"100755 ") {
                "100755"
            } else {
                "100644"
            };
            capture(
                command().args(["update-index", "--add", "--cacheinfo", mode, &id, path]),
                Vec::new(),
            )?;
        } else {
            capture(
                command().args(["update-index", "--force-remove", "--", path]),
                Vec::new(),
            )?;
        }
    }
    capture(command().arg("write-tree"), Vec::new())
}

fn entries(text: &str) -> Result<(String, Vec<String>)> {
    let mut rest = text;
    let mut plain = String::new();
    let mut entries = Vec::new();
    while let Some((before, after)) = rest.split_once(START) {
        if before.contains("clean-dev-cycle:release:") {
            return Err("marcadores de changelog inválidos.".into());
        }
        plain.push_str(before);
        let (entry, remaining) = after
            .split_once(END)
            .ok_or("entrada de release incompleta.")?;
        if entry.contains("clean-dev-cycle:release:") {
            return Err("entrada de release aninhada.".into());
        }
        entries.push(entry.into());
        rest = remaining;
    }
    if rest.contains("clean-dev-cycle:release:") {
        return Err("marcadores de changelog inválidos.".into());
    }
    plain.push_str(rest);
    Ok((plain, entries))
}

fn normalized(repo: &Repository, revision: &str) -> Result<String> {
    let mut edits = vec![(MANIFEST, None)];
    if let Some(cargo) = file(repo, revision, "Cargo.toml")? {
        let (name, _) = version(&cargo)?;
        edits.push((
            "Cargo.toml",
            Some(set_version(&cargo, &name, "0.0.0", false)?),
        ));
        if let Some(lock) = file(repo, revision, "Cargo.lock")? {
            edits.push((
                "Cargo.lock",
                Some(set_version(&lock, &name, "0.0.0", true)?),
            ));
        }
    }
    if let Some(log) = file(repo, revision, "CHANGELOG.md")? {
        let plain = entries(&log)?.0;
        edits.push((
            "CHANGELOG.md",
            if plain == HEADER { None } else { Some(plain) },
        ));
    }
    tree(repo, revision, &edits)
}

fn manifest(repo: &Repository, revision: &str) -> Result<Option<Manifest>> {
    file(repo, revision, MANIFEST)?
        .map(|s| {
            serde_json::from_str(&s).map_err(|e| format!("manifesto de release inválido: {e}"))
        })
        .transpose()
}

fn fingerprint(
    repo: &Repository,
    content: &str,
    base: Option<&str>,
    tag: Option<&str>,
) -> Result<String> {
    hash(
        repo,
        serde_json::json!({"content": content, "base": base, "base_tag": tag})
            .to_string()
            .as_bytes(),
        false,
    )
}

fn latest_published(repo: &Repository, revision: &str) -> Result<Option<(String, String)>> {
    let host = github::Github::discover(repo)?;
    let mut releases: Vec<_> = github::releases(&host)?
        .into_iter()
        .filter_map(|r| github::version(&r).map(|v| (v, r)))
        .collect();
    releases.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, release) in releases {
        let tag = release["tag_name"].as_str().unwrap();
        let sha = github::tag_commit(&host, tag)?.ok_or("release publicada sem tag.")?;
        // Missing history must not silently turn a later release into a first release.
        repo.resolve_commit(OsStr::new(&sha)).map_err(|_| "atualize o histórico com jj git fetch --remote origin antes de preparar a release.")?;
        if repo.read(&["merge-base", &sha, revision])? == format!("{sha}\n").as_bytes() {
            return Ok(Some((tag.into(), sha)));
        }
    }
    Ok(None)
}

fn next_version(previous: &Version, description: &str, initial: bool) -> Version {
    if initial {
        previous.clone()
    } else {
        VersionUpdater::default()
            .with_features_always_increment_minor(true)
            .increment(previous, [description])
    }
}

pub fn prepare(
    repo: &Repository,
    revision: &str,
    description: &str,
    options: &CommitOptions,
    cancelled: &AtomicBool,
) -> Result<Option<Prepared>> {
    if !enabled(repo, revision)? || !eligible(description)? {
        if options.entry_file.is_some() {
            return Err(
                "--entry-file exige uma mudança elegível com release.enabled = true.".into(),
            );
        }
        return Ok(None);
    }
    if repo.read(&["rev-parse", "--is-shallow-repository"])? != b"false\n" {
        return Err("releases exigem histórico completo.".into());
    }
    let parent = parent(repo, revision)?;
    let cargo = file(repo, revision, "Cargo.toml")?.ok_or("Cargo.toml ausente.")?;
    let (name, current) = version(&cargo)?;
    let previous = file(repo, &parent, "Cargo.toml")?
        .map(|s| version(&s).map(|v| v.1))
        .transpose()?
        .unwrap_or(current);
    let published = latest_published(repo, &parent)?;
    let initial = manifest(repo, &parent)?.is_none() && published.is_none();
    let next = next_version(&previous, description, initial).to_string();
    let base = published.as_ref().map(|(_, sha)| sha.clone());
    let base_tag = published.map(|(tag, _)| tag);
    let content = normalized(repo, revision)?;
    let fingerprint = fingerprint(repo, &content, base.as_deref(), base_tag.as_deref())?;
    let notes = if let Some(path) = &options.entry_file {
        changelog::validate(&provider::read_text(path, message::MAX_MESSAGE_BYTES)?)?
    } else {
        let from = if let Some(base) = &base {
            normalized(repo, base)?
        } else {
            capture(repo.command().args(["mktree"]), Vec::new())?
        };
        let diff = repo.diff_between(&from, &content).map_err(|e| {
            format!("{e}\nUse commit --entry-file para fornecer notas revisadas sem IA.")
        })?;
        let context = options
            .generation
            .context_file
            .as_ref()
            .map(|p| provider::read_text(p, 16 * 1024))
            .transpose()?
            .unwrap_or_default();
        provider::generate_with(
            &options.generation,
            &diff,
            &context,
            cancelled,
            changelog::INSTRUCTIONS,
            changelog::validate,
        )?
    };
    let existing_log = file(repo, revision, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let parent_log = file(repo, &parent, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let (_, previous_entries) = entries(&parent_log)?;
    let (plain, current_entries) = entries(&existing_log)?;
    // Regenerating an edited local candidate replaces only its own entry.
    let old = manifest(repo, revision)?;
    let expected_entries = if let Some(old) = old.as_ref()
        && file(repo, revision, MANIFEST)? != file(repo, &parent, MANIFEST)?
    {
        let prefix = format!("## {}\n\n", old.version);
        if !current_entries
            .first()
            .is_some_and(|entry| entry.starts_with(&prefix))
        {
            return Err("entrada local não corresponde à versão preparada.".into());
        }
        current_entries.get(1..).ok_or("entrada local ausente.")?
    } else {
        &current_entries
    };
    if expected_entries != previous_entries {
        return Err("entradas anteriores do changelog foram alteradas.".into());
    }
    if !plain.starts_with(HEADER) {
        return Err("CHANGELOG.md deve começar com '# Changelog' e uma linha em branco.".into());
    }
    let mut log = String::from(HEADER);
    log.push_str(&format!("{START}## {next}\n\n{notes}\n{END}"));
    for entry in &previous_entries {
        log.push_str(&format!("{START}{entry}{END}"));
    }
    log.push_str(&plain[HEADER.len()..]);
    let metadata = Manifest {
        schema: 1,
        version: next.clone(),
        parent,
        base,
        base_tag,
        fingerprint,
        message: description.into(),
        notes_hash: hash(repo, notes.as_bytes(), false)?,
    };
    let lock = file(repo, revision, "Cargo.lock")?.ok_or("Cargo.lock precisa estar versionado.")?;
    let edits = [
        (
            "Cargo.toml",
            Some(set_version(&cargo, &name, &next, false)?),
        ),
        ("Cargo.lock", Some(set_version(&lock, &name, &next, true)?)),
        ("CHANGELOG.md", Some(log)),
        (
            MANIFEST,
            Some(format!(
                "{}\n",
                serde_json::to_string_pretty(&metadata).map_err(|e| e.to_string())?
            )),
        ),
    ];
    Ok(Some(Prepared {
        tree: tree(repo, revision, &edits)?,
        version: next,
        notes,
    }))
}

pub fn check(repo: &Repository, revision: &str) -> Result<Option<(Manifest, String)>> {
    if !enabled(repo, revision)? {
        return Ok(None);
    }
    let parent = parent(repo, revision)?;
    let description = String::from_utf8(repo.read(&["show", "-s", "--format=%B", revision])?)
        .map_err(|e| e.to_string())?;
    let description = message::validate(&description)?;
    let before = file(repo, &parent, MANIFEST)?;
    let after = file(repo, revision, MANIFEST)?;
    let cargo = file(repo, revision, "Cargo.toml")?.ok_or("Cargo.toml ausente.")?;
    let (name, current) = version(&cargo)?;
    let previous = file(repo, &parent, "Cargo.toml")?
        .map(|s| version(&s).map(|v| v.1))
        .transpose()?
        .unwrap_or(current.clone());
    let lock = file(repo, revision, "Cargo.lock")?.ok_or("Cargo.lock ausente.")?;
    if lock_version(&lock, &name)? != current.to_string() {
        return Err("Cargo.lock e Cargo.toml divergem.".into());
    }
    let old_log = file(repo, &parent, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let log = file(repo, revision, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let old_entries = entries(&old_log)?.1;
    let all_entries = entries(&log)?.1;
    if !eligible(&description)? {
        if before != after || previous != current || old_entries != all_entries {
            return Err(format!("mudança interna alterou uma release. {REPAIR}"));
        }
        return Ok(None);
    }
    if after.is_none() || after == before {
        return Err(format!("release não preparada. {REPAIR}"));
    }
    let metadata = manifest(repo, revision)?.unwrap();
    let initial = before.is_none() && metadata.base.is_none();
    if metadata.schema != 1
        || metadata.parent != parent
        || metadata.message != description
        || metadata.version != current.to_string()
        || next_version(&previous, &description, initial) != current
        || metadata.fingerprint
            != fingerprint(
                repo,
                &normalized(repo, revision)?,
                metadata.base.as_deref(),
                metadata.base_tag.as_deref(),
            )?
    {
        return Err(format!(
            "release desatualizada ou versão inválida. {REPAIR}"
        ));
    }
    match (&metadata.base, &metadata.base_tag) {
        (Some(base), Some(tag)) => {
            let base = repo.resolve_commit(OsStr::new(base))?;
            if repo.read(&["merge-base", &base, &parent])? != format!("{base}\n").as_bytes()
                || !tag.starts_with('v')
                || Version::parse(&tag[1..]).is_err()
            {
                return Err("base de release inválida.".into());
            }
        }
        (None, None) => {}
        _ => return Err("base de release incompleta.".into()),
    }
    if all_entries.len() != old_entries.len() + 1 || all_entries[1..] != old_entries {
        return Err("a release precisa preservar todas as entradas anteriores.".into());
    }
    let prefix = format!("## {}\n\n", metadata.version);
    let notes = all_entries[0]
        .strip_prefix(&prefix)
        .ok_or("entrada da versão ausente.")?;
    let notes = changelog::validate(notes)?;
    if hash(repo, notes.as_bytes(), false)? != metadata.notes_hash {
        return Err(format!("notas alteradas após a revisão. {REPAIR}"));
    }
    Ok(Some((metadata, notes)))
}

pub fn revise_notes(repo: &Repository, prepared: &mut Prepared, notes: &str) -> Result<()> {
    let notes = changelog::validate(notes)?;
    let log = file(repo, &prepared.tree, "CHANGELOG.md")?.ok_or("changelog preparado ausente.")?;
    let old = format!(
        "{START}## {}\n\n{}\n{END}",
        prepared.version, prepared.notes
    );
    let new = format!("{START}## {}\n\n{notes}\n{END}", prepared.version);
    let mut metadata = manifest(repo, &prepared.tree)?.ok_or("manifesto preparado ausente.")?;
    metadata.notes_hash = hash(repo, notes.as_bytes(), false)?;
    prepared.tree = tree(
        repo,
        &prepared.tree,
        &[
            ("CHANGELOG.md", Some(log.replacen(&old, &new, 1))),
            (
                MANIFEST,
                Some(format!(
                    "{}\n",
                    serde_json::to_string_pretty(&metadata).map_err(|e| e.to_string())?
                )),
            ),
        ],
    )?;
    prepared.notes = notes;
    Ok(())
}

pub fn run(reference: &OsStr, publish: bool) -> Result<()> {
    let repo = Repository::discover()?;
    let revision = repo.resolve_commit(reference)?;
    let Some((metadata, notes)) = check(&repo, &revision)? else {
        println!("Nenhuma nova release neste commit.");
        return Ok(());
    };
    if publish {
        if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
            || std::env::var("GITHUB_SHA").as_deref() != Ok(&revision)
            || std::env::var("GITHUB_REF").as_deref() != Ok("refs/heads/main")
        {
            return Err("publique no job de release do GitHub Actions, após o CI, usando o SHA do evento na main.".into());
        }
        let host = github::Github::discover(&repo)?;
        if std::env::var("GITHUB_REPOSITORY").as_deref() != Ok(&host.repository) {
            return Err("repositório do evento diverge de origin.".into());
        }
        github::publish(
            &host,
            &revision,
            &Version::parse(&metadata.version).map_err(|e| e.to_string())?,
            &notes,
        )?;
        println!("Release v{} publicada sobre {revision}.", metadata.version);
    } else {
        println!("Release v{} verificada sobre {revision}.", metadata.version);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_policy_is_independent_of_ai_and_internal_types_do_not_release() {
        let pre = Version::new(0, 2, 3);
        let stable = Version::new(1, 2, 3);
        for (message, zero, one) in [
            ("feat: recurso", "0.3.0", "1.3.0"),
            ("fix: corrigir", "0.2.4", "1.2.4"),
            ("perf: acelerar", "0.2.4", "1.2.4"),
            ("feat!: mudar contrato", "0.3.0", "2.0.0"),
            (
                "chore: incompatibilidade\n\nBREAKING CHANGE: contrato mudou",
                "0.3.0",
                "2.0.0",
            ),
        ] {
            assert!(eligible(message).unwrap());
            assert_eq!(next_version(&pre, message, false).to_string(), zero);
            assert_eq!(next_version(&stable, message, false).to_string(), one);
            assert_eq!(next_version(&pre, message, true), pre);
        }
        for kind in [
            "docs", "chore", "ci", "test", "style", "build", "refactor", "revert",
        ] {
            assert!(!eligible(&format!("{kind}: ajuste interno")).unwrap());
        }
    }

    #[test]
    fn malformed_or_nested_changelog_markers_are_rejected() {
        for text in [
            START.to_owned(),
            END.into(),
            format!("{START}{START}text{END}"),
        ] {
            assert!(entries(&text).is_err());
        }
        assert_eq!(
            entries(&format!("before\n{START}one\n{END}after")).unwrap(),
            ("before\nafter".into(), vec!["one\n".into()])
        );
    }
}

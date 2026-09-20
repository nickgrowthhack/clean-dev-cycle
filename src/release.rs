use crate::{
    Result, changelog,
    cli::CommitOptions,
    git::{self, Repository},
    github, message, process, provider,
};
use next_version::VersionUpdater;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::PathBuf, sync::atomic::AtomicBool};

pub const MANIFEST: &str = ".clean-dev-cycle-release.json";
pub const FILES: [&str; 2] = ["CHANGELOG.md", MANIFEST];
const CONFIG: &str = "clean-dev-cycle.toml";
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

impl Manifest {
    fn render(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map(|json| format!("{json}\n"))
            .map_err(|e| e.to_string())
    }
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

fn stable(text: &str) -> Result<Version> {
    let version = Version::parse(text).map_err(|e| e.to_string())?;
    if !version.pre.is_empty() || !version.build.is_empty() || version < Version::new(0, 1, 0) {
        return Err("use uma versão estável a partir de 0.1.0, sem metadados de build.".into());
    }
    Ok(version)
}

// Reads [release] from the commit itself, so every check is deterministic per revision.
// Returns the initial version when releases are enabled.
pub fn settings(repo: &Repository, revision: &str) -> Result<Option<Version>> {
    let Some(config) = file(repo, revision, CONFIG)? else {
        return Ok(None);
    };
    let config: toml::Table = config
        .parse()
        .map_err(|e| format!("configuração inválida: {e}"))?;
    let Some(release) = config.get("release") else {
        return Ok(None);
    };
    let release = release.as_table().ok_or("[release] deve ser uma tabela.")?;
    if let Some(key) = release
        .keys()
        .find(|k| !matches!(k.as_str(), "enabled" | "initial_version"))
    {
        return Err(format!("chave desconhecida em [release]: {key}."));
    }
    let enabled = release
        .get("enabled")
        .and_then(toml::Value::as_bool)
        .ok_or("release.enabled deve ser booleano.")?;
    let initial_version = match release.get("initial_version") {
        None => Version::new(0, 1, 0),
        Some(value) => {
            let text = value
                .as_str()
                .ok_or("release.initial_version deve ser um texto.")?;
            stable(text).map_err(|e| format!("release.initial_version: {e}"))?
        }
    };
    Ok(enabled.then_some(initial_version))
}

pub fn eligible(description: &str) -> Result<bool> {
    let description = message::validate(description)?;
    let commit = git_conventional::Commit::parse(&description).map_err(|e| e.to_string())?;
    Ok(commit.breaking() || matches!(commit.type_().as_str(), "feat" | "fix" | "perf"))
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
    String::from_utf8(git::run(command, input, &process::NONE)?)
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

// The reviewed content excludes everything the release preparation itself writes.
fn normalized(repo: &Repository, revision: &str) -> Result<String> {
    let mut edits = vec![(MANIFEST, None)];
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
    let Some(text) = file(repo, revision, MANIFEST)? else {
        return Ok(None);
    };
    let manifest: Manifest =
        serde_json::from_str(&text).map_err(|e| format!("manifesto de release inválido: {e}"))?;
    stable(&manifest.version).map_err(|e| format!("manifesto de release inválido: {e}"))?;
    Ok(Some(manifest))
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

fn tag_version(tag: &str) -> Result<Version> {
    tag.strip_prefix('v')
        .ok_or("tag de release inválida.".to_owned())
        .and_then(|v| Version::parse(v).map_err(|e| e.to_string()))
}

// The version before this change: the pending stack, then the published tag, then the start.
fn previous_version(
    parent: Option<&Manifest>,
    base_tag: Option<&str>,
    initial: &Version,
) -> Result<Version> {
    if let Some(parent) = parent {
        return Version::parse(&parent.version).map_err(|e| e.to_string());
    }
    if let Some(tag) = base_tag {
        return tag_version(tag);
    }
    Ok(initial.clone())
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
    let initial_version = match settings(repo, revision)? {
        Some(initial) if eligible(description)? => initial,
        _ => {
            if options.entry_file.is_some() {
                return Err(
                    "--entry-file exige uma mudança elegível com release.enabled = true.".into(),
                );
            }
            return Ok(None);
        }
    };
    repo.ensure_full_history()?;
    let parent = parent(repo, revision)?;
    let published = latest_published(repo, &parent)?;
    let parent_manifest = manifest(repo, &parent)?;
    let initial = parent_manifest.is_none() && published.is_none();
    let base = published.as_ref().map(|(_, sha)| sha.clone());
    let base_tag = published.map(|(tag, _)| tag);
    let previous = previous_version(
        parent_manifest.as_ref(),
        base_tag.as_deref(),
        &initial_version,
    )?;
    let next = next_version(&previous, description, initial).to_string();
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
        provider::generate(
            &options.generation,
            &diff,
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
    let edits = [
        ("CHANGELOG.md", Some(log)),
        (MANIFEST, Some(metadata.render()?)),
    ];
    Ok(Some(Prepared {
        tree: tree(repo, revision, &edits)?,
        version: next,
        notes,
    }))
}

pub fn check(repo: &Repository, revision: &str) -> Result<Option<(Manifest, String)>> {
    let Some(initial_version) = settings(repo, revision)? else {
        return Ok(None);
    };
    let parent = parent(repo, revision)?;
    let description = String::from_utf8(repo.read(&["show", "-s", "--format=%B", revision])?)
        .map_err(|e| e.to_string())?;
    let description = message::validate(&description)?;
    let before = file(repo, &parent, MANIFEST)?;
    let after = file(repo, revision, MANIFEST)?;
    let old_log = file(repo, &parent, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let log = file(repo, revision, "CHANGELOG.md")?.unwrap_or_else(|| HEADER.into());
    let old_entries = entries(&old_log)?.1;
    let all_entries = entries(&log)?.1;
    if !eligible(&description)? {
        if before != after || old_entries != all_entries {
            return Err(format!("mudança interna alterou uma release. {REPAIR}"));
        }
        return Ok(None);
    }
    if after.is_none() || after == before {
        return Err(format!("release não preparada. {REPAIR}"));
    }
    let metadata = manifest(repo, revision)?.unwrap();
    let current = Version::parse(&metadata.version).map_err(|e| e.to_string())?;
    let parent_manifest = manifest(repo, &parent)?;
    let initial = parent_manifest.is_none() && metadata.base.is_none();
    let previous = previous_version(
        parent_manifest.as_ref(),
        metadata.base_tag.as_deref(),
        &initial_version,
    )?;
    if metadata.schema != 1
        || metadata.parent != parent
        || metadata.message != description
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
                || tag_version(tag).is_err()
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
            (MANIFEST, Some(metadata.render()?)),
        ],
    )?;
    prepared.notes = notes;
    Ok(())
}

pub fn run(reference: &OsStr, publish: bool, assets: &[PathBuf]) -> Result<()> {
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
        let version = Version::parse(&metadata.version).map_err(|e| e.to_string())?;
        let release = github::publish(&host, &revision, &version, &notes)?;
        println!("Release v{version} publicada sobre {revision}.");
        if !assets.is_empty() {
            github::upload_assets(&host, &format!("v{version}"), &release, assets)?;
        }
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
    fn previous_version_prefers_pending_stack_then_published_tag_then_start() {
        let initial = Version::new(1, 2, 0);
        let parent = Manifest {
            schema: 1,
            version: "0.7.0".into(),
            parent: String::new(),
            base: None,
            base_tag: None,
            fingerprint: String::new(),
            message: String::new(),
            notes_hash: String::new(),
        };
        assert_eq!(
            previous_version(Some(&parent), Some("v0.4.0"), &initial).unwrap(),
            Version::new(0, 7, 0)
        );
        assert_eq!(
            previous_version(None, Some("v0.4.0"), &initial).unwrap(),
            Version::new(0, 4, 0)
        );
        assert_eq!(
            previous_version(None, None, &initial).unwrap(),
            Version::new(1, 2, 0)
        );
        assert!(previous_version(None, Some("0.4.0"), &initial).is_err());
        for invalid in ["0.0.9", "1.0.0-rc1", "1.0.0+build", "x"] {
            assert!(stable(invalid).is_err(), "{invalid}");
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

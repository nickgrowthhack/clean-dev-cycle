use crate::{Result, git::Repository, process};
use semver::Version;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub struct Github {
    pub repository: String,
}

// Kept at the API boundary so interruption and retry behavior can be tested offline.
pub trait Hosting {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>>;
    fn upload(&self, tag: &str, path: &Path) -> Result<()>;
}

impl Github {
    pub fn discover(repo: &Repository) -> Result<Self> {
        let remote = String::from_utf8(repo.read(&["remote", "get-url", "origin"])?)
            .map_err(|_| "origin inválido.")?;
        let name = remote
            .trim()
            .strip_prefix("git@github.com:")
            .or_else(|| remote.trim().strip_prefix("https://github.com/"))
            .or_else(|| remote.trim().strip_prefix("ssh://git@github.com/"))
            .ok_or("releases exigem origin no github.com e gh autenticado.")?
            .trim_end_matches(".git");
        let parts: Vec<_> = name.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
            })
        {
            return Err("origin precisa identificar OWNER/REPO no GitHub.".into());
        }
        Ok(Self {
            repository: name.into(),
        })
    }
}

impl Hosting for Github {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let mut command = Command::new("gh");
        command
            .args(["api", "--hostname", "github.com", "--method", method])
            .arg(format!("repos/{}/{}", self.repository, path))
            .env("GH_PROMPT_DISABLED", "1");
        let input = if let Some(body) = body {
            command.args(["--input", "-"]);
            serde_json::to_vec(&body).map_err(|e| e.to_string())?
        } else {
            Vec::new()
        };
        let output = process::capture(
            &mut command,
            input,
            Duration::from_secs(120),
            8 * 1024 * 1024,
            &process::NONE,
        )?;
        if !output.status.success() {
            let error = process::diagnostic(&output.stderr);
            if method == "GET" && error.contains("HTTP 404") {
                return Ok(None);
            }
            return Err(format!("GitHub: {error}"));
        }
        serde_json::from_slice(&output.stdout)
            .map(Some)
            .map_err(|_| "resposta inválida do GitHub.".into())
    }

    fn upload(&self, tag: &str, path: &Path) -> Result<()> {
        let mut command = Command::new("gh");
        command
            .args(["release", "upload", "--repo", &self.repository, tag])
            .arg(path)
            .env("GH_PROMPT_DISABLED", "1");
        let output = process::capture(
            &mut command,
            Vec::new(),
            Duration::from_secs(10 * 60),
            4 * 1024 * 1024,
            &process::NONE,
        )?;
        if !output.status.success() {
            return Err(format!("GitHub: {}", process::diagnostic(&output.stderr)));
        }
        Ok(())
    }
}

pub fn releases(host: &impl Hosting) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    for page in 1..=1000 {
        let value = host
            .request("GET", &format!("releases?per_page=100&page={page}"), None)?
            .ok_or("não foi possível listar releases; confira a autenticação do gh.")?;
        let entries = value.as_array().ok_or("lista de releases inválida.")?;
        result.extend(entries.iter().cloned());
        if entries.len() < 100 {
            return Ok(result);
        }
    }
    Err("lista de releases excedeu o limite de paginação.".into())
}

pub fn version(release: &Value) -> Option<Version> {
    if release["draft"].as_bool()? || release["prerelease"].as_bool()? {
        return None;
    }
    let version = Version::parse(release["tag_name"].as_str()?.strip_prefix('v')?).ok()?;
    if version.pre.is_empty() && version.build.is_empty() {
        Some(version)
    } else {
        None
    }
}

pub fn tag_commit(host: &impl Hosting, tag: &str) -> Result<Option<String>> {
    let Some(mut value) = host.request("GET", &format!("git/ref/tags/{tag}"), None)? else {
        return Ok(None);
    };
    for _ in 0..8 {
        let object = &value["object"];
        let sha = object["sha"].as_str().ok_or("tag sem SHA.")?;
        if !matches!(sha.len(), 40 | 64) || !sha.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("SHA inválido na tag.".into());
        }
        match object["type"].as_str() {
            Some("commit") => return Ok(Some(sha.into())),
            Some("tag") => {
                value = host
                    .request("GET", &format!("git/tags/{sha}"), None)?
                    .ok_or("tag anotada ausente.")?
            }
            _ => return Err("a tag não aponta para um commit.".into()),
        }
    }
    Err("encadeamento de tags excedeu o limite.".into())
}

pub fn publish(host: &impl Hosting, sha: &str, version: &Version, notes: &str) -> Result<Value> {
    let tag = format!("v{version}");
    match tag_commit(host, &tag)? {
        Some(existing) if existing != sha => {
            return Err(format!(
                "a tag {tag} já aponta para outro SHA; nada será sobrescrito."
            ));
        }
        Some(_) => {}
        None => {
            let create = host.request(
                "POST",
                "git/refs",
                Some(json!({"ref": format!("refs/tags/{tag}"), "sha": sha})),
            );
            // The request may have succeeded before the connection was interrupted.
            if tag_commit(host, &tag)?.as_deref() != Some(sha) {
                create?;
                return Err("a tag não foi confirmada no SHA aprovado.".into());
            }
        }
    }
    if let Some(existing) = host.request("GET", &format!("releases/tags/{tag}"), None)? {
        verify_release(&existing, &tag, notes)?;
        return Ok(existing);
    }
    let newer = releases(host)?
        .iter()
        .filter_map(self::version)
        .any(|v| v > *version);
    let created = host.request(
        "POST",
        "releases",
        Some(json!({
            "tag_name": tag, "target_commitish": sha, "name": tag, "body": notes,
            "draft": false, "prerelease": false, "make_latest": if newer { "false" } else { "true" }
        })),
    );
    let existing = host.request("GET", &format!("releases/tags/{tag}"), None)?;
    if let Some(existing) = existing {
        verify_release(&existing, &tag, notes)?;
        return Ok(existing);
    }
    created?;
    Err("a release ainda não foi confirmada. Reexecute o job de publicação.".into())
}

fn asset_size(host: &impl Hosting, id: u64, name: &str) -> Result<Option<u64>> {
    let value = host
        .request("GET", &format!("releases/{id}/assets?per_page=100"), None)?
        .ok_or("não foi possível listar os assets da release.")?;
    for asset in value.as_array().ok_or("lista de assets inválida.")? {
        if asset["name"] == name {
            return asset["size"]
                .as_u64()
                .map(Some)
                .ok_or("asset sem tamanho.".into());
        }
    }
    Ok(None)
}

// Assets derive from the approved SHA, so a rerun completes the set without replacing anything.
pub fn upload_assets(
    host: &impl Hosting,
    tag: &str,
    release: &Value,
    paths: &[PathBuf],
) -> Result<()> {
    let id = release["id"].as_u64().ok_or("release sem identificador.")?;
    for path in paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty() && !n.contains('#'))
            .ok_or_else(|| format!("nome de asset inválido: {}.", path.display()))?;
        let size = std::fs::metadata(path)
            .map_err(|e| format!("asset {name}: {e}."))?
            .len();
        if size == 0 {
            return Err(format!("asset {name} está vazio."));
        }
        match asset_size(host, id, name)? {
            Some(existing) if existing == size => {
                println!("Asset {name} já enviado.");
                continue;
            }
            Some(_) => {
                return Err(format!(
                    "asset {name} já existe com outro tamanho; nada será sobrescrito."
                ));
            }
            None => {}
        }
        host.upload(tag, path)?;
        if asset_size(host, id, name)? != Some(size) {
            return Err(format!("asset {name} não foi confirmado na release."));
        }
        println!("Asset {name} enviado.");
    }
    Ok(())
}

fn verify_release(value: &Value, tag: &str, notes: &str) -> Result<()> {
    if value["tag_name"] != tag
        || value["name"] != tag
        || value["body"] != notes
        || value["draft"] != false
        || value["prerelease"] != false
    {
        return Err(format!(
            "release {tag} existente diverge das notas preparadas; nada será sobrescrito."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct State {
        tag: Option<String>,
        release: Option<Value>,
        posts: usize,
        fail_release: bool,
        lost_response: bool,
        newer: bool,
        assets: Vec<(String, u64)>,
        uploads: usize,
    }
    #[derive(Default)]
    struct Fake(RefCell<State>);
    impl Hosting for Fake {
        fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
            let mut state = self.0.borrow_mut();
            match (method, path) {
                ("GET", p) if p.starts_with("git/ref/tags/") => Ok(state
                    .tag
                    .as_ref()
                    .map(|sha| json!({"object":{"type":"commit","sha":sha}}))),
                ("GET", p) if p.starts_with("releases/tags/") => Ok(state.release.clone()),
                ("GET", "releases/7/assets?per_page=100") => Ok(Some(json!(
                    state
                        .assets
                        .iter()
                        .map(|(name, size)| json!({"name": name, "size": size}))
                        .collect::<Vec<_>>()
                ))),
                ("GET", p) if p.starts_with("releases?") => Ok(Some(if state.newer {
                    json!([{"tag_name":"v1.0.0","draft":false,"prerelease":false}])
                } else {
                    json!([])
                })),
                ("POST", "git/refs") => {
                    state.posts += 1;
                    state.tag = Some(body.unwrap()["sha"].as_str().unwrap().into());
                    Ok(Some(json!({})))
                }
                ("POST", "releases") => {
                    state.posts += 1;
                    if state.fail_release {
                        return Err("interrupção".into());
                    }
                    let mut release = body.unwrap();
                    release["id"] = json!(7);
                    state.release = Some(release.clone());
                    if state.lost_response {
                        return Err("resposta perdida".into());
                    }
                    Ok(Some(release))
                }
                _ => panic!("requisição inesperada: {method} {path}"),
            }
        }
        fn upload(&self, tag: &str, path: &Path) -> Result<()> {
            assert_eq!(tag, "v0.1.0");
            let mut state = self.0.borrow_mut();
            state.uploads += 1;
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            let size = std::fs::metadata(path).unwrap().len();
            state.assets.push((name, size));
            Ok(())
        }
    }
    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn assets_are_uploaded_once_and_never_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("cli-linux.tar.gz");
        let second = directory.path().join("cli-windows.zip");
        std::fs::write(&first, b"linux").unwrap();
        std::fs::write(&second, b"windows").unwrap();
        let host = Fake::default();
        let release = publish(&host, SHA, &Version::new(0, 1, 0), "notas").unwrap();
        let paths = [first.clone(), second.clone()];
        upload_assets(&host, "v0.1.0", &release, &paths).unwrap();
        assert_eq!(host.0.borrow().uploads, 2);
        upload_assets(&host, "v0.1.0", &release, &paths).unwrap();
        assert_eq!(host.0.borrow().uploads, 2);
        std::fs::write(&second, b"rebuilt differently").unwrap();
        assert!(upload_assets(&host, "v0.1.0", &release, &paths).is_err());
        assert_eq!(host.0.borrow().uploads, 2);
        let empty = directory.path().join("empty.zip");
        std::fs::write(&empty, b"").unwrap();
        assert!(upload_assets(&host, "v0.1.0", &release, &[empty]).is_err());
        assert!(upload_assets(&host, "v0.1.0", &release, &[directory.path().join("x")]).is_err());
    }
    #[test]
    fn interrupted_publication_resumes_without_recreating_tag_or_notes() {
        let host = Fake::default();
        host.0.borrow_mut().fail_release = true;
        assert!(publish(&host, SHA, &Version::new(0, 1, 0), "notas").is_err());
        assert_eq!(host.0.borrow().tag.as_deref(), Some(SHA));
        host.0.borrow_mut().fail_release = false;
        publish(&host, SHA, &Version::new(0, 1, 0), "notas").unwrap();
        let posts = host.0.borrow().posts;
        publish(&host, SHA, &Version::new(0, 1, 0), "notas").unwrap();
        assert_eq!(host.0.borrow().posts, posts);
        assert!(publish(&host, SHA, &Version::new(0, 1, 0), "divergentes").is_err());
    }
    #[test]
    fn conflicting_tag_is_never_overwritten() {
        let host = Fake::default();
        host.0.borrow_mut().tag = Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into());
        assert!(publish(&host, SHA, &Version::new(0, 1, 0), "notas").is_err());
        assert_eq!(host.0.borrow().posts, 0);
    }
    #[test]
    fn delayed_release_does_not_replace_latest_and_lost_response_is_reconciled() {
        let host = Fake::default();
        host.0.borrow_mut().newer = true;
        host.0.borrow_mut().lost_response = true;
        publish(&host, SHA, &Version::new(0, 1, 0), "notas").unwrap();
        assert_eq!(
            host.0.borrow().release.as_ref().unwrap()["make_latest"],
            "false"
        );
    }
}

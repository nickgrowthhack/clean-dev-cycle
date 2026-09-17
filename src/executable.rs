use crate::Result;
#[cfg(windows)]
use std::fs;
use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub fn codex(requested: &OsStr) -> Result<PathBuf> {
    let current = env::current_dir().map_err(|error| error.to_string())?;
    resolve(
        requested,
        env::var_os("PATH").as_deref(),
        &current,
        env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
    )
}

fn resolve(
    requested: &OsStr,
    search_path: Option<&OsStr>,
    current: &Path,
    local_app_data: Option<&Path>,
) -> Result<PathBuf> {
    let requested_path = Path::new(requested);
    if requested_path.components().count() > 1 {
        let path = if requested_path.is_absolute() {
            requested_path.to_owned()
        } else {
            current.join(requested_path)
        };
        return executable(&path).then_some(path).ok_or_else(|| format!("não foi possível executar {}: arquivo ausente ou não executável. Confira --codex CAMINHO.", requested_path.display()));
    }
    if let Some(path) = search_path.and_then(|paths| on_path(requested, paths, current)) {
        return Ok(path);
    }
    #[cfg(windows)]
    if requested == "codex"
        && let Some(local_app_data) = local_app_data
        && let Some(path) = desktop_codex(local_app_data)?
    {
        return Ok(path);
    }
    #[cfg(not(windows))]
    let _ = local_app_data;
    Err(format!(
        "não foi possível executar {}: comando não encontrado. Instale o Codex CLI no PATH ou informe --codex CAMINHO.",
        requested.to_string_lossy()
    ))
}

fn on_path(name: &OsStr, search_path: &OsStr, current: &Path) -> Option<PathBuf> {
    for directory in env::split_paths(search_path).filter(|path| !path.as_os_str().is_empty()) {
        let directory = if directory.is_absolute() {
            directory
        } else {
            current.join(directory)
        };
        let path = directory.join(name);
        #[cfg(windows)]
        {
            if path.extension().is_some() {
                if executable(&path) {
                    return Some(path);
                }
            } else {
                for extension in ["exe", "com", "cmd", "bat"] {
                    let candidate = path.with_extension(extension);
                    if executable(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }
        #[cfg(not(windows))]
        if executable(&path) {
            return Some(path);
        }
    }
    None
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(windows)]
fn desktop_codex(local_app_data: &Path) -> Result<Option<PathBuf>> {
    let root = local_app_data.join("OpenAI/Codex/bin");
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "não foi possível consultar a instalação do aplicativo Codex: {error}. Informe --codex CAMINHO."
            ));
        }
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path().join("codex.exe");
        if executable(&path) {
            candidates.push(path);
        }
    }
    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.pop()),
        _ => Err("há mais de uma versão do Codex no aplicativo. Escolha uma com --codex CAMINHO ou instale o Codex CLI no PATH.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_executable(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "executável de exemplo").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }

    #[test]
    fn explicit_paths_are_resolved_before_changing_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pasta d'ação/provedor.exe");
        create_executable(&path);
        let result = resolve(
            OsStr::new("./pasta d'ação/provedor.exe"),
            None,
            directory.path(),
            None,
        )
        .unwrap();
        assert_eq!(result.canonicalize().unwrap(), path.canonicalize().unwrap());
        assert!(
            resolve(OsStr::new("./ausente.exe"), None, directory.path(), None)
                .unwrap_err()
                .contains("--codex")
        );
    }

    #[test]
    fn path_resolution_ignores_unlisted_working_directory() {
        let directory = tempfile::tempdir().unwrap();
        let name = format!("codex{}", env::consts::EXE_SUFFIX);
        create_executable(&directory.path().join(&name));
        let tools = directory.path().join("tools");
        let from_path = tools.join(name);
        create_executable(&from_path);
        assert_eq!(
            resolve(
                OsStr::new("codex"),
                Some(tools.as_os_str()),
                directory.path(),
                None
            )
            .unwrap(),
            from_path
        );
        assert!(resolve(OsStr::new("codex"), None, directory.path(), None).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn desktop_is_used_only_when_default_command_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let installed = directory.path().join("OpenAI/Codex/bin/versao/codex.exe");
        create_executable(&installed);
        fs::create_dir_all(directory.path().join("OpenAI/Codex/bin/outro-componente")).unwrap();
        assert_eq!(
            resolve(
                OsStr::new("codex"),
                None,
                directory.path(),
                Some(directory.path())
            )
            .unwrap(),
            installed
        );
        let tools = directory.path().join("tools");
        let from_path = tools.join("codex.cmd");
        create_executable(&from_path);
        assert_eq!(
            resolve(
                OsStr::new("codex"),
                Some(tools.as_os_str()),
                directory.path(),
                Some(directory.path())
            )
            .unwrap(),
            from_path
        );
        assert!(
            resolve(
                OsStr::new("outro-codex"),
                None,
                directory.path(),
                Some(directory.path())
            )
            .is_err()
        );
        create_executable(
            &directory
                .path()
                .join("OpenAI/Codex/bin/outra-versao/codex.exe"),
        );
        assert!(
            resolve(
                OsStr::new("codex"),
                None,
                directory.path(),
                Some(directory.path())
            )
            .unwrap_err()
            .contains("mais de uma versão")
        );
    }
}

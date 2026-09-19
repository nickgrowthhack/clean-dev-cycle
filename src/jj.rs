use crate::{Result, git::Repository, process};
use serde_json::Value;
use std::{ffi::OsStr, process::Command, sync::atomic::AtomicBool, time::Duration};

pub struct Jujutsu {
    pub git: Repository,
    workspace: Option<std::path::PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Revision {
    pub id: String,
    pub change: String,
    pub parents: Vec<String>,
    pub description: String,
    pub author_complete: bool,
    pub committer_complete: bool,
}

#[derive(PartialEq, Eq)]
pub struct Snapshot {
    pub operation: String,
    pub revision: Revision,
}

impl Jujutsu {
    pub fn discover() -> Result<Self> {
        let git = Repository::discover()?;
        if !git.root.join(".jj").is_dir() {
            return Err(
                "execute em um workspace Jujutsu colocated (jj git init --colocate).".into(),
            );
        }
        let repo = Self {
            git,
            workspace: None,
        };
        repo.run(&["root"])?;
        Ok(repo)
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        self.execute(args.iter().map(OsStr::new))
    }

    fn execute<'a>(&self, args: impl IntoIterator<Item = &'a OsStr>) -> Result<String> {
        let args: Vec<_> = args.into_iter().collect();
        let isolated = args.contains(&OsStr::new("--no-integrate-operation"));
        let mut command = Command::new("jj");
        command
            .current_dir(self.workspace.as_ref().unwrap_or(&self.git.root))
            .args(["--no-pager", "--color=never"])
            .args(args);
        let output = process::capture(
            &mut command,
            Vec::new(),
            Duration::from_secs(120),
            4 * 1024 * 1024,
            &AtomicBool::new(false),
        )?;
        if !output.status.success() {
            return Err(format!("Jujutsu: {}", process::diagnostic(&output.stderr)));
        }
        if isolated {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let operation = stderr.lines().find_map(|line| line.strip_prefix("Operation left uncommitted because --no-integrate-operation was requested: "))
                .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(|| format!("jj 0.45.1 não retornou a operação isolada: {stderr}"))?;
            return Ok(operation.into());
        }
        String::from_utf8(output.stdout).map_err(|_| "saída do jj não está em UTF-8.".into())
    }

    pub fn revision(&self, revision: &OsStr, operation: Option<&str>) -> Result<Revision> {
        let mut args = vec![OsStr::new("--ignore-working-copy")];
        if let Some(op) = operation {
            args.extend([OsStr::new("--at-operation"), OsStr::new(op)]);
        }
        args.extend([
            OsStr::new("log"),
            OsStr::new("--no-graph"),
            OsStr::new("-r"),
            revision,
            OsStr::new("-T"),
            OsStr::new("json(self)"),
        ]);
        let output = self.execute(args)?;
        let value: Value = serde_json::from_str(&output)
            .map_err(|_| "a revisão deve selecionar exatamente uma mudança.".to_owned())?;
        let text = |field: &str| {
            value[field]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("campo ausente na revisão jj: {field}."))
        };
        let parents = value["parents"]
            .as_array()
            .ok_or("pais ausentes na revisão jj.")?
            .iter()
            .map(|p| {
                p.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "pai inválido.".to_owned())
            })
            .collect::<Result<Vec<_>>>()?;
        let complete = |role: &str| {
            ["name", "email"].iter().all(|field| {
                value[role][field]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty())
            })
        };
        Ok(Revision {
            id: text("commit_id")?,
            change: text("change_id")?,
            parents,
            description: text("description")?,
            author_complete: complete("author"),
            committer_complete: complete("committer"),
        })
    }

    pub fn ensure_configured_identity(&self) -> Result<()> {
        for field in ["user.name", "user.email"] {
            if self
                .run(&["--ignore-working-copy", "config", "get", field])?
                .trim()
                .is_empty()
            {
                return Err(format!(
                    "identidade do Jujutsu incompleta ({field}). Configure nome e e-mail com jj config set --repo user.name NOME e jj config set --repo user.email EMAIL antes de gerar a mensagem."
                ));
            }
        }
        Ok(())
    }

    pub fn ensure_author(&self, revision: &Revision) -> Result<()> {
        if !revision.author_complete {
            return Err(format!(
                "a mudança tem autor incompleto. Configure sua identidade e, se a mudança for sua, execute jj metaedit -r {} --update-author antes de continuar.",
                revision.change
            ));
        }
        Ok(())
    }

    pub fn ensure_publishable_identity(&self, revision: &Revision) -> Result<()> {
        self.ensure_author(revision)?;
        if !revision.committer_complete {
            return Err(format!(
                "a mudança tem identidade de committer incompleta. Configure sua identidade e execute jj metaedit -r {} --force-rewrite para atualizar os metadados sem alterar o conteúdo ou a mensagem.",
                revision.change
            ));
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        self.run(&["status"])?;
        let operation = self.run(&[
            "--ignore-working-copy",
            "op",
            "log",
            "--limit",
            "1",
            "--no-graph",
            "-T",
            "id",
        ])?;
        let revision = self.revision(OsStr::new("@"), Some(&operation))?;
        self.ensure_no_conflicts(&revision)?;
        Ok(Snapshot {
            operation,
            revision,
        })
    }

    pub fn ensure_unchanged(&self, snapshot: &Snapshot) -> Result<()> {
        if self.snapshot()? != *snapshot {
            return Err("a mudança ou o estado do Jujutsu mudou durante a revisão. Revise e execute novamente.".into());
        }
        Ok(())
    }

    pub fn ensure_no_conflicts(&self, revision: &Revision) -> Result<()> {
        let object = self.git.read(&["cat-file", "commit", &revision.id])?;
        let text = String::from_utf8_lossy(&object);
        let headers = text.split_once("\n\n").ok_or("commit inválido.")?.0;
        if headers.lines().any(|line| line.starts_with("jj:trees ")) {
            return Err("a mudança contém conflitos. Resolva-os com jj antes de continuar.".into());
        }
        Ok(())
    }

    pub fn diff(&self, revision: &Revision) -> Result<String> {
        let base = match revision.parents.as_slice() {
            [] => String::from_utf8(self.git.read(&["hash-object", "-t", "tree", "--stdin"])?)
                .map_err(|_| "árvore vazia inválida.")?
                .trim()
                .to_owned(),
            [parent] if parent.chars().all(|c| c == '0') => {
                String::from_utf8(self.git.read(&["hash-object", "-t", "tree", "--stdin"])?)
                    .map_err(|_| "árvore vazia inválida.")?
                    .trim()
                    .to_owned()
            }
            [parent] => parent.clone(),
            _ => return Err("conclua uma mudança linear, sem commit de merge.".into()),
        };
        self.git.diff_between(&base, &revision.id)
    }

    pub fn finish(&self, snapshot: &Snapshot, message: &str) -> Result<()> {
        self.ensure_configured_identity()?;
        self.ensure_unchanged(snapshot)?;
        // Pin the operation: even edits arriving after the check cannot enter this commit.
        // Jujutsu retains later working-copy edits in the new change when it next snapshots.
        self.run(&[
            "--at-operation",
            &snapshot.operation,
            "commit",
            "--message",
            message,
        ])?;
        self.run(&["status"])?;
        Ok(())
    }

    pub fn finish_release(
        &self,
        snapshot: &Snapshot,
        message: &str,
        tree: &str,
        cancelled: &AtomicBool,
    ) -> Result<()> {
        self.ensure_configured_identity()?;
        self.ensure_unchanged(snapshot)?;
        let directory = tempfile::Builder::new()
            .prefix("cdc-release-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let name = directory
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("nome temporário inválido.")?;
        let root = directory.path().join("workspace");
        self.run(&[
            "--at-operation",
            &snapshot.operation,
            "workspace",
            "add",
            "--name",
            name,
            "--revision",
            &snapshot.revision.id,
            root.to_str().ok_or("caminho temporário inválido.")?,
        ])?;
        let isolated = Self {
            git: Repository {
                root: self.git.root.clone(),
            },
            workspace: Some(root.clone()),
        };
        let stage = (|| {
            for path in crate::release::FILES {
                let content = crate::release::file(&self.git, tree, path)?
                    .ok_or("arquivo preparado ausente.")?;
                std::fs::write(root.join(path), content).map_err(|e| e.to_string())?;
            }
            isolated
                .snapshot()
                .map_err(|e| format!("workspace temporário: {e}"))
        })();
        // Forget the temporary workspace on every path. The user's workspace is never edited here.
        let cleanup = self.run(&["--ignore-working-copy", "workspace", "forget", name]);
        let stage = stage?;
        cleanup?;
        let expected = self
            .snapshot()
            .map_err(|e| format!("workspace original antes da integração: {e}"))?;
        if expected.revision != snapshot.revision {
            return Err(
                "a mudança mudou durante a preparação isolada. Revise e execute novamente.".into(),
            );
        }
        let squashed = self.run(&[
            "--at-operation",
            &expected.operation,
            "--no-integrate-operation",
            "squash",
            "--from",
            &stage.revision.id,
            "--into",
            &snapshot.revision.id,
            "--message",
            message,
        ])?;
        let finished = self.run(&[
            "--at-operation",
            squashed.trim(),
            "--no-integrate-operation",
            "commit",
            "--message",
            message,
        ])?;
        let finished = finished.trim();
        let revision = self.revision(OsStr::new("@-"), Some(finished))?;
        if self
            .git
            .read(&["rev-parse", &format!("{}^{{tree}}", revision.id)])?
            != format!("{tree}\n").as_bytes()
        {
            return Err(
                "a operação isolada diverge dos arquivos revisados; nada foi integrado.".into(),
            );
        }
        crate::release::check(&self.git, &revision.id)?;
        self.ensure_unchanged(&expected)?;
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("operação cancelada.".into());
        }
        // Integrate the already completed transaction. Late edits stay in the next change.
        self.run(&[
            "--at-operation",
            &expected.operation,
            "op",
            "integrate",
            finished,
        ])?;
        self.run(&["workspace", "update-stale"])?;
        self.run(&["status"])?;
        Ok(())
    }
}

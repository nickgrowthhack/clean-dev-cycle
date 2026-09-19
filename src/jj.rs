use crate::{Result, git::Repository, process};
use serde_json::Value;
use std::{ffi::OsStr, process::Command, sync::atomic::AtomicBool, time::Duration};

pub struct Jujutsu {
    pub git: Repository,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Revision {
    pub id: String,
    pub change: String,
    pub parents: Vec<String>,
    pub description: String,
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
        let repo = Self { git };
        repo.run(&["root"])?;
        Ok(repo)
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        self.execute(args.iter().map(OsStr::new))
    }

    fn execute<'a>(&self, args: impl IntoIterator<Item = &'a OsStr>) -> Result<String> {
        let mut command = Command::new("jj");
        command
            .current_dir(&self.git.root)
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
        Ok(Revision {
            id: text("commit_id")?,
            change: text("change_id")?,
            parents,
            description: text("description")?,
        })
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
}

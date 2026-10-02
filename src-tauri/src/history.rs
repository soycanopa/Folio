use std::collections::HashSet;

use serde::Serialize;

use crate::state::{AppState, Project};

/// Commits recientes que tocaron un archivo (UX.md: "pocos commits").
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CommitInfo {
    pub oid: String,
    pub message: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    /// true si el commit no llegó al upstream: se etiqueta local.
    pub local: bool,
    /// Link al commit en GitHub; solo en modo remoto (su entry-history
    /// enlaza html_url).
    pub html_url: Option<String>,
}

const MAX_COMMITS: usize = 20;

pub fn file_history_impl(
    repo: &git2::Repository,
    path: &str,
) -> Result<Vec<CommitInfo>, String> {
    let mut walker = repo.revwalk().map_err(|e| format!("revwalk: {e}"))?;
    walker.push_head().map_err(|e| format!("HEAD: {e}"))?;

    // Lo alcanzable desde el tracking ref del upstream llegó al remoto;
    // el resto es local (si no hay upstream, todo es local).
    let mut pushed: HashSet<git2::Oid> = HashSet::new();
    if let Ok(head_ref) = repo.head() {
        let branch = git2::Branch::wrap(head_ref);
        if let Ok(upstream) = branch.upstream() {
            if let Ok(upstream_commit) = upstream.get().peel_to_commit() {
                if let Ok(mut w) = repo.revwalk() {
                    if w.push(upstream_commit.id()).is_ok() {
                        for oid in w.flatten() {
                            pushed.insert(oid);
                        }
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    for oid in walker.flatten() {
        let commit = repo
            .find_commit(oid)
            .map_err(|e| format!("commit {oid}: {e}"))?;
        let touched = {
            let tree = commit.tree().map_err(|e| format!("tree: {e}"))?;
            let parent_tree = if commit.parent_count() > 0 {
                Some(
                    commit
                        .parent(0)
                        .map_err(|e| format!("parent: {e}"))?
                        .tree()
                        .map_err(|e| format!("tree padre: {e}"))?,
                )
            } else {
                None
            };
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(path);
            let diff = repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
                .map_err(|e| format!("diff: {e}"))?;
            diff.deltas().next().is_some()
        };
        if touched {
            out.push(CommitInfo {
                oid: oid.to_string(),
                message: commit
                    .summary()
                    .ok()
                    .flatten()
                    .map(str::to_string)
                    .unwrap_or_default(),
                author: commit.author().name().unwrap_or("?").to_string(),
                time: commit.time().seconds(),
                local: !pushed.contains(&oid),
                html_url: None,
            });
            if out.len() >= MAX_COMMITS {
                break;
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn file_history(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<Vec<CommitInfo>, String> {
    let ctx = {
        let guard = state.lock().unwrap();
        match guard.as_ref().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => return file_history_impl(&st.repo, &path),
            Project::Remote(rs) => crate::remote::remote_ctx(rs),
        }
    };
    let token = crate::remote::current_token()?;
    let (t, c, p) = (token.clone(), ctx.clone(), path.clone());
    // Su entries/[path]/history: /commits?path=&sha={branch}. En remoto
    // nada queda "local": todo vive en la rama.
    tauri::async_runtime::spawn_blocking(move || {
        let commits =
            crate::gh_api::list_commits(&t, &c.owner, &c.repo, &p, &c.branch, MAX_COMMITS)
                .map_err(crate::remote::gh_error_ui)?;
        Ok(commits
            .into_iter()
            .map(|gc| {
                let author = gc.author_name();
                let time = crate::gh_api::iso_to_unix(&gc.commit.author.date).unwrap_or(0);
                CommitInfo {
                    oid: gc.sha,
                    message: gc.commit.message,
                    author,
                    time,
                    local: false,
                    html_url: Some(gc.html_url),
                }
            })
            .collect())
    })
    .await
    .map_err(|e| format!("historial: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    fn commit_file(repo: &git2::Repository, root: &Path, rel: &str, content: &str, msg: &str) -> String {
        let full = root.join(rel);
        if let Some(p) = full.parent() {
            fs::create_dir_all(p).unwrap();
        }
        fs::write(&full, content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(rel)).unwrap();
        let tree_oid = index.write_tree().unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@test").unwrap();
        let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = head.iter().collect();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
        oid.to_string()
    }

    #[test]
    fn historia_solo_del_archivo_pedidos() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let c1 = commit_file(&repo, dir.path(), "src/a.md", "v1\n", "feat: crea a");
        commit_file(&repo, dir.path(), "src/b.md", "x\n", "feat: crea b");
        let c3 = commit_file(&repo, dir.path(), "src/a.md", "v2\n", "fix: toca a");

        let h = file_history_impl(&repo, "src/a.md").unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].oid, c3);
        assert_eq!(h[0].message, "fix: toca a");
        assert_eq!(h[1].oid, c1);
        // Sin upstream: todo local.
        assert!(h.iter().all(|c| c.local));

        assert!(file_history_impl(&repo, "src/nope.md").unwrap().is_empty());
    }

    #[test]
    fn local_solo_lo_que_no_llego_al_upstream() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let c1 = commit_file(&repo, dir.path(), "src/a.md", "v1\n", "feat: crea a");

        // Remote bare local + upstream, como `git push -u`.
        let bare = tempfile::tempdir().unwrap();
        git2::Repository::init_bare(bare.path()).unwrap();
        repo.remote("origin", bare.path().to_str().unwrap()).unwrap();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        let mut cfg = repo.config().unwrap();
        cfg.set_str(&format!("branch.{branch}.remote"), "origin").unwrap();
        cfg.set_str(&format!("branch.{branch}.merge"), &format!("refs/heads/{branch}")).unwrap();
        let out = std::process::Command::new("git")
            .arg("push")
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{:?}", out.stderr);

        commit_file(&repo, dir.path(), "src/a.md", "v2\n", "fix: toca a");
        let h = file_history_impl(&repo, "src/a.md").unwrap();
        assert_eq!(h.len(), 2);
        assert!(h[0].local, "el segundo commit no llegó al remoto");
        assert!(!h[1].local, "c1 ({c1}) sí llegó");
    }
}

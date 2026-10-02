use std::{path::Path, process::Command};

use crate::state::{AppState, Project};

/// Push de la rama actual con el `git` del sistema, para que use las
/// credenciales ya configuradas de la máquina (TRD.md). Si no hay
/// upstream, error explícito; Folio no crea remotos.
pub fn ensure_upstream(repo: &git2::Repository) -> Result<(), String> {
    let branch = repo
        .head()
        .map_err(|_| "la rama actual no tiene commits; no hay nada que empujar".to_string())?
        .shorthand()
        .map_err(|e| format!("HEAD: {e}"))?
        .to_string();
    // Igual que `git push` sin args: se guía por branch.<name>.remote y
    // .merge del config. Branch::upstream() exigiría que la ref
    // remota ya exista, y en el primer push todavía no existe.
    let cfg = repo.config().map_err(|e| format!("config: {e}"))?;
    let has_upstream = cfg
        .get_string(&format!("branch.{branch}.remote"))
        .is_ok()
        && cfg.get_string(&format!("branch.{branch}.merge")).is_ok();
    if !has_upstream {
        return Err(format!(
            "la rama {branch} no tiene upstream; no hay a dónde empujar"
        ));
    }
    Ok(())
}

pub fn run_git_push(root: &Path) -> Result<(), String> {
    let out = Command::new("git")
        .arg("push")
        .current_dir(root)
        .output()
        .map_err(|e| format!("ejecutar git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

pub fn push_with_git(repo: &git2::Repository, root: &Path) -> Result<(), String> {
    ensure_upstream(repo)?;
    run_git_push(root)
}

#[tauri::command]
pub fn push(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // El lock no se retiene durante el push: puede pedir credenciales
    // y tardar; no bloquea la UI con el estado del repo.
    let root = {
        let guard = state.lock().unwrap();
        match guard.as_ref().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => {
                ensure_upstream(&st.repo)?;
                st.root.clone()
            }
            Project::Remote(_) => {
                return Err(
                    "push es de proyectos locales; en remoto cada save publica su commit en la rama"
                        .to_string(),
                )
            }
        }
    };
    run_git_push(&root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::commit_touched;
    use crate::entry::write_entry_tracked;
    use serde_yaml_ng::Value;

    const CONFIG_YAML: &str =
        "content:\n  - name: blog\n    path: src/content/blog\n    fields: []\n";

    #[test]
    fn push_empuja_la_rama_a_un_remote_bare() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let repo = git2::Repository::init(&root).unwrap();
        let mut st = crate::state::RepoState {
            repo,
            root: root.clone(),
            config: Some(crate::config::parse_config(CONFIG_YAML).unwrap()),
            touched: Vec::new(),
        };

        let fm: Value = serde_yaml_ng::from_str("title: Push\n").unwrap();
        write_entry_tracked(&mut st, "src/content/blog/push.md", fm, "x\n").unwrap();
        commit_touched(&mut st, "folio: test push").unwrap();

        // Remote bare local + upstream configurado, como `git push -u`.
        let bare_dir = tempfile::tempdir().unwrap();
        git2::Repository::init_bare(bare_dir.path()).unwrap();
        st.repo
            .remote(
                "origin",
                bare_dir.path().to_str().unwrap(),
            )
            .unwrap();
        let branch = st.repo.head().unwrap().shorthand().unwrap().to_string();
        let mut cfg = st.repo.config().unwrap();
        cfg.set_str(&format!("branch.{branch}.remote"), "origin").unwrap();
        cfg.set_str(&format!("branch.{branch}.merge"), &format!("refs/heads/{branch}"))
            .unwrap();

        push_with_git(&st.repo, &root).unwrap();

        let bare = git2::Repository::open_bare(bare_dir.path()).unwrap();
        let local_head = st.repo.head().unwrap().target().unwrap();
        let remote_head = bare
            .find_reference(&format!("refs/heads/{branch}"))
            .unwrap()
            .target()
            .unwrap();
        assert_eq!(local_head, remote_head);
    }

    #[test]
    fn push_sin_upstream_es_error() {
        // Con commits pero sin remote configurado.
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut st = crate::state::RepoState {
            repo,
            root: dir.path().to_path_buf(),
            config: Some(crate::config::parse_config(CONFIG_YAML).unwrap()),
            touched: Vec::new(),
        };
        let fm: Value = serde_yaml_ng::from_str("title: X\n").unwrap();
        write_entry_tracked(&mut st, "src/content/blog/x.md", fm, "x\n").unwrap();
        commit_touched(&mut st, "folio: init").unwrap();
        let err = push_with_git(&st.repo, dir.path()).unwrap_err();
        assert!(err.contains("upstream"), "err: {err}");

        // Sin commits tampoco se puede empujar.
        let empty = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(empty.path()).unwrap();
        let err = push_with_git(&repo, empty.path()).unwrap_err();
        assert!(err.contains("commits"), "err: {err}");
    }
}

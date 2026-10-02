use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct RepoSummary {
    pub root: String,
    pub branch: String,
}

#[derive(Serialize, Clone)]
pub struct EntryRef {
    /// Path relativo a la raíz del repo, separadores '/'.
    pub path: String,
}

#[tauri::command]
pub fn open_repo(state: tauri::State<'_, AppState>, path: &str) -> Result<RepoSummary, String> {
    let repo = git2::Repository::open(path).map_err(|e| format!("no es un repo git: {e}"))?;
    let root = repo
        .workdir()
        .ok_or_else(|| "repo bare: Folio necesita working dir".to_string())?
        .to_path_buf();
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(str::to_string))
        .unwrap_or_else(|| "(sin commits)".to_string());
    let summary = RepoSummary {
        root: root.display().to_string(),
        branch,
    };
    *state.lock().unwrap() = Some(crate::state::RepoState {
        repo,
        root,
        touched: Vec::new(),
    });
    Ok(summary)
}

/// `collection` es el `path` de la colección bajo la raíz (p.ej.
/// `src/content/blog`). Hasta el parser de `.pages.yml` (Fase 1) llega
/// como string desde la UI.
#[tauri::command]
pub fn list_entries(
    state: tauri::State<'_, AppState>,
    collection: &str,
) -> Result<Vec<EntryRef>, String> {
    let guard = state.lock().unwrap();
    let st = guard.as_ref().ok_or("no hay repo abierto")?;
    list_entries_impl(&st.root, collection)
}

fn list_entries_impl(root: &Path, collection: &str) -> Result<Vec<EntryRef>, String> {
    let dir = root.join(collection);
    if !dir.is_dir() {
        return Err(format!("no existe la carpeta de la colección: {collection}"));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_markdown(&dir, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(root).map_err(|e| e.to_string())?;
            Ok(EntryRef {
                path: rel.to_string_lossy().replace('\\', "/"),
            })
        })
        .collect()
}

fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("leer {dir:?}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        git2::Repository::init(&root).unwrap();
        let blog = root.join("src/content/blog");
        fs::create_dir_all(&blog).unwrap();
        fs::write(blog.join("b-second.md"), "---\ntitle: B\n---\n\nbody b").unwrap();
        fs::write(blog.join("a-first.md"), "---\ntitle: A\n---\n\nbody a").unwrap();
        fs::create_dir_all(blog.join("drafts")).unwrap();
        fs::write(blog.join("drafts/c.md"), "---\ntitle: C\n---\n\nbody c").unwrap();
        fs::write(blog.join("not-md.txt"), "ignorado").unwrap();
        fs::write(root.join("README.md"), "no relacionado").unwrap();
        (dir, root)
    }

    #[test]
    fn lista_solo_md_de_la_coleccion_ordenado() {
        let (_keep, root) = fixture_repo();
        let entries = list_entries_impl(&root, "src/content/blog").unwrap();
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "src/content/blog/a-first.md",
                "src/content/blog/b-second.md",
                "src/content/blog/drafts/c.md",
            ]
        );
    }

    #[test]
    fn coleccion_inexistente_es_error() {
        let (_keep, root) = fixture_repo();
        assert!(list_entries_impl(&root, "src/content/nope").is_err());
    }

    #[test]
    fn abrir_carpeta_sin_git_es_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(git2::Repository::open(dir.path()).is_err());
    }
}

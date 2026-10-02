use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    config::{self, PagesConfig},
    state::{AppState, Project, RepoState},
};

#[derive(Serialize)]
pub struct RepoSummary {
    pub root: String,
    pub branch: String,
    pub has_config: bool,
    /// Motivo por el que el `.pages.yml` no se pudo usar, si aplica.
    pub config_error: Option<String>,
    /// "local" | "remote".
    pub mode: String,
    /// "owner/repo" cuando se conoce (clone desde GitHub o remoto).
    pub owner_repo: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EntryRef {
    /// Path relativo a la raíz del repo, separadores '/'.
    pub path: String,
}

#[derive(Serialize)]
pub struct RepoStatus {
    pub branch: String,
    pub dirty: bool,
    pub ahead: usize,
    pub behind: usize,
    pub has_upstream: bool,
}

/// Lee y parsea el `.pages.yml` de la raíz. Ausente → `None` sin error
/// (estado vacío explicado en la UI); presente pero roto → error.
fn load_config(root: &Path) -> (Option<PagesConfig>, Option<String>) {
    match fs::read_to_string(root.join(".pages.yml")) {
        Err(_) => (None, None),
        Ok(raw) => match config::parse_config(&raw) {
            Ok(cfg) => (Some(cfg), None),
            Err(e) => (None, Some(e)),
        },
    }
}

#[tauri::command]
pub fn open_repo(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    path: &str,
) -> Result<RepoSummary, String> {
    let summary = open_repo_at(&state, path)?;
    allow_asset_scope(&app, &summary.root)?;
    Ok(summary)
}

/// El webview muestra miniaturas vía `convertFileSrc`; el scope del
/// asset protocol se amplía al repo abierto, nada más.
fn allow_asset_scope(app: &tauri::AppHandle, root: &str) -> Result<(), String> {
    use tauri::Manager;
    app.asset_protocol_scope()
        .allow_directory(root, true)
        .map_err(|e| format!("asset scope: {e}"))
}

fn open_repo_at(state: &AppState, path: &str) -> Result<RepoSummary, String> {
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
    let (cfg, config_error) = load_config(&root);
    let summary = RepoSummary {
        root: root.display().to_string(),
        branch,
        has_config: cfg.is_some(),
        config_error,
        mode: "local".to_string(),
        owner_repo: None,
    };
    *state.lock().unwrap() = Some(Project::Local(RepoState {
        repo,
        root,
        config: cfg,
        touched: Vec::new(),
    }));
    Ok(summary)
}

/// "https://github.com/owner/repo.git" → "owner/repo".
fn owner_repo_from_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://github.com/")?;
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(format!("{owner}/{name}"))
}

/// Clona con el `git` del sistema (credenciales de la máquina) y deja
/// el resultado abierto, igual que `open_repo`.
#[tauri::command]
pub fn clone_repo(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: &str,
    dest: &str,
) -> Result<RepoSummary, String> {
    clone_into(url, dest)?;
    let mut summary = open_repo_at(&state, dest)?;
    summary.owner_repo = owner_repo_from_url(url);
    allow_asset_scope(&app, &summary.root)?;
    Ok(summary)
}

pub fn clone_into(url: &str, dest: &str) -> Result<(), String> {
    if url.trim().is_empty() || dest.trim().is_empty() {
        return Err("URL y carpeta destino son obligatorias".to_string());
    }
    // Con sesión de GitHub, el clone usa el token vía environment del
    // credential helper; sin sesión, las credenciales del sistema.
    let mut cmd = crate::github::clone_command(url, dest);

    let out = cmd
        .output()
        .map_err(|e| format!("ejecutar git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

/// `collection` es el `path` de la colección bajo la raíz (p.ej.
/// `src/content/blog`), tal como viene del `.pages.yml`.
#[tauri::command]
pub async fn list_entries(
    state: tauri::State<'_, AppState>,
    collection: String,
) -> Result<Vec<EntryRef>, String> {
    let (ctx, ext) = {
        let guard = state.lock().unwrap();
        match guard.as_ref().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => {
                let ext = owning_extension(st.config.as_ref(), &collection);
                return list_entries_impl(&st.root, &collection, &ext);
            }
            Project::Remote(rs) => {
                let ext = owning_extension(rs.config.as_ref(), &collection);
                (crate::remote::remote_ctx(rs), ext)
            }
        }
    };
    let token = crate::remote::current_token()?;
    let (t, c, dir) = (token.clone(), ctx.clone(), collection.clone());
    let (mut paths, files) = tauri::async_runtime::spawn_blocking(move || {
        let mut paths = Vec::new();
        let mut files = std::collections::HashMap::new();
        crate::remote::fetch_collection(&t, &c, &dir, &ext, &mut paths, &mut files)?;
        Ok::<_, String>((paths, files))
    })
    .await
    .map_err(|e| format!("listar entradas: {e}"))??;
    {
        let mut guard = state.lock().unwrap();
        if let Some(Project::Remote(rs)) = guard.as_mut() {
            if crate::remote::is_same_remote(rs, &ctx) {
                rs.files.extend(files);
            }
        }
    }
    paths.sort();
    Ok(paths.into_iter().map(|path| EntryRef { path }).collect())
}

#[tauri::command]
pub fn repo_status(state: tauri::State<'_, AppState>) -> Result<RepoStatus, String> {
    let guard = state.lock().unwrap();
    let st = match guard.as_ref().ok_or("no hay proyecto abierto")? {
        Project::Local(st) => st,
        Project::Remote(_) => {
            return Err(
                "repo_status es de proyectos locales; en remoto cada save publica en la rama"
                    .to_string(),
            )
        }
    };
    let repo = &st.repo;

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(str::to_string))
        .unwrap_or_else(|| "(sin commits)".to_string());

    let dirty = {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| format!("status: {e}"))?;
        statuses.iter().count() > 0
    };

    let mut ahead = 0;
    let mut behind = 0;
    let mut has_upstream = false;
    if let Ok(head_ref) = repo.head() {
        let branch_ref = git2::Branch::wrap(head_ref);
        if let Ok(upstream) = branch_ref.upstream() {
            if let (Ok(local), Ok(remote)) = (
                repo.head().unwrap().peel_to_commit(),
                upstream.get().peel_to_commit(),
            ) {
                let (a, b) = repo
                    .graph_ahead_behind(local.id(), remote.id())
                    .map_err(|e| format!("ahead/behind: {e}"))?;
                ahead = a;
                behind = b;
                has_upstream = true;
            }
        }
    }

    Ok(RepoStatus {
        branch,
        dirty,
        ahead,
        behind,
        has_upstream,
    })
}

/// Extensión exigida a los archivos de la colección (su schema
/// derivation); sin config, "md".
fn owning_extension(cfg: Option<&crate::config::PagesConfig>, path: &str) -> String {
    cfg.and_then(|c| crate::entry::owning_item(c, path))
        .map(|i| i.extension.clone())
        .unwrap_or_else(|| "md".to_string())
}

fn list_entries_impl(root: &Path, collection: &str, ext: &str) -> Result<Vec<EntryRef>, String> {
    let dir = root.join(collection);
    if !dir.is_dir() {
        return Err(format!("no existe la carpeta de la colección: {collection}"));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_entries(&dir, ext, &mut paths)?;
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

fn collect_entries(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("leer {dir:?}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_entries(&path, ext, out)?;
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
        {
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
        let entries = list_entries_impl(&root, "src/content/blog", "md").unwrap();
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
    fn lista_por_la_extension_del_schema() {
        // El portfolio real usa .mdx (filename '{primary}.mdx'); el
        // .md hardcodeado dejaba la tabla vacía.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let blog = root.join("src/content/blog");
        fs::create_dir_all(&blog).unwrap();
        fs::write(blog.join("post.mdx"), "---\ntitle: A\n---\n\nbody").unwrap();
        fs::write(blog.join("viejo.md"), "no es de esta colección").unwrap();
        let yaml = "content:\n  - name: blog\n    path: src/content/blog\n    filename: '{primary}.mdx'\n    fields: []\n";
        let cfg = Some(crate::config::parse_config(yaml).unwrap());
        let entries = list_entries_impl(&root, "src/content/blog", &owning_extension(cfg.as_ref(), "src/content/blog"))
            .unwrap();
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["src/content/blog/post.mdx"]);
    }

    #[test]
    fn coleccion_inexistente_es_error() {
        let (_keep, root) = fixture_repo();
        assert!(list_entries_impl(&root, "src/content/nope", "md").is_err());
    }

    #[test]
    fn abrir_carpeta_sin_git_es_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(git2::Repository::open(dir.path()).is_err());
    }

    #[test]
    fn load_config_ausente_y_roto() {
        let dir = tempfile::tempdir().unwrap();
        let (cfg, err) = load_config(dir.path());
        assert!(cfg.is_none() && err.is_none());
        fs::write(dir.path().join(".pages.yml"), "content: [").unwrap();
        let (cfg, err) = load_config(dir.path());
        assert!(cfg.is_none() && err.is_some());
    }

    #[test]
    fn clona_y_abre_un_repo_local() {
        // Fuente: un bare local con un commit (git del sistema, sin red).
        let src = tempfile::tempdir().unwrap();
        git2::Repository::init_bare(src.path()).unwrap();
        let work = tempfile::tempdir().unwrap();
        git2::Repository::init(work.path()).unwrap();
        {
            let repo = git2::Repository::open(work.path()).unwrap();
            fs::write(work.path().join("a.md"), "x").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("a.md")).unwrap();
            let tree_oid = index.write_tree().unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = git2::Signature::now("T", "t@t").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        let out = std::process::Command::new("git")
            .args([
                "push",
                src.path().to_str().unwrap(),
                "HEAD:main",
            ])
            .current_dir(work.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{:?}", out.stderr);

        let dest_dir = tempfile::tempdir().unwrap();
        let dest = dest_dir.path().join("clon");
        clone_into(src.path().to_str().unwrap(), dest.to_str().unwrap()).unwrap();

        let state: crate::state::AppState = std::sync::Mutex::new(None);
        let summary = open_repo_at(&state, dest.to_str().unwrap()).unwrap();
        assert!(dest.join(".git").exists());
        assert!(!summary.has_config);
        assert!(state.lock().unwrap().is_some());
    }

    #[test]
    fn clone_con_args_vacios_es_error() {
        assert!(clone_into("", "/tmp/x").is_err());
        assert!(clone_into("url", "").is_err());
    }

    #[test]
    fn owner_repo_desde_url() {
        assert_eq!(
            owner_repo_from_url("https://github.com/hunvreus/pagescms.git"),
            Some("hunvreus/pagescms".to_string())
        );
        assert_eq!(
            owner_repo_from_url("https://github.com/a/b"),
            Some("a/b".to_string())
        );
        assert_eq!(owner_repo_from_url("https://gitlab.com/a/b"), None);
        assert_eq!(owner_repo_from_url("/ruta/local"), None);
    }
}

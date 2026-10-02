//! Modo remoto: abrir y editar un repo de GitHub sin clonar, igual
//! que la web de Pages CMS — sin su server, con el token del usuario
//! llamando directo a la API. El Save publica directo a la rama
//! (decisión del dueño), así que aquí no hay working dir ni `touched`.

use std::collections::HashMap;

use crate::commit_message::{self, CommitContext};
use crate::config::{self, ContentItem, PagesConfig};
use crate::gh_api::{self, GhError};
use crate::github::{self, GithubSession};
use crate::repo::RepoSummary;
use crate::state::{AppState, Project, RemoteFile, RemoteState};

/// Lo que hace falta para hablar con la API de un proyecto remoto,
/// clonable para soltar el lock antes de la red.
#[derive(Clone)]
pub struct RemoteCtx {
    pub owner: String,
    pub repo: String,
    pub branch: String,
}

pub fn remote_ctx(rs: &RemoteState) -> RemoteCtx {
    RemoteCtx {
        owner: rs.owner.clone(),
        repo: rs.repo.clone(),
        branch: rs.branch.clone(),
    }
}

/// Token de la sesión (Keychain + caché en memoria).
pub fn current_token() -> Result<String, String> {
    github::load_session()?
        .ok_or_else(|| "sin sesión de GitHub: inicia sesión para usar proyectos remotos".to_string())
        .map(|s| s.token)
}

/// Sesión completa (login para los tokens de plantilla de commit).
pub fn current_session() -> Result<GithubSession, String> {
    github::load_session()?.ok_or_else(|| {
        "sin sesión de GitHub: inicia sesión para usar proyectos remotos".to_string()
    })
}

/// Traduce errores de la API al copy de la web donde aplica. Un 401
/// invalida la sesión, como su wrapper de octokit
/// (`GithubAuthExpired`).
pub fn gh_error_ui(e: GhError) -> String {
    if e.status == 401 {
        let _ = github::clear_session();
    }
    e.ui()
}

pub struct OpenRemote {
    pub state: RemoteState,
    pub config_error: Option<String>,
}

/// Abre un repo remoto: meta del repo (rama default, visibilidad,
/// permiso de push) + `.pages.yml` via Contents API, como su
/// `getRepoSnapshot` + `fetchConfigFromGithub`. No clona nada.
pub fn open_remote(token: &str, owner: &str, repo: &str) -> Result<OpenRemote, String> {
    let meta = gh_api::get_repo(token, owner, repo).map_err(gh_error_ui)?;
    if !meta.can_push() {
        return Err(format!(
            "{owner}/{repo}: sin permiso de push. La edición remota publica commits en la rama; pide acceso o clona localmente."
        ));
    }
    let branch = meta.default_branch;
    let (cfg, config_error) = match gh_api::get_content(token, owner, repo, ".pages.yml", &branch) {
        Err(e) if e.status == 404 => (None, None),
        Err(e) => return Err(gh_error_ui(e)),
        Ok(file) => {
            let raw = file
                .content
                .as_deref()
                .map(gh_api::decode_content)
                .transpose()
                .map_err(|e| e)?
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .ok_or_else(|| ".pages.yml no es UTF-8".to_string())?;
            match config::parse_config(&raw) {
                Ok(cfg) => (Some(cfg), None),
                Err(e) => (None, Some(e)),
            }
        }
    };
    Ok(OpenRemote {
        state: RemoteState {
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: branch.clone(),
            is_public: !meta.private,
            config: cfg,
            files: Default::default(),
        },
        config_error,
    })
}

/// Comando asíncrono: la red va en `spawn_blocking` — los comandos
/// síncronos corren en el main thread y congelarían la UI (doc Tauri
/// 2, "Calling Rust"). El lock del estado nunca cruza un await.
#[tauri::command]
pub async fn open_remote_repo(
    state: tauri::State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<RepoSummary, String> {
    let token = current_token()?;
    let opened = tauri::async_runtime::spawn_blocking(move || open_remote(&token, &owner, &repo))
        .await
        .map_err(|e| format!("abrir repo remoto: {e}"))??;
    let OpenRemote { state: rs, config_error } = opened;
    let summary = RepoSummary {
        root: String::new(),
        branch: rs.branch.clone(),
        has_config: rs.config.is_some(),
        config_error,
        mode: "remote".to_string(),
        owner_repo: Some(format!("{}/{}", rs.owner, rs.repo)),
    };
    let mut guard = state.lock().unwrap();
    *guard = Some(Project::Remote(rs));
    Ok(summary)
}

// ---- lecturas ----

/// Lista una colección con la query GraphQL Tree de la web (una petición
/// por carpeta, como su fetchCollectionDirectoryEntries), recursando en
/// subcarpetas. Devuelve los paths `.md` ordenables y los blobs para la
/// caché de shas. `text` null (binario/grande) no se cachea: el
/// read_entry hará GET.
pub fn fetch_collection(
    token: &str,
    ctx: &RemoteCtx,
    dir: &str,
    out: &mut Vec<String>,
    files: &mut HashMap<String, RemoteFile>,
) -> Result<(), String> {
    let entries =
        gh_api::fetch_tree_dir(token, &ctx.owner, &ctx.repo, &ctx.branch, dir).map_err(gh_error_ui)?;
    let mut subdirs = Vec::new();
    for e in entries {
        match e.kind.as_str() {
            "blob" => {
                if e.path.ends_with(".md") {
                    out.push(e.path.clone());
                }
                if let Some(obj) = e.object {
                    files.insert(
                        e.path,
                        RemoteFile {
                            sha: obj.oid,
                            text: obj.text.unwrap_or_default(),
                        },
                    );
                }
            }
            "tree" => subdirs.push(e.path),
            _ => {}
        }
    }
    for sub in subdirs {
        fetch_collection(token, ctx, &sub, out, files)?;
    }
    Ok(())
}

/// Texto y sha de un archivo por Contents API (la caché de sesión se
/// consulta bajo lock en el comando). Un archivo existente con
/// contenido ausente pero size > 0 se rechaza en vez de devolver "": un
/// save con eso pisaría el contenido real.
pub fn fetch_text(token: &str, ctx: &RemoteCtx, path: &str) -> Result<(String, String), String> {
    let file = gh_api::get_content(token, &ctx.owner, &ctx.repo, path, &ctx.branch).map_err(gh_error_ui)?;
    if file.content.is_none() && file.size.unwrap_or(0) > 0 {
        return Err(format!("{path}: demasiado grande para editarlo remoto"));
    }
    let bytes = gh_api::decode_content(file.content.as_deref().unwrap_or(""))?;
    let text = String::from_utf8(bytes).map_err(|e| format!("{path}: no es UTF-8: {e}"))?;
    Ok((text, file.sha))
}

// ---- publicación ----

pub struct PutOutcome {
    pub path: String,
    pub sha: String,
    /// true cuando el 422 forzó un `name-N` (aviso "saved but renamed").
    pub renamed: bool,
}

fn commit_msg(
    cfg: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    action: &str,
    ctx: &RemoteCtx,
    login: &str,
    path: &str,
    old_path: Option<&str>,
    new_path: Option<&str>,
) -> String {
    commit_message::resolve_commit_message(
        cfg,
        item,
        &CommitContext {
            action,
            owner: &ctx.owner,
            repo: &ctx.repo,
            branch: &ctx.branch,
            name: item.map(|i| i.name.as_str()),
            user: login,
            path: Some(path),
            old_path,
            new_path,
        },
    )
}

/// PUT contents de un archivo de texto: un archivo = un commit, el save
/// de la web (`githubSaveFile`). Sin `sha` es create (con auto-rename
/// en 422); con `sha`, update (409 si cambió afuera).
pub fn put_text_file(
    token: &str,
    ctx: &RemoteCtx,
    cfg: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    login: &str,
    path: &str,
    text: &str,
    sha: Option<&str>,
) -> Result<PutOutcome, String> {
    let action = if sha.is_some() { "update" } else { "create" };
    let message = commit_msg(cfg, item, action, ctx, login, path, None, None);
    let b64 = gh_api::encode_content(text.as_bytes());
    match gh_api::put_content(token, &ctx.owner, &ctx.repo, path, &ctx.branch, &message, &b64, sha)
    {
        Ok(put) => Ok(PutOutcome {
            path: path.to_string(),
            sha: saved_sha(token, ctx, &put, path)?,
            renamed: false,
        }),
        Err(e) if e.status == 409 => Err(conflict_message(&e, sha.is_some())),
        Err(e) if e.status == 422 && sha.is_none() => {
            auto_rename_put(token, ctx, cfg, item, login, path, &b64, &e)
        }
        Err(e) => Err(gh_error_ui(e)),
    }
}

/// El copy de sus 409: reglas de repo piden PR; sha → el archivo cambió
/// desde que se cargó. La web dice "refresh the page"; aquí se refresca
/// la entrada.
fn conflict_message(e: &GhError, had_sha: bool) -> String {
    if e.message.contains("Repository rule violations found") {
        return "This repository requires changes through a pull request. Save to a different branch or fork, or ask a maintainer to relax the repository rule for direct edits.".to_string();
    }
    if had_sha {
        return "File has changed since you last loaded it. Please refresh the entry and try again."
            .to_string();
    }
    gh_error_ui(e.clone())
}

/// Su auto-rename 422: lista el directorio padre, arma `name-N` sin
/// pisar y reintenta hasta 3 veces con mensaje de create por path.
fn auto_rename_put(
    token: &str,
    ctx: &RemoteCtx,
    cfg: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    login: &str,
    path: &str,
    b64: &str,
    first_err: &GhError,
) -> Result<PutOutcome, String> {
    let parent = parent_dir(path);
    let base = path.rsplit('/').next().unwrap_or(path);
    let (filename, extension) = split_base(base);
    let dir = gh_api::get_content_dir(token, &ctx.owner, &ctx.repo, &parent, &ctx.branch)
        .map_err(gh_error_ui)?;
    let names: Vec<String> = dir.iter().map(|f| f.name.clone()).collect();
    let candidates = gh_api::rename_candidates(&names, &filename, &extension);
    for (i, candidate) in candidates.iter().enumerate() {
        let new_path = format!("{parent}/{candidate}");
        let message = commit_msg(cfg, item, "create", ctx, login, &new_path, None, None);
        match gh_api::put_content(
            token,
            &ctx.owner,
            &ctx.repo,
            &new_path,
            &ctx.branch,
            &message,
            b64,
            None,
        ) {
            Ok(put) => {
                let sha = saved_sha(token, ctx, &put, &new_path)?;
                return Ok(PutOutcome {
                    path: new_path,
                    sha,
                    renamed: true,
                });
            }
            // Su loop: el 422 reintenta salvo en el tercer intento; el
            // resto de errores corta.
            Err(e) if e.status == 422 && i + 1 < candidates.len() => continue,
            Err(e) => return Err(gh_error_ui(e)),
        }
    }
    Err(gh_error_ui(first_err.clone()))
}

/// El PUT puede volver con `content: null` (paths largos); el sha se
/// recupera con un GET, en vez de fallar como la web.
fn saved_sha(
    token: &str,
    ctx: &RemoteCtx,
    put: &gh_api::PutContent,
    path: &str,
) -> Result<String, String> {
    if let Some(sha) = put.content.as_ref().map(|c| c.sha.clone()) {
        return Ok(sha);
    }
    Ok(gh_api::get_content(token, &ctx.owner, &ctx.repo, path, &ctx.branch)
        .map_err(gh_error_ui)?
        .sha)
}

/// Rename por Git Data, 5 pasos como su `githubRenameFile` (preserva
/// historial; Contents API no sabe renombrar). El árbol nuevo copia el
/// real con solo el path cambiado.
pub fn rename_file_remote(
    token: &str,
    ctx: &RemoteCtx,
    cfg: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    login: &str,
    path: &str,
    new_path: &str,
) -> Result<String, String> {
    let message = commit_msg(cfg, item, "rename", ctx, login, path, Some(path), Some(new_path));
    let head = gh_api::get_branch_head(token, &ctx.owner, &ctx.repo, &ctx.branch)
        .map_err(gh_error_ui)?;
    let tree = gh_api::get_tree_recursive(token, &ctx.owner, &ctx.repo, &head).map_err(gh_error_ui)?;
    let new_tree = gh_api::build_rename_tree(&tree, path, new_path);
    let tree_sha = gh_api::create_tree(token, &ctx.owner, &ctx.repo, &new_tree).map_err(gh_error_ui)?;
    let commit_sha =
        gh_api::create_commit(token, &ctx.owner, &ctx.repo, &message, &tree_sha, &[head])
            .map_err(gh_error_ui)?;
    gh_api::update_branch_ref(token, &ctx.owner, &ctx.repo, &ctx.branch, &commit_sha)
        .map_err(gh_error_ui)?;
    Ok(commit_sha)
}

/// DELETE contents con sha, como su `deleteFile`.
pub fn delete_file_remote(
    token: &str,
    ctx: &RemoteCtx,
    cfg: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    login: &str,
    path: &str,
    sha: &str,
) -> Result<(), String> {
    let message = commit_msg(cfg, item, "delete", ctx, login, path, None, None);
    gh_api::delete_content(token, &ctx.owner, &ctx.repo, path, &ctx.branch, &message, sha)
        .map_err(gh_error_ui)
}

fn parent_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

fn split_base(base: &str) -> (String, String) {
    match base.rfind('.') {
        // Su lastDotIndex > 0: un punto inicial no separa extensión.
        Some(i) if i > 0 => (base[..i].to_string(), base[i + 1..].to_string()),
        _ => (base.to_string(), String::new()),
    }
}

/// ¿Sigue abierto el mismo proyecto remoto? La red suelta el lock; al
/// volver, no se cachea en un proyecto distinto.
pub fn is_same_remote(rs: &RemoteState, ctx: &RemoteCtx) -> bool {
    rs.owner == ctx.owner && rs.repo == ctx.repo && rs.branch == ctx.branch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicto_409_copia_los_mensajes_de_la_web() {
        let rules = GhError {
            status: 409,
            message: "Repository rule violations found for \"push\"".into(),
            retry_after: None,
        };
        assert!(conflict_message(&rules, true).contains("requires changes through a pull request"));

        let sha = GhError {
            status: 409,
            message: "does not match".into(),
            retry_after: None,
        };
        let msg = conflict_message(&sha, true);
        assert!(
            msg.contains("File has changed since you last loaded it"),
            "{msg}"
        );
        // Sin sha (create) el 409 no es de sha: error normal.
        assert!(conflict_message(&sha, false).contains("GitHub API (409)"));
    }

    #[test]
    fn parent_y_split_de_nombre() {
        assert_eq!(parent_dir("src/content/blog/a.md"), "src/content/blog");
        assert_eq!(parent_dir("a.md"), "");
        assert_eq!(split_base("post.md"), ("post".to_string(), "md".to_string()));
        assert_eq!(split_base("a.b.c"), ("a.b".to_string(), "c".to_string()));
        // Punto inicial no separa extensión (igual que su lastDotIndex > 0).
        assert_eq!(split_base(".gitkeep"), (".gitkeep".to_string(), String::new()));
        assert_eq!(split_base("sinext"), ("sinext".to_string(), String::new()));
    }
}

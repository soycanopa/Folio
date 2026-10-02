//! Modo remoto: abrir y editar un repo de GitHub sin clonar, igual
//! que la web de Pages CMS — sin su server, con el token del usuario
//! llamando directo a la API. El Save publica directo a la rama
//! (decisión del dueño), así que aquí no hay working dir ni `touched`.

use crate::config;
use crate::gh_api::{self, GhError};
use crate::github;
use crate::repo::RepoSummary;
use crate::state::{AppState, Project, RemoteState};

/// Token de la sesión (Keychain + caché en memoria).
pub fn current_token() -> Result<String, String> {
    github::load_session()?
        .ok_or_else(|| "sin sesión de GitHub: inicia sesión para usar proyectos remotos".to_string())
        .map(|s| s.token)
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

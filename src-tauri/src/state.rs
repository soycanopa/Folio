use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use crate::config::PagesConfig;

/// Repo abierto, su config parseada y los paths (relativos a la raíz)
/// que Folio escribió en la sesión. El commit stagea solo esa lista;
/// nunca `add -A`.
pub struct RepoState {
    pub repo: git2::Repository,
    pub root: PathBuf,
    pub config: Option<PagesConfig>,
    pub touched: Vec<PathBuf>,
}

/// Proyecto remoto: se edita contra la API de GitHub sin clonar. La
/// caché `files` guarda el contenido y el `sha` con el que se cargó
/// cada path — el PUT lo exige y su 409 detecta cambios ajenos. La
/// caché `media` guarda el listado (sha + download_url) para las
/// previews privadas.
pub struct RemoteState {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub is_public: bool,
    pub config: Option<PagesConfig>,
    pub files: HashMap<String, RemoteFile>,
    pub media: HashMap<String, crate::gh_api::GhFile>,
}

#[derive(Clone)]
pub struct RemoteFile {
    pub sha: String,
    pub text: String,
}

/// Un proyecto abierto a la vez: local (git2 + working dir) o remoto
/// (API de GitHub). Los comandos dispatchean sobre este enum.
pub enum Project {
    Local(RepoState),
    Remote(RemoteState),
}

pub type AppState = Mutex<Option<Project>>;

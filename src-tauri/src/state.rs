use std::{path::PathBuf, sync::Mutex};

/// Repo abierto más los paths (relativos a la raíz) que Folio escribió
/// en la sesión. El commit stagea solo esa lista; nunca `add -A`.
pub struct RepoState {
    pub repo: git2::Repository,
    pub root: PathBuf,
    pub touched: Vec<PathBuf>,
}

pub type AppState = Mutex<Option<RepoState>>;

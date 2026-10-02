use std::{path::PathBuf, sync::Mutex};

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

pub type AppState = Mutex<Option<RepoState>>;

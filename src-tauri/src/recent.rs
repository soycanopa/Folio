use std::path::Path;

use serde::{Deserialize, Serialize};

// El home de Folio: los proyectos (repos) que ya abriste, como la
// grilla de repos de Pages CMS pero local. La lista vive en el disco
// de la app, no en el webview (invariante: Rust posee el disco).

const MAX_RECENT: usize = 12;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecentRepo {
    pub path: String,
    /// Unix seconds de la última apertura; 0 si el registro es viejo.
    #[serde(default)]
    pub last_open: i64,
    /// owner/repo si el proyecto vino de GitHub (avatar en el home).
    #[serde(default)]
    pub owner_repo: Option<String>,
}

pub fn load_from(file: &Path) -> Vec<RecentRepo> {
    let raw = match std::fs::read_to_string(file) {
        Ok(raw) => raw,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_to(file: &Path, list: &[RecentRepo]) -> Result<(), String> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("crear {parent:?}: {e}"))?;
    }
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(file, json).map_err(|e| format!("escribir: {e}"))
}

/// El más reciente al frente, sin duplicados, con tope. Registra el
/// momento de la apertura para la sección "Recently visited".
pub fn add(
    list: &[RecentRepo],
    path: &str,
    now: i64,
    owner_repo: Option<&str>,
) -> Vec<RecentRepo> {
    let mut out: Vec<RecentRepo> = list
        .iter()
        .filter(|r| r.path != path)
        .cloned()
        .collect();
    // Conserva el origen GitHub previo si el llamado no da uno.
    let owner_repo = owner_repo
        .map(str::to_string)
        .or_else(|| list.iter().find(|r| r.path == path).and_then(|r| r.owner_repo.clone()));
    out.insert(
        0,
        RecentRepo {
            path: path.to_string(),
            last_open: now,
            owner_repo,
        },
    );
    out.truncate(MAX_RECENT);
    out
}

pub fn remove(list: &[RecentRepo], path: &str) -> Vec<RecentRepo> {
    list.iter()
        .filter(|r| r.path != path)
        .cloned()
        .collect()
}

fn recent_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    Ok(dir.join("recent-repos.json"))
}

#[tauri::command]
pub fn list_recent_repos(app: tauri::AppHandle) -> Result<Vec<RecentRepo>, String> {
    Ok(load_from(&recent_file(&app)?))
}

#[tauri::command]
pub fn add_recent_repo(
    app: tauri::AppHandle,
    path: String,
    owner_repo: Option<String>,
) -> Result<Vec<RecentRepo>, String> {
    let file = recent_file(&app)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let list = add(&load_from(&file), &path, now, owner_repo.as_deref());
    save_to(&file, &list)?;
    Ok(list)
}

#[tauri::command]
pub fn remove_recent_repo(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<RecentRepo>, String> {
    let file = recent_file(&app)?;
    let list = remove(&load_from(&file), &path);
    save_to(&file, &list)?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dedupe_reciente_y_tope() {
        let mk = |p: &str| RecentRepo {
            path: p.to_string(),
            last_open: 0,
            owner_repo: None,
        };
        let l = add(&[mk("/a"), mk("/b")], "/c", 100, None);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].path, "/c");
        assert_eq!(l[0].last_open, 100);

        let l = add(&l, "/a", 200, None);
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].path, "/a");
        assert_eq!(l[0].last_open, 200);

        let many: Vec<RecentRepo> = (0..15).map(|i| mk(&format!("/r{i}"))).collect();
        assert_eq!(add(&many, "/new", 1, None).len(), MAX_RECENT);
    }

    #[test]
    fn remove_y_roundtrip_de_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("recent.json");
        let mk = |p: &str| RecentRepo {
            path: p.to_string(),
            last_open: 0,
            owner_repo: None,
        };

        let l = add(&[], "/x", 5, None);
        save_to(&file, &l).unwrap();
        assert_eq!(
            load_from(&file),
            vec![RecentRepo { path: "/x".into(), last_open: 5, owner_repo: None }]
        );

        // Registro viejo sin last_open: default 0, no error.
        std::fs::write(&file, r#"[{"path":"/old"}]"#).unwrap();
        assert_eq!(load_from(&file), vec![mk("/old")]);

        assert!(remove(&load_from(&file), "/old").is_empty());
        assert!(load_from(&dir.path().join("nope.json")).is_empty());
    }
}

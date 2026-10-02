use std::{fs, path::Path};

use serde_yaml_ng::Value;

use crate::{
    entry::{ensure_writable, read_entry_at, safe_join, write_entry_tracked, EntryContent},
    state::{AppState, Project, RepoState},
};

// Entradas `type: file` del config: un único archivo sin tabla.
// JSON → el objeto completo es el contenido. MD → front matter + cuerpo
// como una entrada de colección.

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn read_file_entry_at(root: &Path, path: &str) -> Result<EntryContent, String> {
    match extension(path).as_str() {
        "json" => {
            let full = safe_join(root, path)?;
            let raw = fs::read_to_string(&full).map_err(|e| format!("leer {path}: {e}"))?;
            let json: serde_json::Value =
                serde_json::from_str(&raw).map_err(|e| format!("{path}: {e}"))?;
            let data: Value = serde_json::from_value(json)
                .map_err(|e| format!("{path}: no es un objeto: {e}"))?;
            Ok(EntryContent {
                frontmatter: data,
                body: String::new(),
            })
        }
        "md" => read_entry_at(root, path),
        other => Err(format!(
            "formato de file entry no soportado: .{other} (v1: json y md)"
        )),
    }
}

/// JSON pretty + salto de línea final: el texto que se escribe al
/// disco local o se publica por PUT.
pub(crate) fn json_entry_text(data: &Value) -> Result<String, String> {
    let json = serde_json::to_value(data).map_err(|e| format!("serializar: {e}"))?;
    let mut out = serde_json::to_string_pretty(&json).map_err(|e| format!("serializar: {e}"))?;
    out.push('\n');
    Ok(out)
}

/// Contenido de texto de un file entry según su extensión (lo que va
/// al disco o al PUT).
pub(crate) fn file_entry_text(
    path: &str,
    data: &Value,
    body: &str,
) -> Result<String, String> {
    match extension(path).as_str() {
        "json" => json_entry_text(data),
        "md" => {
            let fm = match data {
                Value::Mapping(m) => m.clone(),
                other => {
                    return Err(format!("front matter: se espera un objeto, llegó {other:?}"))
                }
            };
            crate::entry::serialize_entry(&fm, body)
        }
        other => Err(format!(
            "formato de file entry no soportado: .{other} (v1: json y md)"
        )),
    }
}

pub fn write_file_entry_tracked(
    st: &mut RepoState,
    path: &str,
    data: Value,
    body: &str,
) -> Result<String, String> {
    match extension(path).as_str() {
        "json" => {
            // El write de un file entry también queda dentro de los paths
            // del config (invariante de AGENTS.md).
            ensure_writable(st, path)?;
            let out = json_entry_text(&data)?;
            let full = safe_join(&st.root, path)?;
            fs::write(&full, out).map_err(|e| format!("escribir {path}: {e}"))?;
            let rel = std::path::PathBuf::from(path);
            if !st.touched.contains(&rel) {
                st.touched.push(rel);
            }
            Ok(path.to_string())
        }
        "md" => write_entry_tracked(st, path, data, body),
        other => Err(format!(
            "formato de file entry no soportado: .{other} (v1: json y md)"
        )),
    }
}

#[tauri::command]
pub async fn read_file_entry(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<EntryContent, String> {
    let ctx = {
        let mut guard = state.lock().unwrap();
        match guard.as_mut().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => return read_file_entry_at(&st.root, &path),
            Project::Remote(rs) => {
                if let Some(f) = rs.files.get(&path) {
                    return parse_file_entry_text(&path, &f.text);
                }
                crate::remote::remote_ctx(rs)
            }
        }
    };
    let token = crate::remote::current_token()?;
    let (t, c, p) = (token.clone(), ctx.clone(), path.clone());
    let (text, sha) = tauri::async_runtime::spawn_blocking(move || {
        crate::remote::fetch_text(&t, &c, &p)
    })
    .await
    .map_err(|e| format!("leer file entry: {e}"))??;
    let out = parse_file_entry_text(&path, &text)?;
    let mut guard = state.lock().unwrap();
    if let Some(Project::Remote(rs)) = guard.as_mut() {
        if crate::remote::is_same_remote(rs, &ctx) {
            rs.files.insert(path, crate::state::RemoteFile { sha, text });
        }
    }
    Ok(out)
}

/// Un archivo de file entry en texto → EntryContent (json → objeto
/// completo; md → front matter + cuerpo).
pub(crate) fn parse_file_entry_text(path: &str, raw: &str) -> Result<EntryContent, String> {
    match extension(path).as_str() {
        "json" => {
            let json: serde_json::Value =
                serde_json::from_str(raw).map_err(|e| format!("{path}: {e}"))?;
            let data: Value = serde_json::from_value(json)
                .map_err(|e| format!("{path}: no es un objeto: {e}"))?;
            Ok(EntryContent {
                frontmatter: data,
                body: String::new(),
            })
        }
        "md" => {
            let (fm, body) = crate::entry::parse_entry(raw)?;
            Ok(EntryContent {
                frontmatter: Value::Mapping(fm),
                body,
            })
        }
        other => Err(format!(
            "formato de file entry no soportado: .{other} (v1: json y md)"
        )),
    }
}

#[tauri::command]
pub async fn write_file_entry(
    state: tauri::State<'_, AppState>,
    path: String,
    frontmatter: Value,
    body: String,
) -> Result<String, String> {
    let content = file_entry_text(&path, &frontmatter, &body)?;
    let (ctx, cfg, item, sha) = {
        let mut guard = state.lock().unwrap();
        match guard.as_mut().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => {
                return write_file_entry_tracked(st, &path, frontmatter, &body)
            }
            Project::Remote(rs) => {
                let cfg = rs
                    .config
                    .clone()
                    .ok_or("sin .pages.yml: no se puede validar el destino del write")?;
                crate::entry::ensure_writable_config(&cfg, &path)?;
                let item = crate::entry::owning_item(&cfg, &path).cloned();
                let sha = rs.files.get(&path).map(|f| f.sha.clone());
                (crate::remote::remote_ctx(rs), cfg, item, sha)
            }
        }
    };
    let token = crate::remote::current_token()?;
    let login = crate::remote::current_session()?.login;
    let (t, c, cfg2, item2, p, text) = (
        token.clone(),
        ctx.clone(),
        Some(cfg.clone()),
        item.clone(),
        path.clone(),
        content.clone(),
    );
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        crate::remote::put_text_file(
            &t,
            &c,
            cfg2.as_ref(),
            item2.as_ref(),
            &login,
            &p,
            &text,
            sha.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("guardar file entry: {e}"))??;
    let mut guard = state.lock().unwrap();
    if let Some(Project::Remote(rs)) = guard.as_mut() {
        if crate::remote::is_same_remote(rs, &ctx) {
            rs.files.insert(
                outcome.path.clone(),
                crate::state::RemoteFile {
                    sha: outcome.sha,
                    text: content,
                },
            );
        }
    }
    Ok(outcome.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CONFIG_YAML: &str = "content:\n  - name: hero\n    type: file\n    path: src/content/hero.json\n    fields:\n      - name: heading\n        type: string\n  - name: about\n    type: file\n    path: src/content/about.md\n    fields: []\n  - name: blog\n    path: src/content/blog\n    fields: []\n";

    fn st_with_config(dir: &Path) -> RepoState {
        let repo = git2::Repository::init(dir).unwrap();
        RepoState {
            repo,
            root: dir.to_path_buf(),
            config: Some(crate::config::parse_config(CONFIG_YAML).unwrap()),
            touched: Vec::new(),
        }
    }

    #[test]
    fn json_roundtrip_conserva_claves_desconocidas() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src/content")).unwrap();
        fs::write(
            dir.path().join("src/content/hero.json"),
            "{\n  \"heading\": \"Hi\",\n  \"custom\": 42\n}\n",
        )
        .unwrap();

        let mut st = st_with_config(dir.path());
        let entry = read_file_entry_at(&st.root, "src/content/hero.json").unwrap();
        let mut data = match entry.frontmatter {
            Value::Mapping(m) => m,
            other => panic!("no es objeto: {other:?}"),
        };
        data.insert(Value::from("heading"), Value::from("Hola"));
        let path = write_file_entry_tracked(
            &mut st,
            "src/content/hero.json",
            Value::Mapping(data),
            "",
        )
        .unwrap();

        assert_eq!(path, "src/content/hero.json");
        assert_eq!(st.touched, vec![PathBuf::from("src/content/hero.json")]);
        let raw = fs::read_to_string(dir.path().join("src/content/hero.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["heading"], "Hola");
        assert_eq!(json["custom"], 42);
    }

    #[test]
    fn md_file_entry_va_por_el_camino_de_siempre() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src/content")).unwrap();
        fs::write(
            dir.path().join("src/content/about.md"),
            "---\ntitle: About\n---\n\nBody.\n",
        )
        .unwrap();
        let mut st = st_with_config(dir.path());
        let crate::entry::EntryContent { frontmatter, body } =
            read_file_entry_at(&st.root, "src/content/about.md").unwrap();
        let fm = match &frontmatter {
            Value::Mapping(m) => m.clone(),
            _ => unreachable!(),
        };
        assert_eq!(fm.get("title"), Some(&Value::from("About")));
        assert_eq!(body, "Body.\n");
        write_file_entry_tracked(&mut st, "src/content/about.md", frontmatter, "Nuevo body.\n")
            .unwrap();
        let raw = fs::read_to_string(dir.path().join("src/content/about.md")).unwrap();
        assert!(raw.contains("Nuevo body."));
    }

    #[test]
    fn formato_no_soportado_y_fuera_del_config_son_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path());
        let fm = Value::Mapping(serde_yaml_ng::Mapping::new());
        assert!(write_file_entry_tracked(&mut st, "src/content/hero.toml", fm.clone(), "").is_err());
        assert!(write_file_entry_tracked(&mut st, "package.json", fm, "").is_err());
    }
}

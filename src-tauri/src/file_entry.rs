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
            let json = serde_json::to_value(&data)
                .map_err(|e| format!("serializar {path}: {e}"))?;
            let mut out = serde_json::to_string_pretty(&json)
                .map_err(|e| format!("serializar {path}: {e}"))?;
            out.push('\n');
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
pub fn read_file_entry(
    state: tauri::State<'_, AppState>,
    path: &str,
) -> Result<EntryContent, String> {
    let guard = state.lock().unwrap();
    match guard.as_ref().ok_or("no hay proyecto abierto")? {
        Project::Local(st) => read_file_entry_at(&st.root, path),
        Project::Remote(_) => Err(crate::repo::WIP_REMOTE.to_string()),
    }
}

#[tauri::command]
pub fn write_file_entry(
    state: tauri::State<'_, AppState>,
    path: &str,
    frontmatter: Value,
    body: &str,
) -> Result<String, String> {
    let mut guard = state.lock().unwrap();
    match guard.as_mut().ok_or("no hay proyecto abierto")? {
        Project::Local(st) => write_file_entry_tracked(st, path, frontmatter, body),
        Project::Remote(_) => Err(crate::repo::WIP_REMOTE.to_string()),
    }
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

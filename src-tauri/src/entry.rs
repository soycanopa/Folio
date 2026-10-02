use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

use crate::state::{AppState, RepoState};

#[derive(Serialize)]
pub struct EntryContent {
    pub frontmatter: Value,
    pub body: String,
}

// Invariante Folio: un write de contenido resuelve dentro de las rutas que
// el config declara. El parser de `.pages.yml` llega en Fase 1; lo exigible
// en el spike es que el path no escape de la raíz del repo.
fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(format!("path absoluto rechazado: {rel}"));
    }
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(format!("path con '..' rechazado: {rel}"));
    }
    Ok(root.join(p))
}

/// Separa `---` front matter YAML del cuerpo Markdown. Sin delimitadores
/// devuelve front matter vacío y el archivo entero como cuerpo.
pub fn parse_entry(raw: &str) -> Result<(Mapping, String), String> {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return Ok((Mapping::new(), raw.to_string()));
    };
    let Some(end) = rest.find("\n---") else {
        return Ok((Mapping::new(), raw.to_string()));
    };
    // El `---` hallado tiene que ser una línea completa, no `----` ni
    // `--- texto`.
    let after_marker = &rest[end + 4..];
    if !(after_marker.starts_with('\n') || after_marker.is_empty()) {
        return Ok((Mapping::new(), raw.to_string()));
    }
    let yaml = &rest[..end];
    // Un `\n` cierra la línea del delimitador; otro más es la línea en
    // blanco habitual antes del cuerpo.
    let body = after_marker
        .strip_prefix('\n')
        .and_then(|b| b.strip_prefix('\n').or(Some(b)))
        .unwrap_or(after_marker);
    let value: Value = serde_yaml_ng::from_str(yaml).map_err(|e| format!("front matter: {e}"))?;
    match value {
        Value::Mapping(m) => Ok((m, body.to_string())),
        other => Err(format!("front matter no es un mapa: {other:?}")),
    }
}

pub fn serialize_entry(fm: &Mapping, body: &str) -> Result<String, String> {
    if fm.is_empty() {
        return Ok(body.to_string());
    }
    let yaml = serde_yaml_ng::to_string(&Value::Mapping(fm.clone()))
        .map_err(|e| format!("serializar front matter: {e}"))?;
    Ok(format!("---\n{yaml}---\n\n{body}"))
}

pub fn read_entry_at(root: &Path, path: &str) -> Result<EntryContent, String> {
    let full = safe_join(root, path)?;
    let raw = fs::read_to_string(&full).map_err(|e| format!("leer {path}: {e}"))?;
    let (fm, body) = parse_entry(&raw)?;
    Ok(EntryContent {
        frontmatter: Value::Mapping(fm),
        body,
    })
}

/// Escribe la entrada y registra el path en `touched` para que el commit
/// lo stagee. Devuelve el path relativo escrito.
pub fn write_entry_tracked(
    st: &mut RepoState,
    path: &str,
    frontmatter: Value,
    body: &str,
) -> Result<String, String> {
    let fm = match frontmatter {
        Value::Mapping(m) => m,
        other => return Err(format!("front matter: se espera un objeto, llegó {other:?}")),
    };
    let content = serialize_entry(&fm, body)?;
    let full = safe_join(&st.root, path)?;
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("crear {path}: {e}"))?;
    }
    fs::write(&full, content).map_err(|e| format!("escribir {path}: {e}"))?;
    let rel = PathBuf::from(path);
    if !st.touched.contains(&rel) {
        st.touched.push(rel);
    }
    Ok(path.to_string())
}

#[tauri::command]
pub fn read_entry(state: tauri::State<'_, AppState>, path: &str) -> Result<EntryContent, String> {
    let guard = state.lock().unwrap();
    let st = guard.as_ref().ok_or("no hay repo abierto")?;
    read_entry_at(&st.root, path)
}

#[tauri::command]
pub fn write_entry(
    state: tauri::State<'_, AppState>,
    path: &str,
    frontmatter: Value,
    body: &str,
) -> Result<String, String> {
    let mut guard = state.lock().unwrap();
    let st = guard.as_mut().ok_or("no hay repo abierto")?;
    write_entry_tracked(st, path, frontmatter, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\ntitle: Hello world\npubDate: 2026-09-28\ncustom: keep-me\n---\n\nHello body.\n";

    #[test]
    fn parsea_front_matter_y_cuerpo() {
        let (fm, body) = parse_entry(SAMPLE).unwrap();
        assert_eq!(fm.get("title"), Some(&Value::from("Hello world")));
        assert_eq!(body, "Hello body.\n");
    }

    #[test]
    fn roundtrip_conserva_campos_desconocidos_y_orden() {
        let (mut fm, body) = parse_entry(SAMPLE).unwrap();
        fm.insert(Value::from("title"), Value::from("Nuevo título"));
        let out = serialize_entry(&fm, &body).unwrap();
        assert_eq!(
            out,
            "---\ntitle: Nuevo título\npubDate: 2026-09-28\ncustom: keep-me\n---\n\nHello body.\n"
        );
        // El campo que el config de Folio no describe sobrevive al guardar.
        let (fm2, _) = parse_entry(&out).unwrap();
        assert_eq!(fm2.get("custom"), Some(&Value::from("keep-me")));
    }

    #[test]
    fn sin_front_matter_es_solo_cuerpo() {
        let (fm, body) = parse_entry("solo cuerpo\n").unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, "solo cuerpo\n");
        assert_eq!(serialize_entry(&fm, &body).unwrap(), "solo cuerpo\n");
    }

    #[test]
    fn yaml_invalido_es_error() {
        assert!(parse_entry("---\n[broken\n---\n\nx\n").is_err());
    }

    #[test]
    fn rechaza_paths_que_escapan() {
        let root = Path::new("/tmp/folio-test");
        assert!(safe_join(root, "../fuera.md").is_err());
        assert!(safe_join(root, "/etc/passwd").is_err());
        assert!(safe_join(root, "src/content/blog/a.md").is_ok());
    }

    #[test]
    fn write_y_read_vuelven_por_el_mismo_camino() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut st = RepoState {
            repo,
            root: dir.path().to_path_buf(),
            touched: Vec::new(),
        };
        let fm = serde_yaml_ng::from_str::<Value>("title: Nuevo\npubDate: 2026-10-01\n")
            .unwrap();
        let fm = match fm {
            Value::Mapping(m) => m,
            _ => unreachable!(),
        };
        write_entry_tracked(
            &mut st,
            "src/content/blog/nuevo.md",
            Value::Mapping(fm),
            "cuerpo nuevo\n",
        )
        .unwrap();
        assert_eq!(st.touched, vec![PathBuf::from("src/content/blog/nuevo.md")]);
        let back = read_entry_at(&st.root, "src/content/blog/nuevo.md").unwrap();
        let back_fm = match back.frontmatter {
            Value::Mapping(m) => m,
            _ => unreachable!(),
        };
        assert_eq!(back_fm.get("title"), Some(&Value::from("Nuevo")));
        assert_eq!(back.body, "cuerpo nuevo\n");
    }
}

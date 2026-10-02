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
pub(crate) fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, String> {
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

// Invariante Folio: un write de contenido tiene que resolver dentro de
// `content[].path` o `media.input` del `.pages.yml` (AGENTS.md). Sin
// config parseada no hay contra qué validar, así que se rechaza.
pub(crate) fn ensure_writable(st: &RepoState, rel: &str) -> Result<(), String> {
    let cfg = st
        .config
        .as_ref()
        .ok_or("sin .pages.yml: no se puede validar el destino del write")?;
    let rel_path = Path::new(rel);
    let in_collection = cfg
        .content
        .iter()
        .any(|c| rel_path.starts_with(Path::new(&c.path)));
    let in_media = cfg
        .media
        .input
        .as_deref()
        .is_some_and(|m| rel_path.starts_with(Path::new(m)));
    if !(in_collection || in_media) {
        return Err(format!("write fuera de las rutas del config: {rel}"));
    }
    Ok(())
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
    ensure_writable(st, path)?;
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

#[derive(Serialize)]
pub struct NewEntry {
    pub path: String,
    pub existed: bool,
}

/// Resuelve el path de una entrada nueva (`filename` del config o
/// `{slug}.md`). No escribe nada: si el archivo existe, no se pisa; la
/// UI lo abre (FLOW.md). El primer Save lo materializa.
pub fn create_entry_impl(
    st: &RepoState,
    collection: &str,
    slug: &str,
) -> Result<NewEntry, String> {
    let cfg = st
        .config
        .as_ref()
        .ok_or("sin .pages.yml: no hay colecciones")?;
    let item = cfg
        .content
        .iter()
        .find(|c| c.name == collection)
        .ok_or_else(|| format!("colección desconocida: {collection}"))?;
    let slug = slug.trim();
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!("slug inválido: {slug:?} (solo a-z, 0-9 y -)"));
    }
    let tpl = item.filename.as_deref().unwrap_or("{slug}.md");
    let filename = tpl.replace("{slug}", slug);
    if filename.contains('{') {
        return Err(format!(
            "filename con placeholders que v0 no resuelve: {tpl} (solo {{slug}})"
        ));
    }
    let rel = format!("{}/{}", item.path.trim_end_matches('/'), filename);
    let full = safe_join(&st.root, &rel)?;
    Ok(NewEntry {
        path: rel,
        existed: full.is_file(),
    })
}

#[tauri::command]
pub fn create_entry(
    state: tauri::State<'_, AppState>,
    collection: &str,
    slug: &str,
) -> Result<NewEntry, String> {
    let guard = state.lock().unwrap();
    let st = guard.as_ref().ok_or("no hay repo abierto")?;
    create_entry_impl(st, collection, slug)
}

/// Colección que contiene a `path`, según los paths del config.
fn owning_collection<'a>(
    st: &'a RepoState,
    path: &str,
) -> Result<&'a crate::config::ContentItem, String> {
    let rel = Path::new(path);
    st.config
        .as_ref()
        .ok_or("sin .pages.yml")?
        .content
        .iter()
        .find(|c| rel.starts_with(Path::new(&c.path)))
        .ok_or_else(|| format!("write fuera de las rutas del config: {path}"))
}

fn track(st: &mut RepoState, path: &str) {
    let rel = PathBuf::from(path);
    if !st.touched.contains(&rel) {
        st.touched.push(rel);
    }
}

/// Renombra el archivo de una entrada dentro de su colección. El
/// destino es un nombre de archivo simple, junto al original.
pub fn rename_entry_impl(st: &mut RepoState, path: &str, new_name: &str) -> Result<String, String> {
    ensure_writable(st, path)?;
    let item = owning_collection(st, path)?;
    if !item.operations.rename {
        return Err(format!("la colección {} no permite renombrar", item.name));
    }
    let name = Path::new(new_name);
    if name.file_name().map(|n| n.to_string_lossy().to_string()).as_deref() != Some(new_name)
        || new_name.contains('/')
        || new_name.contains('\\')
        || new_name.starts_with('.')
    {
        return Err(format!("nombre de archivo inválido: {new_name:?}"));
    }
    let old_full = safe_join(&st.root, path)?;
    if !old_full.is_file() {
        return Err(format!("no existe {path}"));
    }
    let new_rel = format!(
        "{}/{}",
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default(),
        new_name
    );
    ensure_writable(st, &new_rel)?;
    let new_full = safe_join(&st.root, &new_rel)?;
    if new_full.exists() {
        return Err(format!("ya existe {new_rel}; no se pisa"));
    }
    fs::rename(&old_full, &new_full).map_err(|e| format!("renombrar: {e}"))?;
    track(st, path);
    track(st, &new_rel);
    Ok(new_rel)
}

/// Borra la entrada. El commit de Folio stagea el borrado (commit.rs).
pub fn delete_entry_impl(st: &mut RepoState, path: &str) -> Result<(), String> {
    ensure_writable(st, path)?;
    let item = owning_collection(st, path)?;
    if !item.operations.delete {
        return Err(format!("la colección {} no permite borrar", item.name));
    }
    let full = safe_join(&st.root, path)?;
    if !full.is_file() {
        return Err(format!("no existe {path}"));
    }
    fs::remove_file(&full).map_err(|e| format!("borrar {path}: {e}"))?;
    track(st, path);
    Ok(())
}

#[tauri::command]
pub fn rename_entry(
    state: tauri::State<'_, AppState>,
    path: &str,
    new_name: &str,
) -> Result<String, String> {
    let mut guard = state.lock().unwrap();
    let st = guard.as_mut().ok_or("no hay repo abierto")?;
    rename_entry_impl(st, path, new_name)
}

#[tauri::command]
pub fn delete_entry(state: tauri::State<'_, AppState>, path: &str) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let st = guard.as_mut().ok_or("no hay repo abierto")?;
    delete_entry_impl(st, path)
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
    use crate::config::parse_config;

    const CONFIG_YAML: &str = "media:\n  input: src/content/media\n  output: /images\ncontent:\n  - name: blog\n    label: Blog\n    path: src/content/blog\n    fields:\n      - name: title\n        type: string\n";

    fn st_with_config(dir: &Path, yaml: &str) -> RepoState {
        let repo = git2::Repository::init(dir).unwrap();
        RepoState {
            repo,
            root: dir.to_path_buf(),
            config: Some(parse_config(yaml).unwrap()),
            touched: Vec::new(),
        }
    }

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
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let fm = match serde_yaml_ng::from_str::<Value>(
            "title: Nuevo\npubDate: 2026-10-01\n",
        )
        .unwrap()
        {
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

    #[test]
    fn write_fuera_del_config_se_rechaza() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let fm = Value::Mapping(Mapping::new());
        // Fuera de src/content/blog y de src/content/media.
        assert!(write_entry_tracked(&mut st, "README.md", fm.clone(), "x").is_err());
        assert!(write_entry_tracked(&mut st, "src/other/a.md", fm.clone(), "x").is_err());
        // Media.input sí es escribible (ahí caen las imágenes, v1).
        assert!(write_entry_tracked(&mut st, "src/content/media/a.png", fm, "x").is_ok());
    }

    #[test]
    fn write_sin_config_se_rechaza() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut st = RepoState {
            repo,
            root: dir.path().to_path_buf(),
            config: None,
            touched: Vec::new(),
        };
        let fm = Value::Mapping(Mapping::new());
        assert!(write_entry_tracked(&mut st, "src/content/blog/a.md", fm, "x").is_err());
    }

    #[test]
    fn create_entry_resuelve_filename_y_no_pisa() {
        let dir = tempfile::tempdir().unwrap();
        let st = st_with_config(dir.path(), CONFIG_YAML);
        let n = create_entry_impl(&st, "blog", "mi-post").unwrap();
        assert_eq!(n.path, "src/content/blog/mi-post.md");
        assert!(!n.existed);

        // Ya existe: se avisa, no se escribe nada aquí.
        fs::create_dir_all(dir.path().join("src/content/blog")).unwrap();
        fs::write(dir.path().join("src/content/blog/mi-post.md"), "previo").unwrap();
        let n = create_entry_impl(&st, "blog", "mi-post").unwrap();
        assert!(n.existed);
        assert_eq!(
            fs::read_to_string(dir.path().join("src/content/blog/mi-post.md")).unwrap(),
            "previo"
        );
    }

    #[test]
    fn create_entry_con_filename_del_config() {
        let yaml = "content:\n  - name: blog\n    path: src/content/blog\n    filename: \"post-{slug}.md\"\n    fields: []\n";
        let dir = tempfile::tempdir().unwrap();
        let st = st_with_config(dir.path(), yaml);
        let n = create_entry_impl(&st, "blog", "x").unwrap();
        assert_eq!(n.path, "src/content/blog/post-x.md");
        // Placeholder que v0 no resuelve: error explícito.
        let yaml = "content:\n  - name: blog\n    path: src/content/blog\n    filename: \"{pubDate}-{slug}.md\"\n    fields: []\n";
        let st = st_with_config(dir.path(), yaml);
        assert!(create_entry_impl(&st, "blog", "x").is_err());
    }

    #[test]
    fn create_entry_valida_slug() {
        let dir = tempfile::tempdir().unwrap();
        let st = st_with_config(dir.path(), CONFIG_YAML);
        assert!(create_entry_impl(&st, "blog", "Hola").is_err());
        assert!(create_entry_impl(&st, "blog", "a/b").is_err());
        assert!(create_entry_impl(&st, "blog", "").is_err());
        assert!(create_entry_impl(&st, "nope", "ok").is_err());
        assert!(create_entry_impl(&st, "blog", "ok-2").is_ok());
    }

    #[test]
    fn rename_mueve_dentro_de_la_coleccion_y_deja_touched() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let src = dir.path().join("src/content/blog/viejo.md");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "---\ntitle: X\n---\n\nb\n").unwrap();

        let nuevo = rename_entry_impl(&mut st, "src/content/blog/viejo.md", "nuevo.md").unwrap();
        assert_eq!(nuevo, "src/content/blog/nuevo.md");
        assert!(!src.exists());
        assert!(dir.path().join("src/content/blog/nuevo.md").is_file());
        assert_eq!(
            st.touched,
            vec![
                PathBuf::from("src/content/blog/viejo.md"),
                PathBuf::from("src/content/blog/nuevo.md")
            ]
        );

        // Ya existe: no se pisa. Fuera de la colección o con ruta: rechazado.
        assert!(rename_entry_impl(&mut st, nuevo.as_str(), "nuevo.md").is_err());
        assert!(rename_entry_impl(&mut st, nuevo.as_str(), "sub/dir.md").is_err());
        assert!(rename_entry_impl(&mut st, "README.md", "x.md").is_err());
    }

    #[test]
    fn delete_borra_y_respecta_operations() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let src = dir.path().join("src/content/blog/x.md");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::write(&src, "b\n").unwrap();

        delete_entry_impl(&mut st, "src/content/blog/x.md").unwrap();
        assert!(!src.exists());
        assert_eq!(st.touched, vec![PathBuf::from("src/content/blog/x.md")]);
        assert!(delete_entry_impl(&mut st, "src/content/blog/x.md").is_err());

        // operations.delete: false en el config → rechazado.
        let yaml = "content:\n  - name: blog\n    path: src/content/blog\n    operations:\n      delete: false\n    fields: []\n";
        let mut st2 = st_with_config(dir.path(), yaml);
        fs::write(dir.path().join("src/content/blog/y.md"), "b\n").unwrap();
        assert!(delete_entry_impl(&mut st2, "src/content/blog/y.md").is_err());
    }
}

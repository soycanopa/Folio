use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    entry::{ensure_writable, safe_join},
    state::{AppState, Project, RepoState},
};

const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "avif", "svg"];

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct MediaRef {
    /// Path repo-relativo, p.ej. `src/content/media/foo.png`.
    pub path: String,
    pub name: String,
    /// Ruta pública para el front matter: `media.output` + nombre.
    pub public_path: String,
    /// true si la extensión es de imagen (thumbnail en el grid).
    pub is_image: bool,
}

fn is_image_ext(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
}

fn media_input(st: &RepoState) -> Result<String, String> {
    media_input_cfg(st.config.as_ref().ok_or("sin .pages.yml")?)
}

pub(crate) fn media_input_cfg(cfg: &crate::config::PagesConfig) -> Result<String, String> {
    cfg.media
        .input
        .clone()
        .ok_or_else(|| "el config no declara media.input".to_string())
}

fn public_path(st: &RepoState, name: &str) -> String {
    public_path_cfg(st.config.as_ref(), name)
}

pub(crate) fn public_path_cfg(cfg: Option<&crate::config::PagesConfig>, name: &str) -> String {
    let output = cfg
        .and_then(|c| c.media.output.as_deref())
        .unwrap_or("");
    if output.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", output.trim_end_matches('/'), name)
    }
}

fn to_base36(mut n: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(ALPHABET[(n % 36) as usize]);
        n /= 36;
    }
    if out.is_empty() {
        "0".to_string()
    } else {
        out.into_iter().rev().map(|b| b as char).collect()
    }
}

/// Igual que Pages CMS (`lib/utils/file.ts`): slug del stem, extensión
/// en minúsculas, fallback `file`.
fn safe_name(original: &str) -> String {
    let p = Path::new(original);
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let slug: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                c
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if slug.is_empty() { "file".into() } else { slug };
    if ext.is_empty() {
        base
    } else {
        format!("{base}.{ext}")
    }
}

fn random_name(ext: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let rand = (now.subsec_nanos() as u64) ^ (std::process::id() as u64).rotate_left(32);
    let suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    format!("{}-{}{}", to_base36(now.as_millis() as u64), to_base36(rand), suffix)
}

fn upload_name(original: &str, rename: Option<&str>) -> String {
    match rename {
        Some("random") => {
            let ext = Path::new(original)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            random_name(&ext)
        }
        Some(_) => safe_name(original),
        None => original.to_string(),
    }
}

/// Resuelve el destino de una subida a partir del config (valida el
/// nombre y el invariante de media.input): lo comparten el copiado
/// local y el PUT remoto.
pub(crate) fn upload_target(
    cfg: &crate::config::PagesConfig,
    original: &str,
) -> Result<(String, String), String> {
    let input = media_input_cfg(cfg)?;
    if original.starts_with('.') {
        return Err(format!("nombre de archivo inválido: {original:?}"));
    }
    let name = upload_name(original, cfg.media.rename.as_deref());
    if name.trim().is_empty() || name.contains('/') {
        return Err(format!("nombre de archivo inválido: {name:?}"));
    }
    let rel = format!("{}/{}", input.trim_end_matches('/'), name);
    // Invariante: el write queda bajo media.input del config.
    crate::entry::ensure_writable_config(cfg, &rel)?;
    Ok((rel, name))
}

fn collect_files(dir: &Path, root: &Path, out: &mut Vec<MediaRef>, st: &RepoState) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("leer {dir:?}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, root, out, st)?;
        } else {
            let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
            let rel = rel.to_string_lossy().replace('\\', "/");
            let name = entry.file_name().to_string_lossy().to_string();
            out.push(MediaRef {
                public_path: public_path(st, &name),
                is_image: is_image_ext(&name),
                path: rel,
                name,
            });
        }
    }
    Ok(())
}

pub fn list_media_impl(st: &RepoState) -> Result<Vec<MediaRef>, String> {
    let input = media_input(st)?;
    let dir = st.root.join(&input);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    collect_files(&dir, &st.root, &mut out, st)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Copia un archivo a `media.input` respetando `rename` y lo registra
/// en `touched`: entra en el mismo commit que el `.md` (FLOW.md, v1).
/// Acepta cualquier tipo (image y file fields); si ya existe, no se pisa.
pub fn import_media_impl(st: &mut RepoState, src: &str) -> Result<MediaRef, String> {
    let cfg = st
        .config
        .as_ref()
        .ok_or("sin .pages.yml")?
        .clone();
    let original = Path::new(src)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("no se puede leer el nombre del archivo")?;
    let (rel, name) = upload_target(&cfg, &original)?;

    let dest = safe_join(&st.root, &rel)?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("crear {rel}: {e}"))?;
    }
    if dest.exists() {
        return Err(format!("ya existe {rel}; no se pisa"));
    }
    fs::copy(src, &dest).map_err(|e| format!("copiar {src}: {e}"))?;
    track(st, &rel);
    Ok(MediaRef {
        public_path: public_path_cfg(Some(&cfg), &name),
        is_image: is_image_ext(&name),
        path: rel,
        name,
    })
}

/// Borra un archivo de `media.input`; el commit stagea el borrado.
pub fn delete_media_impl(st: &mut RepoState, path: &str) -> Result<(), String> {
    ensure_writable(st, path)?;
    let full = safe_join(&st.root, path)?;
    if !full.is_file() {
        return Err(format!("no existe {path}"));
    }
    fs::remove_file(&full).map_err(|e| format!("borrar {path}: {e}"))?;
    track(st, path);
    Ok(())
}

fn track(st: &mut RepoState, rel: &str) {
    let relp = std::path::PathBuf::from(rel);
    if !st.touched.contains(&relp) {
        st.touched.push(relp);
    }
}

#[tauri::command]
pub async fn list_media(state: tauri::State<'_, AppState>) -> Result<Vec<MediaRef>, String> {
    let (ctx, cfg) = {
        let guard = state.lock().unwrap();
        match guard.as_ref().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => return list_media_impl(st),
            Project::Remote(rs) => {
                let cfg = rs.config.clone().ok_or("sin .pages.yml")?;
                (crate::remote::remote_ctx(rs), cfg)
            }
        }
    };
    let input = media_input_cfg(&cfg)?;
    let token = crate::remote::current_token()?;
    let (t, c, dir) = (token.clone(), ctx.clone(), input.clone());
    let files = tauri::async_runtime::spawn_blocking(move || {
        // Su fetchMediaDirectoryEntries: REST contents por carpeta;
        // Folio aplana subcarpetas como su grid local.
        let mut out = Vec::new();
        crate::remote::fetch_media_dir(&t, &c, &dir, &mut out)?;
        Ok::<_, String>(out)
    })
    .await
    .map_err(|e| format!("listar media: {e}"))??;
    let mut refs: Vec<MediaRef> = files
        .iter()
        .map(|f| MediaRef {
            public_path: public_path_cfg(Some(&cfg), &f.name),
            is_image: is_image_ext(&f.name),
            path: f.path.clone(),
            name: f.name.clone(),
        })
        .collect();
    refs.sort_by(|a, b| a.path.cmp(&b.path));
    {
        let mut guard = state.lock().unwrap();
        if let Some(Project::Remote(rs)) = guard.as_mut() {
            if crate::remote::is_same_remote(rs, &ctx) {
                rs.media = files.into_iter().map(|f| (f.path.clone(), f)).collect();
            }
        }
    }
    Ok(refs)
}

#[tauri::command]
pub async fn import_media(
    state: tauri::State<'_, AppState>,
    src: String,
) -> Result<MediaRef, String> {
    let (ctx, cfg) = {
        let mut guard = state.lock().unwrap();
        match guard.as_mut().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => return import_media_impl(st, &src),
            Project::Remote(rs) => {
                let cfg = rs.config.clone().ok_or("sin .pages.yml")?;
                (crate::remote::remote_ctx(rs), cfg)
            }
        }
    };
    // El core lee los bytes del archivo local elegido (el webview no
    // toca el disco) y publica un PUT por archivo, como su MediaUpload.
    let original = Path::new(&src)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("no se puede leer el nombre del archivo")?;
    let (rel, _name) = upload_target(&cfg, &original)?;
    let bytes = fs::read(&src).map_err(|e| format!("leer {src}: {e}"))?;
    let token = crate::remote::current_token()?;
    let login = crate::remote::current_session()?.login;
    let (t, c, cfg2, p, b) = (
        token.clone(),
        ctx.clone(),
        Some(cfg.clone()),
        rel.clone(),
        bytes,
    );
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        crate::remote::put_bytes_file(&t, &c, cfg2.as_ref(), None, &login, &p, &b, None)
    })
    .await
    .map_err(|e| format!("subir media: {e}"))??;
    let name = outcome
        .path
        .rsplit('/')
        .next()
        .unwrap_or(&outcome.path)
        .to_string();
    {
        let mut guard = state.lock().unwrap();
        if let Some(Project::Remote(rs)) = guard.as_mut() {
            if crate::remote::is_same_remote(rs, &ctx) {
                // Refresca el listado cacheado con el archivo nuevo.
                if let Ok(f) =
                    crate::gh_api::get_content(&token, &ctx.owner, &ctx.repo, &outcome.path, &ctx.branch)
                {
                    rs.media.insert(outcome.path.clone(), f);
                }
            }
        }
    }
    Ok(MediaRef {
        public_path: public_path_cfg(Some(&cfg), &name),
        is_image: is_image_ext(&name),
        path: outcome.path,
        name,
    })
}

#[tauri::command]
pub async fn delete_media(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let (ctx, cfg, sha) = {
        let mut guard = state.lock().unwrap();
        match guard.as_mut().ok_or("no hay proyecto abierto")? {
            Project::Local(st) => return delete_media_impl(st, &path),
            Project::Remote(rs) => {
                let cfg = rs.config.clone().ok_or("sin .pages.yml")?;
                crate::entry::ensure_writable_config(&cfg, &path)?;
                let sha = rs.media.get(&path).map(|f| f.sha.clone());
                (crate::remote::remote_ctx(rs), cfg, sha)
            }
        }
    };
    let token = crate::remote::current_token()?;
    let login = crate::remote::current_session()?.login;
    let (t, c, cfg2, p, sha) = (
        token.clone(),
        ctx.clone(),
        Some(cfg),
        path.clone(),
        sha,
    );
    tauri::async_runtime::spawn_blocking(move || {
        let sha = match sha {
            Some(s) => s,
            None => match crate::gh_api::get_content(&t, &c.owner, &c.repo, &p, &c.branch) {
                Ok(f) => f.sha,
                Err(e) if e.status == 404 => return Err(format!("no existe {p}")),
                Err(e) => return Err(crate::remote::gh_error_ui(e)),
            },
        };
        crate::remote::delete_file_remote(&t, &c, cfg2.as_ref(), None, &login, &p, &sha)
    })
    .await
    .map_err(|e| format!("borrar media: {e}"))??;
    let mut guard = state.lock().unwrap();
    if let Some(Project::Remote(rs)) = guard.as_mut() {
        if crate::remote::is_same_remote(rs, &ctx) {
            rs.media.remove(&path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_YAML: &str = "media:\n  input: src/content/media\n  output: /images\ncontent:\n  - name: blog\n    path: src/content/blog\n    fields: []\n";

    fn st_with_config(dir: &Path, yaml: &str) -> RepoState {
        let repo = git2::Repository::init(dir).unwrap();
        RepoState {
            repo,
            root: dir.to_path_buf(),
            config: Some(crate::config::parse_config(yaml).unwrap()),
            touched: Vec::new(),
        }
    }

    fn make_png(path: &Path) {
        fs::write(path, b"\x89PNG\r\n\x1a\nfake").unwrap();
    }

    fn src_png(dir: &Path, name: &str) -> String {
        let p = dir.join(name);
        make_png(&p);
        p.to_string_lossy().to_string()
    }

    #[test]
    fn import_conserva_nombre_y_queda_touched() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let src = src_png(dir.path(), "foto.png");
        let m = import_media_impl(&mut st, &src).unwrap();
        assert_eq!(m.path, "src/content/media/foto.png");
        assert_eq!(m.public_path, "/images/foto.png");
        assert!(m.is_image);
        assert!(dir.path().join("src/content/media/foto.png").is_file());
        assert_eq!(st.touched, vec![std::path::PathBuf::from("src/content/media/foto.png")]);

        // Ya existe: no se pisa.
        make_png(&dir.path().join("seed.png"));
        fs::copy(dir.path().join("seed.png"), dir.path().join("src/content/media/foto.png")).unwrap();
        let m2 = src_png(dir.path(), "foto.png");
        assert!(import_media_impl(&mut st, &m2).unwrap_err().contains("no se pisa"));
    }

    #[test]
    fn rename_safe_slugifica() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "media:\n  input: m\n  output: /i\n  rename: safe\ncontent: []\n";
        let mut st = st_with_config(dir.path(), yaml);
        let src = src_png(dir.path(), "My Photo (1).PNG");
        let m = import_media_impl(&mut st, &src).unwrap();
        assert_eq!(m.name, "my-photo-1.png");
        assert_eq!(m.public_path, "/i/my-photo-1.png");
    }

    #[test]
    fn rename_random_genera_nombre_con_extension() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "media:\n  input: m\n  rename: random\ncontent: []\n";
        let mut st = st_with_config(dir.path(), yaml);
        let src = src_png(dir.path(), "cover.png");
        let m = import_media_impl(&mut st, &src).unwrap();
        assert_ne!(m.name, "cover.png");
        assert!(m.name.ends_with(".png"), "name: {}", m.name);
        assert!(m.name.len() > ".png".len() + 4);
    }

    #[test]
    fn import_acepta_cualquier_archivo_y_sin_media_input_es_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let txt = dir.path().join("notes.txt");
        fs::write(&txt, "x").unwrap();
        let m = import_media_impl(&mut st, txt.to_str().unwrap()).unwrap();
        assert!(!m.is_image);
        assert!(dir.path().join("src/content/media/notes.txt").is_file());

        let yaml = "content: []\n";
        let mut st2 = st_with_config(dir.path(), yaml);
        let png = src_png(dir.path(), "a.png");
        assert!(import_media_impl(&mut st2, &png).is_err());
    }

    #[test]
    fn delete_media_borra_y_deja_touched() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_with_config(dir.path(), CONFIG_YAML);
        let src = src_png(dir.path(), "foto.png");
        let m = import_media_impl(&mut st, &src).unwrap();
        delete_media_impl(&mut st, &m.path).unwrap();
        assert!(!dir.path().join(&m.path).exists());
        assert!(st.touched.contains(&std::path::PathBuf::from(&m.path)));
        // Fuera de media.input: rechazado por ensure_writable.
        assert!(delete_media_impl(&mut st, "README.md").is_err());
    }

    #[test]
    fn upload_target_valida_y_resuelve() {
        let cfg = crate::config::parse_config(CONFIG_YAML).unwrap();
        let (rel, name) = upload_target(&cfg, "foto.png").unwrap();
        assert_eq!(rel, "src/content/media/foto.png");
        assert_eq!(name, "foto.png");
        // Dot-file: rechazado, igual que el copiado local.
        assert!(upload_target(&cfg, ".hidden").is_err());
    }

    #[test]
    fn lista_archivos_recursivo_y_ordenado_con_flag_imagen() {
        let dir = tempfile::tempdir().unwrap();
        let st = st_with_config(dir.path(), CONFIG_YAML);
        fs::create_dir_all(dir.path().join("src/content/media/sub")).unwrap();
        make_png(&dir.path().join("src/content/media/b.png"));
        make_png(&dir.path().join("src/content/media/sub/a.png"));
        fs::write(dir.path().join("src/content/media/doc.pdf"), "x").unwrap();
        let list = list_media_impl(&st).unwrap();
        let paths: Vec<&str> = list.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "src/content/media/b.png",
                "src/content/media/doc.pdf",
                "src/content/media/sub/a.png"
            ]
        );
        let by_path = |p: &str| list.iter().find(|m| m.path == p).unwrap();
        assert!(by_path("src/content/media/b.png").is_image);
        assert!(!by_path("src/content/media/doc.pdf").is_image);
        assert_eq!(by_path("src/content/media/b.png").public_path, "/images/b.png");
    }
}

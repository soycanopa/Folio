use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::fs;

use crate::state::AppState;

/// Tipos de campo que Folio sabe editar. Portado del set core de su
/// app (`fields/core`); lo demás se muestra deshabilitado con su nombre.
pub const FIELD_TYPES: [&str; 10] = [
    "string", "text", "date", "image", "file", "rich-text", "number",
    "boolean", "select", "code",
];

/// Semántica de su `lib/operations.ts`: defaults por tipo de ítem y
/// override del config con `!== false`.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Operations {
    pub create: bool,
    pub rename: bool,
    pub delete: bool,
}

fn resolve_operations(kind: &str, m: &Mapping) -> Operations {
    let (d_create, d_rename, d_delete) = if kind == "file" {
        (true, false, true)
    } else {
        (true, true, true)
    };
    let configured = m.get("operations").and_then(Value::as_mapping);
    let flag = |key: &str, default: bool| -> bool {
        default
            && configured
                .and_then(|c| c.get(key))
                .and_then(Value::as_bool)
                != Some(false)
    };
    Operations {
        create: flag("create", d_create),
        rename: flag("rename", d_rename),
        delete: flag("delete", d_delete),
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Media {
    pub input: Option<String>,
    pub output: Option<String>,
    /// `false`/ausente → None (conserva el nombre). `"safe"` slugifica,
    /// `"random"` genera nombre. Booleano true se trata como safe, como
    /// hace Pages CMS en su `lib/utils/file.ts`.
    pub rename: Option<String>,
    /// `media.extensions` con las categorías expandidas (vacío = sin
    /// restricción), como su normalización en `lib/config.ts`.
    pub extensions: Vec<String>,
}

/// Sus categorías de `lib/utils/file.ts`: `extensions: [image]` en el
/// config expande a la lista concreta.
const EXTENSION_CATEGORIES: [(&str, &[&str]); 8] = [
    ("image", &["jpg", "jpeg", "apng", "png", "gif", "svg", "ico", "avif", "bmp", "tif", "tiff", "webp"]),
    ("document", &["pdf", "doc", "docx", "ppt", "pptx", "vxls", "xlsx", "txt", "rtf"]),
    ("video", &["mp4", "avi", "mov", "wmv", "flv", "mpeg", "webm", "ogv", "ts", "3gp", "3g2"]),
    ("audio", &["mp3", "wav", "aac", "ogg", "flac", "weba", "oga", "opus", "mid", "midi", "3gp", "3g2"]),
    ("compressed", &["zip", "rar", "7z", "tar", "gz", "tgz", "bz", "bz2"]),
    ("code", &["js", "jsx", "ts", "tsx", "html", "css", "scss", "json", "xml", "yaml", "yml", "md", "py", "rb", "php", "java", "c", "cpp", "h", "cs", "go", "rs", "sql"]),
    ("font", &["ttf", "otf", "woff", "woff2", "eot"]),
    ("spreadsheet", &["csv", "tsv", "ods"]),
];

fn parse_extensions(v: Option<&Value>, warnings: &mut Vec<String>) -> Vec<String> {
    let Some(seq) = v.and_then(Value::as_sequence) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in seq {
        match item.as_str() {
            Some(s) => {
                if let Some((_, list)) = EXTENSION_CATEGORIES
                    .iter()
                    .find(|(cat, _)| *cat == s)
                {
                    out.extend(list.iter().map(|e| e.to_string()));
                } else {
                    out.push(s.to_string());
                }
            }
            None => warnings.push(format!(
                "media.extensions: valor no string ignorado ({item:?})"
            )),
        }
    }
    out
}

/// Su `getFileExtension`: texto tras el último punto; los dotfiles sin
/// otro punto no tienen extensión.
pub(crate) fn ext_of(path: &str) -> String {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if filename.starts_with('.') && !filename[1..].contains('.') {
        return String::new();
    }
    match filename.rfind('.') {
        Some(i) => filename[i + 1..].to_string(),
        None => String::new(),
    }
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct View {
    pub fields: Vec<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

/// Plantillas de commit por acción, como su `lib/commit-message.ts`.
/// Solo cuentan las strings no vacías; el resto cae al default en el
/// resolve.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct CommitTemplates {
    pub create: Option<String>,
    pub update: Option<String>,
    pub delete: Option<String>,
    pub rename: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// `settings.commit.templates`. `identity` no se parsea: Folio no
    /// tiene GitHub App, el commit remoto firma siempre con el
    /// usuario del token (divergencia en third_party/NOTICE).
    pub commit_templates: CommitTemplates,
}

fn parse_commit_templates(
    v: Option<&Value>,
    warnings: &mut Vec<String>,
    ctx: &str,
) -> Option<CommitTemplates> {
    let m = v?.as_mapping()?;
    let mut out = CommitTemplates::default();
    for key in ["create", "update", "delete", "rename"] {
        match m.get(key) {
            Some(Value::String(s)) if !s.trim().is_empty() => match key {
                "create" => out.create = Some(s.clone()),
                "update" => out.update = Some(s.clone()),
                "delete" => out.delete = Some(s.clone()),
                _ => out.rename = Some(s.clone()),
            },
            Some(Value::String(_)) => {}
            Some(other) => warnings.push(format!(
                "{ctx}: commit.templates.{key} no es string; se ignora ({other:?})"
            )),
            None => {}
        }
    }
    Some(out)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub help: Option<String>,
    /// Options de un select (`options.values`).
    pub values: Vec<String>,
}

/// Colecciones (`type: collection`) y archivos únicos (`type: file`),
/// que abren el formulario directo sin tabla. `type: group` queda
/// fuera; se avisa y se ignora.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ContentItem {
    pub name: String,
    /// "collection" | "file".
    pub kind: String,
    pub label: String,
    /// Carpeta de la colección o archivo único del `type: file`.
    pub path: String,
    pub filename: Option<String>,
    pub fields: Vec<Field>,
    pub view: Option<View>,
    pub operations: Operations,
    /// Label del `type: group` que lo contiene, si está agrupado.
    pub group: Option<String>,
    /// `commit.templates` del ítem (su `schemaCommitTemplates`).
    pub commit_templates: Option<CommitTemplates>,
    /// Extensión exigida a los archivos del ítem, derivada del
    /// `filename`/path como su normalización (`lib/config.ts`).
    pub extension: String,
    /// `subfolders: false` → solo archivos directos en el path.
    pub subfolders: Option<bool>,
    /// `actions:` del ítem (scope collection o entry según el uso que la
    /// UI le dé; su `getSchemaActions` filtra por `action.scope`).
    #[serde(default)]
    pub actions: Vec<Action>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PagesConfig {
    pub media: Media,
    pub content: Vec<ContentItem>,
    /// `actions:` de la raíz (scope repo). Su `getRootActions`.
    #[serde(default)]
    pub actions: Vec<Action>,
    pub warnings: Vec<String>,
    pub settings: Settings,
}

/// Una action del config: dispara un workflow de GitHub Actions
/// (`workflow_dispatch`). Puerto de su `RepoActionConfig` (lib/actions.ts).
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Action {
    pub name: String,
    pub label: String,
    /// Filename del workflow (el `workflow_id` del dispatch).
    pub workflow: String,
    /// `ref` del dispatch; ausente/"current" → rama actual (su
    /// `resolveActionRef`, se resuelve al correr).
    pub action_ref: Option<String>,
    /// Dónde vive el botón: "collection" (header de la colección),
    /// "entry" (header de la entrada) o ausente (solo la página Actions
    /// y el grupo raíz, como su `getSchemaActions`).
    pub scope: Option<String>,
    /// Default true en su UI; `false` oculta "Cancel run".
    pub cancelable: Option<bool>,
    /// `confirm: bool` o `{title, message, button}`; default true.
    pub confirm: Option<Confirm>,
    pub fields: Vec<ActionField>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Confirm {
    pub enabled: bool,
    pub title: Option<String>,
    pub message: Option<String>,
    pub button: Option<String>,
}

/// Campo del formulario al correr una action. Puerto de su
/// `RepoActionField`: text | textarea | select | checkbox | number.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ActionField {
    pub name: String,
    pub label: String,
    pub field_type: String,
    pub required: bool,
    /// Su default es string|number|bool; se conserva como string y la UI
    /// lo tipa por field_type.
    pub default: Option<String>,
    /// Values del select (su `options: [a, b]` / `options.values`).
    pub values: Vec<String>,
}

fn get_str(m: &Mapping, key: &str) -> Option<String> {
    m.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_rename(v: Option<&Value>, warnings: &mut Vec<String>) -> Option<String> {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => None,
        Some(Value::Bool(true)) => Some("safe".to_string()),
        Some(Value::String(s)) if s == "safe" || s == "random" => Some(s.clone()),
        Some(other) => {
            warnings.push(format!(
                "media.rename no reconocido: {other:?}; se slugifica (safe)"
            ));
            Some("safe".to_string())
        }
    }
}

fn media_from_mapping(m: &Mapping, warnings: &mut Vec<String>) -> Media {
    Media {
        input: get_str(m, "input"),
        output: get_str(m, "output"),
        rename: parse_rename(m.get("rename"), warnings),
        extensions: parse_extensions(m.get("extensions"), warnings),
    }
}

fn parse_media(v: Option<&Value>, warnings: &mut Vec<String>) -> Media {
    let empty = Media {
        input: None,
        output: None,
        rename: None,
        extensions: Vec::new(),
    };
    match v {
        None | Some(Value::Null) => empty,
        Some(Value::Mapping(m)) => media_from_mapping(m, warnings),
        Some(Value::Sequence(seq)) => {
            warnings.push(
                "media: hay varios sources; v0 usa solo el primero".to_string(),
            );
            match seq.first().and_then(Value::as_mapping) {
                Some(m) => media_from_mapping(m, warnings),
                None => empty,
            }
        }
        Some(_) => {
            warnings.push("media: formato no reconocido; se ignora".to_string());
            empty
        }
    }
}

fn parse_fields(
    fields: Option<&Value>,
    collection: &str,
    warnings: &mut Vec<String>,
) -> Vec<Field> {
    let Some(seq) = fields.and_then(Value::as_sequence) else {
        warnings.push(format!("content {collection}: sin fields"));
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            warnings.push(format!("content {collection}: campo sin estructura; ignorado"));
            continue;
        };
        let Some(name) = get_str(m, "name") else {
            warnings.push(format!("content {collection}: campo sin name; ignorado"));
            continue;
        };
        let label = get_str(m, "label").unwrap_or_else(|| name.clone());
        let field_type = get_str(m, "type").unwrap_or_else(|| "string".to_string());
        if !FIELD_TYPES.contains(&field_type.as_str()) {
            warnings.push(format!(
                "content {collection}.{name}: tipo {field_type} sin soporte; se muestra deshabilitado"
            ));
        }
        let required = m
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let help = get_str(m, "help");
        // Su schema admite `options: [a, b]` y `options: {values: [a, b]}`.
        let values = match m.get("options") {
            Some(Value::Sequence(seq)) => seq
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            Some(Value::Mapping(_)) => m
                .get("options")
                .and_then(Value::as_mapping)
                .and_then(|o| o.get("values"))
                .and_then(Value::as_sequence)
                .map(|s| {
                    s.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        out.push(Field {
            name,
            label,
            field_type,
            required,
            help,
            values,
        });
    }
    out
}

/// Sus `actions:` (raíz o ítem). Sin name/workflow se avisa y se ignora
/// la action; lo demás toma defaults como su `RepoActionConfig`.
fn parse_actions(v: Option<&Value>, ctx: &str, warnings: &mut Vec<String>) -> Vec<Action> {
    let Some(seq) = v.and_then(Value::as_sequence) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            warnings.push(format!("{ctx}: action sin estructura; ignorada"));
            continue;
        };
        let Some(name) = get_str(m, "name") else {
            warnings.push(format!("{ctx}: action sin name; ignorada"));
            continue;
        };
        let Some(workflow) = get_str(m, "workflow") else {
            warnings.push(format!("{ctx}: action {name} sin workflow; ignorada"));
            continue;
        };
        let confirm = match m.get("confirm") {
            None | Some(Value::Null) => None,
            Some(Value::Bool(b)) => Some(Confirm {
                enabled: *b,
                title: None,
                message: None,
                button: None,
            }),
            Some(Value::Mapping(c)) => Some(Confirm {
                // confirm: false no convive con el objeto en su schema;
                // un objeto implica confirmar.
                enabled: true,
                title: get_str(c, "title"),
                message: get_str(c, "message"),
                button: get_str(c, "button"),
            }),
            Some(other) => {
                warnings.push(format!(
                    "{ctx}: action {name} confirm no reconocido; se ignora ({other:?})"
                ));
                None
            }
        };
        let scope = match get_str(m, "scope") {
            None => None,
            Some(s) if s == "collection" || s == "entry" => Some(s),
            Some(other) => {
                warnings.push(format!(
                    "{ctx}: action {name} scope {other:?} no reconocido; se ignora"
                ));
                None
            }
        };
        out.push(Action {
            label: get_str(m, "label").unwrap_or_else(|| name.clone()),
            name,
            workflow,
            action_ref: get_str(m, "ref"),
            scope,
            cancelable: m.get("cancelable").and_then(Value::as_bool),
            confirm,
            fields: parse_action_fields(m.get("fields"), ctx, warnings),
        });
    }
    out
}

fn parse_action_fields(
    fields: Option<&Value>,
    ctx: &str,
    warnings: &mut Vec<String>,
) -> Vec<ActionField> {
    let Some(seq) = fields.and_then(Value::as_sequence) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in seq {
        let Some(m) = item.as_mapping() else {
            warnings.push(format!("{ctx}: action field sin estructura; ignorado"));
            continue;
        };
        let Some(name) = get_str(m, "name") else {
            warnings.push(format!("{ctx}: action field sin name; ignorado"));
            continue;
        };
        let field_type = get_str(m, "type").unwrap_or_else(|| "text".to_string());
        if !matches!(
            field_type.as_str(),
            "text" | "textarea" | "select" | "checkbox" | "number"
        ) {
            warnings.push(format!(
                "{ctx}: action field {name} de tipo {field_type} sin soporte; se trata como text"
            ));
        }
        // Su default es string|number|bool; se conserva su texto y la UI
        // lo convierte según el tipo.
        let default = match m.get("default") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Bool(b)) => Some(b.to_string()),
            Some(Value::Number(n)) => Some(n.to_string()),
            Some(other) => {
                warnings.push(format!(
                    "{ctx}: action field {name} default no escalar; se ignora ({other:?})"
                ));
                None
            }
        };
        let values = match m.get("options") {
            Some(Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|o| {
                    o.as_mapping()
                        .and_then(|om| om.get("value"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect(),
            _ => Vec::new(),
        };
        out.push(ActionField {
            label: get_str(m, "label").unwrap_or_else(|| name.clone()),
            name,
            field_type,
            required: m.get("required").and_then(Value::as_bool).unwrap_or(false),
            default,
            values,
        });
    }
    out
}

pub fn parse_config(raw: &str) -> Result<PagesConfig, String> {
    let root: Value =
        serde_yaml_ng::from_str(raw).map_err(|e| format!(".pages.yml inválido: {e}"))?;
    let mut warnings = Vec::new();
    let Some(root_map) = root.as_mapping().map(|m| m.to_owned()) else {
        return Err(".pages.yml inválido: la raíz no es un mapa".to_string());
    };

    let media = parse_media(root_map.get("media"), &mut warnings);

    let settings = Settings {
        commit_templates: parse_commit_templates(
            root_map
                .get("settings")
                .and_then(Value::as_mapping)
                .and_then(|s| s.get("commit"))
                .and_then(|c| c.get("templates")),
            &mut warnings,
            "settings",
        )
        .unwrap_or_default(),
    };

    let mut content = Vec::new();
    match root_map.get("content") {
        Some(Value::Sequence(items)) => {
            parse_content_items(items, None, &mut content, &mut warnings, 0);
        }
        _ => warnings.push("content ausente o no es una lista; no hay colecciones".to_string()),
    }

    let actions = parse_actions(root_map.get("actions"), "actions", &mut warnings);

    Ok(PagesConfig {
        media,
        content,
        actions,
        warnings,
        settings,
    })
}

/// Resuelve los ítems de `content`. Un `type: group` no es una ruta:
/// agrupa visualmente los ítems de su `items` (un nivel; anidados se
/// avisan y se aplanan).
fn parse_content_items(
    items: &[Value],
    group: Option<&str>,
    out: &mut Vec<ContentItem>,
    warnings: &mut Vec<String>,
    depth: usize,
) {
    for item in items {
        let Some(m) = item.as_mapping() else {
            warnings.push("content: ítem sin estructura; ignorado".to_string());
            continue;
        };
        let Some(name) = get_str(m, "name") else {
            warnings.push("content: ítem sin name; ignorado".to_string());
            continue;
        };
        let kind = match get_str(m, "type").as_deref() {
            None | Some("collection") => "collection",
            Some("file") => "file",
            Some("group") => {
                if depth > 0 {
                    warnings.push(format!(
                        "content {name}: group anidado; se aplana"
                    ));
                }
                let label = get_str(m, "label").unwrap_or_else(|| name.clone());
                if let Some(inner) = m.get("items").and_then(Value::as_sequence) {
                    parse_content_items(inner, Some(&label), out, warnings, depth + 1);
                } else {
                    warnings.push(format!(
                        "content {name}: group sin items; ignorado"
                    ));
                }
                continue;
            }
            Some(other) => {
                warnings.push(format!(
                    "content {name}: type {other} desconocido; ignorado"
                ));
                continue;
            }
        }
        .to_string();
        let Some(path) = get_str(m, "path") else {
            warnings.push(format!("content {name}: colección sin path; ignorada"));
            continue;
        };
        let fields = parse_fields(m.get("fields"), &name, warnings);
        let label = get_str(m, "label").unwrap_or_else(|| name.clone());
        let view = m.get("view").and_then(Value::as_mapping).map(|vm| View {
            fields: vm
                .get("fields")
                .and_then(Value::as_sequence)
                .map(|s| {
                    s.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            sort: get_str(vm, "sort"),
            order: get_str(vm, "order"),
        });
        let commit_templates = parse_commit_templates(
            m.get("commit").and_then(|c| c.get("templates")),
            warnings,
            &format!("content {name}"),
        );
        // Su normalización: extensión del filename (colecciones) o del
        // propio path (files); sin filename el default termina en .md.
        let filename_for_ext = if kind == "file" {
            path.clone()
        } else {
            get_str(m, "filename").unwrap_or_else(|| "{slug}.md".to_string())
        };
        let extension = ext_of(&filename_for_ext);
        let subfolders = m.get("subfolders").and_then(Value::as_bool);
        let item_actions = parse_actions(
            m.get("actions"),
            &format!("content {name}"),
            warnings,
        );
        out.push(ContentItem {
            name,
            kind: kind.clone(),
            label,
            path,
            filename: get_str(m, "filename"),
            fields,
            view,
            operations: resolve_operations(&kind, m),
            group: group.map(str::to_string),
            commit_templates,
            extension,
            subfolders,
            actions: item_actions,
        });
    }
}

#[tauri::command]
pub fn read_config(state: tauri::State<'_, AppState>) -> Result<PagesConfig, String> {
    let guard = state.lock().unwrap();
    match guard.as_ref().ok_or("no hay proyecto abierto")? {
        crate::state::Project::Local(st) => st
            .config
            .clone()
            .ok_or_else(|| "sin .pages.yml en la raíz del repo".to_string()),
        crate::state::Project::Remote(st) => st
            .config
            .clone()
            .ok_or_else(|| "el repo remoto no tiene .pages.yml".to_string()),
    }
}

// ---- editor Configuration (su página /configuration) ----
// Solo edita el `.pages.yml` existente: crear configuración quedó fuera
// de alcance (decisión del dueño, 2026-10-02), igual que en la web, donde
// un repo sin config no entra a esta página.

/// Resultado de guardar: el path escrito y la config re-parseada que
/// queda viva en el estado (el sidebar se re-arma con ella).
#[derive(Serialize, Debug)]
pub struct ConfigSave {
    pub path: String,
    pub config: PagesConfig,
}

/// YAML parseable, nada más: el lint que su Entry corre sobre `.pages.yml`
/// (`parseAndValidateConfig`); la UI lo muestra debajo del editor.
#[tauri::command]
pub fn validate_config(raw: String) -> Result<(), String> {
    parse_config(&raw).map(|_| ())
}

fn write_config_local(st: &mut crate::state::RepoState, raw: &str) -> Result<ConfigSave, String> {
    // Revalida aquí: un config roto no entra a disco aunque un caller
    // se salte el chequeo del comando.
    let cfg = parse_config(raw)?;
    crate::entry::ensure_writable(st, ".pages.yml")?;
    let full = crate::entry::safe_join(&st.root, ".pages.yml")?;
    fs::write(&full, raw.as_bytes()).map_err(|e| format!("escribir .pages.yml: {e}"))?;
    crate::entry::track(st, ".pages.yml");
    st.config = Some(cfg.clone());
    Ok(ConfigSave {
        path: ".pages.yml".to_string(),
        config: cfg,
    })
}

/// Texto crudo del `.pages.yml` (local: disco; remoto: Contents API con
/// caché de sesión para el sha del PUT).
#[tauri::command]
pub async fn read_config_raw(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let ctx = {
        let guard = state.lock().unwrap();
        match guard.as_ref().ok_or("no hay proyecto abierto")? {
            crate::state::Project::Local(st) => {
                st.config
                    .as_ref()
                    .ok_or_else(|| "sin .pages.yml en la raíz del repo".to_string())?;
                let full = crate::entry::safe_join(&st.root, ".pages.yml")?;
                return fs::read_to_string(&full).map_err(|e| format!("leer .pages.yml: {e}"));
            }
            crate::state::Project::Remote(rs) => {
                rs.config
                    .as_ref()
                    .ok_or_else(|| "el repo remoto no tiene .pages.yml".to_string())?;
                if let Some(f) = rs.files.get(".pages.yml") {
                    return Ok(f.text.clone());
                }
                crate::remote::remote_ctx(rs)
            }
        }
    };
    let token = crate::remote::current_token()?;
    let (t, c) = (token.clone(), ctx.clone());
    let (text, sha) = tauri::async_runtime::spawn_blocking(move || {
        crate::remote::fetch_text(&t, &c, ".pages.yml")
    })
    .await
    .map_err(|e| format!("leer .pages.yml: {e}"))??;
    let mut guard = state.lock().unwrap();
    if let Some(crate::state::Project::Remote(rs)) = guard.as_mut() {
        if crate::remote::is_same_remote(rs, &ctx) {
            rs.files.insert(
                ".pages.yml".to_string(),
                crate::state::RemoteFile { sha, text: text.clone() },
            );
        }
    }
    Ok(text)
}

/// Guarda el `.pages.yml`. El YAML se valida ANTES de escribir (un config
/// roto no entra a disco, como el lint de su Entry) y al éxito el estado
/// recarga la config para que el sidebar refleje los cambios.
#[tauri::command]
pub async fn write_config(state: tauri::State<'_, AppState>, raw: String) -> Result<ConfigSave, String> {
    let cfg = parse_config(&raw)?;
    let (ctx, sha) = {
        let mut guard = state.lock().unwrap();
        match guard.as_mut().ok_or("no hay proyecto abierto")? {
            crate::state::Project::Local(st) => return write_config_local(st, &raw),
            crate::state::Project::Remote(rs) => (
                crate::remote::remote_ctx(rs),
                rs.files.get(".pages.yml").map(|f| f.sha.clone()),
            ),
        }
    };
    let token = crate::remote::current_token()?;
    let login = crate::remote::current_session()?.login;
    let (t, c, cfg2, text) = (token.clone(), ctx.clone(), Some(cfg.clone()), raw.clone());
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        // Sin sha cacheado (el .pages.yml no pasa por read_entry): GET
        // primero — con 404 se rechaza porque Folio no crea configuración.
        let sha = match sha {
            Some(s) => Some(s),
            None => Some(match crate::gh_api::get_content(&t, &c.owner, &c.repo, ".pages.yml", &c.branch) {
                Ok(f) => f.sha,
                Err(e) if e.status == 404 => {
                    return Err("el repo remoto no tiene .pages.yml; Folio no crea configuración".to_string())
                }
                Err(e) => return Err(crate::remote::gh_error_ui(e)),
            }),
        };
        crate::remote::put_text_file(&t, &c, cfg2.as_ref(), None, &login, ".pages.yml", &text, sha.as_deref())
    })
    .await
    .map_err(|e| format!("guardar configuración: {e}"))??;
    let mut guard = state.lock().unwrap();
    if let Some(crate::state::Project::Remote(rs)) = guard.as_mut() {
        if crate::remote::is_same_remote(rs, &ctx) {
            rs.files.insert(
                ".pages.yml".to_string(),
                crate::state::RemoteFile { sha: outcome.sha, text: raw },
            );
            rs.config = Some(cfg.clone());
        }
    }
    Ok(ConfigSave {
        path: outcome.path,
        config: cfg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_YAML: &str = include_str!("../../fixtures/blog/.pages.yml");

    #[test]
    fn parsea_el_fixture_completo() {
        let cfg = parse_config(FIXTURE_YAML).unwrap();
        assert_eq!(cfg.content.len(), 1);
        let blog = &cfg.content[0];
        assert_eq!(blog.name, "blog");
        assert_eq!(blog.label, "Blog");
        assert_eq!(blog.path, "src/content/blog");
        assert_eq!(blog.fields.len(), 5);
        assert_eq!(blog.fields[0].name, "title");
        assert_eq!(blog.fields[0].field_type, "string");
        assert!(!blog.fields[0].required);
        assert_eq!(blog.fields[4].field_type, "rich-text");
        assert_eq!(
            blog.view.as_ref().unwrap().fields,
            vec!["title".to_string(), "pubDate".to_string()]
        );
        assert_eq!(cfg.media.input.as_deref(), Some("src/content/media"));
        assert_eq!(cfg.media.output.as_deref(), Some("/images"));
        assert!(cfg.warnings.is_empty(), "{:?}", cfg.warnings);
    }

    #[test]
    fn tipo_file_y_group_se_resuelven() {
        let raw = "content:\n  - name: hero\n    label: Hero\n    type: file\n    path: src/content/hero.json\n    fields:\n      - name: heading\n        type: string\n  - name: sitio\n    label: Sitio\n    type: group\n    items:\n      - name: blog\n        path: src/content/blog\n        fields:\n          - name: title\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(cfg.content.len(), 2);
        assert_eq!(cfg.content[0].kind, "file");
        assert_eq!(cfg.content[0].path, "src/content/hero.json");
        assert_eq!(cfg.content[1].name, "blog");
        assert_eq!(cfg.content[1].group.as_deref(), Some("Sitio"));
        assert_eq!(cfg.content[1].kind, "collection");
    }

    #[test]
    fn campo_de_tipo_desconocido_queda_como_disabled() {
        let raw = "content:\n  - name: blog\n    path: src/content/blog\n    fields:\n      - name: meta\n        type: object\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(cfg.content[0].fields[0].field_type, "object");
        assert!(cfg
            .warnings
            .iter()
            .any(|w| w.contains("object") && w.contains("deshabilitado")));
    }

    #[test]
    fn sin_content_avisa() {
        let cfg = parse_config("media:\n  input: m\n").unwrap();
        assert!(cfg.content.is_empty());
        assert!(cfg.warnings.iter().any(|w| w.contains("content ausente")));
    }

    #[test]
    fn operations_con_defaults_y_override_como_su_lib() {
        // Colección: todo true por defecto.
        let raw = "content:\n  - name: blog\n    path: src/content/blog\n    fields: []\n";
        let cfg = parse_config(raw).unwrap();
        assert!(cfg.content[0].operations.create);
        assert!(cfg.content[0].operations.rename);
        assert!(cfg.content[0].operations.delete);

        // File: rename false por defecto; override explícito lo apaga.
        let raw = "content:\n  - name: hero\n    type: file\n    path: src/hero.json\n    operations:\n      delete: false\n    fields: []\n";
        let cfg = parse_config(raw).unwrap();
        assert!(!cfg.content[0].operations.rename);
        assert!(!cfg.content[0].operations.delete);
        assert!(cfg.content[0].operations.create);
    }

    #[test]
    fn campos_del_set_completo_y_select_con_values() {
        let raw = "content:\n  - name: blog\n    path: src/content/blog\n    fields:\n      - name: n\n        type: number\n      - name: draft\n        type: boolean\n      - name: tags\n        type: select\n        options:\n          values: [a, b]\n      - name: src\n        type: code\n      - name: attachment\n        type: file\n";
        let cfg = parse_config(raw).unwrap();
        assert!(cfg.warnings.is_empty(), "{:?}", cfg.warnings);
        let fields = &cfg.content[0].fields;
        assert_eq!(fields[2].values, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn yaml_roto_es_error() {
        assert!(parse_config("content: [").is_err());
    }

    #[test]
    fn settings_commit_templates_se_parsean() {
        let raw = "media:\n  input: m\nsettings:\n  commit:\n    templates:\n      update: \"[{branch}] {path} editado por {user}\"\n      create: \"\"\n      delete: 42\ncontent: []\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(
            cfg.settings.commit_templates.update.as_deref(),
            Some("[{branch}] {path} editado por {user}")
        );
        // create vacío no cuenta (cae al default en el resolve).
        assert_eq!(cfg.settings.commit_templates.create, None);
        // delete no-string avisa y se ignora.
        assert!(cfg
            .warnings
            .iter()
            .any(|w| w.contains("settings") && w.contains("delete")));

        // Sin settings: defaults vacíos y sin warnings.
        let cfg = parse_config("content: []\n").unwrap();
        assert_eq!(cfg.settings.commit_templates, CommitTemplates::default());
        assert!(cfg.warnings.is_empty(), "{:?}", cfg.warnings);
    }

    #[test]
    fn commit_templates_por_item_se_parsean() {
        let raw = "content:\n  - name: blog\n    path: src/content/blog\n    commit:\n      templates:\n        update: \"Blog: {filename}\"\n    fields: []\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(
            cfg.content[0]
                .commit_templates
                .as_ref()
                .and_then(|t| t.update.as_deref()),
            Some("Blog: {filename}")
        );
        // Otro ítem sin commit → None.
        let raw = "content:\n  - name: blog\n    path: a\n    fields: []\n  - name: otro\n    path: b\n    fields: []\n";
        let cfg = parse_config(raw).unwrap();
        assert!(cfg.content[1].commit_templates.is_none());
    }

    #[test]
    fn extension_se_deriva_del_filename_como_su_normalizacion() {
        // Sin filename: el default termina en .md.
        let cfg = parse_config("content:\n  - name: blog\n    path: a\n    fields: []\n").unwrap();
        assert_eq!(cfg.content[0].extension, "md");
        // filename explícito manda.
        let cfg = parse_config(
            "content:\n  - name: blog\n    path: a\n    filename: \"{slug}.json\"\n    fields: []\n",
        )
        .unwrap();
        assert_eq!(cfg.content[0].extension, "json");
        // type: file la deriva del propio path.
        let cfg = parse_config(
            "content:\n  - name: hero\n    type: file\n    path: src/hero.yaml\n    fields: []\n",
        )
        .unwrap();
        assert_eq!(cfg.content[0].extension, "yaml");
        // subfolders llega tal cual.
        let cfg = parse_config(
            "content:\n  - name: blog\n    path: a\n    subfolders: false\n    fields: []\n",
        )
        .unwrap();
        assert_eq!(cfg.content[0].subfolders, Some(false));
    }

    #[test]
    fn media_extensions_expande_categorias() {
        let cfg = parse_config("media:\n  input: m\n  extensions: [image, pdf]\ncontent: []\n")
            .unwrap();
        assert!(cfg.media.extensions.contains(&"png".to_string()));
        assert!(cfg.media.extensions.contains(&"webp".to_string()));
        assert!(cfg.media.extensions.contains(&"pdf".to_string()));
        assert!(!cfg.media.extensions.contains(&"exe".to_string()));

        // Valor no string avisa y se ignora.
        let cfg = parse_config("media:\n  input: m\n  extensions: [png, 42]\ncontent: []\n")
            .unwrap();
        assert!(cfg.media.extensions.contains(&"png".to_string()));
        assert!(cfg
            .warnings
            .iter()
            .any(|w| w.contains("media.extensions")));
    }

    #[test]
    fn select_con_options_de_lista_plana() {
        // El portfolio real usa `options: [reading, read, pending]`.
        let cfg = parse_config("content:\n  - name: b\n    path: x\n    fields:\n      - name: status\n        type: select\n        options: [reading, read, pending]\n").unwrap();
        assert_eq!(
            cfg.content[0].fields[0].values,
            vec!["reading".to_string(), "read".to_string(), "pending".to_string()]
        );
    }

    #[test]
    fn ext_of_como_su_get_file_extension() {
        assert_eq!(ext_of("a/b/post.md"), "md");
        assert_eq!(ext_of("post.tar.gz"), "gz");
        assert_eq!(ext_of("sinext"), "");
        assert_eq!(ext_of(".gitkeep"), "");
        assert_eq!(ext_of(".config.json"), "json");
    }

    // ---- editor Configuration ----

    fn st_local(dir: &std::path::Path, yaml: &str) -> crate::state::RepoState {
        let repo = git2::Repository::init(dir).unwrap();
        crate::state::RepoState {
            repo,
            root: dir.to_path_buf(),
            config: Some(parse_config(yaml).unwrap()),
            touched: Vec::new(),
        }
    }

    #[test]
    fn write_config_local_escribe_trackea_y_recarga() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_local(dir.path(), FIXTURE_YAML);
        let nuevo = "media:\n  input: otro\ncontent:\n  - name: pages\n    label: Pages\n    path: content\n    fields:\n      - name: title\n        type: string\n";
        let save = write_config_local(&mut st, nuevo).unwrap();
        assert_eq!(save.path, ".pages.yml");
        assert_eq!(st.touched, vec![std::path::PathBuf::from(".pages.yml")]);
        // El estado quedó con la config nueva (el sidebar se re-arma).
        assert_eq!(st.config.as_ref().unwrap().content[0].name, "pages");
        // Y el disco tiene el texto exacto.
        assert_eq!(
            fs::read_to_string(dir.path().join(".pages.yml")).unwrap(),
            nuevo
        );
    }

    #[test]
    fn write_config_con_yaml_roto_no_toca_disco() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = st_local(dir.path(), FIXTURE_YAML);
        fs::write(dir.path().join(".pages.yml"), FIXTURE_YAML).unwrap();
        let antes = fs::read_to_string(dir.path().join(".pages.yml")).unwrap();
        let err = write_config_local(&mut st, "content: [").unwrap_err();
        assert!(err.contains(".pages.yml inválido"), "{err}");
        assert_eq!(
            fs::read_to_string(dir.path().join(".pages.yml")).unwrap(),
            antes
        );
        assert!(st.touched.is_empty());
    }

    #[test]
    fn validate_config_refleja_al_parser() {
        assert!(validate_config("content: []\n".to_string()).is_ok());
        assert!(validate_config("content: [".to_string()).is_err());
    }

    #[test]
    fn actions_de_raiz_y_de_item_se_parsean() {
        let raw = "actions:\n  - name: deploy\n    label: Deploy site\n    workflow: deploy.yml\n    ref: v1\n    cancelable: false\n    confirm:\n      title: Deploy?\n      message: This will deploy.\n      button: Deploy\n    fields:\n      - name: env\n        label: Environment\n        type: select\n        required: true\n        default: production\n        options:\n          - {label: Prod, value: production}\n          - {label: Staging, value: staging}\ncontent:\n  - name: blog\n    path: src/content/blog\n    actions:\n      - name: purge\n        workflow: purge.yml\n    fields: []\n";
        let cfg = parse_config(raw).unwrap();
        let a = &cfg.actions[0];
        assert_eq!(a.name, "deploy");
        assert_eq!(a.workflow, "deploy.yml");
        assert_eq!(a.action_ref.as_deref(), Some("v1"));
        assert_eq!(a.cancelable, Some(false));
        let confirm = a.confirm.as_ref().unwrap();
        assert!(confirm.enabled && confirm.title.as_deref() == Some("Deploy?"));
        let f = &a.fields[0];
        assert_eq!(f.field_type, "select");
        assert!(f.required);
        assert_eq!(f.default.as_deref(), Some("production"));
        assert_eq!(f.values, vec!["production".to_string(), "staging".to_string()]);
        // Per-ítem y defaults (label = name, sin confirm, sin fields).
        let purge = &cfg.content[0].actions[0];
        assert_eq!(purge.name, "purge");
        assert_eq!(purge.label, "purge");
        assert!(purge.confirm.is_none() && purge.fields.is_empty());
        assert!(cfg.warnings.is_empty(), "{:?}", cfg.warnings);
    }

    #[test]
    fn action_mala_avisa_y_no_corta() {
        let raw = "actions:\n  - label: sin name\n    workflow: a.yml\n  - name: sin workflow\n  - name: ok\n    workflow: ok.yml\ncontent: []\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(cfg.actions.len(), 1);
        assert_eq!(cfg.actions[0].name, "ok");
        assert_eq!(cfg.warnings.len(), 2, "{:?}", cfg.warnings);

        // confirm: false explícito → deshabilita la confirmación.
        let cfg = parse_config("actions:\n  - name: a\n    workflow: w.yml\n    confirm: false\ncontent: []\n").unwrap();
        assert_eq!(cfg.actions[0].confirm.as_ref().unwrap().enabled, false);
    }
}

use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

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
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct View {
    pub fields: Vec<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
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
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PagesConfig {
    pub media: Media,
    pub content: Vec<ContentItem>,
    pub warnings: Vec<String>,
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

fn parse_media(v: Option<&Value>, warnings: &mut Vec<String>) -> Media {
    let empty = Media {
        input: None,
        output: None,
        rename: None,
    };
    match v {
        None | Some(Value::Null) => empty,
        Some(Value::Mapping(m)) => Media {
            input: get_str(m, "input"),
            output: get_str(m, "output"),
            rename: parse_rename(m.get("rename"), warnings),
        },
        Some(Value::Sequence(seq)) => {
            warnings.push(
                "media: hay varios sources; v0 usa solo el primero".to_string(),
            );
            match seq.first().and_then(Value::as_mapping) {
                Some(m) => Media {
                    input: get_str(m, "input"),
                    output: get_str(m, "output"),
                    rename: parse_rename(m.get("rename"), warnings),
                },
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
        let values = m
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
            .unwrap_or_default();
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

pub fn parse_config(raw: &str) -> Result<PagesConfig, String> {
    let root: Value =
        serde_yaml_ng::from_str(raw).map_err(|e| format!(".pages.yml inválido: {e}"))?;
    let mut warnings = Vec::new();
    let Some(root_map) = root.as_mapping().map(|m| m.to_owned()) else {
        return Err(".pages.yml inválido: la raíz no es un mapa".to_string());
    };

    let media = parse_media(root_map.get("media"), &mut warnings);

    let mut content = Vec::new();
    match root_map.get("content") {
        Some(Value::Sequence(items)) => {
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
                        warnings.push(format!(
                            "content {name}: type group sin soporte aún; ignorado"
                        ));
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
                let fields = parse_fields(m.get("fields"), &name, &mut warnings);
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
                content.push(ContentItem {
                    name,
                    kind: kind.clone(),
                    label,
                    path,
                    filename: get_str(m, "filename"),
                    fields,
                    view,
                    operations: resolve_operations(&kind, m),
                });
            }
        }
        _ => warnings.push("content ausente o no es una lista; no hay colecciones".to_string()),
    }

    Ok(PagesConfig {
        media,
        content,
        warnings,
    })
}

#[tauri::command]
pub fn read_config(state: tauri::State<'_, AppState>) -> Result<PagesConfig, String> {
    let guard = state.lock().unwrap();
    let st = guard.as_ref().ok_or("no hay repo abierto")?;
    st.config
        .clone()
        .ok_or_else(|| "sin .pages.yml en la raíz del repo".to_string())
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
    fn tipo_file_se_parsea_y_group_sigue_ignorado() {
        let raw = "content:\n  - name: hero\n    label: Hero\n    type: file\n    path: src/content/hero.json\n    fields:\n      - name: heading\n        type: string\n  - name: grupo\n    type: group\n    path: x\n  - name: blog\n    path: src/content/blog\n    fields:\n      - name: title\n";
        let cfg = parse_config(raw).unwrap();
        assert_eq!(cfg.content.len(), 2);
        assert_eq!(cfg.content[0].kind, "file");
        assert_eq!(cfg.content[0].path, "src/content/hero.json");
        assert_eq!(cfg.content[1].kind, "collection");
        assert!(cfg.warnings.iter().any(|w| w.contains("grupo") && w.contains("group")));
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
}

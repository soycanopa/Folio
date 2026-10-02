//! Mensajes de commit del modo remoto. Puerto de su
//! `lib/commit-message.ts`: plantillas default con "(via Folio)"
//! (atribución verdadera; el resto es copia), override por `.pages.yml`
//! (`settings.commit.templates` y `{content}.commit.templates`),
//! tokens, espacios colapsados y recorte a 200.
//!
//! Divergencia documentada (third_party/NOTICE): Folio no manda
//! `committer` — GitHub firma con el usuario del token —, así que
//! `{userName}`/`{userEmail}` van vacíos y `{user}` es el login.

use crate::config::{CommitTemplates, ContentItem, PagesConfig};

fn default_template(action: &str) -> &'static str {
    match action {
        "create" => "Create {path} (via Folio)",
        "delete" => "Delete {path} (via Folio)",
        "rename" => "Rename {oldPath} to {newPath} (via Folio)",
        _ => "Update {path} (via Folio)",
    }
}

pub struct CommitContext<'a> {
    pub action: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
    pub branch: &'a str,
    /// `contentName` en su buildCommitTokens: el name del schema.
    pub name: Option<&'a str>,
    /// Su `user`: email || name || id; Folio tiene el login.
    pub user: &'a str,
    pub path: Option<&'a str>,
    pub old_path: Option<&'a str>,
    pub new_path: Option<&'a str>,
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn token_value<'a>(name: &str, ctx: &CommitContext<'a>) -> &'a str {
    match name {
        "action" => ctx.action,
        "owner" => ctx.owner,
        "repo" => ctx.repo,
        "branch" => ctx.branch,
        "name" => ctx.name.unwrap_or(""),
        "user" => ctx.user,
        "userName" => "",
        "userEmail" => "",
        "path" => ctx.path.unwrap_or(""),
        "filename" => ctx.path.map(file_name).unwrap_or(""),
        "oldPath" => ctx.old_path.unwrap_or(""),
        "oldFilename" => ctx.old_path.map(file_name).unwrap_or(""),
        "newPath" => ctx.new_path.unwrap_or(""),
        "newFilename" => ctx.new_path.map(file_name).unwrap_or(""),
        _ => "",
    }
}

/// Su renderCommitTemplate: cada `{word}` se reemplaza por el token o
/// por "" (regex `\{([a-zA-Z0-9_]+)\}`), escaneando a mano.
fn render_template(template: &str, ctx: &CommitContext) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(brace) = rest.find('{') {
        out.push_str(&rest[..brace]);
        let after = &rest[brace + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let name = &after[..end];
        if !name.is_empty() && after[end..].starts_with('}') {
            out.push_str(&token_value(name, ctx));
            rest = &after[end + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn template_for<'a>(templates: &'a CommitTemplates, action: &str) -> Option<&'a str> {
    match action {
        "create" => templates.create.as_deref(),
        "update" => templates.update.as_deref(),
        "delete" => templates.delete.as_deref(),
        "rename" => templates.rename.as_deref(),
        _ => None,
    }
}

fn usable(t: Option<&str>) -> Option<&str> {
    t.filter(|s| !s.trim().is_empty())
}

/// Su resolveCommitMessage: override del ítem → global → default;
/// luego `\s+` → " ", trim y slice(0, 200).
pub fn resolve_commit_message(
    config: Option<&PagesConfig>,
    item: Option<&ContentItem>,
    ctx: &CommitContext,
) -> String {
    let per_item = item
        .and_then(|i| i.commit_templates.as_ref())
        .and_then(|t| template_for(t, ctx.action));
    let global = config
        .map(|c| &c.settings.commit_templates)
        .and_then(|t| template_for(t, ctx.action));
    let template = usable(per_item)
        .or_else(|| usable(global))
        .unwrap_or_else(|| default_template(ctx.action));
    let rendered = render_template(template, ctx);
    let collapsed = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_config;

    fn ctx(action: &'static str) -> CommitContext<'static> {
        CommitContext {
            action,
            owner: "soycanopa",
            repo: "blog",
            branch: "main",
            name: Some("blog"),
            user: "soycanopa",
            path: Some("src/content/blog/mi-post.md"),
            old_path: None,
            new_path: None,
        }
    }

    #[test]
    fn defaults_por_accion() {
        assert_eq!(
            resolve_commit_message(None, None, &ctx("create")),
            "Create src/content/blog/mi-post.md (via Folio)"
        );
        assert_eq!(
            resolve_commit_message(None, None, &ctx("update")),
            "Update src/content/blog/mi-post.md (via Folio)"
        );
        let rename = CommitContext {
            action: "rename",
            old_path: Some("src/content/blog/viejo.md"),
            new_path: Some("src/content/blog/nuevo.md"),
            ..ctx("rename")
        };
        assert_eq!(
            resolve_commit_message(None, None, &rename),
            "Rename src/content/blog/viejo.md to src/content/blog/nuevo.md (via Folio)"
        );
    }

    #[test]
    fn override_global_y_por_item() {
        let cfg = parse_config(
            "content:\n  - name: blog\n    path: src/content/blog\n    commit:\n      templates:\n        update: \"Blog: {filename}\"\n    fields: []\nsettings:\n  commit:\n    templates:\n      update: \"Global {path}\"\n      delete: \"Bye {filename}\"\n",
        )
        .unwrap();
        // El ítem gana sobre el global.
        assert_eq!(
            resolve_commit_message(Some(&cfg), Some(&cfg.content[0]), &ctx("update")),
            "Blog: mi-post.md"
        );
        // Sin ítem (media): cae al global.
        assert_eq!(
            resolve_commit_message(Some(&cfg), None, &ctx("update")),
            "Global src/content/blog/mi-post.md"
        );
        // Acción sin override: default.
        assert_eq!(
            resolve_commit_message(Some(&cfg), Some(&cfg.content[0]), &ctx("delete")),
            "Bye mi-post.md"
        );
    }

    #[test]
    fn tokens_desconocidos_se_eliminan_y_los_conocidos_resuelven() {
        let cfg = parse_config(
            "settings:\n  commit:\n    templates:\n      update: \"{action} {owner}/{repo}@{branch} name={name} user={user} {foo} {path}\"\ncontent: []\n",
        )
        .unwrap();
        assert_eq!(
            resolve_commit_message(Some(&cfg), None, &ctx("update")),
            "update soycanopa/blog@main name=blog user=soycanopa src/content/blog/mi-post.md"
        );
        // userName/userEmail vacíos (sin committer).
        let cfg = parse_config(
            "settings:\n  commit:\n    templates:\n      update: \"[{user}|{userName}|{userEmail}]\"\ncontent: []\n",
        )
        .unwrap();
        assert_eq!(
            resolve_commit_message(Some(&cfg), None, &ctx("update")),
            "[soycanopa||]"
        );
    }

    #[test]
    fn llaves_que_no_son_token_quedan_literal() {
        let cfg = parse_config(
            "settings:\n  commit:\n    templates:\n      update: \"a{ b}c{}d{{path}}\"\ncontent: []\n",
        )
        .unwrap();
        // "{ b" no arranca un token (espacio), "{}" vacío, "{{path}}" deja
        // "{path}" literal: igual que su regex, que exige [A-Za-z0-9_]+.
        assert_eq!(
            resolve_commit_message(Some(&cfg), None, &ctx("update")),
            "a{ b}c{}d{src/content/blog/mi-post.md}"
        );
    }

    #[test]
    fn espacios_se_colapsan_y_recorta_a_200() {
        let cfg = parse_config(
            "settings:\n  commit:\n    templates:\n      update: \"  a\\n\\t b   c  {path} \"\ncontent: []\n",
        )
        .unwrap();
        assert_eq!(
            resolve_commit_message(Some(&cfg), None, &ctx("update")),
            "a b c src/content/blog/mi-post.md"
        );
        let long = "x".repeat(300);
        let cfg = parse_config(&format!(
            "settings:\n  commit:\n    templates:\n      update: \"{long}\"\ncontent: []\n"
        ))
        .unwrap();
        assert_eq!(resolve_commit_message(Some(&cfg), None, &ctx("update")).len(), 200);
    }
}

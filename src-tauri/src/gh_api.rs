//! Cliente de la API de GitHub (REST + GraphQL) para el modo remoto.
//!
//! Endpoints y comportamiento portados de Pages CMS: lectura de
//! config/entradas de su `lib/config-store.ts` y
//! `lib/github-cache-folders.ts`; save/delete/auto-rename de su
//! `app/api/[owner]/[repo]/[branch]/files/[path]/route.ts`; rename Git
//! Data de su `files/[path]/rename/route.ts`. Este módulo solo pide y
//! tipa; el estado del proyecto remoto vive en `remote.rs`.

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::github::urlencode;

const API: &str = "https://api.github.com";

static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

/// `http_status_as_error(false)`: ureq 3 no expone el body desde
/// `Error::StatusCode` y el `message` de GitHub (409 de sha, reglas,
/// rate limit) vive en ese body. El timeout global evita llamadas
/// colgadas para siempre.
fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build();
    AGENT.get_or_init(|| -> ureq::Agent { config.into() }).clone()
}

// ---- error ----

/// Error de la API con lo que la UI necesita. `retry_after` sale de
/// `retry-after` o de `x-ratelimit-remaining: 0` + reset, como su
/// wrapper de octokit (`lib/utils/octokit.ts`).
#[derive(Debug, Clone, PartialEq)]
pub struct GhError {
    pub status: u16,
    pub message: String,
    pub retry_after: Option<u64>,
}

impl GhError {
    pub fn ui(&self) -> String {
        match self.status {
            401 => "La sesión de GitHub expiró o el token es inválido; inicia sesión de nuevo."
                .to_string(),
            403 | 429 if self.retry_after.is_some() => format!(
                "GitHub API rate limit reached. Please wait {} seconds and try again.",
                self.retry_after.unwrap_or(0)
            ),
            0 => format!("GitHub: {}", self.message),
            s => format!("GitHub API ({s}): {}", self.message),
        }
    }
}

fn parse_retry_after(headers: &ureq::http::HeaderMap) -> Option<u64> {
    if let Some(n) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
    {
        return Some(n);
    }
    let remaining = headers.get("x-ratelimit-remaining")?.to_str().ok()?;
    if remaining != "0" {
        return None;
    }
    let reset = headers
        .get("x-ratelimit-reset")?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(reset.saturating_sub(now).max(1))
}

fn parse_error_body(body: &str, reason: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| {
            if reason.is_empty() {
                body.chars().take(200).collect()
            } else {
                reason.to_string()
            }
        })
}

// ---- envío ----

type Resp = ureq::http::Response<ureq::Body>;

fn call(req: ureq::RequestBuilder<ureq::typestate::WithoutBody>, token: &str) -> Result<Resp, GhError> {
    finish(
        req.header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "folio")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call(),
    )
}

fn call_json(
    req: ureq::RequestBuilder<ureq::typestate::WithBody>,
    token: &str,
    body: Value,
) -> Result<Resp, GhError> {
    finish(
        req.header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "folio")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send_json(body),
    )
}

fn finish(out: Result<Resp, ureq::Error>) -> Result<Resp, GhError> {
    let mut resp = out.map_err(|e| GhError {
        status: 0,
        message: e.to_string(),
        retry_after: None,
    })?;
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status().as_u16();
    let reason = resp.status().canonical_reason().unwrap_or("").to_string();
    let retry_after = parse_retry_after(resp.headers());
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    Err(GhError {
        status,
        message: parse_error_body(&body, &reason),
        retry_after,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(mut resp: Resp) -> Result<T, GhError> {
    resp.body_mut()
        .read_json::<T>()
        .map_err(|e| GhError {
            status: 0,
            message: format!("respuesta inválida: {e}"),
            retry_after: None,
        })
}

fn get_json<T: serde::de::DeserializeOwned>(token: &str, url: String) -> Result<T, GhError> {
    read_json(call(agent().get(url), token)?)
}

fn put_json<T: serde::de::DeserializeOwned>(
    token: &str,
    url: String,
    body: Value,
) -> Result<T, GhError> {
    read_json(call_json(agent().put(url), token, body)?)
}

/// Codifica un path repo-relativo para URL (segmento a segmento, sin
/// tocar los '/'): `a b/c.png` → `a%20b/c.png`.
pub(crate) fn encode_path(path: &str) -> String {
    path.split('/').map(urlencode).collect::<Vec<_>>().join("/")
}

// ---- base64 ----

pub fn encode_content(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// El Contents API devuelve el base64 con saltos de línea cada 60
/// caracteres; se limpian antes de decodificar.
pub fn decode_content(b64: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let cleaned: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(cleaned)
        .map_err(|e| format!("base64: {e}"))
}

// ---- actions (workflow_dispatch) ----

/// Una corrida de GitHub Actions, recortada a lo que la UI muestra.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct GhActionRun {
    pub id: i64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub head_sha: Option<String>,
    #[serde(default)]
    pub head_branch: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    /// Quien disparó la corrida (columna "Triggered by" de su tabla).
    #[serde(default)]
    pub actor: Option<GhActor>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub run_started_at: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct GhActor {
    #[serde(default)]
    pub login: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowRuns {
    #[serde(default)]
    workflow_runs: Vec<GhActionRun>,
}

/// POST `.../actions/workflows/{workflow}/dispatches` — 204 sin body.
/// `workflow` es el filename (el `workflow_id` acepta filename).
pub fn dispatch_workflow(
    token: &str,
    owner: &str,
    repo: &str,
    workflow: &str,
    dispatch_ref: &str,
    inputs: &Value,
) -> Result<(), GhError> {
    let url = format!("{API}/repos/{owner}/{repo}/actions/workflows/{workflow}/dispatches");
    call_json(
        agent().post(url),
        token,
        json!({ "ref": dispatch_ref, "inputs": inputs }),
    )?;
    Ok(())
}

/// Corridas de un workflow en una rama (`branch` filtra como su
/// `listWorkflowRuns`; paginación manual con `page`).
pub fn list_workflow_runs(
    token: &str,
    owner: &str,
    repo: &str,
    workflow: &str,
    branch: &str,
    per_page: u32,
    page: u32,
) -> Result<Vec<GhActionRun>, GhError> {
    let url = format!("{API}/repos/{owner}/{repo}/actions/workflows/{workflow}/runs");
    let req = agent()
        .get(url)
        .query("branch", branch)
        .query("per_page", per_page.to_string())
        .query("page", page.to_string());
    Ok(read_json::<WorkflowRuns>(call(req, token)?)?.workflow_runs)
}

pub fn get_workflow_run(token: &str, owner: &str, repo: &str, run_id: i64) -> Result<GhActionRun, GhError> {
    get_json(token, format!("{API}/repos/{owner}/{repo}/actions/runs/{run_id}"))
}

pub fn cancel_workflow_run(token: &str, owner: &str, repo: &str, run_id: i64) -> Result<(), GhError> {
    let url = format!("{API}/repos/{owner}/{repo}/actions/runs/{run_id}/cancel");
    call_json(agent().post(url), token, json!({}))?;
    Ok(())
}

pub fn rerun_workflow_run(token: &str, owner: &str, repo: &str, run_id: i64) -> Result<(), GhError> {
    let url = format!("{API}/repos/{owner}/{repo}/actions/runs/{run_id}/rerun");
    call_json(agent().post(url), token, json!({}))?;
    Ok(())
}

// ---- repo / contents ----

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct RepoMeta {
    pub full_name: String,
    #[serde(rename = "default_branch")]
    pub default_branch: String,
    pub private: bool,
    #[serde(default)]
    permissions: Option<RepoPerms>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
struct RepoPerms {
    #[serde(default)]
    push: bool,
}

impl RepoMeta {
    /// Sin bloque `permissions` (p. ej. al ver un repo ajeno) se
    /// considera sin push, como el filtro de `github_list_repos`.
    pub fn can_push(&self) -> bool {
        self.permissions.as_ref().is_some_and(|p| p.push)
    }
}

pub fn get_repo(token: &str, owner: &str, repo: &str) -> Result<RepoMeta, GhError> {
    get_json(
        token,
        format!("{API}/repos/{owner}/{repo}"),
    )
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct GhFile {
    pub name: String,
    pub path: String,
    pub sha: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

fn content_url(owner: &str, repo: &str, path: &str) -> String {
    format!("{API}/repos/{owner}/{repo}/contents/{}", encode_path(path))
}

/// GET de un archivo concreto (`?ref=rama`). 404 si no existe.
pub fn get_content(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
) -> Result<GhFile, GhError> {
    let req = agent().get(content_url(owner, repo, path)).query("ref", git_ref);
    read_json(call(req, token)?)
}

/// GET de un directorio: lista metadatos (con `sha` y `download_url`,
/// sin `content`). El listado de media de la web usa esto.
pub fn get_content_dir(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
) -> Result<Vec<GhFile>, GhError> {
    let dir = if path.is_empty() { ".".to_string() } else { path.to_string() };
    let req = agent().get(content_url(owner, repo, &dir)).query("ref", git_ref);
    read_json(call(req, token)?)
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PutContent {
    /// Puede venir null (paths largos); el sha nuevo se recupera con
    /// un GET posterior si hace falta.
    pub content: Option<GhFile>,
    pub commit: Option<CommitSha>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct CommitSha {
    pub sha: String,
}

/// `PUT /repos/{o}/{r}/contents/{path}`: un archivo = un commit, el
/// save de la web. Sin `sha` es create; con `sha`, update.
pub fn put_content(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
    message: &str,
    content_b64: &str,
    sha: Option<&str>,
) -> Result<PutContent, GhError> {
    let mut body = json!({
        "message": message,
        "content": content_b64,
        "branch": branch,
    });
    if let Some(sha) = sha {
        body["sha"] = json!(sha);
    }
    put_json(token, content_url(owner, repo, path), body)
}

/// `DELETE /repos/{o}/{r}/contents/{path}` con sha/branch/message en
/// query params, como el `deleteFile` de octokit que usa su server.
pub fn delete_content(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
    message: &str,
    sha: &str,
) -> Result<(), GhError> {
    let req = agent().delete(content_url(owner, repo, path))
        .query("branch", branch)
        .query("sha", sha)
        .query("message", message);
    let _: PutContent = read_json(call(req, token)?)?;
    Ok(())
}

// ---- GraphQL (listado de colecciones) ----

/// Query de su `fetchCollectionDirectoryEntries`: un request trae
/// nombres, paths, tipos y contenido de toda la carpeta.
pub const TREE_QUERY: &str = r#"
    query ($owner: String!, $repo: String!, $expression: String!) {
      repository(owner: $owner, name: $repo) {
        object(expression: $expression) {
          ... on Tree {
            entries {
              name
              path
              type
              object {
                ... on Blob {
                  text
                  oid
                  byteSize
                }
              }
            }
          }
        }
      }
    }
"#;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    /// "blob" | "tree".
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub object: Option<TreeBlob>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct TreeBlob {
    pub text: Option<String>,
    pub oid: String,
    #[serde(rename = "byteSize", default)]
    pub byte_size: Option<u64>,
}

pub fn fetch_tree_dir(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    dir: &str,
) -> Result<Vec<TreeEntry>, GhError> {
    let data = graphql(
        token,
        TREE_QUERY,
        json!({
            "owner": owner,
            "repo": repo,
            "expression": format!("{branch}:{dir}"),
        }),
    )?;
    parse_tree_response(&data)
}

/// Su manejo de la respuesta: `repository` ausente **o null** → no
/// encontrado; `object` null → carpeta vacía (git no trackea
/// directorios vacíos).
pub fn parse_tree_response(data: &Value) -> Result<Vec<TreeEntry>, GhError> {
    let not_found = || GhError {
        status: 404,
        message: "Repository was not found.".to_string(),
        retry_after: None,
    };
    let repository = data
        .get("repository")
        .filter(|v| !v.is_null())
        .ok_or_else(not_found)?;
    match repository.get("object") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(_) => {
            let entries = repository
                .get("object")
                .and_then(|o| o.get("entries"))
                .cloned()
                .unwrap_or(Value::Null);
            serde_json::from_value::<Vec<TreeEntry>>(entries).map_err(|e| GhError {
                status: 0,
                message: format!("tree inválido: {e}"),
                retry_after: None,
            })
        }
    }
}

fn graphql(token: &str, query: &str, variables: Value) -> Result<Value, GhError> {
    let resp = call_json(
        agent().post(format!("{API}/graphql")),
        token,
        json!({ "query": query, "variables": variables }),
    )?;
    let body: Value = read_json(resp)?;
    if let Some(first) = body
        .get("errors")
        .and_then(Value::as_array)
        .and_then(|errs| errs.first())
    {
        return Err(GhError {
            status: 200,
            message: first
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("error de GraphQL")
                .to_string(),
            retry_after: None,
        });
    }
    body.get("data").cloned().ok_or_else(|| GhError {
        status: 0,
        message: "respuesta GraphQL sin data".to_string(),
        retry_after: None,
    })
}

// ---- Git Data (rename, como su githubRenameFile) ----

#[derive(Deserialize)]
struct RawBranch {
    commit: RawBranchHead,
}

#[derive(Deserialize)]
struct RawBranchHead {
    sha: String,
}

pub fn get_branch_head(token: &str, owner: &str, repo: &str, branch: &str) -> Result<String, GhError> {
    let raw: RawBranch = get_json(
        token,
        format!("{API}/repos/{owner}/{repo}/branches/{}", urlencode(branch)),
    )?;
    Ok(raw.commit.sha)
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TreeItem {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub sha: String,
}

#[derive(Deserialize)]
struct TreeResponse {
    tree: Vec<TreeItem>,
}

pub fn get_tree_recursive(
    token: &str,
    owner: &str,
    repo: &str,
    tree_sha: &str,
) -> Result<Vec<TreeItem>, GhError> {
    let req = agent().get(format!("{API}/repos/{owner}/{repo}/git/trees/{tree_sha}"))
        .query("recursive", "true");
    let raw: TreeResponse = read_json(call(req, token)?)?;
    Ok(raw.tree)
}

/// El árbol del rename: blobs del árbol real con solo el path
/// renombrado (sin `base_tree`, como su implementación).
pub fn build_rename_tree(tree: &[TreeItem], path: &str, new_path: &str) -> Vec<TreeItem> {
    tree.iter()
        .filter(|item| item.kind != "tree")
        .map(|item| {
            let mut item = item.clone();
            if item.path == path {
                item.path = new_path.to_string();
            }
            item
        })
        .collect()
}

#[derive(Deserialize)]
struct ShaResponse {
    sha: String,
}

pub fn create_tree(
    token: &str,
    owner: &str,
    repo: &str,
    items: &[TreeItem],
) -> Result<String, GhError> {
    let body = json!({ "tree": items });
    let raw: ShaResponse = put_json(token, format!("{API}/repos/{owner}/{repo}/git/trees"), body)?;
    Ok(raw.sha)
}

pub fn create_commit(
    token: &str,
    owner: &str,
    repo: &str,
    message: &str,
    tree_sha: &str,
    parents: &[String],
) -> Result<String, GhError> {
    let raw: ShaResponse = put_json(
        token,
        format!("{API}/repos/{owner}/{repo}/git/commits"),
        json!({ "message": message, "tree": tree_sha, "parents": parents }),
    )?;
    Ok(raw.sha)
}

pub fn update_branch_ref(
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    sha: &str,
) -> Result<(), GhError> {
    let _: Value = call_json(
        agent().patch(format!("{API}/repos/{owner}/{repo}/git/refs/heads/{}", urlencode(branch))),
        token,
        json!({ "sha": sha, "force": false }),
    )
    .and_then(read_json)?;
    Ok(())
}

// ---- historial ----

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct GhCommit {
    pub sha: String,
    pub html_url: String,
    pub commit: GhCommitMeta,
    #[serde(default)]
    pub author: Option<GhCommitLogin>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct GhCommitMeta {
    pub message: String,
    #[serde(default)]
    pub author: GhSig,
}

#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
pub struct GhSig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct GhCommitLogin {
    pub login: String,
}

impl GhCommit {
    /// Prioriza el login de la cuenta; si no, la firma del commit.
    pub fn author_name(&self) -> String {
        self.author
            .as_ref()
            .map(|a| a.login.clone())
            .unwrap_or_else(|| self.commit.author.name.clone())
    }
}

/// `GET /repos/{o}/{r}/commits?path=&sha=` — el historial de la web.
pub fn list_commits(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
    per_page: usize,
) -> Result<Vec<GhCommit>, GhError> {
    let req = agent().get(format!("{API}/repos/{owner}/{repo}/commits"))
        .query("path", path)
        .query("sha", git_ref)
        .query("per_page", per_page.to_string());
    read_json(call(req, token)?)
}

// ---- descarga de media ----

/// Bytes de un archivo. Primero Contents API con Accept octet-stream
/// (api.github.com directo, sin redirects). Si falla y hay
/// `download_url` (raw → media.githubusercontent, que redirige), se
/// persigue el redirect a mano: ureq por defecto nunca reenvía
/// `Authorization` tras un redirect.
pub fn download_file(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
    download_url: Option<&str>,
) -> Result<Vec<u8>, GhError> {
    let first = download_via_contents(token, owner, repo, path, git_ref);
    match (first, download_url) {
        (Ok(bytes), _) => Ok(bytes),
        (Err(err), Some(url)) => download_with_redirects(token, url).or(Err(err)),
        (Err(err), None) => Err(err),
    }
}

fn download_via_contents(
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    git_ref: &str,
) -> Result<Vec<u8>, GhError> {
    let resp = finish(
        agent().get(content_url(owner, repo, path))
            .header("Accept", "application/octet-stream")
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "folio")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .query("ref", git_ref)
            .call(),
    )?;
    body_bytes(resp)
}

/// Sigue hasta 3 redirects llevando el Authorization, pero solo a
/// hosts de GitHub: nunca se filtra el token a un host ajeno.
fn download_with_redirects(token: &str, url: &str) -> Result<Vec<u8>, GhError> {
    let mut current = url.to_string();
    for _ in 0..3 {
        let resp = finish(
            agent().get(&current)
                .header("Accept", "application/octet-stream")
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "folio")
                .call(),
        )?;
        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| GhError {
                    status: 0,
                    message: "redirect sin Location".to_string(),
                    retry_after: None,
                })?
                .to_string();
            if !is_github_host(&location) {
                return Err(GhError {
                    status: 0,
                    message: format!("redirect a un host ajeno rechazado: {location}"),
                    retry_after: None,
                });
            }
            current = location;
            continue;
        }
        return body_bytes(resp);
    }
    Err(GhError {
        status: 0,
        message: "demasiados redirects".to_string(),
        retry_after: None,
    })
}

fn is_github_host(url: &str) -> bool {
    let host = url
        .strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("");
    host == "github.com" || host.ends_with(".githubusercontent.com") || host == "githubusercontent.com"
}

fn body_bytes(mut resp: Resp) -> Result<Vec<u8>, GhError> {
    let mut buf = Vec::new();
    resp.body_mut()
        .as_reader()
        .read_to_end(&mut buf)
        .map_err(|e| GhError {
            status: 0,
            message: format!("leer cuerpo: {e}"),
            retry_after: None,
        })?;
    Ok(buf)
}

// ---- auto-rename 422 ----

/// Puerto de su auto-rename (`files/[path]/route.ts`): con los nombres
/// del directorio arma `name-<N+1..N+3>` sin pisar los `name-<k>`
/// existentes. Prefijo/sufijo en vez de su regex (mismo efecto).
pub fn rename_candidates(dir_names: &[String], filename: &str, extension: &str) -> Vec<String> {
    let max = dir_names
        .iter()
        .filter_map(|n| conflict_number(n, filename, extension))
        .max()
        .unwrap_or(0);
    (1..=3)
        .map(|i| {
            if extension.is_empty() {
                format!("{filename}-{}", max + i)
            } else {
                format!("{filename}-{}.{extension}", max + i)
            }
        })
        .collect()
}

fn conflict_number(name: &str, filename: &str, extension: &str) -> Option<u64> {
    let prefix = format!("{filename}-");
    let suffix = if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    };
    let stem = name.strip_prefix(&prefix)?.strip_suffix(&suffix)?;
    stem.parse().ok()
}

// ---- fechas ----

/// GitHub manda fechas ISO 8601 UTC (`2026-01-02T15:04:05Z`); el
/// historial local usa unix secs. Días-from-civil de Howard Hinnant.
pub fn iso_to_unix(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let sep = |i: usize, c: u8| b.get(i) == Some(&c);
    if !(sep(4, b'-') && sep(7, b'-') && (sep(10, b'T') || sep(10, b't') || sep(10, b' ')))
        || !(sep(13, b':') && sep(16, b':'))
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> { s.get(r)?.parse().ok() };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hh, mm, ss) = (num(11..13)?, num(14..16)?, num(17..19)?);
    Some(days_from_civil(y, m, d) * 86_400 + hh * 3_600 + mm * 60 + ss)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsea_corridas_de_workflow() {
        let raw = r#"{
            "total_count": 2,
            "workflow_runs": [
                {
                    "id": 1234,
                    "status": "completed",
                    "conclusion": "success",
                    "html_url": "https://github.com/o/r/actions/runs/1234",
                    "head_sha": "abc1234567890abcdef1234567890abcdef1234",
                    "head_branch": "main",
                    "event": "workflow_dispatch",
                    "created_at": "2026-10-02T19:33:23Z",
                    "updated_at": "2026-10-02T19:35:01Z",
                    "run_started_at": "2026-10-02T19:33:24Z"
                },
                { "id": 1233, "status": "in_progress" }
            ]
        }"#;
        let runs: Vec<GhActionRun> = serde_json::from_str::<WorkflowRuns>(raw)
            .unwrap()
            .workflow_runs;
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].conclusion.as_deref(), Some("success"));
        assert_eq!(runs[0].event.as_deref(), Some("workflow_dispatch"));
        // Campos ausentes → None (defaults), nunca crash.
        assert_eq!(runs[1].conclusion, None);
        assert_eq!(runs[1].created_at, None);
    }

    #[test]
    fn parsea_archivo_de_contents() {
        let raw = r#"{
            "name": "post.md", "path": "src/content/blog/post.md",
            "sha": "abc123", "type": "file", "size": 120,
            "download_url": "https://raw.githubusercontent.com/o/r/main/src/content/blog/post.md",
            "content": "LS0tCnRpdGxlOiBIaQotLS0KCmJvZHk="
        }"#;
        let f: GhFile = serde_json::from_str(raw).unwrap();
        assert_eq!(f.kind, "file");
        assert_eq!(f.sha, "abc123");
        assert_eq!(decode_content(f.content.as_deref().unwrap()).unwrap(), b"---\ntitle: Hi\n---\n\nbody");
    }

    #[test]
    fn base64_del_api_trae_saltos_y_decodifica() {
        // GitHub parte el base64 en líneas de 60 chars.
        let with_nl = "LS0t\ndGl0\nbGU6\nIEhp\n";
        assert_eq!(decode_content(with_nl).unwrap(), b"---title: Hi");
        let roundtrip = decode_content(&encode_content(b"bytes \x00\x01")).unwrap();
        assert_eq!(roundtrip, b"bytes \x00\x01");
    }

    #[test]
    fn parsea_listado_de_directorio() {
        let raw = r#"[
            {"name":"a.png","path":"m/a.png","sha":"s1","type":"file","download_url":"https://raw.githubusercontent.com/o/r/main/m/a.png"},
            {"name":"sub","path":"m/sub","sha":"s2","type":"dir"}
        ]"#;
        let dir: Vec<GhFile> = serde_json::from_str(raw).unwrap();
        assert_eq!(dir.len(), 2);
        assert_eq!(dir[0].kind, "file");
        assert!(dir[0].download_url.is_some());
        assert_eq!(dir[1].kind, "dir");
    }

    #[test]
    fn parsea_respuesta_del_put() {
        let raw = r#"{
            "content": {"name":"a.md","path":"c/a.md","sha":"nuevo","type":"file"},
            "commit": {"sha": "commitsha", "commit": {"message": "Update c/a.md (via Folio)"}}
        }"#;
        let put: PutContent = serde_json::from_str(raw).unwrap();
        assert_eq!(put.content.as_ref().unwrap().sha, "nuevo");
        assert_eq!(put.commit.as_ref().unwrap().sha, "commitsha");

        // Path largo: GitHub puede devolver content null.
        let put: PutContent = serde_json::from_str(r#"{"content": null, "commit": {"sha": "cs"}}"#).unwrap();
        assert!(put.content.is_none());
    }

    #[test]
    fn parsea_arbol_graphql() {
        let data: Value = serde_json::from_str(
            r#"{"repository": {"object": {"entries": [
                {"name":"a.md","path":"blog/a.md","type":"blob","object":{"text":"---\ntitle: A\n","oid":"o1","byteSize":14}},
                {"name":"sub","path":"blog/sub","type":"tree","object":null}
            ]}}}"#,
        )
        .unwrap();
        let entries = parse_tree_response(&data).unwrap();
        assert_eq!(entries.len(), 2);
        let blob = entries.iter().find(|e| e.kind == "blob").unwrap();
        assert_eq!(blob.object.as_ref().unwrap().oid, "o1");
        assert!(blob.object.as_ref().unwrap().text.as_deref().unwrap().contains("title: A"));

        // Carpeta vacía: git no trackea directorios vacíos → object null.
        let data: Value = serde_json::from_str(r#"{"repository": {"object": null}}"#).unwrap();
        assert!(parse_tree_response(&data).unwrap().is_empty());

        // Repo inexistente.
        let data: Value = serde_json::from_str(r#"{"repository": null}"#).unwrap();
        let err = parse_tree_response(&data).unwrap_err();
        assert_eq!(err.status, 404);
    }

    #[test]
    fn parsea_commits_del_historial() {
        let raw = r#"[{
            "sha": "cs1",
            "html_url": "https://github.com/o/r/commit/cs1",
            "commit": {"message": "Update post (via Folio)", "author": {"name": "Carlos", "date": "2026-01-02T15:04:05Z"}},
            "author": {"login": "soycanopa"}
        }]"#;
        let commits: Vec<GhCommit> = serde_json::from_str(raw).unwrap();
        assert_eq!(commits[0].author_name(), "soycanopa");
        assert_eq!(commits[0].commit.author.name, "Carlos");
        assert_eq!(iso_to_unix("2026-01-02T15:04:05Z"), Some(1_767_366_245));

        // Sin cuenta de GitHub asociada cae a la firma del commit.
        let raw = r#"[{"sha":"c","html_url":"u","commit":{"message":"m","author":{"name":"Ana","date":"2026-01-02T15:04:05Z"}}}]"#;
        let commits: Vec<GhCommit> = serde_json::from_str(raw).unwrap();
        assert_eq!(commits[0].author_name(), "Ana");
    }

    #[test]
    fn iso_a_unix_valores_conocidos() {
        assert_eq!(iso_to_unix("1970-01-01T00:00:00Z"), Some(0));
        // Bisiesto: 2000-03-01T00:00:00Z = 951868800.
        assert_eq!(iso_to_unix("2000-02-29T12:00:00Z"), Some(951_825_600));
        assert_eq!(iso_to_unix("2026-10-02T00:00:00Z"), Some(1_790_899_200));
        assert_eq!(iso_to_unix("2026-01-02T15:04:05.123Z"), Some(1_767_366_245));
        assert_eq!(iso_to_unix("fecha"), None);
        assert_eq!(iso_to_unix("2026-01-02"), None);
    }

    #[test]
    fn arbol_de_rename_cambia_solo_el_path_y_tira_trees() {
        let tree = vec![
            TreeItem { path: "README.md".into(), mode: "100644".into(), kind: "blob".into(), sha: "a".into() },
            TreeItem { path: "blog/viejo.md".into(), mode: "100644".into(), kind: "blob".into(), sha: "b".into() },
            TreeItem { path: "blog".into(), mode: "040000".into(), kind: "tree".into(), sha: "t".into() },
        ];
        let out = build_rename_tree(&tree, "blog/viejo.md", "blog/nuevo.md");
        assert_eq!(out.len(), 2); // el tree explícito se cae
        assert!(out.iter().all(|i| i.kind != "tree"));
        let renamed = out.iter().find(|i| i.path == "blog/nuevo.md").unwrap();
        assert_eq!(renamed.sha, "b");
        assert_eq!(renamed.mode, "100644");
    }

    #[test]
    fn candidatos_de_auto_rename_como_su_regex() {
        let names = vec![
            "post.md".to_string(),
            "post-1.md".to_string(),
            "post-3.md".to_string(),
            "post-x.md".to_string(),
            "otro-9.md".to_string(),
        ];
        assert_eq!(
            rename_candidates(&names, "post", "md"),
            vec!["post-4.md", "post-5.md", "post-6.md"]
        );
        // Sin conflicto previo arranca en -1.
        assert_eq!(
            rename_candidates(&[], "a", "png"),
            vec!["a-1.png", "a-2.png", "a-3.png"]
        );
        // Sin extensión.
        assert_eq!(rename_candidates(&names, "post", ""), vec!["post-1", "post-2", "post-3"]);
    }

    #[test]
    fn error_body_y_headers_se_parsean() {
        let msg = parse_error_body(r#"{"message":"Repository rule violations found","documentation_url":"x"}"#, "Conflict");
        assert_eq!(msg, "Repository rule violations found");
        let msg = parse_error_body("no json", "Not Found");
        assert_eq!(msg, "Not Found");

        let mut headers = ureq::http::HeaderMap::new();
        headers.insert("retry-after", "7".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(7));

        let mut headers = ureq::http::HeaderMap::new();
        headers.insert("x-ratelimit-remaining", "0".parse().unwrap());
        let reset = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 30;
        headers.insert("x-ratelimit-reset", reset.to_string().parse().unwrap());
        let wait = parse_retry_after(&headers).unwrap();
        assert!((25..=30).contains(&wait), "wait: {wait}");

        let e = GhError { status: 429, message: "ml".into(), retry_after: Some(9) };
        assert!(e.ui().contains("Please wait 9 seconds"));
    }

    #[test]
    fn repo_meta_sin_permisos_no_puede_pushear() {
        let meta: RepoMeta = serde_json::from_str(
            r#"{"full_name":"o/r","default_branch":"main","private":true}"#,
        )
        .unwrap();
        assert!(meta.private);
        assert!(!meta.can_push());
        let meta: RepoMeta = serde_json::from_str(
            r#"{"full_name":"o/r","default_branch":"main","private":false,"permissions":{"push":true}}"#,
        )
        .unwrap();
        assert!(meta.can_push());
    }

    #[test]
    fn encode_path_codifica_segmentos() {
        assert_eq!(encode_path("a b/c.png"), "a%20b/c.png");
        assert_eq!(encode_path("blog/a.md"), "blog/a.md");
    }

    #[test]
    fn hosts_permitidos_en_redirect() {
        assert!(is_github_host("https://media.githubusercontent.com/media/o/r/b/p"));
        assert!(is_github_host("https://raw.githubusercontent.com/o/r/b/p"));
        assert!(is_github_host("https://github.com/x"));
        assert!(!is_github_host("https://evil.com/media"));
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;

    // Manual: cargo test gh_api_real -- --ignored --nocapture
    #[test]
    #[ignore]
    fn fetch_tree_dir_real() {
        let session = crate::github::load_session().unwrap().expect("sin sesión");
        let entries = fetch_tree_dir(&session.token, "hunvreus", "pagescms", "main", "components").unwrap();
        println!("OK {} entradas", entries.len());
    }
}

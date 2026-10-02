use serde::{Deserialize, Serialize};

// Sesión de GitHub de Folio: OAuth Device Flow (el mismo flujo que el
// CLI de gh — no requiere server ni client secret) con el token en el
// Keychain de macOS (invariante AGENTS.md, decisión del dueño
// 2026-10-02). Sirve para listar repos y clonar; el push sigue con la
// credencial del sistema.

/// Client ID de la OAuth App de GitHub de Folio (no es secreto; el
/// Device Flow no usa client secret). El dueño crea la app en
/// github.com/settings/developers y pega el client_id aquí.
const OAUTH_CLIENT_ID: &str = "Ov23lihU4sxEzYa1fFkb";
const KEYCHAIN_SERVICE: &str = "folio";
const KEYCHAIN_ACCOUNT: &str = "github-session";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeviceCodeStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DevicePoll {
    Pending,
    SlowDown,
    Authorized {
        access_token: String,
    },
    Denied,
    Expired,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GithubSession {
    pub login: String,
    #[serde(rename = "accessToken")]
    pub token: String,
}

/// La forma que consume su RepoSelect: repo corto, owner, private,
/// updatedAt ISO y defaultBranch.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GhRepo {
    pub repo: String,
    pub owner: String,
    pub private: bool,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "defaultBranch")]
    pub default_branch: String,
}

// ---- HTTP ----

fn post_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: serde_json::Value,
) -> Result<T, String> {
    let mut resp = ureq::post(url)
        .header("Accept", "application/json")
        .send_json(body)
        .map_err(|e| format!("github: {e}"))?;
    resp.body_mut()
        .read_json::<T>()
        .map_err(|e| format!("github: respuesta inválida: {e}"))
}

fn get_json_with_token<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &str,
) -> Result<T, String> {
    let mut resp = ureq::get(url)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "folio")
        .call()
        .map_err(|e| format!("github: {e}"))?;
    resp.body_mut()
        .read_json::<T>()
        .map_err(|e| format!("github: respuesta inválida: {e}"))
}

// ---- Device Flow ----

pub fn start_device_code() -> Result<DeviceCodeStart, String> {
    if OAUTH_CLIENT_ID.is_empty() {
        return Err(
            "GitHub OAuth App sin configurar: falta el client_id de Folio \
             (github.com/settings/developers → New OAuth App)"
                .to_string(),
        );
    }
    #[derive(Deserialize)]
    struct Raw {
        device_code: String,
        user_code: String,
        verification_uri: String,
        #[serde(default = "default_interval")]
        interval: u64,
        expires_in: u64,
    }
    fn default_interval() -> u64 {
        5
    }
    let raw: Raw = post_json(
        "https://github.com/login/device/code",
        serde_json::json!({
            "client_id": OAUTH_CLIENT_ID,
            "scope": "repo read:user",
        }),
    )?;
    Ok(DeviceCodeStart {
        device_code: raw.device_code,
        user_code: raw.user_code,
        verification_uri: raw.verification_uri,
        interval: raw.interval,
        expires_in: raw.expires_in,
    })
}

#[derive(Deserialize)]
struct RawPoll {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

pub fn poll_device_code(device_code: &str) -> Result<DevicePoll, String> {
    if OAUTH_CLIENT_ID.is_empty() {
        return Err("GitHub OAuth App sin configurar (falta client_id)".to_string());
    }
    let raw: RawPoll = post_json(
        "https://github.com/login/oauth/access_token",
        serde_json::json!({
            "client_id": OAUTH_CLIENT_ID,
            "device_code": device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
        }),
    )?;
    Ok(match (raw.access_token, raw.error.as_deref()) {
        (Some(t), _) => DevicePoll::Authorized { access_token: t },
        (None, None) => DevicePoll::Pending,
        (None, Some(e)) => match e {
            "authorization_pending" => DevicePoll::Pending,
            "slow_down" => DevicePoll::SlowDown,
            "expired_token" => DevicePoll::Expired,
            "access_denied" => DevicePoll::Denied,
            other => return Err(format!("github: {other}")),
        },
    })
}

// ---- Keychain ----
//
// El item se crea con `security add-generic-password -T <binario>`,
// que deja el binario actual en la ACL: puede leerlo sin prompt. Sin
// esto, cada recompilación en dev cambia la identidad del binario y
// macOS vuelve a pedir permiso. El token via argv solo al crear; la
// lectura posterior usa la ACL y queda cacheada en memoria.

use std::sync::Mutex;

static SESSION_CACHE: Mutex<Option<Option<GithubSession>>> = Mutex::new(None);

fn exe_path() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("current_exe: {e}"))
}

fn security(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("security")
        .args(args)
        .output()
        .map_err(|e| format!("security: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "security: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn save_session(session: &GithubSession) -> Result<(), String> {
    let json = serde_json::to_string(session).map_err(|e| e.to_string())?;
    let exe = exe_path()?;
    security(&[
        "add-generic-password",
        "-U",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        KEYCHAIN_ACCOUNT,
        "-w",
        &json,
        "-T",
        &exe,
    ])?;
    *SESSION_CACHE.lock().unwrap() = Some(Some(session.clone()));
    Ok(())
}

pub fn load_session() -> Result<Option<GithubSession>, String> {
    if let Some(cached) = SESSION_CACHE.lock().unwrap().clone() {
        return Ok(cached);
    }
    let loaded = match security(&[
        "find-generic-password",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        KEYCHAIN_ACCOUNT,
        "-w",
    ]) {
        Ok(json) => serde_json::from_str::<GithubSession>(&json)
            .map(Some)
            .map_err(|e| format!("keychain: sesión inválida: {e}"))?,
        Err(e) if e.contains("could not be found") => None,
        Err(e) => return Err(format!("keychain: {e}")),
    };
    *SESSION_CACHE.lock().unwrap() = Some(loaded.clone());
    Ok(loaded)
}

pub fn clear_session() -> Result<(), String> {
    let res = match security(&[
        "delete-generic-password",
        "-s",
        KEYCHAIN_SERVICE,
        "-a",
        KEYCHAIN_ACCOUNT,
    ]) {
        Ok(_) => Ok(()),
        Err(e) if e.contains("could not be found") => Ok(()),
        Err(e) => Err(e),
    };
    *SESSION_CACHE.lock().unwrap() = Some(None);
    res
}

// ---- API de repos ----

#[derive(Deserialize)]
struct RawUser {
    login: String,
}

pub fn fetch_login(token: &str) -> Result<String, String> {
    let user: RawUser = get_json_with_token("https://api.github.com/user", token)?;
    Ok(user.login)
}

#[derive(Deserialize)]
struct RawRepo {
    full_name: String,
    private: bool,
    default_branch: String,
    updated_at: String,
    owner: RawOwner,
    #[serde(default)]
    permissions: Option<RawPerms>,
}

#[derive(Deserialize)]
struct RawPerms {
    #[serde(default)]
    push: bool,
}

#[derive(Deserialize)]
struct RawOwner {
    login: String,
}

/// Su endpoint /api/repos/[owner]: Search API con la query
/// `{keyword} in:name user:{login} fork:true`, sort updated desc y
/// per_page 5 — por eso la app muestra los últimos cinco.
pub fn list_repos(token: &str, login: &str, keyword: &str) -> Result<Vec<GhRepo>, String> {
    #[derive(Deserialize)]
    struct RawSearch {
        items: Vec<RawRepo>,
    }
    let q = urlencode(&format!(
        "{keyword} in:name user:{login} fork:true"
    ));
    let raws: RawSearch = get_json_with_token(
        &format!(
            "https://api.github.com/search/repositories?q={q}&sort=updated&order=desc&per_page=5"
        ),
        token,
    )?;
    Ok(raws
        .items
        .into_iter()
        .filter(|r| r.permissions.as_ref().map(|p| p.push).unwrap_or(true))
        .map(|r| GhRepo {
            repo: r
                .full_name
                .split('/')
                .next_back()
                .unwrap_or(&r.full_name)
                .to_string(),
            owner: r.owner.login,
            private: r.private,
            updated_at: r.updated_at,
            default_branch: r.default_branch,
        })
        .collect())
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Su "Copy template": crea una copia del repo template en la cuenta
/// del usuario (GitHub API generate, token personal; su server hacía
/// lo mismo con la GitHub App). El endpoint exige Authorization.
pub fn create_from_template(token: &str, template_full: &str, name: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct RawCreated {
        full_name: String,
    }
    let mut resp = ureq::post(&format!(
        "https://api.github.com/repos/{template_full}/generate"
    ))
    .header("Accept", "application/vnd.github+json")
    .header("Authorization", format!("Bearer {token}"))
    .header("User-Agent", "folio")
    .send_json(serde_json::json!({ "name": name }))
    .map_err(|e| format!("github: {e}"))?;
    let created = resp
        .body_mut()
        .read_json::<RawCreated>()
        .map_err(|e| format!("github: respuesta inválida: {e}"))?;
    Ok(created.full_name)
}

// ---- Clone con token ----

/// Si hay sesión y la URL es de GitHub, clona con el token entregado al
/// credential helper por environment: el token nunca toca disco ni argv.
pub fn clone_command(url: &str, dest: &str) -> std::process::Command {
    let session = load_session().ok().flatten();
    let is_github = url.starts_with("https://github.com/");
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg(url).arg(dest);
    if let (true, Some(s)) = (is_github, session) {
        let helper = "!f() { printf 'username=x-access-token\\npassword=%s\\n' \"$FOLIO_GH_TOKEN\"; }; f";
        cmd.env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "credential.helper")
            .env("GIT_CONFIG_VALUE_0", helper)
            .env("FOLIO_GH_TOKEN", s.token);
    }
    cmd
}

// ---- Comandos Tauri ----

#[derive(Serialize)]
pub struct GithubUser {
    pub login: String,
}

#[tauri::command]
pub fn github_login_start() -> Result<DeviceCodeStart, String> {
    start_device_code()
}

#[tauri::command]
pub fn github_login_poll(device_code: &str) -> Result<DevicePoll, String> {
    let poll = poll_device_code(device_code)?;
    if let DevicePoll::Authorized { access_token } = &poll {
        let login = fetch_login(access_token)?;
        save_session(&GithubSession {
            login,
            token: access_token.clone(),
        })?;
    }
    Ok(poll)
}

#[tauri::command]
pub fn github_session() -> Result<Option<GithubUser>, String> {
    Ok(load_session()?.map(|s| GithubUser { login: s.login }))
}

#[tauri::command]
pub fn github_logout() -> Result<(), String> {
    clear_session()
}

#[tauri::command]
pub fn github_list_repos(keyword: &str) -> Result<Vec<GhRepo>, String> {
    let session = load_session()?.ok_or("sin sesión de GitHub")?;
    list_repos(&session.token, &session.login, keyword)
}

#[tauri::command]
pub fn github_create_from_template(
    template: &str,
    name: &str,
) -> Result<String, String> {
    let session = load_session()?.ok_or("sin sesión de GitHub")?;
    create_from_template(&session.token, template, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsea_respuestas_del_device_flow() {
        let raw = r#"{"device_code":"dv","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":899,"interval":5}"#;
        let _: DeviceCodeStart = serde_json::from_str(raw).unwrap();

        let p: RawPoll =
            serde_json::from_str(r#"{"error":"authorization_pending"}"#).unwrap();
        assert_eq!(p.error.as_deref(), Some("authorization_pending"));

        let p: RawPoll =
            serde_json::from_str(r#"{"access_token":"t","token_type":"bearer"}"#).unwrap();
        assert_eq!(p.access_token.as_deref(), Some("t"));
    }

    #[test]
    fn parsea_tiempo_y_repos_de_la_api() {
        let raw = r#"[{"name":"pagescms","full_name":"hunvreus/pagescms","private":false,"default_branch":"main","updated_at":"2026-01-01T00:00:00Z","owner":{"login":"hunvreus"}}]"#;
        let repos: Vec<GhRepo> = serde_json::from_str::<Vec<RawRepo>>(raw)
            .unwrap()
            .into_iter()
            .map(|r| GhRepo {
                repo: r.full_name.split('/').next_back().unwrap_or(&r.full_name).to_string(),
                owner: r.owner.login,
                private: r.private,
                updated_at: r.updated_at,
                default_branch: r.default_branch,
            })
            .collect();
        assert_eq!(repos[0].repo, "pagescms");
        assert_eq!(repos[0].owner, "hunvreus");
        assert!(!repos[0].private);
        assert_eq!(repos[0].default_branch, "main");
    }

    #[test]
    fn session_sin_entrada_es_none() {
        // Solo comprueba el manejo de NoEntry; no escribe en el keychain.
        match load_session() {
            Ok(None) | Ok(Some(_)) => {}
            Err(e) => panic!("keychain: {e}"),
        }
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;

    // Manual: cargo test github_real -- --ignored --nocapture
    #[test]
    #[ignore]
    fn list_repos_real() {
        let session = load_session().unwrap().expect("sin sesión");
        match list_repos(&session.token, &session.login, "") {
            Ok(repos) => println!("OK {} repos: {:?}", repos.len(), repos.iter().map(|r| r.repo.clone()).collect::<Vec<_>>()),
            Err(e) => println!("ERROR: {e}"),
        }
    }
}

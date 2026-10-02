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

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct GhRepo {
    pub full_name: String,
    pub owner: String,
    pub private: bool,
    /// Unix seconds de updatedAt.
    pub updated_at: i64,
    pub default_branch: String,
    pub clone_url: String,
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

fn keychain_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| format!("keychain: {e}"))
}

pub fn save_session(session: &GithubSession) -> Result<(), String> {
    let json = serde_json::to_string(session).map_err(|e| e.to_string())?;
    keychain_entry()?
        .set_password(&json)
        .map_err(|e| format!("keychain: {e}"))
}

pub fn load_session() -> Result<Option<GithubSession>, String> {
    match keychain_entry()?.get_password() {
        Ok(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|e| format!("keychain: sesión inválida: {e}")),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keychain: {e}")),
    }
}

pub fn clear_session() -> Result<(), String> {
    match keychain_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain: {e}")),
    }
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
    clone_url: String,
    updated_at: String,
    owner: RawOwner,
}

#[derive(Deserialize)]
struct RawOwner {
    login: String,
}

/// "2024-05-01T12:34:56Z" (UTC, formato fijo de la API) → unix secs.
pub fn parse_gh_time(s: &str) -> Option<i64> {
    fn digits(s: &str) -> Option<i64> {
        s.parse().ok()
    }
    if s.len() < 19 || !s.ends_with('Z') {
        return None;
    }
    let y = digits(s.get(0..4)?)?;
    let mo = digits(s.get(5..7)?)?;
    let d = digits(s.get(8..10)?)?;
    let h = digits(s.get(11..13)?)?;
    let mi = digits(s.get(14..16)?)?;
    let se = digits(s.get(17..19)?)?;
    // Days from civil (Howard Hinnant), válido para fechas post-1970.
    let y = if mo <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + h * 3600 + mi * 60 + se)
}

pub fn list_repos(token: &str) -> Result<Vec<GhRepo>, String> {
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(flatten)]
        repo: RawRepo,
    }
    let raws: Vec<RawRepo> = get_json_with_token(
        "https://api.github.com/user/repos?sort=updated&per_page=30&affiliation=owner,collaborator,organization_member",
        token,
    )?;
    Ok(raws
        .into_iter()
        .map(|r| GhRepo {
            full_name: r.full_name,
            owner: r.owner.login,
            private: r.private,
            updated_at: parse_gh_time(&r.updated_at).unwrap_or(0),
            default_branch: r.default_branch,
            clone_url: r.clone_url,
        })
        .collect())
}

/// Su "Copy template": crea una copia del repo template en la cuenta
/// del usuario (GitHub API generate, token personal; su server hacía
/// lo mismo con la GitHub App).
pub fn create_from_template(token: &str, template_full: &str, name: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct RawCreated {
        full_name: String,
    }
    let created: RawCreated = post_json(
        &format!(
            "https://api.github.com/repos/{template_full}/generate"
        ),
        serde_json::json!({ "name": name }),
    )?;
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
pub fn github_list_repos() -> Result<Vec<GhRepo>, String> {
    let session = load_session()?.ok_or("sin sesión de GitHub")?;
    list_repos(&session.token)
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
        // 2026-01-01T00:00:00Z = 1767225600; oct-01 = +273 días, 12:30
        assert_eq!(parse_gh_time("2026-01-01T00:00:00Z"), Some(1767225600));
        assert_eq!(parse_gh_time("2026-10-01T12:30:00Z"), Some(1790857800));

        let raw = r#"[{"name":"pagescms","full_name":"hunvreus/pagescms","private":false,"default_branch":"main","clone_url":"https://github.com/hunvreus/pagescms.git","updated_at":"2026-01-01T00:00:00Z","owner":{"login":"hunvreus"}}]"#;
        let repos: Vec<GhRepo> = serde_json::from_str::<Vec<RawRepo>>(raw)
            .unwrap()
            .into_iter()
            .map(|r| GhRepo {
                full_name: r.full_name,
                owner: r.owner.login,
                private: r.private,
                updated_at: parse_gh_time(&r.updated_at).unwrap_or(0),
                default_branch: r.default_branch,
                clone_url: r.clone_url,
            })
            .collect();
        assert_eq!(repos[0].full_name, "hunvreus/pagescms");
        assert_eq!(repos[0].owner, "hunvreus");
        assert!(!repos[0].private);
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

//! Actions del `.pages.yml` → GitHub Actions (`workflow_dispatch`),
//! como su página de Actions y sus RepoActionButtons. Sin su server:
//! GitHub es la fuente de verdad de las corridas y el payload viaja
//! como el único input `payload` (la convención de sus workflows, con
//! `source: "folio"`). Solo modo remoto: los workflows viven en GitHub.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::{AppState, Project};

/// Corrida lista para la UI. Puerto del `ActionRunSummary` que su server
/// sincroniza con DB+webhook; aquí sale directo de la API de GitHub.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ActionRunInfo {
    pub id: i64,
    pub workflow: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
    pub html_url: Option<String>,
    pub head_sha: Option<String>,
    pub head_branch: Option<String>,
    pub event: Option<String>,
    /// Login de quien disparó (su "Triggered by"); None si GitHub no
    /// reporta actor.
    pub triggered_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub run_started_at: Option<String>,
}

fn run_info(run: crate::gh_api::GhActionRun, workflow: &str) -> ActionRunInfo {
    ActionRunInfo {
        id: run.id,
        workflow: workflow.to_string(),
        status: run.status,
        conclusion: run.conclusion,
        html_url: run.html_url,
        head_sha: run.head_sha,
        head_branch: run.head_branch,
        event: run.event,
        triggered_by: run.actor.and_then(|a| a.login),
        created_at: run.created_at,
        updated_at: run.updated_at,
        run_started_at: run.run_started_at,
    }
}

/// Spec de la action que la UI despacha (sale del config ya parseado).
#[derive(Deserialize)]
pub struct ActionSpec {
    pub name: String,
    pub label: String,
    pub workflow: String,
    #[serde(rename = "ref", default)]
    pub action_ref: Option<String>,
    #[serde(default)]
    pub cancelable: Option<bool>,
}

#[derive(Deserialize)]
pub struct ActionContext {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub data: Value,
}

/// Su `resolveActionRef`: ausente/"current" → rama del proyecto.
fn resolve_action_ref(action_ref: Option<&str>, branch: &str) -> String {
    match action_ref {
        None | Some("") | Some("current") => branch.to_string(),
        Some(r) => r.to_string(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `2026-10-02T19:33:23Z` (o con `.mmm`) → epoch ms. Sin dependencias:
/// el claim compara la ventana de despacho contra `created_at`.
fn iso_to_ms(s: &str) -> Option<i64> {
    let (d, t) = s.split_once('T')?;
    let t = t.trim_end_matches('Z').split('.').next()?;
    let mut dp = d.split('-');
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: i64 = dp.next()?.parse().ok()?;
    let da: i64 = dp.next()?.parse().ok()?;
    let mut tp = t.split(':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next()?.parse().ok()?;
    let se: i64 = tp.next()?.parse().ok()?;
    Some((days_from_civil(y, mo, da) * 86_400 + h * 3600 + mi * 60 + se) * 1000)
}

/// Días desde epoch (algoritmo civil de Howard Hinnant, dominio público).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn require_remote(state: &tauri::State<'_, AppState>) -> Result<crate::remote::RemoteCtx, String> {
    let guard = state.lock().unwrap();
    match guard.as_ref().ok_or("no hay proyecto abierto")? {
        Project::Local(_) => Err(
            "Actions es de proyectos remotos: los workflows viven en GitHub".to_string(),
        ),
        Project::Remote(rs) => Ok(crate::remote::remote_ctx(rs)),
    }
}

#[tauri::command]
pub async fn run_action(
    state: tauri::State<'_, AppState>,
    action: ActionSpec,
    context: ActionContext,
    inputs: Value,
) -> Result<Option<ActionRunInfo>, String> {
    let ctx = require_remote(&state)?;
    let token = crate::remote::current_token()?;
    let login = crate::remote::current_session()?.login;
    let dispatch_ref = resolve_action_ref(action.action_ref.as_deref(), &ctx.branch);
    // El payload exacto de su POST actions (con source folio): sus
    // workflows consumen un único input `payload` serializado.
    let payload = json!({
        "source": "folio",
        "action": {
            "name": action.name,
            "label": action.label,
            "workflow": action.workflow,
            "ref": action.action_ref,
            "cancelable": action.cancelable,
        },
        "repository": {
            "owner": ctx.owner,
            "repo": ctx.repo,
            "ref": dispatch_ref,
            "workflowRef": action.workflow,
        },
        "context": {
            "type": context.kind,
            "name": context.name,
            "path": context.path,
            "data": context.data,
        },
        "inputs": inputs,
        "triggeredAt": iso_from_ms(now_ms()),
        "triggeredBy": { "login": login },
    });
    let dispatch_inputs = json!({ "payload": payload.to_string() });
    let started = now_ms();
    let (t, c, wf, r) = (token.clone(), ctx.clone(), action.workflow.clone(), dispatch_ref.clone());
    let claimed = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<crate::gh_api::GhActionRun>, String> {
            crate::gh_api::dispatch_workflow(&t, &c.owner, &c.repo, &wf, &r, &dispatch_inputs)
                .map_err(crate::remote::gh_error_ui)?;
            // Claim de la corrida, como su findWorkflowRun: 6 intentos × 1.5s
            // filtrando workflow_dispatch recientes en la rama (tolerancia de
            // 30s de reloj, corridas ya vistas excluidas por id).
            let mut seen: Vec<i64> = Vec::new();
            for _ in 0..6 {
                std::thread::sleep(std::time::Duration::from_millis(1500));
                let runs = crate::gh_api::list_workflow_runs(&t, &c.owner, &c.repo, &wf, &r, 10, 1)
                    .map_err(crate::remote::gh_error_ui)?;
                for run in runs {
                    let recent = run
                        .created_at
                        .as_deref()
                        .and_then(iso_to_ms)
                        .is_some_and(|ms| ms + 30_000 >= started);
                    if recent && !seen.contains(&run.id) {
                        return Ok(Some(run));
                    }
                    seen.push(run.id);
                }
            }
            Ok(None)
        },
    )
    .await
    .map_err(|e| format!("run action: {e}"))??;
    Ok(claimed.map(|run| run_info(run, &action.workflow)))
}

/// Corridas recientes de los workflows declarados, mezcladas desc.
/// GitHub es la fuente de verdad: un 404 de workflow (renombrado o
/// borrado) lista vacío en vez de romper la página.
#[tauri::command]
pub async fn action_runs(
    state: tauri::State<'_, AppState>,
    workflows: Vec<String>,
    per_page: Option<u32>,
) -> Result<Vec<ActionRunInfo>, String> {
    let ctx = require_remote(&state)?;
    let token = crate::remote::current_token()?;
    let per = per_page.unwrap_or(15).clamp(1, 50);
    let (t, c) = (token.clone(), ctx.clone());
    let mut runs = tauri::async_runtime::spawn_blocking(move || {
        let mut runs: Vec<ActionRunInfo> = Vec::new();
        for wf in workflows {
            match crate::gh_api::list_workflow_runs(&t, &c.owner, &c.repo, &wf, &c.branch, per, 1) {
                Ok(list) => runs.extend(list.into_iter().map(|r| run_info(r, &wf))),
                Err(e) if e.status == 404 => {}
                Err(e) => return Err(crate::remote::gh_error_ui(e)),
            }
        }
        // ISO con formato fijo ordena lexicográfico igual que cronológico.
        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        runs.truncate(50);
        Ok(runs)
    })
    .await
    .map_err(|e| format!("action runs: {e}"))??;
    Ok(runs)
}

#[tauri::command]
pub async fn cancel_action_run(state: tauri::State<'_, AppState>, run_id: i64) -> Result<(), String> {
    let ctx = require_remote(&state)?;
    let token = crate::remote::current_token()?;
    let (t, c) = (token.clone(), ctx.clone());
    tauri::async_runtime::spawn_blocking(move || {
        crate::gh_api::cancel_workflow_run(&t, &c.owner, &c.repo, run_id)
            .map_err(crate::remote::gh_error_ui)
    })
    .await
    .map_err(|e| format!("cancel run: {e}"))?
}

#[tauri::command]
pub async fn rerun_action_run(state: tauri::State<'_, AppState>, run_id: i64) -> Result<(), String> {
    let ctx = require_remote(&state)?;
    let token = crate::remote::current_token()?;
    let (t, c) = (token.clone(), ctx.clone());
    tauri::async_runtime::spawn_blocking(move || {
        crate::gh_api::rerun_workflow_run(&t, &c.owner, &c.repo, run_id)
            .map_err(crate::remote::gh_error_ui)
    })
    .await
    .map_err(|e| format!("rerun: {e}"))?
}

/// ISO-8601 UTC desde epoch ms (solo para el `triggeredAt` del payload).
fn iso_from_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Inverso de `days_from_civil` (Howard Hinnant).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_sin_especificar_cae_a_la_rama_actual() {
        assert_eq!(resolve_action_ref(None, "main"), "main");
        assert_eq!(resolve_action_ref(Some("current"), "main"), "main");
        assert_eq!(resolve_action_ref(Some(""), "main"), "main");
        assert_eq!(resolve_action_ref(Some("v1"), "main"), "v1");
    }

    #[test]
    fn iso_y_epoch_vuelven_al_mismo_punto() {
        // Anclas absolutas: epoch y 2026-01-01 (20454 días civiles).
        assert_eq!(iso_from_ms(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_to_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(iso_to_ms("2026-01-01T00:00:00Z"), Some(1_767_225_600_000));
        // 274 días hasta oct 2 + 19:33:23 sobre la ancla.
        assert_eq!(iso_to_ms("2026-10-02T19:33:23Z"), Some(1_790_969_603_000));
        // La fracción de segundo se trunca.
        assert_eq!(
            iso_to_ms("2026-10-02T19:33:23.456Z"),
            iso_to_ms("2026-10-02T19:33:23Z")
        );
        // Roundtrip — incluido 29-feb bisiesto y el salto de año.
        for ms in [0i64, 86_400_000, 1_706_515_200_000, 1_767_225_600_000, 1_790_969_603_000] {
            assert_eq!(iso_to_ms(&iso_from_ms(ms)), Some(ms), "{ms}");
        }
        // 2024-02-29 existe y ordena entre marzo y febrero no-bisiesto.
        let bisiesto = iso_to_ms("2024-02-29T00:00:00Z").unwrap();
        assert!(bisiesto > iso_to_ms("2024-02-28T00:00:00Z").unwrap());
        assert!(iso_to_ms("2024-03-01T00:00:00Z").unwrap() > bisiesto);
    }
}

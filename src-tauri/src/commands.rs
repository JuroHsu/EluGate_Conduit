//! UI 可呼叫的限定核心操作；不提供任意指令執行入口。

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::askpass::Prompt;
use crate::core::{CmdError, CmdResult, Core, DestCheck, TestResult, TunnelView};
use crate::logs::{Level, LogEntry};
use crate::model::{Prefs, Tunnel, Validation};
use crate::ssh_config::{self, FileSnapshot, HostEntry, HostForm};
use crate::ssh_bin::{self, SshInfo};

type Core_<'a> = State<'a, Arc<Core>>;

#[tauri::command]
pub fn get_state(core: Core_) -> Vec<TunnelView> {
    core.views()
}

/// 目前實際使用的語系（已解析「跟隨系統」）
#[tauri::command]
pub fn get_language() -> &'static str {
    crate::i18n::current()
}

#[tauri::command]
pub fn get_prefs(core: Core_) -> Prefs {
    core.config().prefs
}

#[tauri::command]
pub fn validate_tunnel(core: Core_, tunnel: Tunnel) -> Validation {
    core.validate(&tunnel)
}

#[tauri::command]
pub fn save_tunnel(core: Core_, tunnel: Tunnel) -> CmdResult<Tunnel> {
    core.save_tunnel(tunnel)
}

#[tauri::command]
pub fn duplicate_tunnel(core: Core_, id: String) -> CmdResult<Tunnel> {
    core.duplicate_tunnel(&id)
}

#[tauri::command]
pub async fn delete_tunnel(core: Core_<'_>, id: String) -> CmdResult<()> {
    core.delete_tunnel(&id).await
}

#[tauri::command]
pub async fn connect_tunnel(core: Core_<'_>, id: String) -> CmdResult<()> {
    core.connect(&id).await
}

#[tauri::command]
pub async fn disconnect_tunnel(core: Core_<'_>, id: String) -> CmdResult<()> {
    core.disconnect(&id).await
}

#[tauri::command]
pub async fn switch_tunnel(core: Core_<'_>, id: String) -> CmdResult<()> {
    core.switch_to(&id).await
}

#[tauri::command]
pub async fn restart_tunnel(core: Core_<'_>, id: String) -> CmdResult<()> {
    core.restart(&id).await
}

#[tauri::command]
pub async fn stop_all(core: Core_<'_>) -> CmdResult<()> {
    core.stop_all().await;
    Ok(())
}

#[tauri::command]
pub async fn check_destination(core: Core_<'_>, id: String) -> CmdResult<Option<DestCheck>> {
    Ok(core.check_destination(&id).await)
}

#[tauri::command]
pub async fn test_tunnel(core: Core_<'_>, tunnel: Tunnel) -> CmdResult<TestResult> {
    core.test_tunnel(tunnel).await
}

#[tauri::command]
pub fn get_logs(core: Core_, tunnel_id: Option<String>) -> Vec<LogEntry> {
    core.logger.list(tunnel_id.as_deref())
}

#[tauri::command]
pub fn clear_logs(core: Core_) {
    core.logger.clear();
}

#[tauri::command]
pub fn open_log_dir(app: AppHandle, core: Core_) -> CmdResult<()> {
    app.opener()
        .open_path(core.logger.dir.display().to_string(), None::<&str>)
        .map_err(|e| CmdError::new("error", e.to_string()))
}

// ───────────── SSH 設定檔 ─────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostList {
    path: String,
    exists: bool,
    hosts: Vec<HostEntry>,
}

fn config_path(core: &Core, path: Option<String>) -> PathBuf {
    let path = path.filter(|p| !p.trim().is_empty()).or_else(|| core.config().prefs.ssh_config_path);
    ssh_config::resolve(path.as_deref())
}

#[tauri::command]
pub fn list_hosts(core: Core_, path: Option<String>) -> CmdResult<HostList> {
    let p = config_path(&core, path);
    Ok(HostList { path: p.display().to_string(), exists: p.exists(), hosts: ssh_config::list_hosts(&p)? })
}

#[tauri::command]
pub async fn effective_host(core: Core_<'_>, alias: String, path: Option<String>) -> CmdResult<Vec<(String, String)>> {
    let ssh = core.ssh_path()?;
    let p = config_path(&core, path);
    let cfg = p.exists().then_some(p);
    Ok(ssh_config::effective(&ssh, &alias, cfg.as_deref()).await?)
}

#[tauri::command]
pub fn read_ssh_config(core: Core_, path: Option<String>) -> CmdResult<FileSnapshot> {
    Ok(ssh_config::read(&config_path(&core, path))?)
}

#[tauri::command]
pub fn host_form(core: Core_, alias: String, path: Option<String>) -> CmdResult<Option<HostForm>> {
    let snap = ssh_config::read(&config_path(&core, path))?;
    Ok(ssh_config::read_form(&snap.content, &alias))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    base_hash: String,
    content: String,
    diff: String,
}

#[tauri::command]
pub fn preview_host_form(core: Core_, form: HostForm, path: Option<String>) -> CmdResult<Preview> {
    let snap = ssh_config::read(&config_path(&core, path))?;
    let content = ssh_config::apply_form(&snap.content, &form)?;
    Ok(Preview { diff: ssh_config::diff(&snap.content, &content), base_hash: snap.hash, content })
}

#[tauri::command]
pub fn preview_ssh_config(core: Core_, path: Option<String>, content: String) -> CmdResult<Preview> {
    let snap = ssh_config::read(&config_path(&core, path))?;
    Ok(Preview { diff: ssh_config::diff(&snap.content, &content), base_hash: snap.hash, content })
}

#[tauri::command]
pub async fn save_ssh_config(
    core: Core_<'_>,
    path: Option<String>,
    content: String,
    base_hash: String,
) -> CmdResult<Option<String>> {
    let p = config_path(&core, path);
    match core.ssh_path() {
        Ok(ssh) => ssh_config::validate_content(&ssh, &content, &core.data_dir.join("tmp")).await?,
        Err(_) => core.log(Level::Warn, None, crate::tr!("log.noSshSkipValidation")),
    }
    let backup = ssh_config::write(&p, &content, &base_hash, &core.data_dir.join("backups"))?;
    let msg = match &backup {
        Some(b) => crate::tr!("log.sshConfigUpdated", path = p.display(), backup = b.display()),
        None => crate::tr!("log.sshConfigCreated", path = p.display()),
    };
    core.log(Level::Info, None, msg);
    Ok(backup.map(|b| b.display().to_string()))
}

// ───────────── 環境與偏好 ─────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvInfo {
    ssh: SshInfo,
    data_dir: String,
    log_dir: String,
    default_config: String,
    autostart: bool,
    version: String,
    platform: &'static str,
}

#[tauri::command]
pub async fn get_env(app: AppHandle, core: Core_<'_>) -> CmdResult<EnvInfo> {
    let prefs = core.config().prefs;
    Ok(EnvInfo {
        ssh: ssh_bin::info(prefs.ssh_path.as_deref()).await,
        data_dir: core.data_dir.display().to_string(),
        log_dir: core.logger.dir.display().to_string(),
        default_config: ssh_config::default_path().display().to_string(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
    })
}

#[tauri::command]
pub fn set_prefs(core: Core_, prefs: Prefs) -> CmdResult<()> {
    core.set_prefs(prefs)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> CmdResult<()> {
    let a = app.autolaunch();
    let r = if enabled { a.enable() } else { a.disable() };
    r.map_err(|e| CmdError::new("error", crate::tr!("err.autostart", error = e)))
}

// ───────────── 驗證互動 ─────────────

#[tauri::command]
pub fn get_prompt(core: Core_, id: String) -> Option<Prompt> {
    core.askpass.get().and_then(|a| a.get(&id))
}

#[tauri::command]
pub fn answer_prompt(core: Core_, id: String, value: Option<String>) -> bool {
    core.askpass.get().is_some_and(|a| a.answer(&id, value))
}

// ───────────── 視窗 ─────────────

#[tauri::command]
pub fn show_main(app: AppHandle) {
    crate::windows::show_main(&app);
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    crate::windows::hide_panel(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::quit(&app);
}

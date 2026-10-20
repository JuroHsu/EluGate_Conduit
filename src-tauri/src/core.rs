//! 應用核心：唯一的連線狀態來源。
//!
//! 每個通道對應一個受管理的 ssh 程序與一個監督任務；同一通道的操作以 op 鎖排序，
//! 連接／停止可重複呼叫而不會建立重複程序。狀態變更後統一發布 `state` 事件給所有視窗。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::askpass::{self, AskpassServer, Prompt, PromptSink};
use crate::logs::{Level, Logger};
use crate::model::{self, AppConfig, Store, Tunnel};
use crate::{proc, ssh_bin, tr};

const ESTABLISH_TIMEOUT: Duration = Duration::from_secs(45);
const BACKOFF: [u64; 6] = [2, 5, 10, 20, 30, 60];
/// 連線穩定超過此時間後，下次中斷重新從最短間隔開始重試
const STABLE_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[default]
    Disconnected,
    Connecting,
    WaitingAuth,
    Connected,
    Reconnecting,
    Stopping,
    Failed,
}

impl Status {
    pub fn active(self) -> bool {
        !matches!(self, Status::Disconnected | Status::Failed)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Auth,
    HostKey,
    Config,
    PortInUse,
    PortReserved,
    Network,
    Cancelled,
    SshMissing,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestCheck {
    pub ok: bool,
    pub message: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub status: Status,
    pub message: Option<String>,
    pub error_kind: Option<ErrorKind>,
    pub since: String,
    pub attempt: u32,
    pub next_retry_at: Option<String>,
    pub connected_at: Option<String>,
    pub dest_check: Option<DestCheck>,
    /// 目前執行中的設定；與儲存的設定不同時代表有尚未套用的變更
    pub applied: Option<Tunnel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelView {
    pub tunnel: Tunnel,
    pub runtime: Runtime,
    pub pending_changes: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub id: String,
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<Vec<ConflictInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, String>>,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into(), conflicts: None, fields: None }
    }
}

impl From<anyhow::Error> for CmdError {
    fn from(e: anyhow::Error) -> Self {
        Self::new("error", format!("{e:#}"))
    }
}

pub type CmdResult<T> = Result<T, CmdError>;

#[derive(Default)]
struct Slot {
    rt: Runtime,
    cancel: Option<CancellationToken>,
    task: Option<JoinHandle<()>>,
    op: Arc<tokio::sync::Mutex<()>>,
}

pub struct Core {
    pub app: AppHandle,
    pub logger: Logger,
    pub data_dir: PathBuf,
    store: Store,
    cfg: Mutex<AppConfig>,
    slots: Mutex<HashMap<String, Slot>>,
    pub askpass: OnceLock<Arc<AskpassServer>>,
    /// 測試連線使用的暫時名稱
    temp_names: Mutex<HashMap<String, String>>,
    auth_pending: Mutex<HashSet<String>>,
    auth_cancelled: Mutex<HashSet<String>>,
    open_failed: Mutex<HashMap<String, String>>,
    open_confirmed: Mutex<HashSet<String>>,
    pgids: Mutex<HashMap<String, i32>>,
    wake: Notify,
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

/// 單次 ssh 執行的結果
enum Outcome {
    Cancelled,
    /// 測試模式：已建立並完成目的檢查
    Tested(DestCheck),
    Exited { kind: ErrorKind, message: String, connected_for: Option<Duration> },
}

struct Attempt {
    outcome: Outcome,
    authenticated: bool,
    forwarded: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub status: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ssh: Stage,
    pub forward: Stage,
    pub destination: Stage,
}

impl Core {
    pub fn new(app: AppHandle, data_dir: PathBuf, log_dir: PathBuf) -> Arc<Self> {
        let store = Store::new(&data_dir);
        let (cfg, warning) = store.load();
        crate::i18n::set_language(cfg.prefs.language.as_deref());
        let core = Arc::new(Self {
            app,
            logger: Logger::new(log_dir),
            data_dir,
            store,
            cfg: Mutex::new(cfg),
            slots: Mutex::new(HashMap::new()),
            askpass: OnceLock::new(),
            temp_names: Mutex::new(HashMap::new()),
            auth_pending: Mutex::new(HashSet::new()),
            auth_cancelled: Mutex::new(HashSet::new()),
            open_failed: Mutex::new(HashMap::new()),
            open_confirmed: Mutex::new(HashSet::new()),
            pgids: Mutex::new(HashMap::new()),
            wake: Notify::new(),
        });
        core.log(Level::Info, None, tr!("log.started"));
        if let Some(w) = warning {
            core.log(Level::Warn, None, w);
        }
        for pgid in proc::cleanup_stale(&core.pid_file()) {
            core.log(Level::Warn, None, tr!("log.staleCleaned", pgid = pgid));
        }
        core
    }

    pub async fn init(self: &Arc<Self>) {
        match AskpassServer::start(self.clone()).await {
            Ok(s) => {
                let _ = self.askpass.set(s);
            }
            Err(e) => self.log(Level::Error, None, tr!("log.askpassFailed", error = e)),
        }
        self.spawn_sleep_watchdog();
        let ids: Vec<String> =
            self.cfg.lock().unwrap().tunnels.iter().filter(|t| t.connect_on_launch).map(|t| t.id.clone()).collect();
        for id in ids {
            if let Err(e) = self.connect(&id).await {
                self.log(Level::Warn, Some(&id), tr!("log.autoConnectFailed", error = e.message));
            }
        }
    }

    // ───────────── 日誌與事件 ─────────────

    pub fn log(&self, level: Level, tunnel_id: Option<&str>, msg: impl Into<String>) {
        let entry = self.logger.push(level, tunnel_id, msg);
        let _ = self.app.emit("log", &entry);
    }

    pub fn views(&self) -> Vec<TunnelView> {
        let tunnels = self.cfg.lock().unwrap().tunnels.clone();
        let slots = self.slots.lock().unwrap();
        tunnels
            .into_iter()
            .map(|t| {
                let rt = slots.get(&t.id).map(|s| s.rt.clone()).unwrap_or_default();
                let pending_changes = rt.status.active()
                    && rt.applied.as_ref().is_some_and(|a| a.connection_key() != t.connection_key());
                TunnelView { tunnel: t, runtime: rt, pending_changes }
            })
            .collect()
    }

    pub fn emit_state(&self) {
        let views = self.views();
        crate::tray::update(&self.app, &views);
        let _ = self.app.emit("state", &views);
    }

    fn set_rt(&self, id: &str, f: impl FnOnce(&mut Runtime)) {
        {
            let mut slots = self.slots.lock().unwrap();
            let Some(slot) = slots.get_mut(id) else { return };
            let before = slot.rt.status;
            f(&mut slot.rt);
            if slot.rt.status != before {
                slot.rt.since = now();
            }
        }
        self.emit_state();
    }

    fn status(&self, id: &str) -> Status {
        self.slots.lock().unwrap().get(id).map(|s| s.rt.status).unwrap_or_default()
    }

    fn op_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.slots.lock().unwrap().entry(id.to_string()).or_default().op.clone()
    }

    // ───────────── 設定 ─────────────

    pub fn config(&self) -> AppConfig {
        self.cfg.lock().unwrap().clone()
    }

    pub fn tunnel(&self, id: &str) -> CmdResult<Tunnel> {
        self.cfg
            .lock()
            .unwrap()
            .tunnels
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| CmdError::new("notFound", tr!("err.notFound")))
    }

    fn persist(&self, f: impl FnOnce(&mut AppConfig)) -> CmdResult<()> {
        let snapshot = {
            let mut cfg = self.cfg.lock().unwrap();
            f(&mut cfg);
            cfg.clone()
        };
        self.store.save(&snapshot)?;
        self.emit_state();
        Ok(())
    }

    pub fn validate(&self, t: &Tunnel) -> model::Validation {
        model::validate(t, &self.cfg.lock().unwrap().tunnels)
    }

    pub fn save_tunnel(&self, mut t: Tunnel) -> CmdResult<Tunnel> {
        t.name = t.name.trim().to_string();
        t.ssh_host = t.ssh_host.trim().to_string();
        t.dest_host = t.dest_host.trim().to_string();
        t.bind_address = t.bind_address.trim().to_string();
        t.ssh_config_file = t.ssh_config_file.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let v = self.validate(&t);
        if !v.errors.is_empty() {
            let mut e = CmdError::new("validation", tr!("err.validation"));
            e.fields = Some(v.errors);
            return Err(e);
        }
        let is_new = t.id.is_empty();
        if is_new {
            t.id = uuid::Uuid::new_v4().to_string();
        }
        let saved = t.clone();
        self.persist(|cfg| match cfg.tunnels.iter_mut().find(|x| x.id == t.id) {
            Some(x) => *x = t,
            None => cfg.tunnels.push(t),
        })?;
        self.log(
            Level::Info,
            Some(&saved.id),
            tr!(if is_new { "log.tunnelAdded" } else { "log.tunnelUpdated" }, name = saved.name),
        );
        Ok(saved)
    }

    pub fn duplicate_tunnel(&self, id: &str) -> CmdResult<Tunnel> {
        let mut t = self.tunnel(id)?;
        t.id.clear();
        t.name = tr!("tunnel.copyName", name = t.name);
        t.connect_on_launch = false;
        self.save_tunnel(t)
    }

    /// 刪除前先停止連線，避免設定消失但程序仍在執行
    pub async fn delete_tunnel(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let name = self.tunnel(id)?.name;
        self.disconnect(id).await?;
        self.slots.lock().unwrap().remove(id);
        self.persist(|cfg| cfg.tunnels.retain(|t| t.id != id))?;
        self.log(Level::Info, None, tr!("log.tunnelDeleted", name = name));
        Ok(())
    }

    pub fn set_prefs(&self, prefs: model::Prefs) -> CmdResult<()> {
        let old = crate::i18n::current();
        let lang = crate::i18n::set_language(prefs.language.as_deref());
        self.persist(|cfg| cfg.prefs = prefs)?;
        if lang != old {
            crate::tray::refresh_language(&self.app);
            self.emit_state();
            let _ = self.app.emit("language-changed", lang);
        }
        Ok(())
    }

    pub fn ssh_path(&self) -> CmdResult<PathBuf> {
        let custom = self.cfg.lock().unwrap().prefs.ssh_path.clone();
        ssh_bin::locate(custom.as_deref()).map_err(|m| CmdError::new("sshMissing", m))
    }

    // ───────────── 連線操作 ─────────────

    fn active_conflicts(&self, t: &Tunnel) -> Vec<ConflictInfo> {
        let names: HashMap<String, String> =
            self.cfg.lock().unwrap().tunnels.iter().map(|x| (x.id.clone(), x.name.clone())).collect();
        self.slots
            .lock()
            .unwrap()
            .iter()
            .filter(|(id, s)| *id != &t.id && s.rt.status.active())
            .filter_map(|(id, s)| {
                let a = s.rt.applied.as_ref()?;
                model::binds_overlap(a, t).then(|| ConflictInfo {
                    id: id.clone(),
                    name: names.get(id).cloned().unwrap_or_default(),
                    endpoint: a.local_endpoint(),
                })
            })
            .collect()
    }

    pub async fn connect(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let op = self.op_lock(id);
        let _guard = op.lock().await;
        self.connect_locked(id)
    }

    fn connect_locked(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let t = self.tunnel(id)?;
        if self.status(id).active() {
            return Ok(());
        }
        let conflicts = self.active_conflicts(&t);
        if !conflicts.is_empty() {
            let mut e = CmdError::new(
                "conflict",
                tr!("err.conflict", endpoint = t.local_endpoint(), name = conflicts[0].name),
            );
            e.conflicts = Some(conflicts);
            return Err(e);
        }
        if let Err(e) = self.ssh_path() {
            self.set_rt(id, |rt| {
                rt.status = Status::Failed;
                rt.error_kind = Some(ErrorKind::SshMissing);
                rt.message = Some(e.message.clone());
            });
            return Err(e);
        }
        let cancel = CancellationToken::new();
        {
            let mut slots = self.slots.lock().unwrap();
            let slot = slots.entry(id.to_string()).or_default();
            slot.rt = Runtime { status: Status::Connecting, since: now(), applied: Some(t.clone()), ..Default::default() };
            slot.cancel = Some(cancel.clone());
            slot.task = Some(tokio::spawn(self.clone().supervise(t.clone(), cancel)));
        }
        self.log(Level::Info, Some(id), tr!("log.connecting", name = t.name, endpoint = t.local_endpoint(), host = t.ssh_host));
        self.emit_state();
        Ok(())
    }

    pub async fn disconnect(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let op = self.op_lock(id);
        let _guard = op.lock().await;
        self.disconnect_locked(id).await;
        Ok(())
    }

    async fn disconnect_locked(self: &Arc<Self>, id: &str) {
        let (cancel, task, was_active) = {
            let mut slots = self.slots.lock().unwrap();
            let Some(slot) = slots.get_mut(id) else { return };
            (slot.cancel.take(), slot.task.take(), slot.rt.status.active())
        };
        if was_active {
            self.set_rt(id, |rt| rt.status = Status::Stopping);
        }
        if let Some(a) = self.askpass.get() {
            a.cancel_for(id);
        }
        if let Some(c) = cancel {
            c.cancel();
        }
        if let Some(t) = task {
            let _ = tokio::time::timeout(Duration::from_secs(10), t).await;
        }
        self.set_rt(id, |rt| {
            *rt = Runtime { status: Status::Disconnected, ..Default::default() };
        });
        if was_active {
            self.log(Level::Info, Some(id), tr!("log.stopped"));
        }
    }

    /// 套用新設定：停止後以目前儲存的設定重新連接
    pub async fn restart(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let op = self.op_lock(id);
        let _guard = op.lock().await;
        self.disconnect_locked(id).await;
        self.connect_locked(id)
    }

    /// 停止占用相同入口的通道並連接目標；失敗時恢復原連線
    pub async fn switch_to(self: &Arc<Self>, id: &str) -> CmdResult<()> {
        let t = self.tunnel(id)?;
        let previous: Vec<String> = self.active_conflicts(&t).into_iter().map(|c| c.id).collect();
        for p in &previous {
            self.disconnect(p).await?;
        }
        self.log(Level::Info, Some(id), tr!("log.switching", name = t.name));
        let result = match self.connect(id).await {
            Ok(()) => self.wait_connected(id).await,
            Err(e) => Err(e),
        };
        if let Err(e) = result {
            self.disconnect(id).await?;
            for p in &previous {
                let _ = self.connect(p).await;
            }
            let key = if previous.is_empty() { "err.switchFailed" } else { "err.switchFailedRestored" };
            let msg = tr!(key, error = e.message);
            self.log(Level::Warn, Some(id), msg.clone());
            return Err(CmdError::new("switchFailed", msg));
        }
        Ok(())
    }

    async fn wait_connected(&self, id: &str) -> CmdResult<()> {
        let mut deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let (status, msg) = {
                let slots = self.slots.lock().unwrap();
                let rt = slots.get(id).map(|s| s.rt.clone()).unwrap_or_default();
                (rt.status, rt.message)
            };
            match status {
                Status::Connected => return Ok(()),
                Status::Failed | Status::Disconnected | Status::Reconnecting => {
                    return Err(CmdError::new("failed", msg.unwrap_or_else(|| tr!("err.connectFailed"))))
                }
                Status::WaitingAuth => deadline = Instant::now() + Duration::from_secs(60),
                _ => {}
            }
            if Instant::now() > deadline {
                return Err(CmdError::new("timeout", tr!("msg.establishTimeout")));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn stop_all(self: &Arc<Self>) {
        let ids: Vec<String> =
            self.slots.lock().unwrap().iter().filter(|(_, s)| s.rt.status.active()).map(|(k, _)| k.clone()).collect();
        let tasks: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let core = self.clone();
                tokio::spawn(async move { core.disconnect(&id).await })
            })
            .collect();
        for t in tasks {
            let _ = t.await;
        }
    }

    /// 離開程式前呼叫：停止所有通道並確保程序已清理
    pub async fn shutdown(self: &Arc<Self>) {
        self.log(Level::Info, None, tr!("log.quitting"));
        self.stop_all().await;
        self.kill_all_sync();
    }

    pub fn kill_all_sync(&self) {
        for (_, pgid) in self.pgids.lock().unwrap().drain() {
            proc::kill_group(pgid);
        }
        let _ = std::fs::remove_file(self.pid_file());
    }

    fn pid_file(&self) -> PathBuf {
        self.data_dir.join("running.pids")
    }

    fn track_pgid(&self, id: &str, pgid: Option<i32>) {
        let mut map = self.pgids.lock().unwrap();
        match pgid {
            Some(p) if p > 0 => {
                map.insert(id.to_string(), p);
            }
            _ => {
                map.remove(id);
            }
        }
        if cfg!(unix) {
            let text: String = map.values().map(|p| format!("{p}\n")).collect();
            let _ = std::fs::write(self.pid_file(), text);
        }
    }

    // ───────────── 監督與重連 ─────────────

    async fn supervise(self: Arc<Self>, t: Tunnel, cancel: CancellationToken) {
        let id = t.id.clone();
        let mut attempt: u32 = 0;
        loop {
            let a = self.run_once(&t, &cancel, false).await;
            let (kind, message, connected_for) = match a.outcome {
                Outcome::Cancelled | Outcome::Tested(_) => return,
                Outcome::Exited { kind, message, connected_for } => (kind, message, connected_for),
            };
            if connected_for.is_some_and(|d| d > STABLE_AFTER) {
                attempt = 0;
            }
            let was_connected = connected_for.is_some();
            let retry = t.auto_reconnect && (kind == ErrorKind::Network || (kind == ErrorKind::Unknown && was_connected));
            if !retry {
                let msg = if was_connected && kind == ErrorKind::Network {
                    tr!("msg.noAutoReconnect", message = message)
                } else {
                    message
                };
                self.log(Level::Error, Some(&id), tr!("log.failed", error = msg));
                self.set_rt(&id, |rt| {
                    rt.status = Status::Failed;
                    rt.error_kind = Some(kind);
                    rt.message = Some(msg);
                    rt.next_retry_at = None;
                });
                return;
            }
            let delay = Duration::from_secs(BACKOFF[(attempt as usize).min(BACKOFF.len() - 1)]);
            attempt += 1;
            let retry_at = chrono::Local::now() + chrono::Duration::from_std(delay).unwrap_or_default();
            self.log(Level::Warn, Some(&id), tr!("log.retrying", message = message, seconds = delay.as_secs(), attempt = attempt));
            self.set_rt(&id, |rt| {
                rt.status = Status::Reconnecting;
                rt.error_kind = Some(kind);
                rt.message = Some(message);
                rt.attempt = attempt;
                rt.next_retry_at = Some(retry_at.to_rfc3339());
                rt.dest_check = None;
            });
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(delay) => {}
                // 系統睡眠喚醒時立即重試
                _ = self.wake.notified() => {}
            }
            self.set_rt(&id, |rt| {
                rt.status = Status::Connecting;
                rt.next_retry_at = None;
            });
        }
    }

    /// 執行一次 ssh；測試模式在建立並檢查目的服務後即結束
    async fn run_once(self: &Arc<Self>, t: &Tunnel, cancel: &CancellationToken, test: bool) -> Attempt {
        let id = t.id.as_str();
        let mut att = Attempt {
            outcome: Outcome::Exited { kind: ErrorKind::Unknown, message: String::new(), connected_for: None },
            authenticated: false,
            forwarded: false,
        };
        let fail = |mut att: Attempt, kind, message: String| {
            att.outcome = Outcome::Exited { kind, message, connected_for: None };
            att
        };

        if let Some((kind, msg)) = precheck_port(t) {
            return fail(att, kind, msg);
        }
        let ssh = match self.ssh_path() {
            Ok(p) => p,
            Err(e) => return fail(att, ErrorKind::SshMissing, e.message),
        };
        self.auth_cancelled.lock().unwrap().remove(id);

        let mut cmd = proc::command(&ssh);
        // -vv 才會輸出通道開啟確認（open confirm），用來判斷目的服務是否可連接
        cmd.args(["-N", "-vv"]);
        for opt in [
            "ExitOnForwardFailure=yes",
            "ServerAliveInterval=15",
            "ServerAliveCountMax=3",
            "ConnectTimeout=15",
            "BatchMode=no",
            // 不與外部 SSH 工作共用連線，也不自行背景化，以免停止時影響其他程序或失去追蹤
            "ControlMaster=no",
            "ControlPath=none",
            "ForkAfterAuthentication=no",
        ] {
            cmd.arg("-o").arg(opt);
        }
        cmd.arg("-L").arg(t.forward_spec());
        if let Some(f) = t.ssh_config_file.as_deref().filter(|f| !f.is_empty()) {
            cmd.arg("-F").arg(f);
        }
        cmd.arg("--").arg(&t.ssh_host);

        if let (Some(a), Ok(exe)) = (self.askpass.get(), std::env::current_exe()) {
            cmd.env("SSH_ASKPASS", exe)
                .env("SSH_ASKPASS_REQUIRE", "force")
                .env(askpass::ENV_FLAG, "1")
                .env(askpass::ENV_ADDR, &a.addr)
                .env(askpass::ENV_TOKEN, &a.token)
                .env(askpass::ENV_TUNNEL, id);
            if cfg!(unix) && std::env::var_os("DISPLAY").is_none() {
                cmd.env("DISPLAY", ":0");
            }
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return fail(att, ErrorKind::SshMissing, tr!("err.spawnFailed", error = e)),
        };
        let guard = match proc::ProcGuard::attach(&child) {
            Ok(g) => Some(g),
            Err(e) => {
                self.log(Level::Warn, Some(id), tr!("log.trackFailed", error = e));
                None
            }
        };
        self.track_pgid(id, guard.as_ref().map(|g| g.id()));

        let stderr = child.stderr.take().expect("stderr piped");
        // ssh 在 Windows 以系統字碼頁輸出錯誤訊息，因此逐行讀取位元組再解碼
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut raw = Vec::new();
        let mut recent: Vec<String> = vec![];
        let mut connected_at: Option<Instant> = None;
        let mut deadline = Instant::now() + ESTABLISH_TIMEOUT;
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        let mut timed_out = false;

        let kill = |guard: &Option<proc::ProcGuard>| {
            if let Some(g) = guard {
                g.kill();
            }
        };

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    kill(&guard);
                    let _ = child.kill().await;
                    self.track_pgid(id, None);
                    att.outcome = Outcome::Cancelled;
                    return att;
                }
                _ = tick.tick() => {
                    if connected_at.is_none() {
                        if self.auth_pending.lock().unwrap().contains(id) {
                            deadline = Instant::now() + ESTABLISH_TIMEOUT;
                        } else if Instant::now() > deadline {
                            timed_out = true;
                            kill(&guard);
                            let _ = child.start_kill();
                        }
                    }
                }
                n = reader.read_until(b'\n', &mut raw) => {
                    if !matches!(n, Ok(n) if n > 0) { break; }
                    let line = decode_line(&raw);
                    raw.clear();
                    if line.is_empty() { continue; }
                    let debug = line.starts_with("debug");
                    let l = line.to_ascii_lowercase();
                    if l.contains("open confirm") {
                        self.open_confirmed.lock().unwrap().insert(id.to_string());
                    }
                    if l.contains("authenticated to ") {
                        att.authenticated = true;
                    }
                    if l.contains("local connections to") || l.contains("local forwarding listening on") {
                        att.forwarded = true;
                    }
                    if let Some(pos) = l.find("open failed:") {
                        self.open_failed.lock().unwrap().insert(id.to_string(), line[pos + 12..].trim().to_string());
                        self.log(Level::Warn, Some(id), tr!("log.destOpenFailed", error = line[pos + 12..].trim()));
                    } else if line.starts_with("debug2") || line.starts_with("debug3") {
                        // 只用於判斷，不寫入日誌
                    } else if !test || !debug {
                        self.log(if debug { Level::Debug } else { Level::Info }, Some(id), line.clone());
                    }
                    if !line.starts_with("debug2") {
                        recent.push(line);
                    }
                    if recent.len() > 60 { recent.remove(0); }

                    if connected_at.is_none() && att.authenticated && att.forwarded && probe_listen(t).await {
                        connected_at = Some(Instant::now());
                        if test {
                            let dest = self.check_destination_raw(id, t).await;
                            kill(&guard);
                            let _ = child.kill().await;
                            self.track_pgid(id, None);
                            att.outcome = Outcome::Tested(dest);
                            return att;
                        }
                        self.log(Level::Info, Some(id), tr!("log.established", endpoint = t.local_endpoint(), dest = format!("{}:{}", t.dest_host, t.dest_port)));
                        self.set_rt(id, |rt| {
                            rt.status = Status::Connected;
                            rt.message = None;
                            rt.error_kind = None;
                            rt.attempt = 0;
                            rt.connected_at = Some(now());
                        });
                        let core = self.clone();
                        let tid = id.to_string();
                        tokio::spawn(async move { core.check_destination(&tid).await; });
                    }
                }
            }
        }

        // stderr 結束代表 ssh 已結束（或即將結束）
        let status = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        kill(&guard);
        self.track_pgid(id, None);
        if cancel.is_cancelled() {
            att.outcome = Outcome::Cancelled;
            return att;
        }
        let code = status.ok().and_then(|s| s.ok()).and_then(|s| s.code());
        let (kind, message) = if self.auth_cancelled.lock().unwrap().remove(id) {
            (ErrorKind::Cancelled, tr!("msg.authCancelled"))
        } else if timed_out {
            (ErrorKind::Network, tr!("msg.establishTimeout"))
        } else {
            classify(&recent, code, connected_at.is_some())
        };
        att.outcome = Outcome::Exited { kind, message, connected_for: connected_at.map(|c| c.elapsed()) };
        att
    }

    // ───────────── 目的服務檢查與測試 ─────────────

    pub async fn check_destination(self: &Arc<Self>, id: &str) -> Option<DestCheck> {
        let applied = {
            let slots = self.slots.lock().unwrap();
            let s = slots.get(id)?;
            if s.rt.status != Status::Connected {
                return None;
            }
            s.rt.applied.clone()?
        };
        let check = self.check_destination_raw(id, &applied).await;
        self.log(
            if check.ok { Level::Info } else { Level::Warn },
            Some(id),
            tr!("log.destCheck", message = check.message),
        );
        self.set_rt(id, |rt| rt.dest_check = Some(check.clone()));
        Some(check)
    }

    /// 透過本機入口開啟一條連線，等待 ssh 回報該通道的結果：
    /// 伺服器確認開啟（open confirm）代表目的服務已接受 TCP 連線；
    /// open failed 或本機連線被關閉代表目的服務無法連接。
    async fn check_destination_raw(&self, id: &str, t: &Tunnel) -> DestCheck {
        self.open_failed.lock().unwrap().remove(id);
        self.open_confirmed.lock().unwrap().remove(id);
        let result = |ok: bool, message: String| DestCheck { ok, message, at: now() };
        let target = format!("{}:{}", t.dest_host, t.dest_port);
        let addr = (t.probe_address(), t.local_port);
        let stream = tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(addr)).await;
        let Ok(Ok(mut stream)) = stream else {
            return result(false, tr!("dest.localUnreachable", endpoint = t.local_endpoint()));
        };
        // 部分伺服器（例如 Windows 版 sshd）會先確認通道、之後才回報連線失敗，
        // 因此確認後仍須觀察一段時間，期間連線被關閉即視為失敗
        const OBSERVE: Duration = Duration::from_secs(4);
        let started = Instant::now();
        let deadline = started + Duration::from_secs(12);
        let mut buf = [0u8; 64];
        while Instant::now() < deadline {
            if let Some(m) = self.open_failed.lock().unwrap().get(id).cloned() {
                return result(false, tr!("dest.failed", target = target, error = m));
            }
            match tokio::time::timeout(Duration::from_millis(150), stream.read(&mut buf)).await {
                Ok(Ok(0)) | Ok(Err(_)) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let m = self.open_failed.lock().unwrap().get(id).cloned();
                    return result(
                        false,
                        match m {
                            Some(m) => tr!("dest.failed", target = target, error = m),
                            None => tr!("dest.closed", target = target),
                        },
                    );
                }
                Ok(Ok(_)) => return result(true, tr!("dest.responded", target = target)),
                Err(_) => {}
            }
            if started.elapsed() >= OBSERVE && self.open_confirmed.lock().unwrap().contains(id) {
                return result(true, tr!("dest.accepted", target = target));
            }
        }
        result(false, tr!("dest.noResponse", target = target))
    }

    /// 以暫時的本機 port 測試設定，不影響正在執行的通道
    pub async fn test_tunnel(self: &Arc<Self>, mut t: Tunnel) -> CmdResult<TestResult> {
        let v = model::validate(&t, &[]);
        if !v.errors.is_empty() {
            let mut e = CmdError::new("validation", tr!("err.validation"));
            e.fields = Some(v.errors);
            return Err(e);
        }
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map_err(|e| CmdError::new("error", tr!("err.testPort", error = e)))?
            .port();
        let name = tr!("test.name", name = if t.name.trim().is_empty() { &t.ssh_host } else { t.name.trim() });
        t.id = format!("test-{}", uuid::Uuid::new_v4().simple());
        t.bind_address = "127.0.0.1".into();
        t.local_port = port;
        t.ssh_config_file = t.ssh_config_file.filter(|s| !s.trim().is_empty());
        self.temp_names.lock().unwrap().insert(t.id.clone(), name.clone());
        self.log(Level::Info, None, tr!("log.testing", name = name));

        let cancel = CancellationToken::new();
        let a = tokio::time::timeout(Duration::from_secs(600), self.run_once(&t, &cancel, true)).await;
        self.temp_names.lock().unwrap().remove(&t.id);
        let a = a.map_err(|_| CmdError::new("timeout", tr!("err.testTimeout")))?;

        let ok = |m: String| Stage { status: "ok", message: m };
        let failed = |m: String| Stage { status: "failed", message: m };
        let skipped = || Stage { status: "skipped", message: tr!("test.skipped") };
        let r = match a.outcome {
            Outcome::Tested(dest) => TestResult {
                ssh: ok(tr!("test.sshOk")),
                forward: ok(tr!("test.forwardOk")),
                destination: if dest.ok { ok(dest.message) } else { failed(dest.message) },
            },
            Outcome::Exited { message, .. } if !a.authenticated => {
                TestResult { ssh: failed(message), forward: skipped(), destination: skipped() }
            }
            Outcome::Exited { message, .. } => {
                TestResult { ssh: ok(tr!("test.sshOk")), forward: failed(message), destination: skipped() }
            }
            Outcome::Cancelled => {
                TestResult { ssh: failed(tr!("test.cancelled")), forward: skipped(), destination: skipped() }
            }
        };
        self.log(Level::Info, None, tr!("log.testResult", ssh = r.ssh.status, forward = r.forward.status, dest = r.destination.status));
        Ok(r)
    }

    // ───────────── 睡眠恢復 ─────────────

    fn spawn_sleep_watchdog(self: &Arc<Self>) {
        let core = Arc::downgrade(self);
        tokio::spawn(async move {
            let step = Duration::from_secs(5);
            loop {
                let before = std::time::SystemTime::now();
                tokio::time::sleep(step).await;
                let Some(core) = core.upgrade() else { return };
                let gap = std::time::SystemTime::now().duration_since(before).unwrap_or_default();
                if gap > step + Duration::from_secs(20) {
                    core.log(Level::Info, None, tr!("log.resumed", seconds = gap.as_secs()));
                    core.wake.notify_waiters();
                    let connected: Vec<String> = core
                        .slots
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|(_, s)| s.rt.status == Status::Connected)
                        .map(|(k, _)| k.clone())
                        .collect();
                    for id in connected {
                        let c = core.clone();
                        tokio::spawn(async move { c.check_destination(&id).await });
                    }
                }
            }
        });
    }
}

impl PromptSink for Core {
    fn tunnel_name(&self, tunnel_id: &str) -> Option<String> {
        if let Some(n) = self.temp_names.lock().unwrap().get(tunnel_id) {
            return Some(n.clone());
        }
        self.cfg.lock().unwrap().tunnels.iter().find(|t| t.id == tunnel_id).map(|t| t.name.clone())
    }

    fn prompt_opened(&self, p: &Prompt) {
        self.auth_pending.lock().unwrap().insert(p.tunnel_id.clone());
        self.logger.push(Level::Info, Some(&p.tunnel_id), tr!("log.waitingAuth", kind = format!("{:?}", p.kind)));
        self.set_rt(&p.tunnel_id, |rt| {
            if rt.status == Status::Connecting {
                rt.status = Status::WaitingAuth;
            }
        });
        crate::windows::open_auth(&self.app, p);
    }

    fn prompt_closed(&self, p: &Prompt, answered: bool) {
        self.auth_pending.lock().unwrap().remove(&p.tunnel_id);
        if !answered {
            self.auth_cancelled.lock().unwrap().insert(p.tunnel_id.clone());
        }
        self.set_rt(&p.tunnel_id, |rt| {
            if rt.status == Status::WaitingAuth {
                rt.status = Status::Connecting;
            }
        });
        crate::windows::close_auth(&self.app, p);
    }
}

/// 解碼一行 ssh 輸出；ssh 會把非 ASCII 位元組轉成 \ooo 八進位跳脫字元
fn decode_line(raw: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let oct = raw.get(i + 1..i + 4).filter(|d| raw[i] == b'\\' && d.iter().all(|c| (b'0'..=b'7').contains(c)));
        match oct {
            Some(d) => {
                bytes.push(((d[0] - b'0') << 6) | ((d[1] - b'0') << 3) | (d[2] - b'0'));
                i += 4;
            }
            None => {
                bytes.push(raw[i]);
                i += 1;
            }
        }
    }
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => proc::decode_system(e.as_bytes()),
    };
    text.trim_end().to_string()
}

/// 啟動前確認本機 port 可綁定；外部程序占用時不終止它
fn precheck_port(t: &Tunnel) -> Option<(ErrorKind, String)> {
    let bind = model::normalize_bind(&t.bind_address);
    match std::net::TcpListener::bind((bind.as_str(), t.local_port)) {
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Some((
            ErrorKind::PortInUse,
            tr!("err.portInUse", endpoint = t.local_endpoint()),
        )),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Some((
            ErrorKind::PortReserved,
            tr!("err.portReserved", endpoint = t.local_endpoint(), error = e),
        )),
        Err(_) => None,
    }
}

async fn probe_listen(t: &Tunnel) -> bool {
    let addr = (t.probe_address(), t.local_port);
    matches!(tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(addr)).await, Ok(Ok(_)))
}

/// 依 ssh 輸出分類錯誤；只有網路類錯誤會自動重試
fn classify(lines: &[String], code: Option<i32>, was_connected: bool) -> (ErrorKind, String) {
    let all = lines.join("\n").to_ascii_lowercase();
    let has = |s: &str| all.contains(s);
    // 只取像錯誤訊息的最後一行作為補充說明，略過驗證成功等一般訊息
    let last = lines
        .iter()
        .rev()
        .find(|l| {
            !l.starts_with("debug")
                && !l.starts_with("Authenticated to")
                && !l.starts_with("Warning: Permanently added")
                && !l.starts_with("Transferred:")
        })
        .cloned()
        .unwrap_or_default();
    use ErrorKind::*;
    let (kind, msg) = if has("remote host identification has changed") {
        (HostKey, "cls.hostKeyChanged")
    } else if has("host key verification failed") {
        (HostKey, "cls.hostKeyRejected")
    } else if has("permission denied") || has("too many authentication failures") {
        (Auth, "cls.auth")
    } else if has("bad configuration option")
        || has("no argument after keyword")
        || has("can't open user config file")
        || has("unsupported option")
        || has("terminating, ")
    {
        (Config, "cls.config")
    } else if has("address already in use") || has("cannot listen to port") || has("could not request local forwarding") {
        (PortInUse, "cls.portInUse")
    } else if has("could not resolve hostname") {
        (Network, "cls.resolve")
    } else if has("timeout, server") || has("not responding") {
        (Network, "cls.keepalive")
    } else if has("timed out") {
        (Network, "cls.timeout")
    } else if has("connection refused") && !was_connected {
        (Network, "cls.refused")
    } else if has("network is unreachable") || has("no route to host") {
        (Network, "cls.unreachable")
    } else if has("connection reset") || has("broken pipe") || has("connection closed") || has("client_loop") {
        (Network, "cls.disconnected")
    } else if was_connected {
        (Network, "cls.disconnected")
    } else {
        (Unknown, "cls.unexpected")
    };
    let msg = tr!(msg);
    let text = if !last.is_empty() {
        tr!("cls.withDetail", message = msg, detail = last)
    } else if let Some(c) = code {
        tr!("cls.withExitCode", message = msg, code = c)
    } else {
        msg
    };
    (kind, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_octal_escapes() {
        assert_eq!(decode_line(br"abc \101\102"), "abc AB");
        assert_eq!(decode_line("無法\n".as_bytes()), "無法");
    }

    #[test]
    fn classifies_errors() {
        let l = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(classify(&l(&["user@h: Permission denied (publickey)."]), Some(255), false).0, ErrorKind::Auth);
        assert_eq!(classify(&l(&["Host key verification failed."]), Some(255), false).0, ErrorKind::HostKey);
        assert_eq!(classify(&l(&["Bad configuration option: foo"]), Some(255), false).0, ErrorKind::Config);
        let (k, m) = classify(&l(&["Authenticated to h ([1.2.3.4]:22) using \"publickey\"."]), None, true);
        assert_eq!(k, ErrorKind::Network);
        assert_eq!(m, tr!("cls.disconnected"));
    }
}

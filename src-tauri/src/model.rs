//! 設定模型與 App 設定檔儲存。

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub ssh_host: String,
    /// 空值代表使用者預設的 `~/.ssh/config`
    #[serde(default)]
    pub ssh_config_file: Option<String>,
    pub bind_address: String,
    pub local_port: u16,
    pub dest_host: String,
    pub dest_port: u16,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub connect_on_launch: bool,
}

fn default_true() -> bool {
    true
}

impl Tunnel {
    /// 影響連線行為的欄位；名稱與啟動選項變更不需要重新連接
    pub fn connection_key(&self) -> (String, Option<String>, String, u16, String, u16) {
        (
            self.ssh_host.clone(),
            self.ssh_config_file.clone().filter(|s| !s.trim().is_empty()),
            normalize_bind(&self.bind_address),
            self.local_port,
            self.dest_host.clone(),
            self.dest_port,
        )
    }

    pub fn local_endpoint(&self) -> String {
        format!("{}:{}", bracket(&self.bind_address), self.local_port)
    }

    pub fn forward_spec(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            bracket(&self.bind_address),
            self.local_port,
            bracket(&self.dest_host),
            self.dest_port
        )
    }

    /// 用於檢查本機入口的位址：萬用位址改連 loopback
    pub fn probe_address(&self) -> String {
        match normalize_bind(&self.bind_address).as_str() {
            "0.0.0.0" => "127.0.0.1".into(),
            "::" => "::1".into(),
            other => other.to_string(),
        }
    }
}

fn bracket(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub fn normalize_bind(addr: &str) -> String {
    let a = addr.trim().trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
    match a.as_str() {
        "" | "*" => "0.0.0.0".into(),
        "localhost" => "127.0.0.1".into(),
        _ => a,
    }
}

/// 兩個監聽位址是否可能互相衝突（萬用位址與任何同族位址重疊）
pub fn binds_overlap(a: &Tunnel, b: &Tunnel) -> bool {
    if a.local_port != b.local_port {
        return false;
    }
    let (x, y) = (normalize_bind(&a.bind_address), normalize_bind(&b.bind_address));
    if x == y {
        return true;
    }
    let wild = |s: &str| s == "0.0.0.0" || s == "::";
    let v6 = |s: &str| s.contains(':');
    // "::" 在多數系統為雙堆疊，也會占用 IPv4
    (x == "::" || y == "::") || (wild(&x) && !v6(&y)) || (wild(&y) && !v6(&x))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    /// 指定 ssh 執行檔；空值時自動尋找
    #[serde(default)]
    pub ssh_path: Option<String>,
    /// SSH 主機頁預設開啟的設定檔
    #[serde(default)]
    pub ssh_config_path: Option<String>,
    /// 介面語系；空值或 "auto" 時跟隨系統
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub tunnels: Vec<Tunnel>,
    #[serde(default)]
    pub prefs: Prefs,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { version: CONFIG_VERSION, tunnels: vec![], prefs: Prefs::default() }
    }
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(data_dir: &Path) -> Self {
        Self { path: data_dir.join("config.json") }
    }

    /// 讀取設定；檔案損毀時另存備份並回傳預設值與警告
    pub fn load(&self) -> (AppConfig, Option<String>) {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(_) => return (AppConfig::default(), None),
        };
        match serde_json::from_str::<serde_json::Value>(&text)
            .map_err(anyhow::Error::from)
            .and_then(migrate)
        {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                let backup = self.path.with_extension(format!(
                    "broken-{}.json",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                ));
                let _ = std::fs::copy(&self.path, &backup);
                (AppConfig::default(), Some(crate::tr!("config.loadFailed", error = e, path = backup.display())))
            }
        }
    }

    pub fn save(&self, cfg: &AppConfig) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(cfg)?)?;
        std::fs::rename(&tmp, &self.path).with_context(|| crate::tr!("config.saveFailed"))?;
        Ok(())
    }
}

/// 設定版本遷移；目前僅有第 1 版
fn migrate(mut v: serde_json::Value) -> Result<AppConfig> {
    let version = v.get("version").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
    if version > CONFIG_VERSION {
        anyhow::bail!(crate::tr!("config.versionTooNew", version = version));
    }
    v["version"] = CONFIG_VERSION.into();
    Ok(serde_json::from_value(v)?)
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Validation {
    pub errors: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

fn safe_token(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-:[]%@".contains(c))
}

pub fn validate(t: &Tunnel, others: &[Tunnel]) -> Validation {
    let mut v = Validation::default();
    let mut err = |k: &str, key: &str| {
        v.errors.insert(k.into(), crate::tr!(key));
    };
    if t.name.trim().is_empty() {
        err("name", "valid.nameRequired");
    }
    if t.ssh_host.trim().is_empty() {
        err("sshHost", "valid.hostRequired");
    } else if !safe_token(t.ssh_host.trim()) {
        err("sshHost", "valid.hostInvalid");
    }
    let bind = t.bind_address.trim();
    if !(bind == "localhost" || bind == "*" || bind.trim_matches(['[', ']']).parse::<IpAddr>().is_ok()) {
        err("bindAddress", "valid.bindInvalid");
    }
    if t.local_port == 0 {
        err("localPort", "valid.portRange");
    }
    if t.dest_port == 0 {
        err("destPort", "valid.portRange");
    }
    if t.dest_host.trim().is_empty() {
        err("destHost", "valid.destRequired");
    } else if !safe_token(t.dest_host.trim()) {
        err("destHost", "valid.destInvalid");
    }
    if let Some(f) = &t.ssh_config_file {
        if !f.trim().is_empty() && !Path::new(f.trim()).exists() {
            err("sshConfigFile", "valid.configMissing");
        }
    }

    let n = normalize_bind(bind);
    if n != "127.0.0.1" && n != "::1" {
        v.warnings.push(crate::tr!("warn.notLoopback"));
    }
    if t.local_port < 1024 && !cfg!(windows) {
        v.warnings.push(crate::tr!("warn.privilegedPort"));
    }
    for o in others.iter().filter(|o| o.id != t.id) {
        if binds_overlap(t, o) {
            v.warnings.push(crate::tr!("warn.sameEndpoint", name = o.name, endpoint = o.local_endpoint()));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(bind: &str, port: u16) -> Tunnel {
        Tunnel {
            id: bind.into(),
            name: bind.into(),
            ssh_host: "h".into(),
            ssh_config_file: None,
            bind_address: bind.into(),
            local_port: port,
            dest_host: "127.0.0.1".into(),
            dest_port: 80,
            auto_reconnect: true,
            connect_on_launch: false,
        }
    }

    #[test]
    fn bind_overlap_considers_address_scope() {
        assert!(binds_overlap(&t("127.0.0.1", 16000), &t("localhost", 16000)));
        assert!(binds_overlap(&t("0.0.0.0", 16000), &t("127.0.0.1", 16000)));
        assert!(binds_overlap(&t("::", 16000), &t("127.0.0.1", 16000)));
        assert!(!binds_overlap(&t("127.0.0.1", 16000), &t("127.0.0.2", 16000)));
        assert!(!binds_overlap(&t("127.0.0.1", 16000), &t("127.0.0.1", 16001)));
        assert!(!binds_overlap(&t("0.0.0.0", 16000), &t("::1", 16000)));
    }

    #[test]
    fn rejects_option_injection() {
        let mut x = t("127.0.0.1", 16000);
        x.ssh_host = "-oProxyCommand=calc".into();
        assert!(validate(&x, &[]).errors.contains_key("sshHost"));
    }
}

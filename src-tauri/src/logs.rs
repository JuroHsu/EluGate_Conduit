//! 日誌：記憶體環狀緩衝提供 UI 查詢，同時寫入可輪替的檔案。

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;

const MAX_ENTRIES: usize = 3000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const KEEP_FILES: usize = 3;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: u64,
    pub ts: String,
    pub level: Level,
    pub tunnel_id: Option<String>,
    pub message: String,
}

pub struct Logger {
    entries: Mutex<VecDeque<LogEntry>>,
    file: Mutex<Option<File>>,
    pub dir: PathBuf,
    next: AtomicU64,
}

impl Logger {
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let file = open_log(&dir);
        Self { entries: Mutex::new(VecDeque::new()), file: Mutex::new(file), dir, next: AtomicU64::new(1) }
    }

    pub fn push(&self, level: Level, tunnel_id: Option<&str>, message: impl Into<String>) -> LogEntry {
        let entry = LogEntry {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            ts: chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            level,
            tunnel_id: tunnel_id.map(str::to_string),
            message: redact(&message.into()),
        };
        {
            let mut q = self.entries.lock().unwrap();
            q.push_back(entry.clone());
            while q.len() > MAX_ENTRIES {
                q.pop_front();
            }
        }
        self.write_file(&entry);
        entry
    }

    fn write_file(&self, e: &LogEntry) {
        let mut f = self.file.lock().unwrap();
        let too_big = f.as_ref().and_then(|f| f.metadata().ok()).is_some_and(|m| m.len() > MAX_FILE_BYTES);
        if too_big {
            *f = None;
            rotate(&self.dir);
            *f = open_log(&self.dir);
        }
        if let Some(file) = f.as_mut() {
            let _ = writeln!(
                file,
                "{} [{:?}] {}{}",
                e.ts,
                e.level,
                e.tunnel_id.as_deref().map(|t| format!("<{t}> ")).unwrap_or_default(),
                e.message
            );
        }
    }

    pub fn list(&self, tunnel_id: Option<&str>) -> Vec<LogEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| tunnel_id.is_none() || e.tunnel_id.as_deref() == tunnel_id)
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

fn open_log(dir: &std::path::Path) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(dir.join("conduit.log")).ok()
}

fn rotate(dir: &std::path::Path) {
    for i in (1..KEEP_FILES).rev() {
        let _ = std::fs::rename(dir.join(format!("conduit.{i}.log")), dir.join(format!("conduit.{}.log", i + 1)));
    }
    let _ = std::fs::rename(dir.join("conduit.log"), dir.join("conduit.1.log"));
}

/// 過濾可能的驗證秘密；askpass 的回答本來就不會寫入日誌，這裡是額外防護
pub fn redact(msg: &str) -> String {
    let lower = msg.to_ascii_lowercase();
    for key in ["password=", "password:", "passphrase=", "passwd="] {
        if let Some(pos) = lower.find(key) {
            let end = pos + key.len();
            let rest = &msg[end..];
            let value_len = rest.trim_start().split_whitespace().next().map(str::len).unwrap_or(0);
            if value_len > 0 {
                let ws = rest.len() - rest.trim_start().len();
                return format!("{}{}***{}", &msg[..end], &rest[..ws], &rest[ws + value_len..]);
            }
        }
    }
    if msg.contains("-----BEGIN") && msg.contains("PRIVATE KEY") {
        return crate::tr!("log.privateKeyHidden");
    }
    msg.to_string()
}

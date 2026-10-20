//! OpenSSH 與 GUI 的驗證互動橋接。
//!
//! 啟動 ssh 時設定 `SSH_ASKPASS` 指向本程式並強制使用（`SSH_ASKPASS_REQUIRE=force`）。
//! ssh 需要密碼、私鑰密碼或主機指紋確認時會執行本程式；本程式以 client 模式透過
//! 127.0.0.1 上帶 token 的連線把提示交給主程式，等待使用者在對話窗回答。
//! 回答只經由記憶體與 ssh 的 stdout 管道傳遞，不寫入日誌或設定檔。

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

pub const ENV_FLAG: &str = "CONDUIT_ASKPASS";
pub const ENV_ADDR: &str = "CONDUIT_ASKPASS_ADDR";
pub const ENV_TOKEN: &str = "CONDUIT_ASKPASS_TOKEN";
pub const ENV_TUNNEL: &str = "CONDUIT_TUNNEL_ID";

#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    tunnel_id: String,
    prompt: String,
    kind: String,
}

#[derive(Serialize, Deserialize)]
struct Response {
    ok: bool,
    value: String,
}

pub fn is_client_mode() -> bool {
    std::env::var_os(ENV_FLAG).is_some() && std::env::var_os(ENV_ADDR).is_some()
}

/// askpass client：回傳程序結束碼（0 = 已回答）
pub fn run_client() -> i32 {
    let prompt = std::env::args().nth(1).unwrap_or_default();
    let req = Request {
        token: std::env::var(ENV_TOKEN).unwrap_or_default(),
        tunnel_id: std::env::var(ENV_TUNNEL).unwrap_or_default(),
        prompt,
        kind: std::env::var("SSH_ASKPASS_PROMPT").unwrap_or_default(),
    };
    let Ok(addr) = std::env::var(ENV_ADDR) else { return 1 };
    let Ok(mut stream) = std::net::TcpStream::connect(addr) else { return 1 };
    let Ok(line) = serde_json::to_string(&req) else { return 1 };
    if writeln!(stream, "{line}").is_err() {
        return 1;
    }
    let mut resp = String::new();
    if BufReader::new(&stream).read_line(&mut resp).is_err() {
        return 1;
    }
    match serde_json::from_str::<Response>(&resp) {
        Ok(r) if r.ok => {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{}", r.value);
            let _ = out.flush();
            0
        }
        _ => 1,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptKind {
    /// 主機指紋確認（yes/no）
    HostKey,
    Password,
    Passphrase,
    /// 例如 agent 金鑰使用確認
    Confirm,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub tunnel_id: String,
    pub tunnel_name: String,
    pub kind: PromptKind,
    pub message: String,
}

fn classify(prompt: &str, kind: &str) -> PromptKind {
    let p = prompt.to_ascii_lowercase();
    if p.contains("(yes/no") || p.contains("continue connecting") {
        PromptKind::HostKey
    } else if kind == "confirm" || kind == "none" {
        PromptKind::Confirm
    } else if p.contains("passphrase") {
        PromptKind::Passphrase
    } else if p.contains("password") {
        PromptKind::Password
    } else {
        PromptKind::Other
    }
}

pub struct AskpassServer {
    pub addr: String,
    pub token: String,
    pending: Mutex<HashMap<String, (Prompt, oneshot::Sender<Option<String>>)>>,
}

/// 由核心實作：顯示提示、更新通道狀態
pub trait PromptSink: Send + Sync + 'static {
    fn tunnel_name(&self, tunnel_id: &str) -> Option<String>;
    fn prompt_opened(&self, prompt: &Prompt);
    fn prompt_closed(&self, prompt: &Prompt, answered: bool);
}

impl AskpassServer {
    pub async fn start<S: PromptSink>(sink: std::sync::Arc<S>) -> std::io::Result<std::sync::Arc<Self>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let server = std::sync::Arc::new(Self {
            addr: listener.local_addr()?.to_string(),
            token: uuid::Uuid::new_v4().simple().to_string(),
            pending: Mutex::new(HashMap::new()),
        });
        let s = server.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let s = s.clone();
                let sink = sink.clone();
                tokio::spawn(async move {
                    let _ = s.handle(stream, sink).await;
                });
            }
        });
        Ok(server)
    }

    async fn handle<S: PromptSink>(&self, stream: tokio::net::TcpStream, sink: std::sync::Arc<S>) -> std::io::Result<()> {
        let (r, mut w) = stream.into_split();
        let mut line = String::new();
        tokio::io::BufReader::new(r).read_line(&mut line).await?;
        let Ok(req) = serde_json::from_str::<Request>(&line) else { return Ok(()) };
        if req.token != self.token {
            return Ok(());
        }
        let Some(tunnel_name) = sink.tunnel_name(&req.tunnel_id) else { return Ok(()) };
        let prompt = Prompt {
            id: uuid::Uuid::new_v4().simple().to_string(),
            tunnel_id: req.tunnel_id,
            tunnel_name,
            kind: classify(&req.prompt, &req.kind),
            message: req.prompt.trim().to_string(),
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(prompt.id.clone(), (prompt.clone(), tx));
        sink.prompt_opened(&prompt);

        // 使用者五分鐘內未回應視為取消，避免 ssh 無限等待
        let answer = tokio::time::timeout(std::time::Duration::from_secs(300), rx).await.ok().and_then(|r| r.ok()).flatten();
        self.pending.lock().unwrap().remove(&prompt.id);
        sink.prompt_closed(&prompt, answer.is_some());

        let resp = Response { ok: answer.is_some(), value: answer.unwrap_or_default() };
        let mut out = serde_json::to_string(&resp).unwrap_or_default();
        out.push('\n');
        w.write_all(out.as_bytes()).await?;
        w.shutdown().await
    }

    pub fn get(&self, id: &str) -> Option<Prompt> {
        self.pending.lock().unwrap().get(id).map(|(p, _)| p.clone())
    }

    pub fn list(&self) -> Vec<Prompt> {
        self.pending.lock().unwrap().values().map(|(p, _)| p.clone()).collect()
    }

    pub fn answer(&self, id: &str, value: Option<String>) -> bool {
        match self.pending.lock().unwrap().remove(id) {
            Some((_, tx)) => tx.send(value).is_ok(),
            None => false,
        }
    }

    /// 通道停止時取消其所有等待中的提示
    pub fn cancel_for(&self, tunnel_id: &str) {
        let ids: Vec<String> = self
            .pending
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, (p, _))| p.tunnel_id == tunnel_id)
            .map(|(k, _)| k.clone())
            .collect();
        for id in ids {
            self.answer(&id, None);
        }
    }
}

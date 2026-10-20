//! 尋找系統 OpenSSH Client。

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshInfo {
    pub path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}

pub fn missing_message() -> String {
    if cfg!(windows) {
        crate::tr!("ssh.missingWindows")
    } else {
        crate::tr!("ssh.missingUnix")
    }
}

pub fn locate(custom: Option<&str>) -> Result<PathBuf, String> {
    if let Some(c) = custom.map(str::trim).filter(|c| !c.is_empty()) {
        let p = PathBuf::from(c);
        return if p.is_file() { Ok(p) } else { Err(crate::tr!("ssh.customMissing", path = c)) };
    }
    #[cfg(windows)]
    {
        // 優先使用 Windows 內建 OpenSSH，避免誤用 Git/MSYS 版本（其 askpass 與路徑行為不同）
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let p = PathBuf::from(root).join(r"System32\OpenSSH\ssh.exe");
        if p.is_file() {
            return Ok(p);
        }
    }
    #[cfg(unix)]
    {
        for p in ["/usr/bin/ssh", "/usr/local/bin/ssh", "/opt/homebrew/bin/ssh"] {
            if std::path::Path::new(p).is_file() {
                return Ok(p.into());
            }
        }
    }
    let exe = if cfg!(windows) { "ssh.exe" } else { "ssh" };
    std::env::var_os("PATH")
        .and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(exe)).find(|p| p.is_file()))
        .ok_or_else(missing_message)
}

pub async fn info(custom: Option<&str>) -> SshInfo {
    match locate(custom) {
        Err(e) => SshInfo { path: None, version: None, error: Some(e) },
        Ok(p) => {
            let out = crate::proc::command(&p).arg("-V").output().await;
            let version = out.ok().map(|o| {
                let s = if o.stderr.is_empty() { o.stdout } else { o.stderr };
                String::from_utf8_lossy(&s).trim().to_string()
            });
            SshInfo { path: Some(p.display().to_string()), version, error: None }
        }
    }
}

//! 讀取與最小幅度編輯 OpenSSH 設定檔。
//!
//! 只解析列出主機與表單所需的部分；有效設定一律交由 `ssh -G` 計算。
//! 表單編輯只改動目標 Host 區塊中的特定行，其餘內容（註解、未知選項、順序）保持原樣。

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn default_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".ssh").join("config")
}

pub fn resolve(path: Option<&str>) -> PathBuf {
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => expand_home(p),
        None => default_path(),
    }
}

fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else {
        PathBuf::from(p)
    }
}

pub fn hash(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub path: String,
    pub exists: bool,
    pub content: String,
    pub hash: String,
}

pub fn read(path: &Path) -> Result<FileSnapshot> {
    let (exists, content) = match std::fs::read(path) {
        Ok(b) => (true, String::from_utf8(b).with_context(|| crate::tr!("sshcfg.notUtf8"))?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (false, String::new()),
        Err(e) => return Err(e).with_context(|| crate::tr!("sshcfg.readFailed")),
    };
    Ok(FileSnapshot { path: path.display().to_string(), exists, hash: hash(&content), content })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub alias: String,
    pub source: String,
    pub line: usize,
    /// 是否可用表單編輯（定義於目前檔案、且 Host 行只有此單一別名）
    pub editable: bool,
}

/// 將設定行拆成 (關鍵字, 參數)；支援 `Key value` 與 `Key=value`
fn split_kv(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    let idx = t.find(|c: char| c.is_whitespace() || c == '=')?;
    let key = t[..idx].to_string();
    let rest = t[idx..].trim_start_matches(|c: char| c.is_whitespace()).trim_start_matches('=').trim();
    Some((key, rest.to_string()))
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn list_hosts(path: &Path) -> Result<Vec<HostEntry>> {
    let mut out = vec![];
    collect_hosts(path, path, 0, &mut out)?;
    let mut seen = std::collections::HashSet::new();
    out.retain(|h| seen.insert(h.alias.clone()));
    Ok(out)
}

fn collect_hosts(root: &Path, path: &Path, depth: usize, out: &mut Vec<HostEntry>) -> Result<()> {
    if depth > 8 {
        return Ok(());
    }
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        // 找不到的檔案與無法讀取的 Include 不阻止列出其他主機
        Err(e) if depth > 0 || e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| crate::tr!("sshcfg.readFile", path = path.display())),
    };
    let base = dirs::home_dir().unwrap_or_default().join(".ssh");
    for (i, line) in text.lines().enumerate() {
        let Some((key, val)) = split_kv(line) else { continue };
        match key.to_ascii_lowercase().as_str() {
            "host" => {
                let pats = split_args(&val);
                let single = pats.len() == 1;
                for p in pats {
                    if p.contains(['*', '?', '!']) {
                        continue;
                    }
                    out.push(HostEntry {
                        alias: p,
                        source: path.display().to_string(),
                        line: i + 1,
                        editable: single && path == root,
                    });
                }
            }
            "include" => {
                for inc in split_args(&val) {
                    let mut p = expand_home(&inc);
                    if p.is_relative() {
                        p = base.join(p);
                    }
                    let pattern = p.to_string_lossy().replace('\\', "/");
                    if let Ok(paths) = glob::glob(&pattern) {
                        for f in paths.flatten() {
                            collect_hosts(root, &f, depth + 1, out)?;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// 呼叫 `ssh -G` 取得 OpenSSH 計算後的有效設定
pub async fn effective(ssh: &Path, alias: &str, config: Option<&Path>) -> Result<Vec<(String, String)>> {
    if alias.starts_with('-') || alias.trim().is_empty() {
        bail!(crate::tr!("sshcfg.invalidAlias"));
    }
    let mut cmd = crate::proc::command(ssh);
    cmd.arg("-G");
    if let Some(c) = config {
        cmd.arg("-F").arg(c);
    }
    cmd.arg("--").arg(alias);
    cmd.stdin(std::process::Stdio::null());
    let out = cmd.output().await.with_context(|| crate::tr!("sshcfg.runFailed"))?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once(' ').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect())
}

/// 以 `ssh -G` 驗證候選設定內容是否可被 OpenSSH 解析
pub async fn validate_content(ssh: &Path, content: &str, tmp_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(tmp_dir)?;
    let tmp = tmp_dir.join(format!("validate-{}.conf", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, content)?;
    let r = effective(ssh, "conduit-validate", Some(&tmp)).await;
    let _ = std::fs::remove_file(&tmp);
    r.map(|_| ()).with_context(|| crate::tr!("sshcfg.parseFailed"))
}

pub const FORM_KEYS: [&str; 5] = ["HostName", "User", "Port", "IdentityFile", "ProxyJump"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostForm {
    pub alias: String,
    pub host_name: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub proxy_jump: String,
}

impl HostForm {
    fn values(&self) -> [&str; 5] {
        [&self.host_name, &self.user, &self.port, &self.identity_file, &self.proxy_jump]
    }
}

fn newline_of(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn is_block_start(line: &str) -> bool {
    matches!(split_kv(line), Some((k, _)) if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("match"))
}

/// 回傳 Host 區塊範圍 [start, end)，start 為 Host 行
fn find_block(lines: &[&str], alias: &str) -> Option<(usize, usize)> {
    let start = lines.iter().position(|l| {
        matches!(split_kv(l), Some((k, v)) if k.eq_ignore_ascii_case("host") && split_args(&v) == [alias.to_string()])
    })?;
    let end = lines[start + 1..].iter().position(|l| is_block_start(l)).map(|p| start + 1 + p).unwrap_or(lines.len());
    Some((start, end))
}

pub fn read_form(content: &str, alias: &str) -> Option<HostForm> {
    let lines: Vec<&str> = content.lines().collect();
    let (s, e) = find_block(&lines, alias)?;
    let mut f = HostForm { alias: alias.into(), ..Default::default() };
    for l in &lines[s + 1..e] {
        if let Some((k, v)) = split_kv(l) {
            let v = v.trim_matches('"').to_string();
            let slot = match k.to_ascii_lowercase().as_str() {
                "hostname" => &mut f.host_name,
                "user" => &mut f.user,
                "port" => &mut f.port,
                "identityfile" => &mut f.identity_file,
                "proxyjump" => &mut f.proxy_jump,
                _ => continue,
            };
            if slot.is_empty() {
                *slot = v;
            }
        }
    }
    Some(f)
}

fn quote(v: &str) -> String {
    if v.contains(char::is_whitespace) {
        format!("\"{v}\"")
    } else {
        v.to_string()
    }
}

fn validate_form(f: &HostForm) -> Result<()> {
    let alias = f.alias.trim();
    if alias.is_empty() || alias.starts_with('-') || alias.contains(char::is_whitespace) || alias.contains(['*', '?', '!']) {
        bail!(crate::tr!("sshcfg.aliasInvalid"));
    }
    if !f.port.trim().is_empty() && f.port.trim().parse::<u16>().map(|p| p == 0).unwrap_or(true) {
        bail!(crate::tr!("valid.portRange"));
    }
    for v in f.values() {
        if v.contains(['\r', '\n']) {
            bail!(crate::tr!("sshcfg.noNewline"));
        }
    }
    Ok(())
}

/// 套用表單到設定內容：只修改目標區塊中的對應行；主機不存在時於檔尾新增區塊
pub fn apply_form(content: &str, f: &HostForm) -> Result<String> {
    validate_form(f)?;
    let nl = newline_of(content);
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let Some((start, end)) = find_block(&refs, f.alias.trim()) else {
        let mut out = content.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(nl);
        }
        if !out.trim().is_empty() {
            out.push_str(nl);
        }
        out.push_str(&format!("Host {}{nl}", f.alias.trim()));
        for (k, v) in FORM_KEYS.iter().zip(f.values()) {
            if !v.trim().is_empty() {
                out.push_str(&format!("    {k} {}{nl}", quote(v.trim())));
            }
        }
        return Ok(out);
    };

    let indent = lines[start + 1..end]
        .iter()
        .find(|l| split_kv(l).is_some())
        .map(|l| l[..l.len() - l.trim_start().len()].to_string())
        .unwrap_or_else(|| "    ".into());

    let mut end = end;
    for (key, value) in FORM_KEYS.iter().zip(f.values()) {
        let value = value.trim();
        let found = (start + 1..end)
            .find(|&i| matches!(split_kv(&lines[i]), Some((k, _)) if k.eq_ignore_ascii_case(key)));
        match (found, value.is_empty()) {
            (Some(i), false) => {
                let l = &lines[i];
                let ind = &l[..l.len() - l.trim_start().len()];
                let existing = split_kv(l).map(|(_, v)| v.trim_matches('"').to_string()).unwrap_or_default();
                if existing != value {
                    lines[i] = format!("{ind}{key} {}", quote(value));
                }
            }
            (Some(i), true) => {
                lines.remove(i);
                end -= 1;
            }
            (None, false) => {
                // 插在區塊最後一個非空白行之後，保留區塊間的空行
                let mut at = end;
                while at > start + 1 && lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                lines.insert(at, format!("{indent}{key} {}", quote(value)));
                end += 1;
            }
            (None, true) => {}
        }
    }
    let mut out = lines.join(nl);
    if content.ends_with('\n') {
        out.push_str(nl);
    }
    Ok(out)
}

pub fn diff(old: &str, new: &str) -> String {
    similar::TextDiff::from_lines(old, new).unified_diff().context_radius(3).header(&crate::tr!("sshcfg.diffOld"), &crate::tr!("sshcfg.diffNew")).to_string()
}

/// 寫入前備份；Windows 以原地覆寫保留 ACL，Unix 以暫存檔替換並沿用原權限
pub fn write(path: &Path, content: &str, expected_hash: &str, backup_dir: &Path) -> Result<Option<PathBuf>> {
    let current = read(path)?;
    if current.hash != expected_hash {
        bail!(crate::tr!("sshcfg.externalChange"));
    }
    let mut backup = None;
    if current.exists {
        std::fs::create_dir_all(backup_dir)?;
        let name = format!(
            "{}.{}.bak",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("config"),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let b = backup_dir.join(name);
        std::fs::copy(path, &b).with_context(|| crate::tr!("sshcfg.backupFailed"))?;
        prune_backups(backup_dir, 20);
        backup = Some(b);
    } else if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    #[cfg(unix)]
    {
        use std::io::Write;
        let tmp = path.with_extension(format!("conduit-{}.tmp", std::process::id()));
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(content.as_bytes())?;
            f.sync_all()?;
        }
        let perms = std::fs::metadata(path)
            .map(|m| m.permissions())
            .unwrap_or_else(|_| std::os::unix::fs::PermissionsExt::from_mode(0o600));
        std::fs::set_permissions(&tmp, perms)?;
        std::fs::rename(&tmp, path).with_context(|| crate::tr!("sshcfg.writeFailed"))?;
    }
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    Ok(backup)
}

fn prune_backups(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<_> = rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "bak")).collect();
    files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    let excess = files.len().saturating_sub(keep);
    for e in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(e.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# global\nHost *\n    ServerAliveInterval 30\n\nHost my_server\n    HostName 192.0.2.10\n    # keep comment\n    User user\n    Compression yes\n\nHost a b\n    User x\n";

    #[test]
    fn lists_concrete_hosts() {
        let dir = std::env::temp_dir().join(format!("conduit-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config");
        std::fs::write(&p, SAMPLE).unwrap();
        let hosts = list_hosts(&p).unwrap();
        let names: Vec<_> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(names, ["my_server", "a", "b"]);
        assert!(hosts[0].editable && !hosts[1].editable);
    }

    #[test]
    fn edits_only_target_lines() {
        let mut f = read_form(SAMPLE, "my_server").unwrap();
        assert_eq!(f.host_name, "192.0.2.10");
        f.user = "admin".into();
        f.port = "2222".into();
        f.host_name.clear();
        let out = apply_form(SAMPLE, &f).unwrap();
        assert!(out.contains("    # keep comment\n    User admin\n    Compression yes\n    Port 2222\n\nHost a b"));
        assert!(!out.contains("HostName 192.0.2.10"));
        assert!(out.starts_with("# global\nHost *\n    ServerAliveInterval 30\n"));
    }

    #[test]
    fn appends_new_host_preserving_crlf() {
        let src = "Host x\r\n  User y\r\n";
        let f = HostForm { alias: "new".into(), host_name: "h".into(), ..Default::default() };
        let out = apply_form(src, &f).unwrap();
        assert_eq!(out, "Host x\r\n  User y\r\n\r\nHost new\r\n    HostName h\r\n");
    }
}

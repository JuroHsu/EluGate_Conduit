//! Generate a key pair with OpenSSH, then publish it without replacing existing files.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub private_path: String,
    pub public_path: String,
    pub public_key: String,
}

pub fn ssh_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir().context(crate::tr!("identity.noHome"))?.join(".ssh"))
}

pub fn config_path(path: &Path) -> String {
    let value = path.to_string_lossy().into_owned();
    #[cfg(windows)]
    let value = value.replace('\\', "/");
    value
}

fn keygen(custom_ssh: Option<&str>) -> Result<PathBuf> {
    let ssh = crate::ssh_bin::locate(custom_ssh).map_err(anyhow::Error::msg)?;
    let name = if cfg!(windows) { "ssh-keygen.exe" } else { "ssh-keygen" };
    if let Some(parent) = ssh.parent() {
        let sibling = parent.join(name);
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(name))
                .find(|p| p.is_file())
        })
        .context(crate::tr!("identity.keygenMissing"))
}

fn ensure_absent(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => bail!(crate::tr!("identity.exists", path = path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub async fn generate(custom_ssh: Option<&str>) -> Result<Identity> {
    generate_in(&keygen(custom_ssh)?, &ssh_dir()?).await
}

async fn generate_in(program: &Path, dir: &Path) -> Result<Identity> {
    let private = dir.join("id_ed25519");
    let public = dir.join("id_ed25519.pub");
    ensure_absent(&private)?;
    ensure_absent(&public)?;

    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)?;
    // Keep staging private even when an existing .ssh directory is world-readable.
    // ssh-keygen sets the key's platform permissions; hard links preserve them
    // and fail if a target exists.
    let mut staging = tempfile::Builder::new();
    staging.prefix(".conduit-key-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        staging.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let stage = staging.tempdir_in(dir)?;
    let staged_private = stage.path().join("id_ed25519");
    let staged_public = stage.path().join("id_ed25519.pub");
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        crate::proc::command(program)
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "EluGate Conduit", "-f"])
            .arg(&staged_private)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context(crate::tr!("identity.timeout"))??;
    if !output.status.success() {
        bail!(crate::tr!(
            "identity.failed",
            error = crate::proc::decode_system(&output.stderr).trim()
        ));
    }
    let public_key = std::fs::read_to_string(&staged_public)?;
    std::fs::hard_link(&staged_private, &private)
        .with_context(|| crate::tr!("identity.publishFailed", path = private.display()))?;
    if let Err(e) = std::fs::hard_link(&staged_public, &public) {
        let _ = std::fs::remove_file(&private);
        return Err(e).with_context(|| crate::tr!("identity.publishFailed", path = public.display()));
    }
    Ok(Identity {
        private_path: config_path(&private),
        public_path: config_path(&public),
        public_key: public_key.trim().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_usable_key_pair_and_never_overwrites_it() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("space and 中文").join(".ssh");
        let program = keygen(None).expect("OpenSSH is required for key generation tests");
        let result = generate_in(&program, &dir).await.unwrap();
        assert!(result.public_key.starts_with("ssh-ed25519 "));
        let private = dir.join("id_ed25519");
        let original = std::fs::read(&private).unwrap();
        let derived = crate::proc::command(&program)
            .args(["-y", "-f"])
            .arg(&private)
            .stdin(Stdio::null())
            .output()
            .await
            .unwrap();
        assert!(derived.status.success(), "{}", String::from_utf8_lossy(&derived.stderr));
        assert_eq!(
            String::from_utf8_lossy(&derived.stdout)
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>(),
            result.public_key.split_whitespace().take(2).collect::<Vec<_>>()
        );
        assert!(generate_in(&program, &dir).await.is_err());
        assert_eq!(std::fs::read(&private).unwrap(), original);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(std::fs::metadata(&private).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[tokio::test]
    async fn preserves_an_existing_public_key_without_creating_a_private_key() {
        let dir = tempfile::tempdir().unwrap();
        let public = dir.path().join("id_ed25519.pub");
        std::fs::write(&public, "existing public key").unwrap();
        assert!(generate_in(Path::new("unused"), dir.path()).await.is_err());
        assert_eq!(std::fs::read_to_string(public).unwrap(), "existing public key");
        assert!(!dir.path().join("id_ed25519").exists());
    }

    #[tokio::test]
    async fn failed_keygen_leaves_no_partial_pair_or_staging_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(generate_in(&dir.path().join("missing-keygen"), dir.path())
            .await
            .is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

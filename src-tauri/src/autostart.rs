//! Windows Run entries need a quoted executable path. Keep Unix integration in
//! the Tauri plugin (LaunchAgent / XDG Autostart, including AppImage paths).

use tauri::AppHandle;
#[cfg(not(windows))]
use tauri_plugin_autostart::ManagerExt;

#[cfg(windows)]
mod windows {
    use std::path::Path;
    use windows_registry::{Key, CURRENT_USER};

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const NAME: &str = "EluGate Conduit";

    fn missing(code: i32) -> bool {
        code as u32 == 0x80070002
    }

    fn registration() -> anyhow::Result<Option<String>> {
        match CURRENT_USER.open(RUN).and_then(|k| k.get_string(NAME)) {
            Ok(value) => Ok(Some(value)),
            Err(e) if missing(e.code().0) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn command(path: &Path) -> String {
        format!("\"{}\" {}", path.display(), crate::AUTOSTART_ARG)
    }

    fn registered_path(value: &str) -> Option<&Path> {
        let path = value.trim().strip_suffix(crate::AUTOSTART_ARG)?.trim_end();
        Some(Path::new(path.strip_prefix('"')?.strip_suffix('"')?))
    }

    fn approved(value: &[u8]) -> bool {
        // Windows uses 2 and 6 for enabled entries. Missing entries are handled separately.
        value.len() >= 4 && matches!(u32::from_le_bytes(value[..4].try_into().unwrap()), 2 | 6)
    }

    pub fn is_enabled() -> anyhow::Result<bool> {
        let Some(value) = registration()? else { return Ok(false) };
        if !registered_path(&value).is_some_and(Path::is_file) {
            return Ok(false);
        }
        match CURRENT_USER.open(APPROVED).and_then(|k| k.get_value(NAME)) {
            Ok(value) => Ok(approved(&value)),
            Err(e) if missing(e.code().0) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(enabled: bool) -> anyhow::Result<()> {
        if enabled {
            let path = std::env::current_exe()?;
            CURRENT_USER.create(RUN)?.set_string(NAME, command(&path))?;
            CURRENT_USER.create(APPROVED)?.set_bytes(
                NAME,
                windows_registry::Type::Bytes,
                &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )?;
        } else {
            match CURRENT_USER
                .options()
                .write()
                .open(RUN)
                .and_then(|k: Key| k.remove_value(NAME))
            {
                Ok(()) => (),
                Err(e) if missing(e.code().0) => (),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn repair_existing() -> anyhow::Result<()> {
        // Development builds must never replace the installed application's entry.
        if cfg!(debug_assertions) {
            return Ok(());
        }
        let path = std::env::current_exe()?;
        let legacy = format!("{} {}", path.display(), crate::AUTOSTART_ARG);
        if registration()?.as_deref() == Some(&legacy) {
            CURRENT_USER.create(RUN)?.set_string(NAME, command(&path))?;
        }
        // Preserve Windows' StartupApproved state, including explicit user disables.
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn quotes_executable_without_quoting_arguments() {
            let path = Path::new(r"C:\Program Files\EluGate Conduit\elugate-conduit.exe");
            assert_eq!(
                command(path),
                r#""C:\Program Files\EluGate Conduit\elugate-conduit.exe" --autostart"#
            );
            assert_eq!(registered_path(&command(path)), Some(path));
            assert!(registered_path(r"C:\Program Files\EluGate Conduit\elugate-conduit.exe --autostart").is_none());
        }

        #[test]
        fn honors_windows_disabled_state_even_without_timestamp() {
            assert!(approved(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
            assert!(approved(&[6, 0, 0, 0]));
            assert!(!approved(&[3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
            assert!(!approved(&[]));
        }
    }
}

pub fn is_enabled(app: &AppHandle) -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        let _ = app;
        windows::is_enabled()
    }
    #[cfg(not(windows))]
    {
        Ok(app.autolaunch().is_enabled()?)
    }
}

pub fn set(app: &AppHandle, enabled: bool) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        let _ = app;
        windows::set(enabled)
    }
    #[cfg(not(windows))]
    {
        let manager = app.autolaunch();
        if enabled {
            manager.enable()?;
        } else {
            manager.disable()?;
        }
        Ok(())
    }
}

pub fn repair_existing(app: &AppHandle) -> anyhow::Result<()> {
    let _ = app;
    #[cfg(windows)]
    windows::repair_existing()?;
    Ok(())
}

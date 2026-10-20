//! SSH 程序所有權：Windows 以 Job Object 管理整棵程序樹，Unix 以程序群組管理。

use std::path::Path;

/// 建立不顯示主控台視窗的指令
pub fn command(program: &Path) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    #[cfg(unix)]
    cmd.process_group(0);
    cmd
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::*;

    /// 每個通道一個 Job；關閉 handle（包含 App 異常結束）時系統會終止其中所有程序
    pub struct ProcGuard {
        job: HANDLE,
    }

    unsafe impl Send for ProcGuard {}
    unsafe impl Sync for ProcGuard {}

    impl ProcGuard {
        pub fn attach(child: &tokio::process::Child) -> std::io::Result<Self> {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(std::io::Error::last_os_error());
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                let handle = child.raw_handle().unwrap_or(std::ptr::null_mut());
                if ok == 0 || handle.is_null() || AssignProcessToJobObject(job, handle as HANDLE) == 0 {
                    let e = std::io::Error::last_os_error();
                    CloseHandle(job);
                    return Err(e);
                }
                Ok(Self { job })
            }
        }

        pub fn id(&self) -> i32 {
            0
        }

        pub fn kill(&self) {
            unsafe {
                TerminateJobObject(self.job, 1);
            }
        }
    }

    impl Drop for ProcGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.job);
            }
        }
    }

    pub fn kill_group(_pgid: i32) {}
}

#[cfg(unix)]
mod imp {
    /// 以程序群組追蹤 ssh 及其 ProxyJump 子程序
    pub struct ProcGuard {
        pgid: i32,
    }

    impl ProcGuard {
        pub fn attach(child: &tokio::process::Child) -> std::io::Result<Self> {
            let pid = child.id().ok_or_else(|| std::io::Error::other(crate::tr!("proc.exited")))?;
            Ok(Self { pgid: pid as i32 })
        }

        pub fn id(&self) -> i32 {
            self.pgid
        }

        pub fn kill(&self) {
            kill_group(self.pgid);
        }
    }

    impl Drop for ProcGuard {
        fn drop(&mut self) {
            kill_group(self.pgid);
        }
    }

    pub fn kill_group(pgid: i32) {
        if pgid > 1 {
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
        }
    }
}

pub use imp::*;

/// macOS：清理上次異常結束後殘留、且確認仍為 ssh 的程序群組
#[cfg(unix)]
pub fn cleanup_stale(pid_file: &Path) -> Vec<i32> {
    let mut killed = vec![];
    let Ok(text) = std::fs::read_to_string(pid_file) else { return killed };
    for pgid in text.lines().filter_map(|l| l.trim().parse::<i32>().ok()) {
        let out = std::process::Command::new("ps").args(["-o", "comm=", "-p", &pgid.to_string()]).output();
        if let Ok(out) = out {
            let comm = String::from_utf8_lossy(&out.stdout);
            if comm.trim().ends_with("ssh") {
                kill_group(pgid);
                killed.push(pgid);
            }
        }
    }
    let _ = std::fs::remove_file(pid_file);
    killed
}

#[cfg(windows)]
pub fn cleanup_stale(_pid_file: &Path) -> Vec<i32> {
    // Job Object 已保證 App 結束時終止所有受管理程序
    vec![]
}

/// 以系統 ANSI 字碼頁解碼（例如繁體中文 Windows 的 CP950）
#[cfg(windows)]
pub fn decode_system(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
    if bytes.is_empty() {
        return String::new();
    }
    unsafe {
        let len = MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        if len <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut wide = vec![0u16; len as usize];
        MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), bytes.len() as i32, wide.as_mut_ptr(), len);
        String::from_utf16_lossy(&wide)
    }
}

#[cfg(unix)]
pub fn decode_system(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

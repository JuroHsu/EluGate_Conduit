pub mod askpass;
mod commands;
pub mod i18n;
pub mod core;
mod logs;
mod model;
mod proc;
mod ssh_bin;
mod ssh_config;
mod tray;
mod windows;

use std::sync::Arc;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::core::Core;

const AUTOSTART_ARG: &str = "--autostart";

/// 「離開程式」：停止所有由 App 建立的通道後再結束
pub fn quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(core) = app.try_state::<Arc<Core>>() {
            core.inner().clone().shutdown().await;
        }
        app.exit(0);
    });
}

pub fn run() {
    tauri::Builder::default()
        // 第二次啟動時開啟既有實例，避免重複建立通道
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| windows::show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![AUTOSTART_ARG])))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = app.path().app_data_dir()?;
            let log_dir = app.path().app_log_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let core = Core::new(handle.clone(), data_dir, log_dir);
            app.manage(core.clone());
            tray::build(&handle)?;
            tauri::async_runtime::spawn(async move {
                core.init().await;
                core.emit_state();
            });
            // 登入時自動啟動只常駐系統匣，不打開主視窗
            if !std::env::args().any(|a| a == AUTOSTART_ARG) {
                windows::show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // 主視窗關閉時隱藏，連線持續運作
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false) if window.label() == "panel" => windows::panel_blur(window),
            // 使用者直接關閉驗證視窗視為取消
            WindowEvent::Destroyed => {
                if let Some(id) = window.label().strip_prefix("auth-") {
                    if let Some(core) = window.app_handle().try_state::<Arc<Core>>() {
                        if let Some(a) = core.askpass.get() {
                            a.answer(id, None);
                        }
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_prefs,
            commands::get_language,
            commands::validate_tunnel,
            commands::save_tunnel,
            commands::duplicate_tunnel,
            commands::delete_tunnel,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::switch_tunnel,
            commands::restart_tunnel,
            commands::stop_all,
            commands::check_destination,
            commands::test_tunnel,
            commands::get_logs,
            commands::clear_logs,
            commands::open_log_dir,
            commands::list_hosts,
            commands::effective_host,
            commands::read_ssh_config,
            commands::host_form,
            commands::preview_host_form,
            commands::preview_ssh_config,
            commands::save_ssh_config,
            commands::get_env,
            commands::set_prefs,
            commands::set_autostart,
            commands::get_prompt,
            commands::answer_prompt,
            commands::show_main,
            commands::hide_panel,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start EluGate Conduit")
        .run(|app, event| match event {
            // 視窗全部隱藏時繼續常駐；只有 app.exit() 才真正結束
            RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            RunEvent::Exit => {
                if let Some(core) = app.try_state::<Arc<Core>>() {
                    core.kill_all_sync();
                }
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => windows::show_main(app),
            _ => {}
        });
}

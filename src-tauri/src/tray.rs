//! 系統匣（macOS 選單列）圖示：反映整體狀態並搭配文字提示。

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::core::{Status, TunnelView};
use crate::tr;

const TRAY_ID: &str = "conduit";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overall {
    Idle,
    Connected,
    Busy,
    Error,
}

fn icon(state: Overall) -> Image<'static> {
    let bytes: &[u8] = match state {
        Overall::Idle => include_bytes!("../icons/tray/idle.png"),
        Overall::Connected => include_bytes!("../icons/tray/connected.png"),
        Overall::Busy => include_bytes!("../icons/tray/reconnecting.png"),
        Overall::Error => include_bytes!("../icons/tray/error.png"),
    };
    Image::from_bytes(bytes).expect("bundled tray icon")
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", tr!("tray.open"), true, None::<&str>)?,
            &MenuItem::with_id(app, "stop_all", tr!("tray.stopAll"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", tr!("tray.quit"), true, None::<&str>)?,
        ],
    )
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(Overall::Idle))
        .tooltip(format!("EluGate Conduit — {}", tr!("tray.idle")))
        .menu(&menu(app)?)
        // Linux indicators expose menus, but do not deliver tray click events.
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => crate::windows::show_main(app),
            "stop_all" => {
                let core = app.state::<std::sync::Arc<crate::core::Core>>().inner().clone();
                tauri::async_runtime::spawn(async move { core.stop_all().await });
            }
            "quit" => crate::quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } = event {
                crate::windows::toggle_panel(tray.app_handle(), rect);
            }
        })
        .build(app)?;
    Ok(())
}

/// 切換語系後重建選單，並讓下一次狀態更新重新設定提示文字
pub fn refresh_language(app: &AppHandle) {
    if let (Some(tray), Ok(m)) = (app.tray_by_id(TRAY_ID), menu(app)) {
        let _ = tray.set_menu(Some(m));
    }
    *LAST.lock().unwrap() = None;
}

static LAST: Mutex<Option<(u8, String)>> = Mutex::new(None);

pub fn update(app: &AppHandle, views: &[TunnelView]) {
    let count = |f: &dyn Fn(Status) -> bool| views.iter().filter(|v| f(v.runtime.status)).count();
    let connected = count(&|s| s == Status::Connected);
    let busy = count(&|s| matches!(s, Status::Connecting | Status::WaitingAuth | Status::Reconnecting | Status::Stopping));
    let failed = count(&|s| s == Status::Failed);

    let state = if failed > 0 {
        Overall::Error
    } else if busy > 0 {
        Overall::Busy
    } else if connected > 0 {
        Overall::Connected
    } else {
        Overall::Idle
    };
    let mut parts = vec![];
    if connected > 0 {
        parts.push(tr!("tray.connected", count = connected));
    }
    if busy > 0 {
        parts.push(tr!("tray.busy", count = busy));
    }
    if failed > 0 {
        parts.push(tr!("tray.failed", count = failed));
    }
    let summary = if parts.is_empty() { tr!("tray.idle") } else { parts.join(&tr!("tray.separator")) };
    let text = format!("EluGate Conduit — {summary}");

    let key = (state as u8, text.clone());
    let mut last = LAST.lock().unwrap();
    if last.as_ref() == Some(&key) {
        return;
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon(state)));
        let _ = tray.set_tooltip(Some(&text));
        *last = Some(key);
    }
}

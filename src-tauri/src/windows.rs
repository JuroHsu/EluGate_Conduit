//! 視窗管理：主視窗、系統匣快速面板與驗證對話窗。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindowBuilder, Window};

use crate::askpass::Prompt;

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.hide();
    }
}

static LAST_BLUR_HIDE: Mutex<Option<Instant>> = Mutex::new(None);

/// 面板失去焦點時收起；記錄時間，避免點擊系統匣圖示時「先收起又立刻打開」
pub fn panel_blur(window: &Window) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        *LAST_BLUR_HIDE.lock().unwrap() = Some(Instant::now());
    }
}

pub fn toggle_panel(app: &AppHandle, tray: Rect) {
    let Some(w) = app.get_webview_window("panel") else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    if LAST_BLUR_HIDE.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_millis(300)) {
        return;
    }

    // 依圖示位置與所在螢幕的工作區定位面板，避免超出畫面
    let scale = w.scale_factor().unwrap_or(1.0);
    let pos = tray.position.to_physical::<f64>(scale);
    let size = tray.size.to_physical::<f64>(scale);
    let (cx, top, bottom) = (pos.x + size.width / 2.0, pos.y, pos.y + size.height);
    let monitor = w
        .available_monitors()
        .ok()
        .and_then(|ms| {
            ms.into_iter().find(|m| {
                let (p, s) = (m.position(), m.size());
                cx >= p.x as f64 && cx < p.x as f64 + s.width as f64 && top >= p.y as f64 && top < p.y as f64 + s.height as f64
            })
        })
        .or_else(|| w.primary_monitor().ok().flatten());

    if let Some(m) = monitor {
        // 跨不同 DPI 螢幕時，以目標螢幕的縮放比例計算面板實際大小
        let logical = w.inner_size().map(|s| s.to_logical::<f64>(scale)).unwrap_or(tauri::LogicalSize::new(340.0, 460.0));
        let ms = m.scale_factor();
        let (pw, ph) = (logical.width * ms, logical.height * ms);
        let wa = m.work_area();
        let (wx, wy, ww, wh) = (wa.position.x as f64, wa.position.y as f64, wa.size.width as f64, wa.size.height as f64);
        let margin = 8.0 * ms;
        let x = (cx - pw / 2.0).clamp(wx + margin, (wx + ww - pw - margin).max(wx));
        let y = if top > wy + wh / 2.0 { top - ph - margin } else { bottom + margin };
        let y = y.clamp(wy + margin, (wy + wh - ph - margin).max(wy));
        let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to("panel", "panel-shown", ());
}

fn auth_label(p: &Prompt) -> String {
    format!("auth-{}", p.id)
}

pub fn open_auth(app: &AppHandle, p: &Prompt) {
    let url = WebviewUrl::App(format!("index.html?view=auth&id={}", p.id).into());
    let height = if p.kind == crate::askpass::PromptKind::HostKey { 400.0 } else { 300.0 };
    let r = WebviewWindowBuilder::new(app, auth_label(p), url)
        .title(crate::tr!("auth.windowTitle", name = p.tunnel_name))
        .inner_size(480.0, height)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .build();
    if let Err(e) = r {
        eprintln!("failed to open auth window: {e}");
    }
}

pub fn close_auth(app: &AppHandle, p: &Prompt) {
    if let Some(w) = app.get_webview_window(&auth_label(p)) {
        let _ = w.destroy();
    }
}

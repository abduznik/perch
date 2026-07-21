mod config;
mod ingest;
mod notifications;
mod plugin_installer;
mod sse_client;
mod state_machine;
mod tray;

pub mod settings_store {
    pub fn init() {
        log::info!("Settings store stub initialized");
    }
}

use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Persisted window position
struct WindowPos {
    x: f64,
    y: f64,
}
static SAVED_POS: Mutex<Option<WindowPos>> = Mutex::new(None);

#[tauri::command]
fn start_window_drag(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.start_dragging().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn move_window(app: tauri::AppHandle, x: f64, y: f64) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            x as i32, y as i32,
        )))
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_window_position(app: tauri::AppHandle) -> Result<Option<(f64, f64)>, String> {
    if let Some(w) = app.get_webview_window("main") {
        let pos = w.outer_position().map_err(|e| e.to_string())?;
        Ok(Some((pos.x as f64, pos.y as f64)))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn save_position(x: f64, y: f64) -> Result<(), String> {
    *SAVED_POS.lock().unwrap() = Some(WindowPos { x, y });
    // TODO: persist to disk via settings store
    log::info!("Position saved: ({}, {})", x, y);
    Ok(())
}

#[tauri::command]
fn get_saved_position() -> Result<Option<(f64, f64)>, String> {
    let pos = SAVED_POS.lock().unwrap();
    Ok(pos.as_ref().map(|p| (p.x, p.y)))
}

#[tauri::command]
fn set_click_through(window: tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    log::info!("Starting Perch - opencode companion");
    settings_store::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("Perch")
                    .inner_size(320.0, 280.0)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .visible(false) // hidden by default — show via tray
                    .accept_first_mouse(true)
                    .build()?;

            log::info!("Window created: label={}", window.label());
            position_overlay(&window)?;
            tray::setup_tray(app)?;
            tray::hide_dock_icon();

            let conf = config::AppConfig::load();
            if conf.source() == config::EventSource::Plugin {
                plugin_installer::install_plugin();
            }

            let app_handle = app.handle().clone();
            match conf.source() {
                config::EventSource::Plugin => {
                    let port = conf.ingest_port();
                    log::info!(
                        "Starting ingest server on port {} (plugin bridge mode)",
                        port
                    );
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = ingest::run_ingest_server(app_handle, port).await {
                            log::error!("Ingest server error: {}", e);
                        }
                    });
                }
                config::EventSource::Sse => {
                    let event_url = conf.sse_event_url();
                    log::info!("Starting SSE client for {} (fallback mode)", event_url);
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = sse_client::run_event_loop(app_handle, &event_url).await {
                            log::error!("SSE client error: {}", e);
                        }
                    });
                }
            }
            log::info!("Setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_window_drag,
            move_window,
            get_window_position,
            save_position,
            get_saved_position,
            set_click_through
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn position_overlay(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(monitor) = window.primary_monitor()? {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let ws = window.outer_size()?;
        let x = (size.width as f64 / scale) - (ws.width as f64 / scale) - 20.0;
        let y = (size.height as f64 / scale) - (ws.height as f64 / scale) - 20.0;
        window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))?;
        log::info!("Overlay positioned at ({}, {})", x, y);
    }
    Ok(())
}

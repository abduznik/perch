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

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    log::info!("Starting Perch - opencode companion");

    settings_store::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Perch")
            .inner_size(320.0, 280.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(true)
            .accept_first_mouse(true)
            .build()?;

            log::info!("Window created: label={}, visible={}", window.label(), window.is_visible().unwrap_or(false));

            position_overlay(&window)?;

            // Window is interactive — clicks land on mascot, transparent area passes through visually
            // (small 320x280 window in corner blocks a tiny area, acceptable tradeoff)

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
                    log::info!("Starting ingest server on port {} (plugin bridge mode)", port);
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

            log::info!("Setup complete — overlay window should be visible");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn position_overlay(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(monitor) = window.primary_monitor()? {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let window_size = window.outer_size()?;
        let x = (size.width as f64 / scale) - (window_size.width as f64 / scale) - 20.0;
        let y = (size.height as f64 / scale) - (window_size.height as f64 / scale) - 20.0;
        window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)))?;
        log::info!("Overlay positioned at ({}, {})", x, y);
    }
    Ok(())
}

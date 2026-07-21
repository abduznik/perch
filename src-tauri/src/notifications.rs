use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Send a native OS notification
pub async fn send_notification(app: &AppHandle, title: &str, body: &str) {
    log::info!("Sending notification: {} - {}", title, body);
    
    if let Err(e) = app.notification().builder()
        .title(title)
        .body(body)
        .show() {
        log::error!("Failed to send notification: {}", e);
    }
}

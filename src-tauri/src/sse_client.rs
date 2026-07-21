use futures::StreamExt;
use reqwest;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::ingest::StateChangeEvent;
use crate::notifications;
use crate::state_machine::{OpenCodeEvent, StateMachine};

/// Parse an SSE event line into an OpenCodeEvent
fn parse_sse_event(event_type: &str, data: &str) -> OpenCodeEvent {
    match event_type {
        "session.busy" | "session.working" => OpenCodeEvent::SessionBusy,
        "session.streaming" => OpenCodeEvent::SessionStreaming,
        "session.idle" => OpenCodeEvent::SessionIdle,
        "session.error" => {
            // Try to parse error message from data
            let message = if data.is_empty() {
                None
            } else {
                Some(data.to_string())
            };
            OpenCodeEvent::SessionError { message }
        }
        _ => OpenCodeEvent::Unknown {
            event_type: event_type.to_string(),
        },
    }
}

/// Run the SSE event loop, connecting to opencode's event stream
pub async fn run_event_loop(app: AppHandle, event_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut state_machine = StateMachine::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    log::info!("Starting SSE event loop, connecting to: {}", event_url);

    loop {
        match connect_and_listen(&client, event_url, &mut state_machine, &app).await {
            Ok(()) => {
                log::info!("SSE connection closed gracefully, reconnecting...");
            }
            Err(e) => {
                log::error!("SSE connection error: {}, reconnecting in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Connect to SSE stream and process events
async fn connect_and_listen(
    client: &reqwest::Client,
    url: &str,
    state_machine: &mut StateMachine,
    app: &AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = client
        .get(url)
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut current_event_type = String::new();
    let mut current_data = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() {
                // Empty line = end of event, dispatch it
                if !current_event_type.is_empty() {
                    let event = parse_sse_event(&current_event_type, &current_data);
                    log::debug!("Received event: {:?}", event);

                    if let Some(new_state) = state_machine.process_event(&event) {
                        log::info!("State changed to: {}", new_state);

                        // Emit state change event to frontend
                        let state_event = StateChangeEvent {
                            state: new_state.to_string(),
                        };
                        if let Err(e) = app.emit("mascot-state-change", &state_event) {
                            log::error!("Failed to emit state change: {}", e);
                        }

                        // Fire notification on Done state
                        if state_machine.should_notify(new_state) {
                            let app_handle = app.clone();
                            tokio::spawn(async move {
                                notifications::send_notification(
                                    &app_handle,
                                    "Perch",
                                    "opencode finished",
                                )
                                .await;
                            });
                        }
                    }
                }

                // Reset for next event
                current_event_type.clear();
                current_data.clear();
            } else if line.starts_with("event:") {
                current_event_type = line[6..].trim().to_string();
            } else if line.starts_with("data:") {
                let data = line[5..].trim().to_string();
                if !current_data.is_empty() {
                    current_data.push('\n');
                }
                current_data.push_str(&data);
            }
            // Ignore other SSE fields like id:, retry:, etc.
        }
    }

    Ok(())
}

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::post,
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use crate::notifications;
use crate::state_machine::{OpenCodeEvent, StateMachine};

/// Payload sent by the perch-bridge opencode plugin
#[derive(Debug, Deserialize)]
pub struct IngestPayload {
    /// Event type from opencode (e.g. "session.idle", "session.busy")
    pub event: String,
    /// Optional data/message associated with the event
    #[serde(default)]
    pub data: Option<String>,
}

/// Event payload emitted to the frontend via Tauri events
#[derive(Debug, Clone, Serialize)]
pub struct StateChangeEvent {
    /// The new mascot state (e.g. "Idle", "Working", "Done", "Error")
    pub state: String,
}

/// Shared state for the ingest server
struct IngestState {
    state_machine: tokio::sync::Mutex<StateMachine>,
    app_handle: AppHandle,
}

/// Default port for the local ingest server
pub const DEFAULT_INGEST_PORT: u16 = 4097;

/// Start the local HTTP ingest server
pub async fn run_ingest_server(
    app: AppHandle,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_machine = StateMachine::new();

    let state = Arc::new(IngestState {
        state_machine: tokio::sync::Mutex::new(state_machine),
        app_handle: app.clone(),
    });

    let app_router = Router::new()
        .route("/ingest", post(ingest_handler))
        .route("/health", post(|| async { StatusCode::OK }))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    log::info!("Starting ingest server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app_router)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    Ok(())
}

/// Handle incoming event from the perch-bridge plugin
async fn ingest_handler(
    State(state): State<Arc<IngestState>>,
    Json(payload): Json<IngestPayload>,
) -> StatusCode {
    log::debug!("Ingested event: {} (data: {:?})", payload.event, payload.data);

    let event = parse_ingest_event(&payload.event, payload.data.as_deref());

    let mut sm = state.state_machine.lock().await;
    if let Some(new_state) = sm.process_event(&event) {
        log::info!("State changed to: {}", new_state);

        // Emit state change event to frontend
        let state_event = StateChangeEvent {
            state: new_state.to_string(),
        };
        if let Err(e) = state.app_handle.emit("mascot-state-change", &state_event) {
            log::error!("Failed to emit state change: {}", e);
        }

        // Fire notification on Done state
        if sm.should_notify(new_state) {
            let app_handle = state.app_handle.clone();
            tokio::spawn(async move {
                notifications::send_notification(&app_handle, "Perch", "opencode finished").await;
            });
        }
    }

    StatusCode::OK
}

/// Parse an ingest event payload into an OpenCodeEvent
fn parse_ingest_event(event_type: &str, data: Option<&str>) -> OpenCodeEvent {
    match event_type {
        "session.busy" | "session.working" => OpenCodeEvent::SessionBusy,
        "session.streaming" => OpenCodeEvent::SessionStreaming,
        "session.idle" => OpenCodeEvent::SessionIdle,
        "session.error" => OpenCodeEvent::SessionError {
            message: data.map(|s| s.to_string()),
        },
        _ => OpenCodeEvent::Unknown {
            event_type: event_type.to_string(),
        },
    }
}

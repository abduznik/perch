use std::env;

use crate::ingest;

/// Event source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    /// Plugin bridge mode (default): opencode plugin POSTs to local HTTP server
    Plugin,
    /// SSE mode (fallback): connect directly to opencode's SSE stream
    Sse,
}

impl EventSource {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sse" => EventSource::Sse,
            _ => EventSource::Plugin,
        }
    }
}

/// Default opencode SSE event stream port
const DEFAULT_SSE_PORT: u16 = 4096;

/// Default opencode SSE host
const DEFAULT_SSE_HOST: &str = "127.0.0.1";

/// Application configuration
pub struct AppConfig {
    sse_host: String,
    sse_port: u16,
    ingest_port: u16,
    source: EventSource,
}

impl AppConfig {
    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - PERCH_SOURCE: "plugin" (default) or "sse"
    /// - PERCH_HOST: opencode SSE server host (default: 127.0.0.1)
    /// - PERCH_PORT: opencode SSE server port (default: 4096)
    /// - OPENCODE_PORT: alternative SSE port variable (used by opencode)
    /// - PERCH_INGEST_PORT: local HTTP ingest server port (default: 4097)
    pub fn load() -> Self {
        let source = env::var("PERCH_SOURCE")
            .map(|s| EventSource::from_str(&s))
            .unwrap_or(EventSource::Plugin);

        let sse_host = env::var("PERCH_HOST").unwrap_or_else(|_| DEFAULT_SSE_HOST.to_string());

        let sse_port = env::var("PERCH_PORT")
            .or_else(|_| env::var("OPENCODE_PORT"))
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_SSE_PORT);

        let ingest_port = env::var("PERCH_INGEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(ingest::DEFAULT_INGEST_PORT);

        Self {
            sse_host,
            sse_port,
            ingest_port,
            source,
        }
    }

    /// Get the SSE event URL
    pub fn sse_event_url(&self) -> String {
        format!("http://{}:{}/event", self.sse_host, self.sse_port)
    }

    /// Get the ingest server port
    pub fn ingest_port(&self) -> u16 {
        self.ingest_port
    }

    /// Get the configured event source type
    pub fn source(&self) -> EventSource {
        self.source
    }
}

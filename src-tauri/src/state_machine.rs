use std::fmt;
use std::time::{Duration, Instant};

/// Events parsed from opencode's SSE stream
#[derive(Debug, Clone)]
pub enum OpenCodeEvent {
    /// Session started working (busy/streaming)
    SessionBusy,
    /// Session is streaming data
    SessionStreaming,
    /// Session went idle (finished working)
    SessionIdle,
    /// Session encountered an error
    SessionError { message: Option<String> },
    /// Unknown event type
    Unknown { event_type: String },
}

/// Mascot states for the companion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MascotState {
    /// Idle - waiting for opencode to start working
    Idle,
    /// Working - opencode is actively processing
    Working,
    /// Done - opencode just finished a task
    Done,
    /// Attention - needs user attention (e.g., error occurred)
    Attention,
    /// Error - something went wrong
    Error,
}

impl fmt::Display for MascotState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MascotState::Idle => write!(f, "Idle"),
            MascotState::Working => write!(f, "Working"),
            MascotState::Done => write!(f, "Done"),
            MascotState::Attention => write!(f, "Attention"),
            MascotState::Error => write!(f, "Error"),
        }
    }
}

/// State machine that tracks mascot state based on opencode events
pub struct StateMachine {
    current_state: MascotState,
    /// When the current working period started
    working_started: Option<Instant>,
    /// Minimum working duration before triggering notification
    min_working_duration: Duration,
}

impl StateMachine {
    /// Create a new state machine with default 10-second minimum working duration
    pub fn new() -> Self {
        Self {
            current_state: MascotState::Idle,
            working_started: None,
            min_working_duration: Duration::from_secs(10),
        }
    }

    /// Create a state machine with custom minimum working duration
    #[allow(dead_code)]
    pub fn with_min_duration(min_duration: Duration) -> Self {
        Self {
            current_state: MascotState::Idle,
            working_started: None,
            min_working_duration: min_duration,
        }
    }

    /// Process an event and return the new state
    /// Returns Some(MascotState) if state changed, None otherwise
    pub fn process_event(&mut self, event: &OpenCodeEvent) -> Option<MascotState> {
        let old_state = self.current_state;

        match event {
            OpenCodeEvent::SessionBusy | OpenCodeEvent::SessionStreaming => {
                if self.current_state == MascotState::Idle
                    || self.current_state == MascotState::Done
                    || self.current_state == MascotState::Error
                {
                    self.current_state = MascotState::Working;
                    self.working_started = Some(Instant::now());
                    log::info!("State transition: {} -> {}", old_state, self.current_state);
                }
            }
            OpenCodeEvent::SessionIdle => {
                if self.current_state == MascotState::Working {
                    // Check if we were working long enough
                    let worked_long_enough = self
                        .working_started
                        .map(|start| start.elapsed() >= self.min_working_duration)
                        .unwrap_or(false);

                    if worked_long_enough {
                        self.current_state = MascotState::Done;
                        log::info!(
                            "State transition: {} -> {} (worked long enough)",
                            old_state,
                            self.current_state
                        );
                    } else {
                        // Short work period, go back to idle without notification
                        self.current_state = MascotState::Idle;
                        log::info!(
                            "State transition: {} -> {} (short work, no notification)",
                            old_state,
                            self.current_state
                        );
                    }
                    self.working_started = None;
                }
            }
            OpenCodeEvent::SessionError { message } => {
                self.current_state = MascotState::Error;
                self.working_started = None;
                log::warn!(
                    "State transition: {} -> {} (error: {:?})",
                    old_state,
                    self.current_state,
                    message
                );
            }
            OpenCodeEvent::Unknown { event_type } => {
                log::debug!("Unknown event: {}", event_type);
                return None;
            }
        }

        if self.current_state != old_state {
            Some(self.current_state)
        } else {
            None
        }
    }

    /// Get the current state
    #[allow(dead_code)]
    pub fn current_state(&self) -> MascotState {
        self.current_state
    }

    /// Check if we should fire a notification for the current state transition
    pub fn should_notify(&self, new_state: MascotState) -> bool {
        new_state == MascotState::Done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_idle_to_working() {
        let mut sm = StateMachine::new();
        let result = sm.process_event(&OpenCodeEvent::SessionBusy);
        assert_eq!(result, Some(MascotState::Working));
        assert_eq!(sm.current_state(), MascotState::Working);
    }

    #[test]
    fn test_working_to_done_after_min_duration() {
        let mut sm = StateMachine::with_min_duration(Duration::from_millis(100));
        sm.process_event(&OpenCodeEvent::SessionBusy);

        // Simulate time passing
        std::thread::sleep(Duration::from_millis(150));

        let result = sm.process_event(&OpenCodeEvent::SessionIdle);
        assert_eq!(result, Some(MascotState::Done));
    }

    #[test]
    fn test_working_to_idle_short_duration() {
        let mut sm = StateMachine::with_min_duration(Duration::from_secs(10));
        sm.process_event(&OpenCodeEvent::SessionBusy);

        // Immediately go idle (short work period)
        let result = sm.process_event(&OpenCodeEvent::SessionIdle);
        assert_eq!(result, Some(MascotState::Idle));
    }

    #[test]
    fn test_error_state() {
        let mut sm = StateMachine::new();
        let result = sm.process_event(&OpenCodeEvent::SessionError {
            message: Some("test error".to_string()),
        });
        assert_eq!(result, Some(MascotState::Error));
    }
}

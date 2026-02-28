//! WebSocket session management module.
//!
//! This module provides the actor system for managing WebSocket connections from
//! `ComputerCraft` turtles. The architecture consists of:
//!
//! - `WebSocketSessionManager`: Central registry managing all active sessions
//! - `WebSocketSessionActor`: Individual actor per WebSocket connection
//! - Automatic cleanup of stale sessions via periodic timeout checks
//!
//! # Message Flow
//!
//! 1. Client connects → `NewSession` message → Actor registered
//! 2. Client sends data → Stream attached to actor → State updated
//! 3. REST API queries → `FetchClientData` → Returns all session states
//! 4. Client disconnects → `SessionDisconnected` → Actor removed
//! 5. Periodic timeout check → Stale sessions cleaned up automatically

pub mod actor;
pub mod stream;

use std::collections::HashMap;

use chrono::Utc;
use kameo::prelude::*;
use uuid::Uuid;

#[cfg(feature = "metrics")]
use crate::models::simplify_digging::States;
use crate::websockets::actor::{DiggingData, GetLastKeepalive};

/// Configuration for session timeout and cleanup behavior.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum time in seconds without a keepalive before a session is considered stale
    pub keepalive_timeout_secs: u64,

    /// Interval in seconds between timeout checks
    pub check_interval_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            keepalive_timeout_secs: 120, // 2 minutes
            check_interval_secs: 30,     // Check every 30 seconds
        }
    }
}

/// Central manager actor for all WebSocket sessions.
///
/// Maintains a registry of all active turtle connections and provides
/// query capabilities for monitoring. Automatically cleans up stale sessions
/// based on keepalive timeout configuration.
#[derive(Clone)]
pub struct WebSocketSessionManager {
    /// Map of session UUID to actor reference
    pub sessions: HashMap<Uuid, ActorRef<actor::WebSocketSessionActor>>,

    /// Configuration for session timeouts and cleanup
    pub config: SessionConfig,
}

impl Default for WebSocketSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketSessionManager {
    /// Creates a new session manager with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(SessionConfig::default())
    }

    /// Creates a new session manager with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Session timeout configuration
    #[must_use]
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            sessions: HashMap::new(),
            config,
        }
    }
}

/// Message to register a new WebSocket session.
#[derive(Clone)]
pub struct NewSession {
    pub uuid: Uuid,
    pub actor: ActorRef<actor::WebSocketSessionActor>,
}

/// Message to notify that a session has disconnected.
#[derive(Clone)]
pub struct SessionDisconnected {
    pub uuid: Uuid,
}

/// Message to fetch data from all active sessions.
#[derive(Clone)]
pub struct FetchClientData;

/// Message to fetch data for a specific session.
#[derive(Clone)]
pub struct FetchDataForClient {
    pub session_id: Uuid,
}

/// Internal message to trigger periodic timeout checks.
///
/// Sent by the manager to itself on a scheduled interval.
#[derive(Clone)]
struct CheckTimedOutSessions;

// Implement Actor trait to add lifecycle hooks
impl Actor for WebSocketSessionManager {
    type Args = Self;
    type Error = ();

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let check_interval = args.config.check_interval_secs;

        tracing::info!(
            check_interval_secs = check_interval,
            "Session manager started, spawning timeout checker task"
        );

        // Spawn background task to periodically check for timed-out sessions
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(check_interval));

            loop {
                interval.tick().await;

                // Send timeout check message to self
                if actor_ref.tell(CheckTimedOutSessions).await.is_err() {
                    tracing::warn!("Failed to send timeout check message, actor may be stopped");
                    break;
                }
            }
        });

        Ok(args)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: kameo::actor::WeakActorRef<Self>,
        _reason: kameo::error::ActorStopReason,
    ) -> Result<(), Self::Error> {
        tracing::info!(
            session_count = self.sessions.len(),
            "Session manager stopping, cleaning up {} active sessions",
            self.sessions.len()
        );

        // Stop all session actors gracefully
        for (uuid, session) in &self.sessions {
            tracing::debug!(uuid = ?uuid, "Stopping session actor");
            let _ = session.stop_gracefully().await;
        }

        Ok(())
    }
}

impl Message<NewSession> for WebSocketSessionManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: NewSession,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::info!(
            uuid = ?msg.uuid,
            total_sessions = self.sessions.len() + 1,
            "New session registered"
        );
        self.sessions.insert(msg.uuid, msg.actor);
        
        #[cfg(feature = "metrics")]
        {
            crate::metrics::counter!("sessions.registered.total").increment(1);
            crate::metrics::gauge!("sessions.active").set(self.sessions.len() as f64);
        }
    }
}

impl Message<SessionDisconnected> for WebSocketSessionManager {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionDisconnected,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.sessions.remove(&msg.uuid).is_some() {
            tracing::info!(
                uuid = ?msg.uuid,
                remaining_sessions = self.sessions.len(),
                "Session disconnected and removed"
            );
            
            #[cfg(feature = "metrics")]
            {
                crate::metrics::counter!("sessions.disconnected.total").increment(1);
                crate::metrics::gauge!("sessions.active").set(self.sessions.len() as f64);
            }
        } else {
            tracing::warn!(
                uuid = ?msg.uuid,
                "Attempted to remove non-existent session"
            );
        }
    }
}

impl Message<FetchClientData> for WebSocketSessionManager {
    type Reply = Result<HashMap<Uuid, DiggingData>, ()>;

    async fn handle(
        &mut self,
        _msg: FetchClientData,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut result = HashMap::new();
        for (uuid, session) in &self.sessions {
            // ask() returns Result<DiggingData, SendError> when Reply = Result<DiggingData, ()>
            match session.ask(actor::GetDiggingData).await {
                Ok(data) => {
                    result.insert(*uuid, data);
                }
                Err(e) => {
                    tracing::warn!(
                        uuid = ?uuid,
                        error = ?e,
                        "Failed to communicate with session actor"
                    );
                }
            }
        }

        // Update business metrics
        #[cfg(feature = "metrics")]
        {
            let total_active = result.len() as f64;
            crate::metrics::gauge!("turtles.active").set(total_active);
            
            // Count turtles by state
            let mut state_counts: HashMap<&str, usize> = HashMap::new();
            let mut completion_sum: f32 = 0.0;
            
            for data in result.values() {
                let state_label = match data.state {
                    States::Init => "init",
                    States::Idle => "idle",
                    States::Digging => "digging",
                    States::ReturnHome => "return-home",
                    States::ReturnMine => "return-mine",
                    States::Stuck => "stuck",
                    States::Error => "error",
                    States::Done => "done",
                    States::Teapot => "teapot",
                };
                *state_counts.entry(state_label).or_insert(0) += 1;
                completion_sum += data.completion_percent;
            }
            
            // Record state gauges
            for (state, count) in state_counts {
                crate::metrics::gauge!("turtles.by_state", "state" => state).set(count as f64);
            }
            
            // Calculate and record average completion
            let avg_completion = if total_active > 0.0 {
                completion_sum / total_active as f32
            } else {
                0.0
            };
            crate::metrics::gauge!("turtles.completion.avg").set(avg_completion as f64);
        }

        Ok(result)
    }
}

impl Message<FetchDataForClient> for WebSocketSessionManager {
    type Reply = Option<DiggingData>;

    async fn handle(
        &mut self,
        msg: FetchDataForClient,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(session) = self.sessions.get(&msg.session_id) {
            match session.ask(actor::GetDiggingData).await {
                Ok(data) => return Some(data),
                Err(e) => {
                    tracing::warn!(
                        session_id = ?msg.session_id,
                        error = ?e,
                        "Failed to communicate with session actor"
                    );
                }
            }
        }

        None
    }
}

impl Message<CheckTimedOutSessions> for WebSocketSessionManager {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: CheckTimedOutSessions,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let now = Utc::now();
        let timeout_threshold = chrono::Duration::seconds(
            i64::try_from(self.config.keepalive_timeout_secs).unwrap_or(120),
        );

        let mut timed_out_sessions = Vec::new();

        // Check all sessions for timeouts
        for (uuid, session) in &self.sessions {
            match session.ask(GetLastKeepalive).await {
                Ok(last_keepalive) => {
                    let elapsed: chrono::Duration = now - last_keepalive;
                    if elapsed > timeout_threshold {
                        tracing::warn!(
                            uuid = ?uuid,
                            elapsed_secs = elapsed.num_seconds(),
                            "Session timed out due to no keepalive"
                        );
                        timed_out_sessions.push((*uuid, session.clone()));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        uuid = ?uuid,
                        error = ?e,
                        "Failed to query session keepalive, marking for removal"
                    );
                    timed_out_sessions.push((*uuid, session.clone()));
                }
            }
        }

        // Stop and remove timed out sessions
        for (uuid, session) in timed_out_sessions {
            if let Err(e) = session.stop_gracefully().await {
                tracing::error!(
                    uuid = ?uuid,
                    error = ?e,
                    "Failed to stop timed out session gracefully"
                );
            }
            self.sessions.remove(&uuid);
            #[cfg(feature = "metrics")]
            crate::metrics::counter!("sessions.timeout.total").increment(1);
        }

        if !self.sessions.is_empty() {
            tracing::debug!(
                active_sessions = self.sessions.len(),
                "Timeout check completed"
            );
        }
    }
}

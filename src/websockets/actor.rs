//! WebSocket session actor for managing individual turtle digging connections.
//!
//! This module contains the actor responsible for handling a single WebSocket connection
//! from a `ComputerCraft` turtle. Each connection tracks the turtle's state, position, and
//! progress through incoming messages.
//!
//! # Architecture
//!
//! - Each WebSocket connection spawns one `WebSocketSessionActor`
//! - The actor processes incoming messages via an attached stream
//! - State is updated based on packet type (Init, Status, Completion, etc.)
//! - The actor stops gracefully when the connection closes or errors occur

use bytes::Bytes;
use chrono::Utc;
use kameo::{message::StreamMessage, prelude::*};
use sockudo_ws::protocol::Message as WebSocketMessage;
use tracing::Level;
use uuid::Uuid;

use crate::models::simplify_digging::{Facing, Fuel, Position, SimplifyPacket, States};

/// Type alias for WebSocket stream messages to improve readability
type WebSocketStreamMessage =
    StreamMessage<std::result::Result<sockudo_ws::Message, sockudo_ws::Error>, (), ()>;

/// Actor representing a single WebSocket session with a turtle client.
///
/// Each actor maintains the current state of one turtle, including its position,
/// facing direction, fuel level, and task completion status.
pub struct WebSocketSessionActor {
    // TODO: Future enhancement - Add writer field for bidirectional communication
    // This would allow the server to send commands/responses back to the turtle.
    // pub(crate) writer: SplitWriter<S>,
    /// Unique identifier for this session
    pub uuid: uuid::Uuid,

    /// Timestamp of the last keepalive message received
    /// Used by the session manager to detect and clean up stale connections
    pub last_keepalive: chrono::DateTime<Utc>,

    /// Current digging state data for this turtle
    pub data: DiggingData,
}

/// Complete state snapshot of a turtle's digging operation.
///
/// This structure is serializable and can be queried via the REST API
/// to monitor turtle progress.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct DiggingData {
    /// Computer ID of the turtle
    pub turtle_id: u16,

    /// Current 3D position in the world
    pub pos: Position,

    /// Direction the turtle is facing
    pub facing: Facing,

    /// Current and maximum fuel levels
    pub fuel: Fuel,

    /// Current operational state (e.g., digging, moving, idle)
    pub state: States,

    /// Progress percentage (0.0 to 100.0)
    pub completion_percent: f32,

    /// Arguments passed when the turtle program started
    pub program_arguments: serde_json::Value,
}

/// Message to query the current digging data from a session actor.
#[derive(Clone)]
pub struct GetDiggingData;

/// Message to query the last keepalive timestamp from a session actor.
///
/// Used by the session manager to detect stale connections.
#[derive(Clone)]
pub struct GetLastKeepalive;

impl WebSocketSessionActor {
    /// Creates a new WebSocket session actor with a unique identifier.
    ///
    /// # Arguments
    ///
    /// * `uuid` - Unique session identifier
    #[must_use]
    pub fn new(uuid: Uuid) -> Self {
        let last_keepalive = Utc::now();

        Self {
            uuid,
            last_keepalive,
            data: DiggingData::default(),
        }
    }

    /// Handles incoming text messages from the WebSocket stream.
    ///
    /// Parses JSON packets and delegates to `handle_packet` for processing.
    fn handle_text_message(&mut self, text: &Bytes) {
        let packet = match serde_json::from_slice::<SimplifyPacket>(text.as_ref()) {
            Ok(p) => p,
            Err(e) => {
                let text_str = str::from_utf8(&text).unwrap_or("<invalid UTF-8>");

                tracing::error!(
                    uuid = ?self.uuid,
                    error = ?e,
                    text = text_str,
                    "Failed to parse packet from client"
                );
                
                #[cfg(feature = "metrics")]
                crate::metrics::counter!("ws.messages.parse_errors.total").increment(1);
                return;
            }
        };

        self.handle_packet(packet);
    }

    /// Processes a parsed packet and updates internal state accordingly.
    ///
    /// # Arguments
    ///
    /// * `packet` - The parsed packet from the turtle client
    fn handle_packet(&mut self, packet: SimplifyPacket) {
        // Determine packet type label for metrics
        #[cfg(feature = "metrics")]
        {
            let packet_type = match &packet {
                SimplifyPacket::Init { .. } => "init",
                SimplifyPacket::Completion { .. } => "completion",
                _ => "other",
            };
            crate::metrics::counter!("ws.messages.received.total", "packet_type" => packet_type).increment(1);
        }
        
        match packet {
            SimplifyPacket::Init {
                program_arguments,
                turtle_id,
            } => {
                tracing::info!(
                    uuid = ?self.uuid,
                    turtle_id = turtle_id,
                    "Turtle initialized with ID"
                );
                self.data.turtle_id = turtle_id;
                self.data.program_arguments = program_arguments;
            }
            SimplifyPacket::Keepalive => {
                self.last_keepalive = Utc::now();
            }
            SimplifyPacket::State { state } => {
                tracing::debug!(
                    uuid = ?self.uuid,
                    state = ?state,
                    "Turtle state updated"
                );
                self.data.state = state;
            }
            SimplifyPacket::Status { pos, facing, fuel } => {
                self.data.pos = pos;
                self.data.facing = facing;
                self.data.fuel = fuel;
            }
            SimplifyPacket::Completion { completion_percent } => {
                tracing::info!(
                    uuid = ?self.uuid,
                    completion_percent = completion_percent,
                    "Turtle progress updated"
                );
                self.data.completion_percent = completion_percent;
            }
            _ => {
                tracing::debug!(
                    uuid = ?self.uuid,
                    "Received unhandled packet type"
                );
            }
        }

        // TODO: Future enhancement - Send acknowledgment back to turtle
        // When bidirectional communication is implemented:
        // let response = Response { message: "gotcha :3" };
        // let serialized = serde_json::to_string(&response).expect("serialization failed");
        // self.writer.send(WebSocketMessage::Text(Bytes::from(serialized))).await.ok();
    }

    /// Handles WebSocket connection closure.
    ///
    /// Logs the closure and initiates graceful actor shutdown.
    async fn handle_close(&self, ctx: &Context<Self, ()>) {
        tracing::info!(
            uuid = ?self.uuid,
            "Client closed WebSocket connection"
        );
        ctx.actor_ref().stop_gracefully().await.ok();
    }

    /// Handles WebSocket stream errors.
    ///
    /// Logs the error with context and initiates graceful actor shutdown.
    ///
    /// # Arguments
    ///
    /// * `error` - The WebSocket error that occurred
    async fn handle_stream_error(&self, error: sockudo_ws::Error, ctx: &Context<Self, ()>) {
        tracing::error!(
            uuid = ?self.uuid,
            error = ?error,
            "WebSocket stream error occurred"
        );
        ctx.actor_ref().stop_gracefully().await.ok();
    }

    /// Handles stream completion (normal end of stream).
    ///
    /// Logs the completion and initiates graceful actor shutdown.
    async fn handle_stream_finished(&self, ctx: &Context<Self, ()>) {
        tracing::info!(
            uuid = ?self.uuid,
            "WebSocket stream finished"
        );
        ctx.actor_ref().stop_gracefully().await.ok();
    }
}

impl Actor for WebSocketSessionActor {
    type Args = Self;
    type Error = ();

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

impl Message<GetDiggingData> for WebSocketSessionActor {
    type Reply = Result<DiggingData, ()>;

    async fn handle(
        &mut self,
        _msg: GetDiggingData,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.data.clone())
    }
}

impl Message<GetLastKeepalive> for WebSocketSessionActor {
    type Reply = Result<chrono::DateTime<Utc>, ()>;

    async fn handle(
        &mut self,
        _msg: GetLastKeepalive,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self.last_keepalive)
    }
}

impl Message<WebSocketStreamMessage> for WebSocketSessionActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: WebSocketStreamMessage,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let span = tracing::span!(Level::INFO, "ws_stream_handler", uuid = ?self.uuid);
        let _enter = span.enter();

        match msg {
            StreamMessage::Next(Ok(ws_msg)) => match ws_msg {
                WebSocketMessage::Text(text) => {
                    self.handle_text_message(&text);
                }
                WebSocketMessage::Close(_) => {
                    self.handle_close(ctx).await;
                }
                WebSocketMessage::Ping(_) | WebSocketMessage::Pong(_) => {
                    // Ping/Pong handled automatically by sockudo_ws
                }
                WebSocketMessage::Binary(_) => {
                    tracing::warn!("Received unexpected binary message");
                }
            },
            StreamMessage::Next(Err(e)) => {
                self.handle_stream_error(e, ctx).await;
            }
            StreamMessage::Started(()) => {
                tracing::info!("WebSocket stream started");
            }
            StreamMessage::Finished(()) => {
                self.handle_stream_finished(ctx).await;
            }
        }
    }
}

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use snafu::prelude::*;

/// Main error type for the application
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// WebSocket connection errors
    #[snafu(display("WebSocket connection failed: {}", source))]
    WebSocketConnection { source: std::io::Error },

    /// WebSocket upgrade errors
    #[snafu(display("WebSocket upgrade failed: {}", source))]
    WebSocketUpgrade { source: hyper::Error },

    /// WebSocket send errors
    #[snafu(display("Failed to send WebSocket message: {}", reason))]
    WebSocketSend { reason: String },

    /// WebSocket receive errors
    #[snafu(display("Failed to receive WebSocket message: {}", reason))]
    WebSocketReceive { reason: String },

    /// Actor communication errors
    #[snafu(display("Actor communication failed: {}", reason))]
    ActorCommunication { reason: String },

    /// Actor mailbox error
    #[snafu(display("Actor mailbox error: {}", source))]
    ActorMailbox { source: kameo::error::SendError },

    /// Session not found
    #[snafu(display("Session not found: {}", session_id))]
    SessionNotFound { session_id: uuid::Uuid },

    /// Serialization errors
    #[snafu(display("Serialization failed: {}", source))]
    Serialization { source: serde_json::Error },

    /// Deserialization errors
    #[snafu(display("Deserialization failed: {}", source))]
    Deserialization { source: serde_json::Error },

    /// Invalid message format
    #[snafu(display("Invalid message format: {}", reason))]
    InvalidMessageFormat { reason: String },

    /// Invalid packet type
    #[snafu(display("Invalid packet type: {}", packet_type))]
    InvalidPacketType { packet_type: String },

    /// TCP listener bind error
    #[snafu(display("Failed to bind TCP listener: {}", source))]
    TcpBind { source: std::io::Error },

    /// Server startup error
    #[snafu(display("Server failed to start: {}", reason))]
    ServerStartup { reason: String },

    /// Internal server error
    #[snafu(display("Internal server error: {}", reason))]
    InternalServer { reason: String },

    /// Bad request error
    #[snafu(display("Bad request: {}", reason))]
    BadRequest { reason: String },

    /// Not found error
    #[snafu(display("Resource not found: {}", resource))]
    NotFound { resource: String },

    /// Timeout error
    #[snafu(display("Operation timed out: {}", operation))]
    Timeout { operation: String },

    /// Configuration error
    #[snafu(display("Configuration error: {}", reason))]
    Configuration { reason: String },
}

/// Result type alias using our Error type
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Implement `IntoResponse` for Error to allow it to be returned from Axum handlers
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            Self::WebSocketConnection { .. } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "WEBSOCKET_CONNECTION_ERROR",
                self.to_string(),
            ),
            Self::WebSocketUpgrade { .. } => (
                StatusCode::BAD_REQUEST,
                "WEBSOCKET_UPGRADE_ERROR",
                self.to_string(),
            ),
            Self::WebSocketSend { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "WEBSOCKET_SEND_ERROR",
                self.to_string(),
            ),
            Self::WebSocketReceive { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "WEBSOCKET_RECEIVE_ERROR",
                self.to_string(),
            ),
            Self::ActorCommunication { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ACTOR_COMMUNICATION_ERROR",
                self.to_string(),
            ),
            Self::ActorMailbox { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "ACTOR_MAILBOX_ERROR",
                self.to_string(),
            ),
            Self::SessionNotFound { .. } => {
                (StatusCode::NOT_FOUND, "SESSION_NOT_FOUND", self.to_string())
            }
            Self::Serialization { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR",
                self.to_string(),
            ),
            Self::Deserialization { .. } => (
                StatusCode::BAD_REQUEST,
                "DESERIALIZATION_ERROR",
                self.to_string(),
            ),
            Self::InvalidMessageFormat { .. } => (
                StatusCode::BAD_REQUEST,
                "INVALID_MESSAGE_FORMAT",
                self.to_string(),
            ),
            Self::InvalidPacketType { .. } => (
                StatusCode::BAD_REQUEST,
                "INVALID_PACKET_TYPE",
                self.to_string(),
            ),
            Self::TcpBind { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "TCP_BIND_ERROR",
                self.to_string(),
            ),
            Self::ServerStartup { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERVER_STARTUP_ERROR",
                self.to_string(),
            ),
            Self::InternalServer { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                self.to_string(),
            ),
            Self::BadRequest { .. } => (StatusCode::BAD_REQUEST, "BAD_REQUEST", self.to_string()),
            Self::NotFound { .. } => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            Self::Timeout { .. } => (StatusCode::REQUEST_TIMEOUT, "TIMEOUT", self.to_string()),
            Self::Configuration { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIGURATION_ERROR",
                self.to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

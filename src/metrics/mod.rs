//! Metrics instrumentation module.
//!
//! Provides a facade over the `metrics` crate that's only active when the `metrics` feature is enabled.
//! When the `metrics` feature is disabled, all metric calls are feature-gated and compile out completely.
//!
//! # Usage
//!
//! ```rust
//! use crate::metrics::{counter, gauge, histogram};
//!
//! #[cfg(feature = "metrics")]
//! counter!("requests.total").increment(1);
//!
//! #[cfg(feature = "metrics")]
//! gauge!("connections.active").set(42.0);
//!
//! #[cfg(feature = "metrics")]
//! histogram!("request.duration").record(0.123);
//! ```

#[cfg(feature = "metrics")]
pub use metrics::{
    counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
};

/// Initialize metric descriptions.
///
/// This should be called once at application startup to register metric metadata.
/// When the metrics feature is disabled, this is a no-op.
pub fn describe_metrics() {
    #[cfg(feature = "metrics")]
    {
        // HTTP metrics
        describe_counter!("http.requests.total", "Total HTTP requests");
        describe_histogram!("http.request.duration", "HTTP request duration in seconds");
        describe_gauge!("http.requests.active", "Active HTTP requests");

        // WebSocket metrics
        describe_counter!(
            "ws.connections.total",
            "Total WebSocket connections established"
        );
        describe_gauge!("ws.connections.active", "Active WebSocket connections");
        describe_counter!(
            "ws.connections.closed.total",
            "Total WebSocket connections closed"
        );
        describe_counter!(
            "ws.messages.received.total",
            "Total WebSocket messages received"
        );
        describe_counter!(
            "ws.messages.parse_errors.total",
            "Total WebSocket message parse errors"
        );

        // Session metrics
        describe_counter!("sessions.registered.total", "Total sessions registered");
        describe_gauge!("sessions.active", "Active sessions");
        describe_counter!("sessions.disconnected.total", "Total sessions disconnected");
        describe_counter!("sessions.timeout.total", "Total sessions timed out");

        // Business metrics
        describe_gauge!("turtles.active", "Number of active turtles");
        describe_gauge!("turtles.by_state", "Number of turtles by state");
        describe_gauge!(
            "turtles.completion.avg",
            "Average turtle completion percentage"
        );
    }
}

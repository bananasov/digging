#![feature(str_as_str)]
#![forbid(clippy::self_named_module_files)]
#![deny(clippy::all, clippy::unwrap_used, clippy::undocumented_unsafe_blocks)]
#![warn(clippy::pedantic, clippy::panic, clippy::nursery)]

pub mod errors;
pub mod metrics;
pub mod models;
pub mod routes;
pub mod websockets;

use std::sync::Arc;

use kameo::actor::ActorRef;

pub type SessionManagerState = Arc<ActorRef<websockets::WebSocketSessionManager>>;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use hyper_util::rt::TokioIo;

use kameo::prelude::*;
use sockudo_ws::handshake::generate_accept_key;
use sockudo_ws::{Config, WebSocketStream};
use uuid::Uuid;

use crate::{
    SessionManagerState,
    websockets::{
        NewSession, SessionDisconnected, actor::WebSocketSessionActor,
        stream::WebSocketStreamWrapper,
    },
};

async fn ws_handler(State(state): State<SessionManagerState>, req: Request) -> impl IntoResponse {
    // Validate WebSocket upgrade request
    let key = match req.headers().get("sec-websocket-key") {
        Some(k) => k.to_str().unwrap_or(""),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Generate accept key
    let accept_key = generate_accept_key(key);

    // Spawn handler for after upgrade
    tokio::spawn(async move {
        // Get the upgraded connection
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                let io = TokioIo::new(upgraded);
                handle_socket(io, state.clone()).await;
            }
            Err(e) => {
                tracing::error!("WebSocket upgrade error: {}", e);
            }
        }
    });

    // Return 101 Switching Protocols
    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header("Sec-WebSocket-Accept", accept_key)
        .body(Body::empty())
        .expect("made an invalid body")
}

async fn handle_socket<S>(stream: S, state: SessionManagerState)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Record connection start
    #[cfg(feature = "metrics")]
    {
        crate::metrics::counter!("ws.connections.total").increment(1);
        crate::metrics::gauge!("ws.connections.active").increment(1.0);
    }
    
    // Create sockudo-ws WebSocketStream with our config
    let config = Config::builder()
        .max_payload_length(16 * 1024)
        .idle_timeout(60)
        .build();

    let ws = WebSocketStream::server(stream, config);
    let (reader, _writer) = ws.split();
    let stream = WebSocketStreamWrapper::new(reader);

    let uuid = Uuid::new_v4();
    let actor = WebSocketSessionActor::spawn(WebSocketSessionActor::new(uuid));
    let _ = state
        .tell(NewSession {
            uuid,
            actor: actor.clone(),
        })
        .await;

    if let Err(e) = actor.attach_stream(stream, (), ()).await {
        tracing::error!("Failed to attach stream to actor: {:?}", e);
        #[cfg(feature = "metrics")]
        {
            crate::metrics::gauge!("ws.connections.active").decrement(1.0);
            crate::metrics::counter!("ws.connections.closed.total", "reason" => "attach_error").increment(1);
        }
        return;
    }
    actor.wait_for_shutdown().await;

    let _ = state.tell(SessionDisconnected { uuid }).await;
    
    // Record connection close
    #[cfg(feature = "metrics")]
    {
        crate::metrics::gauge!("ws.connections.active").decrement(1.0);
        crate::metrics::counter!("ws.connections.closed.total", "reason" => "normal").increment(1);
    }
}

pub fn config() -> Router<SessionManagerState> {
    Router::new().route("/", get(ws_handler))
}

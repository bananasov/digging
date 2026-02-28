use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};

use crate::{
    SessionManagerState,
    errors::{Error, Result},
    models::responses::ApiResponse,
    websockets::{FetchClientData, FetchDataForClient},
};

async fn get_all_clients(State(state): State<SessionManagerState>) -> Result<impl IntoResponse> {
    let clients = state
        .ask(FetchClientData)
        .await
        .map_err(|_| Error::ActorCommunication {
            reason: "Failed to fetch client data".to_string(),
        })?;

    let response = ApiResponse::success(clients);
    Ok(Json(response))
}

async fn get_data_for_client(
    State(state): State<SessionManagerState>,
    Path(session_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse> {
    let client = state
        .ask(FetchDataForClient { session_id })
        .await
        .map_err(|_| Error::ActorCommunication {
            reason: "Failed to fetch client data".to_string(),
        })?;

    client.map_or_else(
        || Err(Error::SessionNotFound { session_id }),
        |data| {
            let response = ApiResponse::success(data);
            Ok(Json(response))
        },
    )
}

pub fn config() -> Router<SessionManagerState> {
    Router::new()
        .route("/clients", get(get_all_clients))
        .route("/clients/{session_id}", get(get_data_for_client))
}

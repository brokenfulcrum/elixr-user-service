use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use log::{debug, error, info};
use serde_json::json;

use crate::api::{does_user_exist, emit_event};
use crate::ApiState;
use crate::events::{UserCreatedEvent, UserRegisteredEvent};
use crate::models::User;

pub async fn create_user_account(
    State(state): State<ApiState>,
    Json(params): Json<UserRegisteredEvent>,
) -> impl IntoResponse {
    debug!("Request received: {:#?}", params);
    let user_id = params.user_id.clone();

    // Make sure the user does not exist
    if does_user_exist(&state.firestore_client, &user_id).await? {
        return Err((
            StatusCode::FOUND,
            Json(json!({"status": "User already exists"})),
        ));
    };

    let user = User {
        user_id: params.user_id.clone(),
        username: params.username.clone(),
        email: params.email.clone(),
        password: "DEPRECATED".to_string(), // This is deprecated and should not be used
        created_at: Some(prost_wkt_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
        updated_at: Some(prost_wkt_types::Timestamp {
            seconds: chrono::Utc::now().timestamp(),
            nanos: 0,
        }),
    };
    let returned = match state
        .firestore_client
        .fluent()
        .insert()
        .into("users")
        .document_id(&user.user_id)
        .object(&user)
        .execute::<User>()
        .await
    {
        Ok(_) => {
            info!("User created: {:#?}", user);
            user
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": format!("Failed to create user: {}", e)})),
            ));
        }
    };

    emit_event(
        &state.pubsub_client,
        "UserCreatedEvent",
        &serde_json::to_string(&UserCreatedEvent {
            user: Some(returned.clone()),
        })
            .unwrap(),
    )
        .await?;

    return Ok((
        StatusCode::CREATED,
        Json(json!(returned)),
    ));
}
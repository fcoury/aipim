use std::net::SocketAddr;

use aipim::client::{Client, Message, Response as AipimResponse};
use axum::{
    debug_handler,
    extract::{rejection::JsonRejection, FromRequest, State},
    http,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::Serialize;

enum ApiError {
    JsonRejection(JsonRejection),
    AnyhowError(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            ApiError::JsonRejection(rejection) => (rejection.status(), rejection.body_text()),
            ApiError::AnyhowError(error) => {
                (http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
        };

        (status, ApiJson(ErrorResponse { message })).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::AnyhowError(error)
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
struct ApiJson<T>(T);

impl<T> IntoResponse for ApiJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(Clone)]
struct AppState {
    default_model: String,
}

pub async fn listen(addr: SocketAddr, default_model: impl Into<String>) -> anyhow::Result<()> {
    let default_model = default_model.into();

    log::info!("Default model: {default_model}");
    log::info!("Listening on {addr}...");

    let state = AppState { default_model };

    let app = Router::new()
        .route("/api/messages", post(messages))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[debug_handler]
async fn messages(
    State(state): State<AppState>,
    ApiJson(message): ApiJson<Message>,
) -> Result<ApiJson<AipimResponse>, ApiError> {
    log::debug!("Sending message: {message:?}");
    let client = Client::new(&state.default_model)?;
    client
        .send_message(message)
        .await
        .map(ApiJson)
        .map_err(Into::into)
}

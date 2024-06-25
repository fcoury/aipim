use std::net::SocketAddr;

use aipim::client::Response as AipimResponse;
use axum::{
    extract::{rejection::JsonRejection, FromRequest},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::Serialize;

enum ApiError {
    JsonRejection(JsonRejection),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            ApiError::JsonRejection(rejection) => (rejection.status(), rejection.body_text()),
        };

        (status, ApiJson(ErrorResponse { message })).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
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

pub async fn listen(addr: SocketAddr) -> anyhow::Result<()> {
    let app = Router::new().route("/api/messages", post(messages));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn messages() -> Result<ApiJson<AipimResponse>, ApiError> {
    Ok(ApiJson(AipimResponse::new("Hello, world!")))
}

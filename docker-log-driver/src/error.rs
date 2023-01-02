use axum::{
    http::StatusCode,
    Json,
    response::{
        IntoResponse,
        Response,
    },
};

use serde::Serialize;
use serde_json::json;


#[derive(Debug, Clone, Serialize)]
pub enum HttpError {
    BadRequest(String),
}


pub trait Loggable<T, E> {
    fn log_error<C>(self, context: C) -> Result<T, E>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;


impl<T> Loggable<T, BoxedError> for Result<T, BoxedError> {
    fn log_error<C: std::fmt::Display + Send + Sync + 'static>(self, context: C) -> Result<T, BoxedError> {
        self
            .map_err(|e| {
                tracing::error!("{} {:#?}", context, e);

                e
            })
    }
}


impl HttpError {
    pub fn bad_request(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Bad Request".to_string());

        Self::BadRequest(message)
    }
}


impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            HttpError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
        };

        (status_code, Json(json!({"Err": message}))).into_response()
    }
}

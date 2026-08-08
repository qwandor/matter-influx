use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use eyre::Report;

#[derive(Debug)]
pub struct AppError(Report);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal error: {}", self.0),
        )
            .into_response()
    }
}

impl<E: Into<Report>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

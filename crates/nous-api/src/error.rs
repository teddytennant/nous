use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: 404,
            message: msg.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: 401,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: 500,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::json!({
            "error": {
                "status": self.status,
                "message": self.message,
            }
        });
        (status, axum::Json(body)).into_response()
    }
}

impl From<nous_core::Error> for ApiError {
    fn from(err: nous_core::Error) -> Self {
        match &err {
            nous_core::Error::NotFound(msg) => Self::not_found(msg.clone()),
            nous_core::Error::PermissionDenied(msg) => Self::unauthorized(msg.clone()),
            nous_core::Error::InvalidInput(msg) => Self::bad_request(msg.clone()),
            _ => Self::internal(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes() {
        let err = ApiError::not_found("item not found");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("404"));
        assert!(json.contains("item not found"));
    }

    #[test]
    fn from_core_error() {
        let core_err = nous_core::Error::NotFound("user".into());
        let api_err = ApiError::from(core_err);
        assert_eq!(api_err.status, 404);
    }

    #[test]
    fn status_codes() {
        assert_eq!(ApiError::bad_request("x").status, 400);
        assert_eq!(ApiError::unauthorized("x").status, 401);
        assert_eq!(ApiError::internal("x").status, 500);
    }
}

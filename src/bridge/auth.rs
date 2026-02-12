use axum::{
    extract::Request,
    http::{header, StatusCode},
};

const DEFAULT_ALLOWED_ORIGIN: &str = "https://skills.runwaize.com";
const BRIDGE_TOKEN_HEADER: &str = "x-bridge-token";

#[derive(Clone)]
pub struct BridgeAuth {
    pub session_token: String,
    pub allowed_origin: String,
}

impl BridgeAuth {
    pub fn new(session_token: String, allowed_origin: Option<String>) -> Self {
        Self {
            session_token,
            allowed_origin: allowed_origin
                .unwrap_or_else(|| DEFAULT_ALLOWED_ORIGIN.to_string()),
        }
    }

    pub fn validate_request(&self, request: &Request) -> Result<(), StatusCode> {
        let headers = request.headers();

        let origin = headers
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if origin != self.allowed_origin {
            return Err(StatusCode::FORBIDDEN);
        }

        let token = headers
            .get(BRIDGE_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if token != self.session_token {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(())
    }

    pub fn validate_origin_only(&self, request: &Request) -> Result<(), StatusCode> {
        let origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if origin != self.allowed_origin {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(())
    }
}

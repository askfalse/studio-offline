use axum::{
    Router,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Json},
    routing::post,
};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::services::ServeFile;

use crate::app_state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route_service(
            "/.well-known/openid-configuration",
            ServeFile::new("static/auth/OAuth/openid.json"),
        )
        .route("/v1/token", post(generate_token))
        .route_service(
            "/v1/userinfo",
            ServeFile::new("static/auth/OAuth/userinfo.json"),
        )
        .route_service(
            "/v1/authorize",
            ServeFile::new("static/auth/OAuth/authorize.html"),
        )
}

async fn generate_token() -> impl IntoResponse {
    let file_path = PathBuf::from("static/auth/OAuth/token.json");
    let content = match tokio::fs::read_to_string(file_path).await {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read token.json",
            )
                .into_response();
        }
    };

    let mut token_response: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse token.json",
            )
                .into_response();
        }
    };

    let epoch_time_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut rng = rand::rng();
    let kid_bytes: [u8; 16] = rng.random();
    let kid = general_purpose::URL_SAFE_NO_PAD.encode(kid_bytes);

    let header = serde_json::json!({
        "alg": "none",
        "kid": kid,
        "typ": "JWT"
    });

    let jti_bytes: [u8; 20] = rng.random();
    let jti = "id.".to_owned() + &hex::encode(jti_bytes);

    let aid_bytes: [u8; 16] = rng.random();
    let aid = hex::encode(aid_bytes);

    let access_token_payload = serde_json::json!({
        "sub": "1",
        "aid": aid,
        "scope": "age:read credentials:read openid:read premium:read profile:read roles:read",
        "jti": jti,
        "nbf": epoch_time_now,
        "exp": epoch_time_now,
        "iat": epoch_time_now,
        "iss": "https://apis.roblox.com/oauth/",
        "aud": "1"
    });

    let id_token_payload = serde_json::json!({
        "sub": "1",
        "name": "Roblox",
        "nickname": "Roblox",
        "preferred_username": "Roblox",
        "created_at": epoch_time_now,
        "profile": "https://www.roblox.com/users/1/profile",
        "picture":"http://localhost:8081/headshot",
        "id": jti,
        "nonce":"id-roblox","jti":jti,"nbf":epoch_time_now,"exp":epoch_time_now,"iat":epoch_time_now,"iss":"https://apis.roblox.com/oauth/","aud":"1"
    });

    let base64_url_encode =
        |v: &Value| -> String { general_purpose::URL_SAFE_NO_PAD.encode(v.to_string()) };

    let access_token = format!(
        "{}.{}.",
        base64_url_encode(&header),
        base64_url_encode(&access_token_payload)
    );

    let id_token = format!(
        "{}.{}.",
        base64_url_encode(&header),
        base64_url_encode(&id_token_payload)
    );
    token_response["access_token"] = Value::String(access_token.clone());
    token_response["id_token"] = id_token.into();
    let mut response = Json(token_response).into_response();
    response.headers_mut().insert("set-cookie", HeaderValue::from_str(".ROBLOSECURITY=_|WARNING:-DO-NOT-SHARE-THIS.--Sharing-this-will-allow-someone-to-log-in-as-you-and-to-steal-your-ROBUX-and-items.|offline; domain=localhost:8081; expires=Tue, 16-Nov-2055 02:58:32 GMT; path=/; Secure; SameSite=Lax; HttpOnly").unwrap());
    response
}

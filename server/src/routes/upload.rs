use axum::{
    Json, Router,
    extract::{Multipart, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct AssetRequest {
    assetType: String,
    displayName: String,
    #[serde(default)]
    description: String,
    creationContext: CreationContext,
}

#[derive(Deserialize)]
struct CreationContext {
    creator: Creator,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Creator {
    userId: u64,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct OperationResponse {
    path: String,
    operationId: String,
    done: bool,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct OperationStatus {
    path: String,
    operationId: String,
    done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<AssetResponse>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct AssetResponse {
    path: String,
    revisionId: String,
    revisionCreateTime: String,
    assetId: String,
    displayName: String,
    description: String,
    assetType: String,
    creationContext: ResponseCreationContext,
    moderationResult: ModerationResult,
    state: String,
}

#[derive(Serialize)]
struct ResponseCreationContext {
    creator: ResponseCreator,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct ResponseCreator {
    userId: String,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct ModerationResult {
    moderationState: String,
}

pub struct UploadState {
    operations: Mutex<HashMap<String, OperationData>>,
}

pub struct OperationData {
    asset_request: AssetRequest,
    #[allow(dead_code)]
    file_content: Vec<u8>,
    completed: bool,
    asset_id: Option<String>,
}

use once_cell::sync::Lazy;

static UPLOAD_STATE: Lazy<Arc<UploadState>> = Lazy::new(|| {
    Arc::new(UploadState {
        operations: Mutex::new(HashMap::new()),
    })
});

pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/user-auth/v1/assets", post(handle_create_asset))
        .route("/user-auth/v1/assets/", post(handle_create_asset))
        .route(
            "/user-auth/v1/operations/{operation_id}",
            get(|Path(operation_id): Path<String>| async move {
                handle_get_operation(operation_id).await
            }),
        )
        .route(
            "/user-auth/v1/operations/{operation_id}/",
            get(|Path(operation_id): Path<String>| async move {
                handle_get_operation(operation_id).await
            }),
        )
}

async fn handle_create_asset(mut multipart: Multipart) -> Response {
    let mut asset_request: Option<AssetRequest> = None;
    let mut file_content: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        let data = field.bytes().await.unwrap();

        match name.as_str() {
            "request" => {
                asset_request = Some(serde_json::from_slice(&data).unwrap());
            }
            "fileContent" => {
                file_content = Some(data.to_vec());
            }
            _ => {}
        }
    }

    let operation_id = Uuid::new_v4().to_string();
    let path = format!("operations/{}", operation_id);

    if let Some(req) = asset_request {
        let op_data = OperationData {
            asset_request: req,
            file_content: file_content.unwrap_or_default(),
            completed: false,
            asset_id: None,
        };

        UPLOAD_STATE
            .operations
            .lock()
            .await
            .insert(operation_id.clone(), op_data);
    }

    let response = OperationResponse {
        path: path.clone(),
        operationId: operation_id,
        done: false,
    };

    (StatusCode::OK, Json(response)).into_response()
}

// prevents the 0.000000000000000000001% chance of an asset id collision
async fn generate_unique_asset_id() -> u64 {
    loop {
        let id: u64 = rand::random_range(10000000000000..9999999999999999);
        let path_str = format!("static/assets/{}", id);
        if !fs::try_exists(&path_str).await.unwrap_or(false) {
            return id;
        }
    }
}

async fn handle_get_operation(operation_id: String) -> Response {
    let mut operations = UPLOAD_STATE.operations.lock().await;

    if let Some(op_data) = operations.get_mut(&operation_id) {
        if !op_data.completed {
            op_data.completed = true;
            let asset_id = generate_unique_asset_id().await;
            op_data.asset_id = Some(asset_id.to_string());

            let asset_path = format!("static/assets/{}", asset_id);
            if let Err(e) = fs::write(&asset_path, &op_data.file_content).await {
                eprintln!("Failed to write asset file: {}", e);
            }
        }

        let asset_id = op_data.asset_id.as_ref().unwrap().clone();
        let path = format!("operations/{}", operation_id);

        let response = OperationStatus {
            path: path.clone(),
            operationId: operation_id,
            done: true,
            response: Some(AssetResponse {
                path: format!("assets/{}", asset_id),
                revisionId: "1".to_string(),
                revisionCreateTime: chrono::Utc::now().to_rfc3339(),
                assetId: asset_id,
                displayName: op_data.asset_request.displayName.clone(),
                description: op_data.asset_request.description.clone(),
                assetType: op_data.asset_request.assetType.clone(),
                creationContext: ResponseCreationContext {
                    creator: ResponseCreator {
                        userId: op_data
                            .asset_request
                            .creationContext
                            .creator
                            .userId
                            .to_string(),
                    },
                },
                moderationResult: ModerationResult {
                    moderationState: "Approved".to_string(),
                },
                state: "Active".to_string(),
            }),
        };

        (StatusCode::OK, Json(response)).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

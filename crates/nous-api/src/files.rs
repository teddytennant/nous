use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use nous_files::{FileManifest, StoreStats};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileManifest>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct FileListQuery {
    pub owner: String,
}

pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FileListQuery>,
) -> Result<Json<FileListResponse>, ApiError> {
    let store = state.file_store.read().await;
    let files: Vec<FileManifest> = store.list_files(&query.owner).into_iter().cloned().collect();
    let count = files.len();
    Ok(Json(FileListResponse { files, count }))
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub name: String,
    pub mime_type: String,
    pub owner: String,
    pub data_base64: String,
}

pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadRequest>,
) -> Result<Json<FileManifest>, ApiError> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request("name cannot be empty"));
    }
    if req.owner.is_empty() {
        return Err(ApiError::bad_request("owner cannot be empty"));
    }

    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD
        .decode(&req.data_base64)
        .map_err(|e| ApiError::bad_request(format!("invalid base64: {e}")))?;

    let mut store = state.file_store.write().await;
    let manifest = store
        .put(&req.name, &req.mime_type, &data, &req.owner)
        .map_err(ApiError::from)?;

    Ok(Json(manifest))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub manifest: FileManifest,
    pub data_base64: String,
    pub size: u64,
}

pub async fn get_file(
    State(state): State<Arc<AppState>>,
    Path(manifest_id): Path<String>,
) -> Result<Json<FileContentResponse>, ApiError> {
    let store = state.file_store.read().await;
    let manifest = store
        .get_manifest(&manifest_id)
        .ok_or_else(|| ApiError::not_found(format!("manifest {manifest_id} not found")))?
        .clone();
    let data = store.get(&manifest_id).map_err(ApiError::from)?;
    drop(store);

    use base64::Engine;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(&data);

    Ok(Json(FileContentResponse {
        size: manifest.total_size,
        manifest,
        data_base64,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    pub name: String,
    pub owner: String,
}

pub async fn get_latest(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatestQuery>,
) -> Result<Json<FileContentResponse>, ApiError> {
    let store = state.file_store.read().await;
    let history = store
        .get_history(&query.name, &query.owner)
        .ok_or_else(|| ApiError::not_found(format!("file '{}' not found", query.name)))?;
    let manifest = history.current.clone();
    let data = store.get_by_manifest(&manifest).map_err(ApiError::from)?;
    drop(store);

    use base64::Engine;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(&data);

    Ok(Json(FileContentResponse {
        size: manifest.total_size,
        manifest,
        data_base64,
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionHistoryResponse {
    pub name: String,
    pub owner: String,
    pub version_count: usize,
    pub current_version: u32,
    pub versions: Vec<FileManifest>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub name: String,
    pub owner: String,
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<VersionHistoryResponse>, ApiError> {
    let store = state.file_store.read().await;
    let history = store
        .get_history(&query.name, &query.owner)
        .ok_or_else(|| ApiError::not_found(format!("file '{}' not found", query.name)))?;

    let mut versions = history.history.clone();
    versions.push(history.current.clone());

    Ok(Json(VersionHistoryResponse {
        name: query.name,
        owner: query.owner,
        version_count: history.version_count(),
        current_version: history.current.version,
        versions,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub name: String,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    pub name: String,
    pub freed_bytes: u64,
}

pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let mut store = state.file_store.write().await;
    let freed = store
        .delete(&query.name, &query.owner)
        .map_err(ApiError::from)?;

    Ok(Json(DeleteResponse {
        deleted: true,
        name: query.name,
        freed_bytes: freed,
    }))
}

pub async fn store_stats(
    State(state): State<Arc<AppState>>,
) -> Json<StoreStats> {
    let store = state.file_store.read().await;
    Json(store.stats())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        router(ApiConfig::default())
    }

    fn upload_body(name: &str, data: &[u8], owner: &str) -> Vec<u8> {
        use base64::Engine;
        serde_json::to_vec(&serde_json::json!({
            "name": name,
            "mime_type": "text/plain",
            "owner": owner,
            "data_base64": base64::engine::general_purpose::STANDARD.encode(data)
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn upload_and_get_file() {
        let app = test_app().await;
        let body = upload_body("test.txt", b"hello nous files", "did:key:zOwner");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let manifest: FileManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest.name, "test.txt");

        // Get by manifest ID
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/files/{}", manifest.id.0))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let content: FileContentResponse = serde_json::from_slice(&bytes).unwrap();

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&content.data_base64)
            .unwrap();
        assert_eq!(decoded, b"hello nous files");
    }

    #[tokio::test]
    async fn list_files_for_owner() {
        let app = test_app().await;

        let body = upload_body("a.txt", b"aaa", "did:key:zOwner");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = upload_body("b.txt", b"bbb", "did:key:zOwner");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files?owner=did:key:zOwner")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: FileListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 2);
    }

    #[tokio::test]
    async fn get_latest_version() {
        let app = test_app().await;

        let body = upload_body("doc.md", b"version 1", "did:key:zOwner");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = upload_body("doc.md", b"version 2", "did:key:zOwner");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files/latest?name=doc.md&owner=did:key:zOwner")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let content: FileContentResponse = serde_json::from_slice(&bytes).unwrap();

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&content.data_base64)
            .unwrap();
        assert_eq!(decoded, b"version 2");
    }

    #[tokio::test]
    async fn file_version_history() {
        let app = test_app().await;

        for i in 1..=3 {
            let body = upload_body("versioned.txt", format!("v{i}").as_bytes(), "did:key:zOwner");
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/files")
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files/history?name=versioned.txt&owner=did:key:zOwner")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let history: VersionHistoryResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(history.version_count, 3);
        assert_eq!(history.current_version, 3);
    }

    #[tokio::test]
    async fn delete_file_endpoint() {
        let app = test_app().await;

        let body = upload_body("temp.txt", b"temporary", "did:key:zOwner");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/files?name=temp.txt&owner=did:key:zOwner")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let del: DeleteResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(del.deleted);
        assert!(del.freed_bytes > 0);

        // Verify store is empty
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let stats: StoreStats = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(stats.total_files, 0);
    }

    #[tokio::test]
    async fn store_stats_endpoint() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let stats: StoreStats = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.stored_bytes, 0);
    }

    #[tokio::test]
    async fn upload_rejects_empty_name() {
        let app = test_app().await;
        let body = upload_body("", b"data", "did:key:zOwner");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_nonexistent_file() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/files/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

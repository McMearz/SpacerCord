use std::sync::Arc;
use axum::Json;
use axum::extract::{Path, State, Query};
use serde::Deserialize;

use crate::error::ApiError;
use crate::response::{ApiResponse, ok, mutation_ok, MutationResult};
use crate::state::ApiState;

#[derive(Debug, Deserialize)]
pub struct TableQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SqlRequest {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct ReducerRequest {
    pub args: serde_json::Value,
}

pub async fn get_status(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        let status = runtime.status_json().await;
        return Ok(ok(status));
    }
    
    Ok(ok(serde_json::json!({ "enabled": false })))
}

pub async fn restart(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<ApiResponse<MutationResult>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        runtime.clone().restart().await.map_err(|e| ApiError::Internal(format!("Restart failed: {e}")))?;
        return Ok(mutation_ok("SpacetimeDB restart initiated"));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB is not enabled".to_string()))
}

pub async fn publish(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<ApiResponse<MutationResult>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        runtime.clone().publish_now().await.map_err(|e| ApiError::Internal(format!("Publish failed: {e}")))?;
        return Ok(mutation_ok("SpacetimeDB publish initiated"));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB is not enabled".to_string()))
}

// Proxying to SpacetimeDB HTTP API
pub async fn get_schema(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        let status = runtime.status_json().await;
        let uri = status["uri"].as_str().ok_or_else(|| ApiError::Internal("No URI".into()))?;
        let db = status["db_name"].as_str().ok_or_else(|| ApiError::Internal("No DB name".into()))?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/v1/database/{}/schema", uri, db);
        
        let res = client.get(url).send().await
            .map_err(|e| ApiError::Internal(format!("Failed to fetch schema: {e}")))?;
            
        let json = res.json::<serde_json::Value>().await
            .map_err(|e| ApiError::Internal(format!("Failed to parse schema: {e}")))?;
            
        return Ok(ok(json));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB not enabled".into()))
}

pub async fn get_table_rows(
    State(state): State<Arc<ApiState>>,
    Path(table_name): Path<String>,
    Query(query): Query<TableQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        let status = runtime.status_json().await;
        let uri = status["uri"].as_str().ok_or_else(|| ApiError::Internal("No URI".into()))?;
        let db = status["db_name"].as_str().ok_or_else(|| ApiError::Internal("No DB name".into()))?;
        
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let sql = format!("SELECT * FROM {} LIMIT {} OFFSET {}", table_name, limit, offset);
        
        let client = reqwest::Client::new();
        let url = format!("{}/v1/database/{}/sql", uri, db);
        
        let res = client.post(url).body(sql).send().await
            .map_err(|e| ApiError::Internal(format!("SQL failed: {e}")))?;
            
        let json = res.json::<serde_json::Value>().await
            .map_err(|e| ApiError::Internal(format!("Failed to parse results: {e}")))?;
            
        return Ok(ok(json));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB not enabled".into()))
}

pub async fn execute_sql(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<SqlRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        let status = runtime.status_json().await;
        let uri = status["uri"].as_str().ok_or_else(|| ApiError::Internal("No URI".into()))?;
        let db = status["db_name"].as_str().ok_or_else(|| ApiError::Internal("No DB name".into()))?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/v1/database/{}/sql", uri, db);
        
        let res = client.post(url).body(req.query).send().await
            .map_err(|e| ApiError::Internal(format!("SQL failed: {e}")))?;
            
        let json = res.json::<serde_json::Value>().await
            .map_err(|e| ApiError::Internal(format!("Failed to parse results: {e}")))?;
            
        return Ok(ok(json));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB not enabled".into()))
}

pub async fn call_reducer(
    State(state): State<Arc<ApiState>>,
    Path(reducer_name): Path<String>,
    Json(req): Json<ReducerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    #[cfg(feature = "spacetimedb")]
    if let Some(ref runtime) = state.spacetimedb_runtime {
        let status = runtime.status_json().await;
        let uri = status["uri"].as_str().ok_or_else(|| ApiError::Internal("No URI".into()))?;
        let db = status["db_name"].as_str().ok_or_else(|| ApiError::Internal("No DB name".into()))?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/v1/database/{}/call/{}", uri, db, reducer_name);
        
        let res = client.post(url).json(&req.args).send().await
            .map_err(|e| ApiError::Internal(format!("Reducer call failed: {e}")))?;
            
        let json = res.json::<serde_json::Value>().await
            .unwrap_or(serde_json::Value::Null);
            
        return Ok(ok(json));
    }
    
    Err(ApiError::BadRequest("SpacetimeDB not enabled".into()))
}

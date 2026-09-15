use super::queries;
use super::scan::{self, ScanSummary};
use super::models::Album;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, State};

/// Called from the frontend to scan directories into the library.
#[tauri::command]
pub async fn scan_library(
    app_handle: AppHandle,
    pool: State<'_, SqlitePool>,
    directories: Vec<String>,
    max_depth: Option<usize>,
) -> Result<ScanSummary, String> {
    let covers_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    tokio::fs::create_dir_all(&covers_dir)
        .await
        .map_err(|e| format!("failed to create covers dir: {e}"))?;

    scan::scan_directories(&pool, &covers_dir, &directories, max_depth).await
}

/// Fetch one album with its tracks and genres by id.
#[tauri::command]
pub async fn get_album(
    pool: State<'_, SqlitePool>,
    album_id: String,
) -> Result<Option<Album>, String> {
    queries::get_album_with_tracks(&pool, &album_id)
        .await
        .map_err(|e| e.to_string())
}

/// Fetch the whole library
#[tauri::command]
pub async fn get_albums(pool: State<'_, SqlitePool>) -> Result<Vec<Album>, String> {
    queries::list_albums(&pool)
        .await
        .map_err(|e| e.to_string())
}

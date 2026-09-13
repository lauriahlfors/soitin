use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use tauri::{AppHandle, Manager};

/// Initializes the local database and returns a global database pool connection or error message
pub async fn initialize_database(app_handle: &AppHandle) -> Result<SqlitePool, String> {
    // Get the path for this app's local data directory
    // Linux:   "~/.local/share/com.soitin.app"
    // MacOs:   "~/Library/Application Support/com.soitin.app"
    // Windows: "%APPDATA%\com.soitin.app"
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .inspect_err(|e| println!("[soitin-db]: Failed to resolve application data directory path: {e}"))
        .map_err(|e| e.to_string())?;
    println!(
        "[soitin-db]: Application data directory path found at: {}",
        app_dir.display()
    );

    tokio::fs::create_dir_all(&app_dir)
        .await
        .inspect_err(|e| {
            println!(
                "[soitin-db]: Failed to create directory {}: {e}",
                app_dir.display()
            )
        })
        .map_err(|e| e.to_string())?;

    // Construct path for the database file
    let db_path = app_dir.join("library.db");
    println!("[soitin-db]: Database path: {}", db_path.display());

    // Define connect options for db pool
    let connect_options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal) // pairs well with WAL
        .foreign_keys(true); // required for ON DELETE CASCADE

    // Create a new pool
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(connect_options)
        .await
        .map_err(|e| {
            format!(
                "[soitin-db]: Failed to open database at {}: {e}",
                db_path.display()
            )
        })?;

    // Initialize defined tables for the database
    initialize_tables(&pool)
        .await
        .inspect_err(|e| println!("[soitin-db]: Failed to create tables: {e}"))?;

    println!("[soitin-db]: Database pool connected successfully");
    Ok(pool)
}

/// Initialaized defined tables inside the database
pub async fn initialize_tables(pool: &SqlitePool) -> sqlx::Result<(), String> {
    let mut transaction_pool = pool.begin().await.map_err(|e| e.to_string())?;

    // Album table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS albums (
            id              TEXT PRIMARY KEY,
            title           TEXT NOT NULL,
            artist          TEXT NOT NULL,
            year            INTEGER NOT NULL,
            cover           TEXT,
            duration        INTEGER NOT NULL,
            sample_rate     INTEGER,
            bits_per_sample INTEGER,
            created_at      INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(&mut *transaction_pool)
    .await
    .map_err(|e| format!("[soitin-db]: Failed to create albums table: {}", e))?;

    // Tracks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tracks (
            id          TEXT PRIMARY KEY,
            album_id    TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
            title       TEXT NOT NULL,
            artist      TEXT NOT NULL,
            track_index INTEGER,
            duration    INTEGER NOT NULL,
            file_path   TEXT NOT NULL UNIQUE,
            created_at  INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(&mut *transaction_pool)
    .await
    .map_err(|e| format!("[soitin-db]: Failed to create tracks table: {}", e))?;

    transaction_pool.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// pub async fn get_album_with_tracks

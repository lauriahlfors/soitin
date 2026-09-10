use sqlx::sqlite::SqlitePool;

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
    .map_err(|e| format!("Soitin DB: Failed to create albums table: {}", e))?;

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
    .map_err(|e| format!("Soitin DB: Failed to create tracks table: {}", e))?;

    transaction_pool.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// pub async fn get_album_with_tracks


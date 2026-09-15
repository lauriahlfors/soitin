use super::models::{Album, AlbumRow, Track, TrackRow};
use sqlx::sqlite::SqliteConnection;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

// ════════════════════════════════════════════════════════════════
// DB writes — one inner impl per operation, shared by both the
// single-call (pool) and bulk-scan (transaction) call sites, so the
// SQL only lives in one place. Both `PoolConnection` and
// `Transaction` deref to `SqliteConnection`, which is what makes
// this sharing possible.
// ════════════════════════════════════════════════════════════════

async fn upsert_album_inner(conn: &mut SqliteConnection, album: &Album) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO albums (id, title, artist, year, cover, duration, sample_rate, bits_per_sample)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            title           = excluded.title,
            artist          = excluded.artist,
            year            = excluded.year,
            cover           = COALESCE(excluded.cover, albums.cover),
            duration        = excluded.duration,
            sample_rate     = COALESCE(excluded.sample_rate, albums.sample_rate),
            bits_per_sample = COALESCE(excluded.bits_per_sample, albums.bits_per_sample)
        "#,
    )
    .bind(&album.id)
    .bind(&album.title)
    .bind(&album.artist)
    .bind(album.year as i64)
    .bind(&album.cover)
    .bind(album.duration as i64)
    .bind(album.sample_rate.map(|v| v as i64))
    .bind(album.bits_per_sample.map(|v| v as i64))
    .execute(&mut *conn)
    .await?;

    sqlx::query("DELETE FROM album_genres WHERE album_id = ?")
        .bind(&album.id)
        .execute(&mut *conn)
        .await?;

    for genre in &album.genres {
        sqlx::query("INSERT OR IGNORE INTO album_genres (album_id, genre) VALUES (?, ?)")
            .bind(&album.id)
            .bind(genre)
            .execute(&mut *conn)
            .await?;
    }

    Ok(())
}

async fn insert_track_inner(
    conn: &mut SqliteConnection,
    album_id: &str,
    track: &Track,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO tracks (id, album_id, title, artist, track_index, duration, file_path)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(file_path) DO NOTHING
        "#,
    )
    .bind(&track.id)
    .bind(album_id)
    .bind(&track.title)
    .bind(&track.artist)
    .bind(track.index.map(|v| v as i64))
    .bind(track.duration as i64)
    .bind(&track.file_path)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Ad hoc single-album upsert, e.g. from a future "edit album" command.
/// Opens and commits its own transaction.
pub async fn upsert_album(pool: &SqlitePool, album: &Album) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    upsert_album_inner(&mut tx, album).await?;
    tx.commit().await
}

/// Ad hoc single-track insert. Uses the pool directly — a single
/// statement doesn't need its own transaction.
pub async fn insert_track(pool: &SqlitePool, album_id: &str, track: &Track) -> sqlx::Result<()> {
    insert_track_inner(&mut *pool.acquire().await?, album_id, track).await
}

/// Bulk-scan variant: writes into a transaction the caller already
/// opened, so an entire scan batch commits (or rolls back) as one unit
/// instead of one fsync per album/track. Crate-visible only — used by
/// `scan.rs`, not part of the public API of this module.
pub(crate) async fn upsert_album_tx(
    tx: &mut Transaction<'_, Sqlite>,
    album: &Album,
) -> sqlx::Result<()> {
    upsert_album_inner(tx, album).await
}

pub(crate) async fn insert_track_tx(
    tx: &mut Transaction<'_, Sqlite>,
    album_id: &str,
    track: &Track,
) -> sqlx::Result<()> {
    insert_track_inner(tx, album_id, track).await
}

//
// Database reads
//
pub async fn get_album_with_tracks(
    pool: &SqlitePool,
    album_id: &str,
) -> sqlx::Result<Option<Album>> {
    let Some(row) = sqlx::query_as::<_, AlbumRow>("SELECT * FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };

    let genres: Vec<String> =
        sqlx::query_scalar("SELECT genre FROM album_genres WHERE album_id = ? ORDER BY genre")
            .bind(album_id)
            .fetch_all(pool)
            .await?;

    let track_rows: Vec<TrackRow> =
        sqlx::query_as("SELECT * FROM tracks WHERE album_id = ? ORDER BY track_index")
            .bind(album_id)
            .fetch_all(pool)
            .await?;

    let album_title = row.title.clone();
    let tracks = track_rows
        .into_iter()
        .map(|t| t.into_track(album_title.clone()))
        .collect();

    Ok(Some(row.into_album(genres, tracks)))
}

/// List users every album with its tracks and genres attached.
pub async fn list_albums(pool: &SqlitePool) -> sqlx::Result<Vec<Album>> {
    let album_rows: Vec<AlbumRow> = sqlx::query_as("SELECT * FROM albums ORDER BY artist, title")
        .fetch_all(pool)
        .await?;

    if album_rows.is_empty() {
        return Ok(Vec::new());
    }

    let genre_rows: Vec<(String, String)> =
        sqlx::query_as("SELECT album_id, genre FROM album_genres ORDER BY genre")
            .fetch_all(pool)
            .await?;
    let mut genres_by_album: HashMap<String, Vec<String>> = HashMap::new();
    for (album_id, genre) in genre_rows {
        genres_by_album.entry(album_id).or_default().push(genre);
    }

    let track_rows: Vec<TrackRow> = sqlx::query_as("SELECT * FROM tracks ORDER BY track_index")
        .fetch_all(pool)
        .await?;
    let mut tracks_by_album: HashMap<String, Vec<TrackRow>> = HashMap::new();
    for t in track_rows {
        tracks_by_album
            .entry(t.album_id.clone())
            .or_default()
            .push(t);
    }

    let albums = album_rows
        .into_iter()
        .map(|row| {
            let genres = genres_by_album.remove(&row.id).unwrap_or_default();
            let tracks = tracks_by_album
                .remove(&row.id)
                .unwrap_or_default()
                .into_iter()
                .map(|t| t.into_track(row.title.clone()))
                .collect();
            row.into_album(genres, tracks)
        })
        .collect();

    Ok(albums)
}

pub async fn delete_album(pool: &SqlitePool, album_id: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM albums WHERE id = ?")
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

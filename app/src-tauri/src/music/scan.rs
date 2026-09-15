use super::queries::{insert_track_tx, upsert_album_tx};
use super::models::Album;
use super::tags::{is_audio_file, parse_file};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(crate) const DEFAULT_MAX_DEPTH: usize = 4;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub albums_found: usize,
    pub tracks_found: usize,
    pub errors: Vec<String>,
}

/// Walks each directory, parses every audio file's tags, groups tracks
/// into albums, and writes everything to the database in one transaction.
///
/// Deliberately takes plain `&SqlitePool`/`&Path` rather than Tauri's
/// `State`/`AppHandle` — keeps this function testable and reusable (e.g.
/// from a background task or a future CLI) without needing a running
/// Tauri app around it. `commands.rs` is the only place that talks to
/// Tauri's IPC types; this is where the actual work happens.
pub(crate) async fn scan_directories(
    pool: &SqlitePool,
    covers_dir: &Path,
    directories: &[String],
    max_depth: Option<usize>,
) -> Result<ScanSummary, String> {
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);

    let mut errors = Vec::new();
    let mut files = Vec::new();

    for dir in directories {
        let root = PathBuf::from(dir);
        if !root.exists() || !root.is_dir() {
            errors.push(format!("Not a valid directory: {dir}"));
            continue;
        }
        for entry in WalkDir::new(&root).max_depth(depth).follow_links(false) {
            match entry {
                Ok(entry) if entry.file_type().is_file() && is_audio_file(entry.path()) => {
                    files.push(entry.path().to_path_buf());
                }
                Ok(_) => {} // directories, non-audio files — skip silently
                Err(e) => errors.push(format!("walk error: {e}")),
            }
        }
    }

    // lofty does blocking file I/O — run the whole parse batch on a
    // blocking-pool thread so it doesn't stall the async runtime.
    let covers_dir_for_task = covers_dir.to_path_buf();
    let (parsed, mut parse_errors) = tokio::task::spawn_blocking(move || {
        let mut parsed = Vec::new();
        let mut errs = Vec::new();
        for path in files {
            match parse_file(&path, &covers_dir_for_task) {
                Ok(p) => parsed.push(p),
                Err(e) => errs.push(e),
            }
        }
        (parsed, errs)
    })
    .await
    .map_err(|e| format!("scan task panicked: {e}"))?;

    errors.append(&mut parse_errors);

    // Group parsed tracks into albums, merging album-level fields from
    // whichever track supplies them first.
    let mut albums: HashMap<String, Album> = HashMap::new();
    for p in parsed {
        let album = albums.entry(p.album_id.clone()).or_insert_with(|| Album {
            id: p.album_id.clone(),
            title: p.album_title.clone(),
            artist: p.album_artist.clone(),
            year: p.year,
            cover: None,
            duration: 0,
            genres: Vec::new(),
            tracks: Vec::new(),
            sample_rate: None,
            bits_per_sample: None,
        });

        album.duration += p.track.duration;
        if album.cover.is_none() {
            album.cover = p.cover.clone();
        }
        if album.sample_rate.is_none() {
            album.sample_rate = p.sample_rate;
        }
        if album.bits_per_sample.is_none() {
            album.bits_per_sample = p.bits_per_sample;
        }
        if let Some(genre) = &p.genre {
            if !album.genres.contains(genre) {
                album.genres.push(genre.clone());
            }
        }
        album.tracks.push(p.track);
    }

    let albums_found = albums.len();
    let tracks_found: usize = albums.values().map(|a| a.tracks.len()).sum();

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for album in albums.values() {
        if let Err(e) = upsert_album_tx(&mut tx, album).await {
            errors.push(format!("failed to upsert album {}: {e}", album.id));
            continue; // skip this album's tracks if the album row itself failed
        }
        for track in &album.tracks {
            if let Err(e) = insert_track_tx(&mut tx, &album.id, track).await {
                errors.push(format!("failed to insert track {}: {e}", track.file_path));
            }
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(ScanSummary {
        albums_found,
        tracks_found,
        errors,
    })
}

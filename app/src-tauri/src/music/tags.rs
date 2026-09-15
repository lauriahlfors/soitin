use super::models::Track;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::MimeType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, Tag};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Define audio extensions to look for
const AUDIO_EXTENSIONS: [&str; 8] = ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "wma"];

/// Check if given path is an audio file based on its extension
pub(crate) fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            // Can ignore case here because the extensions are all lowercase
            // and the comparison itself is case-insensitive
            return AUDIO_EXTENSIONS
                .iter()
                .any(|a| a.eq_ignore_ascii_case(ext_str));
        }
    }
    false
}

/// Deterministic album ID from artist + title, so tracks from the same
/// album always resolve to the same album row.
pub(crate) fn derive_album_id(artist: &str, title: &str) -> String {
    let key = format!(
        "{}\u{1}{}",
        artist.trim().to_lowercase(),
        title.trim().to_lowercase()
    );
    let hash = Sha256::digest(key.as_bytes());
    hex::encode(&hash[..8])
}

/// Extracts album cover from a track's metadata.
fn extract_cover(tag: &Tag, album_id: &str, covers_directory: &Path) -> Option<String> {
    let picture = tag.pictures().first()?;
    let extension = match picture.mime_type() {
        Some(MimeType::Png) => "png",
        _ => "jpg",
    };
    let safe_id = album_id.replace(|c: char| !c.is_alphanumeric() && c != '-', "_");
    let dest_path = covers_directory.join(format!("{}.{}", safe_id, extension));

    if !dest_path.exists() {
        fs::write(&dest_path, picture.data()).ok()?;
    }

    Some(format!(
        "asset://localhost/{}",
        dest_path.to_string_lossy().replace('\\', "/")
    ))
}

pub(crate) struct ParsedTrack {
    pub(crate) album_id: String,
    pub(crate) album_title: String,
    pub(crate) album_artist: String,
    pub(crate) year: i32,
    pub(crate) genre: Option<String>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) bits_per_sample: Option<u8>,
    pub(crate) cover: Option<String>,
    pub(crate) track: Track,
}

/// Reads tags and audio properties from file. Uses a fallback value if a
/// specific tag is not found.
pub(crate) fn parse_file(path: &Path, covers_dir: &Path) -> Result<ParsedTrack, String> {
    let tagged_file = Probe::open(path)
        .map_err(|e| format!("probe error for {:?}: {e}", path))?
        .read()
        .map_err(|e| format!("read error for {:?}: {e}", path))?;

    let properties = tagged_file.properties();
    let duration = properties.duration().as_secs() as u32;
    let sample_rate = properties.sample_rate();
    let bits_per_sample = properties.bit_depth();

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown Title".to_string());

    let (title, artist, album_title, year, genre, track_no, cover) = match tag {
        Some(tag) => {
            let artist = tag
                .artist()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown Artist".to_string());
            let album_title = tag
                .album()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown Album".to_string());
            let title = tag
                .title()
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_stem.clone());
            let year = tag.year().unwrap_or(0) as i32;
            let genre = tag.genre().map(|s| s.to_string());
            let track_no = tag.track();

            let album_id = derive_album_id(&artist, &album_title);
            let cover = extract_cover(tag, &album_id, covers_dir);

            (title, artist, album_title, year, genre, track_no, cover)
        }
        None => (
            file_stem,
            "Unknown Artist".to_string(),
            "Unknown Album".to_string(),
            0,
            None,
            None,
            None,
        ),
    };

    let album_id = derive_album_id(&artist, &album_title);

    let track = Track {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        artist: artist.clone(),
        album: album_title.clone(),
        index: track_no,
        duration,
        file_path: path.to_string_lossy().to_string(),
    };

    Ok(ParsedTrack {
        album_id,
        album_title,
        album_artist: artist,
        year,
        genre,
        sample_rate,
        bits_per_sample,
        cover,
        track,
    })
}

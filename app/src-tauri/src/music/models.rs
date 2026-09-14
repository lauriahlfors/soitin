use serde::{Deserialize, Serialize};
use sqlx::FromRow;

//
// Structs fow wanted data model results, used for input output
// functionalities with frontend.
//
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub index: Option<u32>,
    pub duration: u32,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: i32,
    pub cover: Option<String>,
    pub duration: u32,
    pub genres: Vec<String>,
    pub tracks: Vec<Track>,
    pub sample_rate: Option<u32>,
    pub bits_per_sample: Option<u8>,
}

//
// Structs to match the base structs but converted into to "row models" to be
// used in SQLite database.
//
#[derive(FromRow)]
pub(crate) struct AlbumRow {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) year: i64,
    pub(crate) cover: Option<String>,
    pub(crate) duration: i64,
    pub(crate) sample_rate: Option<i64>,
    pub(crate) bits_per_sample: Option<i64>,
}

impl AlbumRow {
    pub(crate) fn into_album(self, genres: Vec<String>, tracks: Vec<Track>) -> Album {
        Album {
            id: self.id,
            title: self.title,
            artist: self.artist,
            year: self.year as i32,
            cover: self.cover,
            duration: self.duration as u32,
            genres,
            tracks,
            sample_rate: self.sample_rate.map(|v| v as u32),
            bits_per_sample: self.bits_per_sample.map(|v| v as u8),
        }
    }
}

#[derive(FromRow)]
pub(crate) struct TrackRow {
    pub(crate) id: String,
    pub(crate) album_id: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) track_index: Option<i64>,
    pub(crate) duration: i64,
    pub(crate) file_path: String,
}

impl TrackRow {
    pub(crate) fn into_track(self, album_title: String) -> Track {
        Track {
            id: self.id,
            title: self.title,
            artist: self.artist,
            album: album_title,
            index: self.track_index.map(|v| v as u32),
            duration: self.duration as u32,
            file_path: self.file_path,
        }
    }
}

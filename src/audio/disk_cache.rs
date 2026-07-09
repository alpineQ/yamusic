use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use yandex_music::model::track::Track;

const KNOWN_EXTS: &[&str] = &["mp3", "aac", "m4a", "flac", "opus", "ogg", "wav"];

#[derive(Serialize, Deserialize)]
pub struct TrackMeta {
    pub id: String,
    pub title: Option<String>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub cover_uri: Option<String>,
    pub duration_ms: Option<u64>,
}

fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME")).map(|d| d.cache_dir().to_path_buf())
}

fn valid_id(track_id: &str) -> bool {
    !track_id.is_empty() && !track_id.contains(['/', '\\'])
}

pub fn cached_path(track_id: &str) -> Option<(PathBuf, String)> {
    if !valid_id(track_id) {
        return None;
    }
    let dir = cache_dir()?;
    for ext in KNOWN_EXTS {
        let path = dir.join(format!("{track_id}.{ext}"));
        if path.is_file() {
            return Some((path, (*ext).to_string()));
        }
    }
    None
}

pub fn is_cached(track_id: &str) -> bool {
    cached_path(track_id).is_some()
}

fn meta_of(track: &Track) -> TrackMeta {
    TrackMeta {
        id: track.id.clone(),
        title: track.title.clone(),
        artists: track.artists.iter().filter_map(|a| a.name.clone()).collect(),
        album: track.albums.first().and_then(|a| a.title.clone()),
        cover_uri: track
            .cover_uri
            .clone()
            .or_else(|| track.albums.first().and_then(|a| a.cover_uri.clone())),
        duration_ms: track.duration.map(|d| d.as_millis() as u64),
    }
}

pub fn write(track: &Track, codec: &str, bytes: &[u8]) {
    if !valid_id(&track.id) || bytes.is_empty() {
        return;
    }
    let ext = if codec.is_empty() { "bin" } else { codec };
    let Some(dir) = cache_dir() else { return };
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::warn!("disk cache: mkdir failed: {e}");
        return;
    }

    let audio = dir.join(format!("{}.{ext}", track.id));
    let audio_tmp = dir.join(format!("{}.{ext}.tmp", track.id));
    if let Err(e) = fs::write(&audio_tmp, bytes).and_then(|_| fs::rename(&audio_tmp, &audio)) {
        tracing::warn!("disk cache: audio write failed: {e}");
        let _ = fs::remove_file(&audio_tmp);
        return;
    }

    if let Ok(json) = serde_json::to_vec(&meta_of(track)) {
        let meta = dir.join(format!("{}.meta.json", track.id));
        let meta_tmp = dir.join(format!("{}.meta.json.tmp", track.id));
        if fs::write(&meta_tmp, &json)
            .and_then(|_| fs::rename(&meta_tmp, &meta))
            .is_err()
        {
            let _ = fs::remove_file(&meta_tmp);
        }
    }
}

use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Serialize;

use crate::util::lrc::parse_lrc;

#[derive(Serialize)]
struct LyricLine {
    #[serde(rename = "Timestamp")]
    timestamp: u64,
    #[serde(rename = "Line")]
    line: String,
}

fn lyrics_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME")).map(|d| d.cache_dir().join(".lyrics"))
}

pub fn write_synced(track_id: &str, lrc: &str) {
    if track_id.is_empty() || track_id.contains(['/', '\\']) {
        return;
    }

    let lines: Vec<LyricLine> = parse_lrc(lrc)
        .into_iter()
        .map(|(timestamp, line)| LyricLine { timestamp, line })
        .collect();
    if lines.is_empty() {
        return;
    }

    let Some(dir) = lyrics_dir() else { return };
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::warn!("lyrics cache: mkdir failed: {e}");
        return;
    }

    let json = match serde_json::to_vec(&lines) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("lyrics cache: serialize failed: {e}");
            return;
        }
    };

    let path = dir.join(format!("{track_id}.json"));
    let tmp = dir.join(format!("{track_id}.json.tmp"));
    if let Err(e) = fs::write(&tmp, &json).and_then(|_| fs::rename(&tmp, &path)) {
        tracing::warn!("lyrics cache: write failed: {e}");
        let _ = fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_waybar_daemon_format() {
        let lines: Vec<LyricLine> = parse_lrc("[00:01.50]hello\n[00:03.00]world")
            .into_iter()
            .map(|(timestamp, line)| LyricLine { timestamp, line })
            .collect();
        let json = serde_json::to_string(&lines).unwrap();
        assert_eq!(
            json,
            r#"[{"Timestamp":1500,"Line":"hello"},{"Timestamp":3000,"Line":"world"}]"#
        );
    }
}

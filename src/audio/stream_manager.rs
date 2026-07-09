use crate::audio::cache::UrlCache;
use crate::audio::disk_cache;
use crate::audio::progress::TrackProgress;
use crate::http::ApiService;
use crate::stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use yandex_music::model::track::Track;

pub struct StreamManager {
    api: Arc<ApiService>,
    url_cache: UrlCache,
    prewarm_cache: Arc<Mutex<HashMap<String, (stream::StreamingSession, Arc<TrackProgress>)>>>,
    http_client: reqwest::blocking::Client,
}

impl StreamManager {
    pub fn new(api: Arc<ApiService>, url_cache: UrlCache) -> Self {
        let http_client = reqwest::blocking::Client::builder()
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to create streaming http client");

        Self {
            api,
            url_cache,
            prewarm_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client,
        }
    }

    pub fn prewarm(&self, track: Track) {
        let id = track.id.clone();
        if self.prewarm_cache.lock().unwrap().contains_key(&id) {
            return;
        }

        let this = self.clone();
        tokio::spawn(async move {
            if let Ok(result) = this.create_stream_session(&track).await {
                this.prewarm_cache.lock().unwrap().insert(track.id, result);
            }
        });
    }

    pub async fn create_stream_session(
        &self,
        track: &Track,
    ) -> color_eyre::Result<(stream::StreamingSession, Arc<TrackProgress>)> {
        if let Some((path, codec)) = disk_cache::cached_path(&track.id) {
            let progress = Arc::new(TrackProgress::new());
            let progress_clone = progress.clone();
            let session = tokio::task::spawn_blocking(move || {
                stream::create_file_session(path, codec, progress_clone)
            })
            .await??;
            return Ok((session, progress));
        }

        {
            let mut cache = self.prewarm_cache.lock().unwrap();
            if let Some((session, progress)) = cache.remove(&track.id) {
                return Ok((session, progress));
            }
        }

        let _start = std::time::Instant::now();
        let (url, codec, bitrate) =
            if let Some((url, codec, bitrate)) = self.url_cache.get(&track.id) {
                (url, codec, bitrate)
            } else {
                let (url, codec, bitrate) = self.api.fetch_track_url(track.id.clone()).await?;
                self.url_cache
                    .insert(track.id.clone(), url.clone(), codec.clone(), bitrate);
                (url, codec, bitrate)
            };

        let progress = Arc::new(TrackProgress::new());
        progress.set_bitrate(bitrate.into());

        let progress_clone = progress.clone();

        let client = self.http_client.clone();
        let session = tokio::task::spawn_blocking(move || {
            stream::create_streaming_session(client, url, codec, bitrate, progress_clone)
        })
        .await??;

        Ok((session, progress))
    }

    pub fn cache_to_disk(&self, track: Track) {
        if disk_cache::is_cached(&track.id) {
            return;
        }

        let this = self.clone();
        tokio::spawn(async move {
            let resolved = if let Some(entry) = this.url_cache.get(&track.id) {
                Some(entry)
            } else {
                match this.api.fetch_track_url(track.id.clone()).await {
                    Ok((url, codec, bitrate)) => {
                        this.url_cache
                            .insert(track.id.clone(), url.clone(), codec.clone(), bitrate);
                        Some((url, codec, bitrate))
                    }
                    Err(e) => {
                        tracing::warn!("disk cache: url fetch failed for {}: {e}", track.id);
                        None
                    }
                }
            };

            let Some((url, codec, _bitrate)) = resolved else {
                return;
            };

            let client = this.http_client.clone();
            let bytes = tokio::task::spawn_blocking(move || {
                client.get(&url).send().and_then(|r| r.bytes())
            })
            .await;

            match bytes {
                Ok(Ok(bytes)) => disk_cache::write(&track, &codec, &bytes),
                Ok(Err(e)) => tracing::warn!("disk cache: download failed for {}: {e}", track.id),
                Err(e) => tracing::warn!("disk cache: join failed for {}: {e}", track.id),
            }
        });
    }
}

impl Clone for StreamManager {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            url_cache: self.url_cache.clone(),
            prewarm_cache: self.prewarm_cache.clone(),
            http_client: self.http_client.clone(),
        }
    }
}

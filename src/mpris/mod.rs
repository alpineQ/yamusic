use std::thread;
use std::time::Duration;

use flume::Sender;
use mpris_server::{Metadata, PlaybackStatus, Player, Time, TrackId};
use yandex_music::model::track::Track;

use crate::audio::signals::AudioSignals;
use crate::event::events::Event;

const BUS_SUFFIX: &str = "yamusic";
const POLL: Duration = Duration::from_millis(200);
const SEEK_EPSILON_MS: i64 = 1200;

pub fn spawn(signals: AudioSignals, event_tx: Sender<Event>) {
    let _ = thread::Builder::new().name("mpris".into()).spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!("mpris: runtime init failed: {e}");
                return;
            }
        };
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async move {
            if let Err(e) = serve(signals, event_tx).await {
                tracing::warn!("mpris: server stopped: {e}");
            }
        });
    });
}

async fn serve(signals: AudioSignals, event_tx: Sender<Event>) -> mpris_server::zbus::Result<()> {
    let player = Player::builder(BUS_SUFFIX)
        .identity("Yamusic")
        .can_play(true)
        .can_pause(true)
        .can_go_next(true)
        .can_go_previous(true)
        .can_seek(true)
        .can_control(true)
        .build()
        .await?;

    connect_controls(&player, &signals, &event_tx);
    tokio::task::spawn_local(player.run());

    let mut last_status: Option<PlaybackStatus> = None;
    let mut last_track_id: Option<Option<String>> = None;
    let mut last_volume: Option<u8> = None;
    let mut last_pos_ms: i64 = 0;

    loop {
        let status = playback_status(&signals);
        if last_status != Some(status) {
            let _ = player.set_playback_status(status).await;
            last_status = Some(status);
        }

        let track_id = signals.current_track_id.get();
        if last_track_id.as_ref() != Some(&track_id) {
            let meta = signals
                .current_track
                .get()
                .map(|t| build_metadata(&t, signals.current_cover_url.get()))
                .unwrap_or_default();
            let _ = player.set_metadata(meta).await;
            last_track_id = Some(track_id);
            last_pos_ms = 0;
        }

        let volume = signals.volume.get();
        if last_volume != Some(volume) {
            let _ = player.set_volume(volume as f64 / 100.0).await;
            last_volume = Some(volume);
        }

        let pos_ms = signals.position_ms.get() as i64;
        player.set_position(Time::from_millis(pos_ms));
        if (pos_ms - last_pos_ms).abs() > SEEK_EPSILON_MS {
            let _ = player.seeked(Time::from_millis(pos_ms)).await;
        }
        last_pos_ms = pos_ms;

        tokio::time::sleep(POLL).await;
    }
}

fn connect_controls(player: &Player, signals: &AudioSignals, event_tx: &Sender<Event>) {
    let tx = event_tx.clone();
    player.connect_play(move |_| {
        let _ = tx.send(Event::Resume);
    });

    let tx = event_tx.clone();
    player.connect_pause(move |_| {
        let _ = tx.send(Event::Pause);
    });

    let tx = event_tx.clone();
    player.connect_stop(move |_| {
        let _ = tx.send(Event::Pause);
    });

    let tx = event_tx.clone();
    let is_playing = signals.is_playing.clone();
    player.connect_play_pause(move |_| {
        let ev = if is_playing.get() {
            Event::Pause
        } else {
            Event::Resume
        };
        let _ = tx.send(ev);
    });

    let tx = event_tx.clone();
    player.connect_next(move |_| {
        let _ = tx.send(Event::Next);
    });

    let tx = event_tx.clone();
    player.connect_previous(move |_| {
        let _ = tx.send(Event::Previous);
    });

    let tx = event_tx.clone();
    let position_ms = signals.position_ms.clone();
    let duration_ms = signals.duration_ms.clone();
    player.connect_seek(move |_, offset: Time| {
        let target = (position_ms.get() as i64 + offset.as_millis()).max(0);
        let total = duration_ms.get() as i64;
        let target = if total > 0 { target.min(total) } else { target };
        let _ = tx.send(Event::Seek(target as u32));
    });

    let tx = event_tx.clone();
    player.connect_set_position(move |_, _: &TrackId, pos: Time| {
        let _ = tx.send(Event::Seek(pos.as_millis().max(0) as u32));
    });

    let tx = event_tx.clone();
    player.connect_set_volume(move |_, volume: f64| {
        let v = (volume * 100.0).round().clamp(0.0, 100.0) as u8;
        let _ = tx.send(Event::Volume(v));
    });
}

fn playback_status(signals: &AudioSignals) -> PlaybackStatus {
    if signals.is_stopped.get() {
        PlaybackStatus::Stopped
    } else if signals.is_playing.get() {
        PlaybackStatus::Playing
    } else {
        PlaybackStatus::Paused
    }
}

fn build_metadata(track: &Track, cover_url: Option<String>) -> Metadata {
    let mut builder = Metadata::builder().trackid(track_id(&track.id));

    if let Some(title) = &track.title {
        builder = builder.title(title.clone());
    }

    let artists: Vec<String> = track.artists.iter().filter_map(|a| a.name.clone()).collect();
    if !artists.is_empty() {
        builder = builder.artist(artists);
    }

    if let Some(album) = track.albums.first().and_then(|a| a.title.clone()) {
        builder = builder.album(album);
    }

    if let Some(duration) = track.duration {
        builder = builder.length(Time::from_micros(duration.as_micros() as i64));
    }

    if let Some(url) = cover_url {
        builder = builder.art_url(url);
    }

    builder.build()
}

fn track_id(id: &str) -> TrackId {
    TrackId::try_from(format!("/org/mpris/MediaPlayer2/yamusic/track/{id}"))
        .unwrap_or(TrackId::NO_TRACK)
}

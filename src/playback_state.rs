// SPDX-License-Identifier: GPL-3.0

use crate::library::MediaMetaData;
use crate::playlist::Track;
use serde::{Deserialize, Serialize};

/// Consolidated playback state
#[derive(Clone)]
pub struct PlaybackState {
    pub session: Option<PlaybackSession>,
    pub status: PlaybackStatus,
    pub progress: f32,
    pub now_playing: Option<MediaMetaData>,
    pub dragging_slider: bool,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            session: None,
            status: PlaybackStatus::Stopped,
            progress: 0.0,
            now_playing: None,
            dragging_slider: false,
        }
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct PlaybackSession {
    pub playlist_id: u32,
    pub order: Vec<Track>,
    pub index: usize,
}

impl std::fmt::Debug for PlaybackSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackSession")
            .field("playlist_id", &self.playlist_id)
            .field("order", &self.order)
            .field("index", &self.index)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RepeatMode {
    One,
    All,
}

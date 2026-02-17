// SPDX-License-Identifier: GPL-3.0

use crate::mpris::MprisCommand;
use crate::playback_state::{PlaybackSession, PlaybackState, PlaybackStatus, RepeatMode};
use crate::player::Player;
use crate::playlist::Playlist;
use gst::prelude::*;
use gstreamer as gst;
use rand::seq::SliceRandom;
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;

/// Events emitted by the playback service during tick
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    TrackEnded,
    Error(String),
    PositionUpdate(f32),
}

pub struct PlaybackService {
    player: Player,
    state: PlaybackState,
    mpris_rx: UnboundedReceiver<MprisCommand>,
}

impl PlaybackService {
    pub fn new(mpris_rx: UnboundedReceiver<MprisCommand>) -> Self {
        Self {
            player: Player::new(),
            state: PlaybackState::new(),
            mpris_rx,
        }
    }

    // ===== State Access =====

    pub fn status(&self) -> PlaybackStatus {
        self.state.status
    }

    pub fn now_playing(&self) -> Option<&crate::library::MediaMetaData> {
        self.state.now_playing.as_ref()
    }

    pub fn progress(&self) -> f32 {
        self.state.progress
    }

    pub fn session(&self) -> Option<&PlaybackSession> {
        self.state.session.as_ref()
    }

    pub fn set_dragging_slider(&mut self, dragging: bool) {
        self.state.dragging_slider = dragging;
    }

    pub fn set_progress(&mut self, progress: f32) {
        self.state.progress = progress;
    }

    // ===== Playback Control =====

    pub fn play(&mut self) {
        self.player.play();
        self.state.status = PlaybackStatus::Playing;
    }

    pub fn pause(&mut self) {
        self.player.pause();
        self.state.status = PlaybackStatus::Paused;
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.state.status = PlaybackStatus::Stopped;
    }

    pub fn play_pause(&mut self) {
        match self.state.status {
            PlaybackStatus::Stopped | PlaybackStatus::Paused => self.play(),
            PlaybackStatus::Playing => self.pause(),
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.player.set_volume(volume);
    }

    pub fn seek(&mut self, time: f32) {
        if let Err(err) = self.player.playbin.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            gst::ClockTime::from_seconds(time as u64),
        ) {
            eprintln!("Failed to seek: {:?}", err);
        }
    }

    // ===== Session Management =====

    /// Start a new playback session from a playlist
    pub fn start_session(&mut self, playlist: &Playlist, index: usize, shuffle: bool) {
        let mut order = playlist.tracks().to_vec();

        let actual_index = if shuffle {
            order.shuffle(&mut rand::rng());

            // Find the clicked track in the shuffled order
            if index < playlist.tracks().len() {
                let clicked = &playlist.tracks()[index];
                order
                    .iter()
                    .position(|t| {
                        t.metadata.id.clone().unwrap_or_default()
                            == clicked.metadata.id.clone().unwrap_or_default()
                            && t.date_added == clicked.date_added
                    })
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            index
        };

        self.state.session = Some(PlaybackSession {
            playlist_id: playlist.id(),
            order,
            index: actual_index,
        });

        self.update_now_playing();
        self.load_current_track();
    }

    /// Update shuffle setting for current session
    pub fn update_session_shuffle(&mut self, playlist: &Playlist, shuffle: bool) -> bool {
        let Some(session) = &self.state.session else {
            return false;
        };

        if session.playlist_id != playlist.id() {
            return false;
        }

        let current_track_id = self.get_current_track_id();
        let mut new_order = playlist.tracks().to_vec();

        if shuffle {
            new_order.shuffle(&mut rand::rng());
        }

        let new_index = if let Some(ref id) = current_track_id {
            new_order
                .iter()
                .position(|t| {
                    t.metadata
                        .id
                        .as_ref()
                        .map_or(false, |track_id| track_id == id)
                })
                .unwrap_or(0)
        } else {
            0
        };

        self.state.session = Some(PlaybackSession {
            playlist_id: session.playlist_id,
            order: new_order,
            index: new_index,
        });

        true
    }

    /// Update session when library is modified
    pub fn update_session_for_library(&mut self, library: &Playlist) -> bool {
        let current_track_id = self.get_current_track_id();

        let Some(session) = &mut self.state.session else {
            return false;
        };

        // Only update if session is playing from library
        if session.playlist_id != library.id() {
            return false;
        }

        // Update tracks in existing order with fresh metadata
        let mut updated_order = Vec::new();

        for old_track in &session.order {
            if let Some(old_id) = &old_track.metadata.id {
                if let Some(new_track) = library
                    .tracks()
                    .iter()
                    .find(|t| t.metadata.id.as_ref() == Some(old_id))
                {
                    updated_order.push(new_track.clone());
                }
            }
        }

        // Find current track in updated order
        let new_index = if let Some(ref id) = current_track_id {
            updated_order.iter().position(|t| {
                t.metadata
                    .id
                    .as_ref()
                    .map_or(false, |track_id| track_id == id)
            })
        } else {
            None
        };

        // If currently playing track was removed, stop playback
        if new_index.is_none() && current_track_id.is_some() {
            self.stop();
            self.state.session = None;
            self.state.now_playing = None;
            return false;
        }

        session.order = updated_order;
        session.index = new_index.unwrap_or(0);

        self.update_now_playing();
        true
    }

    /// Validate and sanitize the session
    pub fn validate_session(&mut self) -> bool {
        let Some(session) = &mut self.state.session else {
            return true;
        };

        // Bounds check
        if session.index >= session.order.len() {
            session.index = session.order.len().saturating_sub(1);
        }

        // Verify metadata validity
        if let Some(track) = session.order.get(session.index) {
            if track.metadata.id.is_none() {
                // Find next track with valid ID
                session.index = session
                    .order
                    .iter()
                    .skip(session.index)
                    .position(|t| t.metadata.id.is_some())
                    .map(|pos| session.index + pos)
                    .unwrap_or(0);
            }
        }

        true
    }

    // ===== Navigation =====

    pub fn next(&mut self, repeat_mode: RepeatMode, repeat_enabled: bool) {
        let Some(session) = &mut self.state.session else {
            return;
        };

        match repeat_mode {
            RepeatMode::One => {
                // Restart current track
                self.load_current_track();
                self.play();
                return;
            }
            RepeatMode::All => {
                if session.index + 1 < session.order.len() {
                    session.index += 1;
                } else if repeat_enabled {
                    // Only wrap to beginning if repeat is enabled
                    session.index = 0;
                } else {
                    // Reached end without repeat - stop playback
                    self.stop();
                    return;
                }
            }
        }

        self.load_current_track();
        self.play();
        self.update_now_playing();
    }

    pub fn prev(&mut self, repeat_mode: RepeatMode) {
        let Some(session) = &mut self.state.session else {
            return;
        };

        match repeat_mode {
            RepeatMode::One => {
                // Restart current track
                self.load_current_track();
                self.play();
                self.update_now_playing();
                return;
            }
            RepeatMode::All => {
                if session.index > 0 {
                    session.index -= 1;
                } else {
                    session.index = session.order.len().saturating_sub(1);
                }
            }
        }

        self.load_current_track();
        self.play();
        self.update_now_playing();
    }

    /// Process one tick cycle - handles GStreamer messages and MPRIS commands
    /// Returns events that the app should handle
    pub fn tick(&mut self) -> Vec<PlaybackEvent> {
        let mut events = Vec::new();

        // Handle GStreamer messages
        if let Some(bus) = self.player.playbin.bus() {
            while let Some(msg) = bus.pop() {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Eos(..) => {
                        events.push(PlaybackEvent::TrackEnded);
                    }
                    MessageView::Error(err) => {
                        events.push(PlaybackEvent::Error(err.error().to_string()));
                    }
                    _ => (),
                }
            }
        }

        // Update position if not dragging slider
        if !self.state.dragging_slider {
            if let Some(pos) = self.player.playbin.query_position::<gst::ClockTime>() {
                self.state.progress = pos.mseconds() as f32 / 1000.0;
                events.push(PlaybackEvent::PositionUpdate(self.state.progress));
            }
        }

        events
    }

    /// Process MPRIS commands
    pub fn process_mpris_commands(&mut self) -> Vec<MprisCommand> {
        let mut commands = Vec::new();

        while let Ok(cmd) = self.mpris_rx.try_recv() {
            commands.push(cmd);
        }

        commands
    }

    // ===== Private Helpers =====

    fn load_current_track(&mut self) {
        if let Some(session) = &self.state.session {
            if let Some(track) = session.order.get(session.index) {
                if let Ok(url) = Url::from_file_path(&track.path) {
                    self.player.stop();
                    self.player.load(url.as_str());
                }
            }
        }
    }

    fn update_now_playing(&mut self) {
        if let Some(session) = &self.state.session {
            if let Some(track) = session.order.get(session.index) {
                self.state.now_playing = Some(track.metadata.clone());
            } else {
                self.state.now_playing = None;
            }
        } else {
            self.state.now_playing = None;
        }
    }

    fn get_current_track_id(&self) -> Option<String> {
        self.state
            .session
            .as_ref()
            .and_then(|s| s.order.get(s.index))
            .and_then(|t| t.metadata.id.clone())
    }
}

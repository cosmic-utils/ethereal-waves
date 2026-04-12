// SPDX-License-Identifier: GPL-3.0

use crate::app::TrackId;
use crate::config::PlaybackTransitionMode;
use crate::constants::{
    DEFAULT_CROSSFADE_DURATION_SECS, MAX_CROSSFADE_DURATION_SECS, MIN_CROSSFADE_DURATION_SECS,
};
use crate::mpris::MprisCommand;
use crate::playback_state::{PlaybackSession, PlaybackState, PlaybackStatus, RepeatMode};
use crate::player::Player;
use crate::playlist::Playlist;
use gst::prelude::*;
use gstreamer as gst;
use rand::seq::SliceRandom;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;

/// Events emitted by the playback service during tick
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    TrackEnded,
    GaplessTrackAdvanced,
    CrossfadeTrackAdvanced,
    Error(String),
    #[allow(dead_code)]
    PositionUpdate(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlayerSlot {
    Primary,
    Secondary,
}

impl PlayerSlot {
    fn other(self) -> Self {
        match self {
            Self::Primary => Self::Secondary,
            Self::Secondary => Self::Primary,
        }
    }
}

#[derive(Debug, Clone)]
struct CrossfadeState {
    fading_out_slot: PlayerSlot,
    started_at: Instant,
    duration_secs: f32,
}

pub struct PlaybackService {
    primary_player: Player,
    secondary_player: Player,
    active_slot: PlayerSlot,
    state: PlaybackState,
    mpris_rx: UnboundedReceiver<MprisCommand>,
    // Mirrors the app-level repeat mode so transition planning stays correct
    repeat_mode: RepeatMode,
    // Whether repeat is enabled at all.
    repeat_enabled: bool,
    transition_mode: PlaybackTransitionMode,
    output_volume: f64,
    // Preferred crossfade duration for future transitions
    crossfade_duration_secs: f64,
    // True between an about-to-finish notification and the subsequent STREAM_START,
    // indicating a gapless transition is in-flight
    gapless_pending: bool,
    // The track ID that was handed to GStreamer for pending gapless transition
    // Used by advance_session_after_gapless to find the right index even if the
    // session order changes
    pending_gapless_track_id: Option<TrackId>,
    crossfade: Option<CrossfadeState>,
}

impl PlaybackService {
    pub fn new(mpris_rx: UnboundedReceiver<MprisCommand>) -> Self {
        Self {
            primary_player: Player::new(),
            secondary_player: Player::new(),
            active_slot: PlayerSlot::Primary,
            state: PlaybackState::new(),
            mpris_rx,
            repeat_mode: RepeatMode::All,
            repeat_enabled: false,
            transition_mode: PlaybackTransitionMode::Gapless,
            output_volume: 1.0,
            crossfade_duration_secs: DEFAULT_CROSSFADE_DURATION_SECS as f64,
            gapless_pending: false,
            pending_gapless_track_id: None,
            crossfade: None,
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

    /// Keep the service's repeat state in sync with app state
    /// Should be called whenever the app toggles repeat or repeat mode
    pub fn set_repeat_state(&mut self, mode: RepeatMode, enabled: bool) {
        self.repeat_mode = mode;
        self.repeat_enabled = enabled;

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.queue_next_uri();
        }
    }

    pub fn set_transition_mode(&mut self, mode: PlaybackTransitionMode) {
        self.transition_mode = mode;
        self.gapless_pending = false;
        self.pending_gapless_track_id = None;
        self.collapse_to_active_player();

        match self.transition_mode {
            PlaybackTransitionMode::Gapless => self.queue_next_uri(),
            PlaybackTransitionMode::Crossfade => self.active_player().set_queued_uri(None),
        }

        self.apply_output_volume();
    }

    // ===== Playback Control =====

    pub fn set_crossfade_duration_secs(&mut self, duration_secs: i32) {
        self.crossfade_duration_secs =
            duration_secs.clamp(MIN_CROSSFADE_DURATION_SECS, MAX_CROSSFADE_DURATION_SECS) as f64;
    }

    pub fn play(&mut self) {
        self.active_player_mut().play();
        self.state.status = PlaybackStatus::Playing;
    }

    pub fn pause(&mut self) {
        self.collapse_to_active_player();
        self.active_player_mut().pause();
        self.state.status = PlaybackStatus::Paused;
    }

    pub fn stop(&mut self) {
        self.stop_all_players();
        self.active_slot = PlayerSlot::Primary;
        self.gapless_pending = false;
        self.pending_gapless_track_id = None;
        self.crossfade = None;
        self.state.status = PlaybackStatus::Stopped;
        self.state.progress = 0.0;
    }

    pub fn play_pause(&mut self) {
        match self.state.status {
            PlaybackStatus::Stopped | PlaybackStatus::Paused => self.play(),
            PlaybackStatus::Playing => self.pause(),
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.output_volume = volume.clamp(0.0, 1.0);
        self.apply_output_volume();
    }

    pub fn seek(&mut self, time: f32) {
        self.collapse_to_active_player();

        if let Err(err) = self.active_player().playbin.seek_simple(
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
                            && t.entry_id == clicked.entry_id
                    })
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            index
        };

        self.stop_all_players();
        self.active_slot = PlayerSlot::Primary;
        self.gapless_pending = false;
        self.pending_gapless_track_id = None;
        self.crossfade = None;

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
                        .is_some_and(|track_id| track_id == id)
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

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.queue_next_uri();
        }

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
                    .is_some_and(|track_id| track_id == id)
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

        let session = self.state.session.as_mut().unwrap();
        session.order = updated_order;
        session.index = new_index.unwrap_or(0);

        self.update_now_playing();

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.queue_next_uri();
        }

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
                // Find next track with valid ID.
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
        self.repeat_mode = repeat_mode.clone();
        self.repeat_enabled = repeat_enabled;
        self.collapse_to_active_player();

        let Some(session) = &mut self.state.session else {
            return;
        };

        match repeat_mode {
            RepeatMode::One => {
                self.load_current_track();
                self.play();
                return;
            }
            RepeatMode::All => {
                if session.index + 1 < session.order.len() {
                    session.index += 1;
                } else if repeat_enabled {
                    session.index = 0;
                } else {
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
        self.collapse_to_active_player();

        let Some(session) = &mut self.state.session else {
            return;
        };

        match repeat_mode {
            RepeatMode::One => {
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

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless)
            && self.active_player().take_about_to_finish()
        {
            self.gapless_pending = true;
        }

        if let Some(event) = self.maybe_start_crossfade() {
            events.push(event);
        }

        self.handle_bus_messages(PlayerSlot::Primary, &mut events);
        self.handle_bus_messages(PlayerSlot::Secondary, &mut events);

        if self.crossfade.is_some() {
            self.apply_output_volume();

            if self.crossfade_ratio() >= 1.0 {
                self.finish_crossfade();
            }
        }

        if !self.state.dragging_slider {
            if let Some(pos) = self
                .active_player()
                .playbin
                .query_position::<gst::ClockTime>()
            {
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

    fn player(&self, slot: PlayerSlot) -> &Player {
        match slot {
            PlayerSlot::Primary => &self.primary_player,
            PlayerSlot::Secondary => &self.secondary_player,
        }
    }

    fn player_mut(&mut self, slot: PlayerSlot) -> &mut Player {
        match slot {
            PlayerSlot::Primary => &mut self.primary_player,
            PlayerSlot::Secondary => &mut self.secondary_player,
        }
    }

    fn active_player(&self) -> &Player {
        self.player(self.active_slot)
    }

    fn active_player_mut(&mut self) -> &mut Player {
        self.player_mut(self.active_slot)
    }

    fn stop_slot(&mut self, slot: PlayerSlot) {
        {
            let player = self.player_mut(slot);
            player.set_queued_uri(None);
            player.stop();
            player.set_volume(0.0);
        }

        if let Some(bus) = self.player(slot).playbin.bus() {
            while bus.pop().is_some() {}
        }

        let _ = self.player(slot).take_about_to_finish();
    }

    fn stop_all_players(&mut self) {
        self.stop_slot(PlayerSlot::Primary);
        self.stop_slot(PlayerSlot::Secondary);
    }

    fn collapse_to_active_player(&mut self) {
        self.crossfade = None;
        self.stop_slot(self.active_slot.other());
        self.gapless_pending = false;
        self.pending_gapless_track_id = None;
        self.active_player().set_queued_uri(None);

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.queue_next_uri();
        }

        self.apply_output_volume();
    }

    fn crossfade_ratio(&self) -> f64 {
        self.crossfade
            .as_ref()
            .map(|crossfade| {
                (crossfade.started_at.elapsed().as_secs_f64() / crossfade.duration_secs as f64)
                    .clamp(0.0, 1.0)
            })
            .unwrap_or(1.0)
    }

    fn apply_output_volume(&mut self) {
        let output_volume = self.output_volume;

        if let Some(crossfade) = &self.crossfade {
            let fade_in_slot = self.active_slot;
            let fade_out_slot = crossfade.fading_out_slot;
            let ratio = self.crossfade_ratio();

            self.player_mut(fade_in_slot)
                .set_volume(output_volume * ratio);
            self.player_mut(fade_out_slot)
                .set_volume(output_volume * (1.0 - ratio));
        } else {
            self.active_player_mut().set_volume(output_volume);
            self.player_mut(self.active_slot.other()).set_volume(0.0);
        }
    }

    fn crossfade_duration_secs(&self, track_duration_secs: f64) -> f64 {
        self.crossfade_duration_secs
            .min((track_duration_secs / 2.0).max(0.0))
    }

    fn maybe_start_crossfade(&mut self) -> Option<PlaybackEvent> {
        if !matches!(self.transition_mode, PlaybackTransitionMode::Crossfade)
            || self.crossfade.is_some()
            || self.state.status != PlaybackStatus::Playing
        {
            return None;
        }

        let session = self.state.session.as_ref()?;
        let next_index = self.compute_next_index()?;

        if next_index == session.index {
            return None;
        }

        let duration = self
            .active_player()
            .playbin
            .query_duration::<gst::ClockTime>()?;
        let position = self
            .active_player()
            .playbin
            .query_position::<gst::ClockTime>()?;
        let duration_secs = duration.mseconds() as f32 / 1000.0;
        let position_secs = position.mseconds() as f32 / 1000.0;

        if duration_secs <= 0.0 || position_secs <= 0.0 {
            return None;
        }

        let crossfade_duration_secs = self.crossfade_duration_secs(duration_secs as f64) as f32;
        let remaining_secs = (duration_secs - position_secs).max(0.0);

        if remaining_secs > crossfade_duration_secs {
            return None;
        }

        let next_track = session.order.get(next_index)?;
        let next_uri = Url::from_file_path(&next_track.path).ok()?.to_string();
        let fading_out_slot = self.active_slot;
        let fade_in_slot = fading_out_slot.other();

        self.stop_slot(fade_in_slot);

        {
            let player = self.player_mut(fade_in_slot);
            player.load(&next_uri);
            player.set_volume(0.0);
            player.play();
        }

        self.active_slot = fade_in_slot;
        self.crossfade = Some(CrossfadeState {
            fading_out_slot,
            started_at: Instant::now(),
            duration_secs: crossfade_duration_secs,
        });

        if let Some(session) = &mut self.state.session {
            session.index = next_index;
        }

        self.update_now_playing();
        self.state.progress = 0.0;
        self.apply_output_volume();

        Some(PlaybackEvent::CrossfadeTrackAdvanced)
    }

    fn finish_crossfade(&mut self) {
        let Some(crossfade) = self.crossfade.take() else {
            return;
        };

        self.stop_slot(crossfade.fading_out_slot);
        self.apply_output_volume();
    }

    fn handle_bus_messages(&mut self, slot: PlayerSlot, events: &mut Vec<PlaybackEvent>) {
        let Some(bus) = self.player(slot).playbin.bus() else {
            return;
        };

        while let Some(msg) = bus.pop() {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => self.handle_eos(slot, events),
                MessageView::StreamStart(..) => self.handle_stream_start(slot, events),
                MessageView::Error(err) => self.handle_error(slot, err.error().to_string(), events),
                _ => (),
            }
        }
    }

    fn handle_eos(&mut self, slot: PlayerSlot, events: &mut Vec<PlaybackEvent>) {
        if self.is_fading_out_slot(slot) {
            self.finish_crossfade();
            return;
        }

        if slot != self.active_slot {
            self.stop_slot(slot);
            return;
        }

        self.gapless_pending = false;
        events.push(PlaybackEvent::TrackEnded);
    }

    fn handle_stream_start(&mut self, slot: PlayerSlot, events: &mut Vec<PlaybackEvent>) {
        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless)
            && slot == self.active_slot
            && self.gapless_pending
        {
            self.gapless_pending = false;
            self.advance_session_after_gapless();
            events.push(PlaybackEvent::GaplessTrackAdvanced);
        }
    }

    fn handle_error(&mut self, slot: PlayerSlot, err: String, events: &mut Vec<PlaybackEvent>) {
        if self.is_fading_out_slot(slot) {
            eprintln!("Crossfade tail error: {err}");
            self.finish_crossfade();
            return;
        }

        if slot != self.active_slot {
            eprintln!("Inactive player error: {err}");
            self.stop_slot(slot);
            return;
        }

        self.gapless_pending = false;
        events.push(PlaybackEvent::Error(err));
    }

    fn is_fading_out_slot(&self, slot: PlayerSlot) -> bool {
        self.crossfade
            .as_ref()
            .is_some_and(|crossfade| crossfade.fading_out_slot == slot)
    }

    fn load_current_track(&mut self) {
        self.collapse_to_active_player();

        if let Some(session) = &self.state.session {
            if let Some(track) = session.order.get(session.index) {
                if let Ok(url) = Url::from_file_path(&track.path) {
                    self.stop_slot(self.active_slot);
                    self.active_player_mut().load(url.as_str());
                }
            }
        }

        if matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.queue_next_uri();
        } else {
            self.active_player().set_queued_uri(None);
        }

        self.state.progress = 0.0;
        self.apply_output_volume();
    }

    fn update_now_playing(&mut self) {
        if let Some(session) = &self.state.session {
            if let Some(track) = session.order.get(session.index) {
                self.state.now_playing = Some(track.metadata.clone());
                return;
            }
        }

        self.state.now_playing = None;
    }

    fn get_current_track_id(&self) -> Option<String> {
        self.state
            .session
            .as_ref()
            .and_then(|session| session.order.get(session.index))
            .and_then(|track| track.metadata.id.clone())
    }

    /// Compute what the next session index should be without mutating state.
    /// Returns None if there is no next track (end of playlist, no repeat).
    fn compute_next_index(&self) -> Option<usize> {
        let session = self.state.session.as_ref()?;
        let len = session.order.len();

        if len == 0 {
            return None;
        }

        match self.repeat_mode {
            RepeatMode::One => Some(session.index),
            RepeatMode::All => {
                let next = session.index + 1;
                if next < len {
                    Some(next)
                } else if self.repeat_enabled {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }

    /// Pre-queue the next track URI in the active player so GStreamer can transition
    /// gaplessly when about-to-finish fires.
    fn queue_next_uri(&mut self) {
        if !matches!(self.transition_mode, PlaybackTransitionMode::Gapless) {
            self.active_player().set_queued_uri(None);
            return;
        }

        let next = self.compute_next_index().and_then(|idx| {
            self.state
                .session
                .as_ref()
                .and_then(|session| session.order.get(idx))
                .and_then(|track| {
                    Url::from_file_path(&track.path)
                        .ok()
                        .map(|url| (url.to_string(), track.metadata.id.clone()))
                })
        });

        match next {
            Some((uri, track_id)) => {
                if !self.gapless_pending {
                    self.pending_gapless_track_id = track_id;
                }
                self.active_player().set_queued_uri(Some(uri));
            }
            None => {
                if !self.gapless_pending {
                    self.pending_gapless_track_id = None;
                }
                self.active_player().set_queued_uri(None);
            }
        }
    }

    /// Called when STREAM_START confirms a gapless transition.
    /// Advances the session index and queues the track after the new current one.
    fn advance_session_after_gapless(&mut self) {
        if let Some(ref pending_id) = self.pending_gapless_track_id.clone() {
            if let Some(session) = &mut self.state.session {
                if let Some(idx) = session
                    .order
                    .iter()
                    .position(|track| track.metadata.id.as_deref() == Some(pending_id.as_str()))
                {
                    session.index = idx;
                }
            }
        } else if let Some(next_idx) = self.compute_next_index() {
            if let Some(session) = &mut self.state.session {
                session.index = next_idx;
            }
        }

        self.pending_gapless_track_id = None;
        self.update_now_playing();
        self.queue_next_uri();
        self.state.progress = 0.0;
    }
}

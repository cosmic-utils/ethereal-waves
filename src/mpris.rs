// SPDX-License-Identifier: GPL-3.0
use crate::playback_state::PlaybackStatus;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;
use zbus::interface;

#[derive(Clone, Default)]
pub struct MprisState {
    pub playback_status: PlaybackStatus,
    pub metadata: HashMap<String, zbus::zvariant::Value<'static>>,
    pub position: i64,
    pub volume: f64,
    pub shuffle: bool,
    pub loop_status: String, // "None", "Track", "Playlist"
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        PlaybackStatus::Stopped
    }
}

pub struct MediaPlayer2Player {
    pub tx: UnboundedSender<MprisCommand>,
    pub state: Arc<Mutex<MprisState>>, // Changed from playback_status
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MediaPlayer2Player {
    fn play(&self) {
        let _ = self.tx.send(MprisCommand::Play);
    }

    fn pause(&self) {
        let _ = self.tx.send(MprisCommand::Pause);
    }

    fn play_pause(&self) {
        let _ = self.tx.send(MprisCommand::PlayPause);
    }

    fn next(&self) {
        let _ = self.tx.send(MprisCommand::Next);
    }

    fn previous(&self) {
        let _ = self.tx.send(MprisCommand::Previous);
    }

    fn stop(&self) {
        let _ = self.tx.send(MprisCommand::Stop);
    }

    fn seek(&self, offset: i64) {
        let _ = self.tx.send(MprisCommand::Seek(offset));
    }

    fn set_position(&self, _track_id: zbus::zvariant::ObjectPath<'_>, position: i64) {
        let _ = self.tx.send(MprisCommand::SetPosition(position));
    }

    fn open_uri(&self, _uri: String) {}

    // Properties
    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.state
            .lock()
            .unwrap()
            .playback_status
            .as_str()
            .to_string()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        self.state.lock().unwrap().loop_status.clone()
    }

    #[zbus(property)]
    fn set_loop_status(&self, status: String) {
        let _ = self.tx.send(MprisCommand::SetLoopStatus(status));
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.state.lock().unwrap().shuffle
    }

    #[zbus(property)]
    fn set_shuffle(&self, shuffle: bool) {
        let _ = self.tx.send(MprisCommand::SetShuffle(shuffle));
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, zbus::zvariant::Value<'static>> {
        self.state.lock().unwrap().metadata.clone()
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.state.lock().unwrap().volume
    }

    #[zbus(property)]
    fn set_volume(&self, volume: f64) {
        let _ = self.tx.send(MprisCommand::SetVolume(volume));
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.state.lock().unwrap().position
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn set_rate(&self, _rate: f64) {}

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }
}

pub struct MediaPlayer2;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MediaPlayer2 {
    fn raise(&self) {}

    fn quit(&self) {}

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "Ethereal Waves"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "com.github.LotusPetal392.ethereal-waves"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["file"]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![
            "audio/mpeg",
            "audio/mp4",
            "audio/ogg",
            "audio/opus",
            "audio/flac",
            "audio/wav",
        ]
    }
}

#[derive(Debug, Clone)]
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    Stop,
    Seek(i64),
    SetPosition(i64),
    SetVolume(f64),
    SetLoopStatus(String),
    SetShuffle(bool),
}

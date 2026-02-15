// SPDX-License-Identifier: GPL-3.0

/// UI Layout Constants
pub const BASE_ROW_HEIGHT: f32 = 5.0;
pub const DIVIDER_HEIGHT: f32 = 1.0;
pub const MIN_SIZE_MULTIPLIER: f32 = 4.0;
pub const MAX_SIZE_MULTIPLIER: f32 = 30.0;
pub const ZOOM_STEP: f32 = 2.0;

/// UI Display Constants
pub const LIST_PREVIEW_ITEMS: usize = 10; // Number shown in info panel
pub const SEARCH_INPUT_WIDTH: f32 = 240.0;

/// File System Constants
pub const LIBRARY_FILENAME: &str = "library.json";
pub const PLAYLISTS_DIR: &str = "playlists";
pub const ARTWORK_DIR: &str = "artwork";
pub const MIN_FILE_SIZE: u64 = 4096;

/// Timing Constants
pub const DOUBLE_CLICK_THRESHOLD_MS: u64 = 400;
pub const TICK_INTERVAL_MS: u64 = 100;
pub const PROGRESS_UPDATE_INTERVAL_MS: u64 = 200;
pub const LIBRARY_UPDATE_INTERVAL_SECS: u64 = 10;
pub const GSTREAMER_TIMEOUT_SECS: u64 = 5;

/// Audio File Extensions
pub const VALID_AUDIO_EXTENSIONS: &[&str] = &["flac", "m4a", "mp3", "ogg", "opus", "wav"];

/// Widget IDs
pub const NEW_PLAYLIST_INPUT_ID: &str = "new_playlist_input_id";
pub const RENAME_PLAYLIST_INPUT_ID: &str = "rename_playlist_input_id";
pub const SEARCH_INPUT_ID: &str = "Text Search";

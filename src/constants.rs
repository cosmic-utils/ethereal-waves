// SPDX-License-Identifier: GPL-3.0

/// UI Layout Constants
pub const HEADER_MENU_MIN_WIDTH: f32 = 260.0;
pub const MENU_ITEM_WIDTH: u16 = 280;
pub const MENU_ITEM_HEIGHT: u16 = 40;
pub const BASE_ROW_HEIGHT: f32 = 5.0;
pub const DIVIDER_HEIGHT: f32 = 1.0;
pub const LIST_MIN_SIZE_MULTIPLIER: f32 = 4.0;
pub const LIST_MAX_SIZE_MULTIPLIER: f32 = 30.0;
pub const GRID_MIN_SIZE_MULTIPLIER: f32 = 8.0;
pub const GRID_MAX_SIZE_MULTIPLIER: f32 = 22.0;
pub const ZOOM_STEP: f32 = 2.0;
pub const FOOTER_CONDENSED_BREAKPOINT: f32 = 700.0;
pub const DEFAULT_FOOTER_VISUALIZER_BAR_COUNT: i32 = 48;
pub const MIN_FOOTER_VISUALIZER_BAR_COUNT: i32 = 8;
pub const MAX_FOOTER_VISUALIZER_BAR_COUNT: i32 = 96;
pub const DEFAULT_FOOTER_VISUALIZER_MINIMUM_ALPHA: i32 = 22;
pub const MIN_FOOTER_VISUALIZER_MINIMUM_ALPHA: i32 = 0;
pub const MAX_FOOTER_VISUALIZER_MINIMUM_ALPHA: i32 = 100;
pub const DEFAULT_FOOTER_VISUALIZER_MAXIMUM_ALPHA: i32 = 56;
pub const MIN_FOOTER_VISUALIZER_MAXIMUM_ALPHA: i32 = 0;
pub const MAX_FOOTER_VISUALIZER_MAXIMUM_ALPHA: i32 = 100;
pub const FOOTER_VISUALIZER_ANALYZER_BANDS: usize = 512;
pub const FOOTER_VISUALIZER_LOWER_CUTOFF_HZ: f32 = 50.0;
pub const FOOTER_VISUALIZER_UPPER_CUTOFF_HZ: f32 = 15_000.0;
pub const FOOTER_VISUALIZER_ASSUMED_SAMPLE_RATE_HZ: f32 = 48_000.0;
pub const DEFAULT_FOOTER_VISUALIZER_COLOR: &str = "#7d5fff";
pub const COMPACT_COLUMN_WIDTH: f32 = 96.0;
pub const DURATION_COLUMN_WIDTH: f32 = 104.0;
pub const GRID_ARTWORK_SCALE: f32 = 12.0;
pub const GRID_MIN_ARTWORK_SIZE: f32 = 56.0;
pub const GRID_MAX_ARTWORK_SIZE: f32 = 256.0;
pub const GRID_CARD_PADDING: f32 = 12.0;
pub const GRID_VIEW_PADDING: f32 = 12.0;
pub const GRID_ITEM_SPACING: f32 = 12.0;
pub const GRID_TITLE_HEIGHT: f32 = 38.0;
pub const GRID_SUBTITLE_HEIGHT: f32 = 20.0;
pub const GRID_INFO_HEIGHT: f32 = 18.0;
pub const GRID_CARD_CONTENT_SPACING: f32 = 4.0;
pub const GRID_STATUS_ICON_SIZE: u16 = 14;
pub const GRID_STATUS_ICON_SLOT: f32 = 16.0;

/// UI Display Constants
pub const TRACK_INFO_LIST_TOTAL: usize = 100;
pub const SEARCH_INPUT_WIDTH: f32 = 240.0;

/// File System Constants
pub const LIBRARY_FILENAME: &str = "library.json";
pub const PLAYLISTS_DIR: &str = "playlists";
pub const ARTWORK_DIR: &str = "artwork";
pub const ARTWORK_MEDIUM_SIZE: u32 = 256;
pub const ARTWORK_SMALL_SIZE: u32 = 128;
pub const ARTWORK_MEDIUM_SUFFIX: &str = "medium";
pub const ARTWORK_SMALL_SUFFIX: &str = "small";
pub const MIN_FILE_SIZE: u64 = 4096;

/// Timing Constants
pub const DOUBLE_CLICK_THRESHOLD_MS: u64 = 400;
pub const TICK_INTERVAL_MS: u64 = 100;
pub const PROGRESS_UPDATE_INTERVAL_MS: u64 = 200;
pub const LIBRARY_UPDATE_INTERVAL_SECS: u64 = 10;
pub const GSTREAMER_TIMEOUT_SECS: u64 = 5;
pub const IMAGE_CACHE_TTL_SECS: u64 = 300;
pub const IMAGE_CACHE_SWEEP_SECS: u64 = 30;

/// Audio File Extensions
pub const VALID_AUDIO_EXTENSIONS: &[&str] = &["flac", "m4a", "mp3", "ogg", "opus", "wav"];

/// Widget IDs
pub const NEW_PLAYLIST_INPUT_ID: &str = "new_playlist_input_id";
pub const RENAME_PLAYLIST_INPUT_ID: &str = "rename_playlist_input_id";
pub const SEARCH_INPUT_ID: &str = "Text Search";
pub const MENU_WIDGET_ID: &str = "responsive_menu";

/// Drag and Drop
pub const MIME_TRACK_IDS: &str = "application/x-ethereal-waves-track-ids";
pub const LIBRARY_TRACK_DROP_PREFIX: &str = "library:";

/// Playback Constants
pub const DEFAULT_CROSSFADE_DURATION_SECS: i32 = 5;
pub const MIN_CROSSFADE_DURATION_SECS: i32 = 1;
pub const MAX_CROSSFADE_DURATION_SECS: i32 = 30;

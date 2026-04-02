// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, SortBy, SortDirection, ViewMode};
use crate::playback_state::RepeatMode;
use cosmic::{
    Application,
    cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry},
    theme,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TitleSortMode {
    Alphabetical,
    TrackNumber,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlaylistDuplicatePolicy {
    Allow,
    Disallow,
    Ask,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppTheme {
    Dark,
    Light,
    System,
}

impl AppTheme {
    pub fn theme(&self) -> theme::Theme {
        match self {
            Self::Dark => theme::Theme::dark(),
            Self::Light => theme::Theme::light(),
            Self::System => theme::system_preference(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ListColumn {
    TrackTotal,
    TrackNumber,
    Title,
    Album,
    AlbumArtist,
    Artist,
    DiscNumber,
    DiscTotal,
    Duration,
    FilePath,
    Genre,
}

impl ListColumn {
    pub const ALL: [Self; 11] = [
        Self::TrackTotal,
        Self::TrackNumber,
        Self::Title,
        Self::Album,
        Self::AlbumArtist,
        Self::Artist,
        Self::DiscNumber,
        Self::DiscTotal,
        Self::Duration,
        Self::FilePath,
        Self::Genre,
    ];

    pub fn default_order() -> Vec<Self> {
        Self::ALL.to_vec()
    }

    pub fn is_toggleable(&self) -> bool {
        matches!(
            self,
            Self::Album
                | Self::AlbumArtist
                | Self::Artist
                | Self::DiscNumber
                | Self::DiscTotal
                | Self::Duration
                | Self::FilePath
                | Self::Genre
                | Self::Title
                | Self::TrackNumber
                | Self::TrackTotal
        )
    }

    pub fn is_visible(&self, config: &Config) -> bool {
        match self {
            Self::Album => config.list_show_album_column,
            Self::AlbumArtist => config.list_show_album_artist_column,
            Self::Artist => config.list_show_artist_column,
            Self::DiscNumber => config.list_show_disc_number_column,
            Self::DiscTotal => config.list_show_disc_total_column,
            Self::Duration => config.list_show_duration_column,
            Self::FilePath => config.list_show_file_path_column,
            Self::Genre => config.list_show_genre_column,
            Self::Title => config.list_show_title_column,
            Self::TrackTotal => config.list_show_track_total_column,
            Self::TrackNumber => config.list_show_track_number_column,
        }
    }

    pub fn sort_by(&self) -> Option<SortBy> {
        match self {
            Self::TrackNumber => None,
            Self::TrackTotal => Some(SortBy::TrackTotal),
            Self::Title => Some(SortBy::Title),
            Self::Album => Some(SortBy::Album),
            Self::AlbumArtist => Some(SortBy::AlbumArtist),
            Self::Artist => Some(SortBy::Artist),
            Self::DiscNumber => Some(SortBy::DiscNumber),
            Self::DiscTotal => Some(SortBy::DiscTotal),
            Self::Duration => Some(SortBy::Duration),
            Self::Genre => Some(SortBy::Genre),
            Self::FilePath => Some(SortBy::FilePath),
        }
    }

    pub fn normalize_order(columns: &[Self]) -> Vec<Self> {
        let mut normalized = Vec::with_capacity(Self::ALL.len());

        for column in columns {
            if Self::ALL.contains(column) && !normalized.contains(column) {
                normalized.push(*column);
            }
        }

        for column in Self::ALL {
            if !normalized.contains(&column) {
                normalized.push(column);
            }
        }

        normalized
    }
}

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[version = 1]
#[serde(default)]
pub struct Config {
    pub app_theme: AppTheme,
    pub library_paths: HashSet<String>,
    pub list_text_wrap: bool,
    pub list_row_align_top: bool,
    pub list_show_album_column: bool,
    pub list_show_album_artist_column: bool,
    pub list_show_artist_column: bool,
    pub list_show_disc_number_column: bool,
    pub list_show_disc_total_column: bool,
    pub list_show_duration_column: bool,
    pub list_show_file_path_column: bool,
    pub list_show_genre_column: bool,
    pub list_show_title_column: bool,
    pub list_show_track_number_column: bool,
    pub list_show_track_total_column: bool,
    pub list_column_order: Vec<ListColumn>,
    pub title_sort: TitleSortMode,
    pub playlist_duplicate_policy: PlaylistDuplicatePolicy,
    pub view_mode: ViewMode,
}

impl Config {
    pub fn load() -> (Option<cosmic_config::Config>, Self) {
        match cosmic_config::Config::new(AppModel::APP_ID, CONFIG_VERSION) {
            Ok(config_handler) => {
                let config = match Self::get_entry(&config_handler) {
                    Ok(ok) => ok,
                    Err((errs, config)) => {
                        log::info!("errors loading config: {errs:?}");
                        config
                    }
                };
                (Some(config_handler), config)
            }
            Err(err) => {
                log::error!("failed to create config handler: {err}");
                (None, Self::default())
            }
        }
    }

    pub fn normalized_list_column_order(&self) -> Vec<ListColumn> {
        ListColumn::normalize_order(&self.list_column_order)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_theme: AppTheme::System,
            library_paths: HashSet::new(),
            list_text_wrap: true,
            list_row_align_top: false,
            list_show_album_column: true,
            list_show_album_artist_column: false,
            list_show_artist_column: true,
            list_show_disc_number_column: false,
            list_show_disc_total_column: false,
            list_show_genre_column: false,
            list_show_duration_column: false,
            list_show_file_path_column: false,
            list_show_title_column: true,
            list_show_track_number_column: false,
            list_show_track_total_column: false,
            list_column_order: ListColumn::default_order(),
            title_sort: TitleSortMode::Alphabetical,
            playlist_duplicate_policy: PlaylistDuplicatePolicy::Allow,
            view_mode: ViewMode::List,
        }
    }
}

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct State {
    pub muted: bool,
    pub playlist_nav_order: Vec<u32>,
    pub repeat: bool,
    pub repeat_mode: RepeatMode,
    pub shuffle: bool,
    pub size_multiplier: f32,
    pub sort_by: SortBy,
    pub sort_direction: SortDirection,
    pub volume: i32,
    pub window_height: f32,
    pub window_width: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            muted: false,
            playlist_nav_order: Vec::new(),
            repeat: false,
            repeat_mode: RepeatMode::All,
            shuffle: false,
            size_multiplier: 8.0,
            sort_by: SortBy::Artist,
            sort_direction: SortDirection::Ascending,
            volume: 100,
            window_height: 1024.0,
            window_width: 768.0,
        }
    }
}

impl State {
    pub fn load() -> (Option<cosmic_config::Config>, Self) {
        match cosmic_config::Config::new_state(AppModel::APP_ID, CONFIG_VERSION) {
            Ok(config_handler) => {
                let config = match Self::get_entry(&config_handler) {
                    Ok(ok) => ok,
                    Err((errs, config)) => {
                        log::info!("errors loading config: {errs:?}");
                        config
                    }
                };
                (Some(config_handler), config)
            }
            Err(err) => {
                log::error!("failed to create config handler: {err}");
                (None, Self::default())
            }
        }
    }
}

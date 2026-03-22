// SPDX-License-Identifier: GPL-3.0

use crate::app::{PlaylistKind, SortBy, SortDirection};
use crate::config::TitleSortMode;
use crate::fl;
use crate::library::MediaMetaData;
use chrono::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt, path::PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct Playlist {
    id: u32,
    name: String,
    kind: PlaylistKind,
    tracks: Vec<Track>,
}

impl Playlist {
    pub fn new(name: String) -> Playlist {
        let mut id: u32;
        loop {
            id = rand::rng().random();
            if id != 0 {
                break;
            }
        }
        Self {
            id: id,
            name: name,
            kind: PlaylistKind::User,
            tracks: Vec::new(),
        }
    }

    pub fn library() -> Self {
        Self {
            id: u32::MAX,
            name: fl!("library"),
            kind: PlaylistKind::Library,
            tracks: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn is_library(&self) -> bool {
        matches!(self.kind, PlaylistKind::Library)
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn sort(
        &mut self,
        sort_by: SortBy,
        sort_direction: SortDirection,
        title_sort: TitleSortMode,
    ) {
        self.tracks.sort_by(|a, b| {
            let ordering = match sort_by {
                SortBy::Artist => a
                    .metadata
                    .artist
                    .cmp(&b.metadata.artist)
                    .then(a.metadata.album.cmp(&b.metadata.album))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::Album => a
                    .metadata
                    .album
                    .cmp(&b.metadata.album)
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::AlbumArtist => a
                    .metadata
                    .album_artist
                    .cmp(&b.metadata.album_artist)
                    .then(a.metadata.album.cmp(&b.metadata.album))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::Title => a.metadata.title.cmp(&b.metadata.title),

                SortBy::TrackTotal => a
                    .metadata
                    .track_count
                    .cmp(&b.metadata.track_count)
                    .then(a.metadata.album.cmp(&b.metadata.album))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::DiscNumber => a
                    .metadata
                    .album_disc_number
                    .cmp(&b.metadata.album_disc_number)
                    .then(a.metadata.track_number.cmp(&b.metadata.track_number))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::DiscTotal => a
                    .metadata
                    .album_disc_count
                    .cmp(&b.metadata.album_disc_count)
                    .then(
                        a.metadata
                            .album_disc_number
                            .cmp(&b.metadata.album_disc_number),
                    )
                    .then(a.metadata.track_number.cmp(&b.metadata.track_number))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::Genre => a
                    .metadata
                    .genre
                    .cmp(&b.metadata.genre)
                    .then(a.metadata.artist.cmp(&b.metadata.artist))
                    .then(a.metadata.album.cmp(&b.metadata.album))
                    .then_with(|| compare_title(a, b, title_sort)),

                SortBy::FilePath => a.path.cmp(&b.path),

                SortBy::Duration => compare_optional_f32(a.metadata.duration, b.metadata.duration)
                    .then_with(|| compare_title(a, b, title_sort)),
            };

            match sort_direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });
    }

    pub fn push(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn select_all(&mut self) {
        for track in self.tracks.iter_mut() {
            track.selected = true;
        }
    }

    pub fn select(&mut self, index: usize) {
        self.tracks[index].selected = true;
    }

    pub fn selected(&self) -> Vec<&Track> {
        self.tracks.iter().filter(|t| t.selected).collect()
    }

    pub fn deselect(&mut self, index: usize) {
        self.tracks[index].selected = false;
    }

    pub fn clear_selected(&mut self) {
        self.tracks.iter_mut().for_each(|t| t.selected = false);
    }

    pub fn remove_selected(&mut self) {
        self.tracks.retain(|t| !t.selected);
    }

    pub fn selected_iter(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.selected)
    }

    pub fn select_range(&mut self, start: usize, end: usize) {
        if start < end {
            for i in start..=end {
                self.tracks[i].selected = true;
            }
        } else if end < start {
            for i in end..=start {
                self.tracks[i].selected = true;
            }
        }
    }
}

impl fmt::Debug for Playlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Playlist {{ id: {}, name: {}, tracks: {:?} }}",
            self.id, self.name, self.tracks
        )
    }
}

fn random_entry_id() -> u32 {
    rand::random()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Track {
    #[serde(default = "random_entry_id")]
    pub entry_id: u32,
    pub path: PathBuf,
    #[serde(skip)]
    pub selected: bool,
    pub metadata: MediaMetaData,
    pub date_added: String,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            entry_id: rand::random(),
            path: PathBuf::new(),
            selected: false,
            metadata: MediaMetaData::new(),
            date_added: Local::now().to_string(),
        }
    }
}

impl Track {
    pub fn new() -> Self {
        Self {
            entry_id: rand::random(),
            path: PathBuf::new(),
            selected: false,
            metadata: MediaMetaData::new(),
            date_added: Local::now().to_string(),
        }
    }

    pub fn generate_entry_id(&mut self) {
        self.entry_id = random_entry_id();
    }

    pub fn update_date_added(&mut self) {
        self.date_added = Local::now().to_string();
    }
}

fn compare_title(a: &Track, b: &Track, title_sort: TitleSortMode) -> Ordering {
    match title_sort {
        TitleSortMode::Alphabetical => a.metadata.title.cmp(&b.metadata.title),
        TitleSortMode::TrackNumber => a
            .metadata
            .album_disc_number
            .cmp(&b.metadata.album_disc_number)
            .then(a.metadata.track_number.cmp(&b.metadata.track_number))
            .then_with(|| a.metadata.title.cmp(&b.metadata.title)),
    }
}

fn compare_optional_f32(a: Option<f32>, b: Option<f32>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
    }
}

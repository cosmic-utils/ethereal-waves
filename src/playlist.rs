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
        case_sensitive: bool,
    ) {
        self.tracks.sort_by(|a, b| {
            let ordering = match sort_by {
                SortBy::Artist => compare_optional_text(
                    a.metadata.artist.as_deref(),
                    b.metadata.artist.as_deref(),
                    case_sensitive,
                )
                .then(compare_optional_text(
                    a.metadata.album.as_deref(),
                    b.metadata.album.as_deref(),
                    case_sensitive,
                ))
                .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::Album => compare_optional_text(
                    a.metadata.album.as_deref(),
                    b.metadata.album.as_deref(),
                    case_sensitive,
                )
                .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::AlbumArtist => compare_optional_text(
                    a.metadata.album_artist.as_deref(),
                    b.metadata.album_artist.as_deref(),
                    case_sensitive,
                )
                .then(compare_optional_text(
                    a.metadata.album.as_deref(),
                    b.metadata.album.as_deref(),
                    case_sensitive,
                ))
                .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::Title => compare_optional_text(
                    a.metadata.title.as_deref(),
                    b.metadata.title.as_deref(),
                    case_sensitive,
                ),

                SortBy::TrackTotal => a
                    .metadata
                    .track_count
                    .cmp(&b.metadata.track_count)
                    .then(compare_optional_text(
                        a.metadata.album.as_deref(),
                        b.metadata.album.as_deref(),
                        case_sensitive,
                    ))
                    .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::DiscNumber => a
                    .metadata
                    .album_disc_number
                    .cmp(&b.metadata.album_disc_number)
                    .then(a.metadata.track_number.cmp(&b.metadata.track_number))
                    .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

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
                    .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::Genre => compare_optional_text(
                    a.metadata.genre.as_deref(),
                    b.metadata.genre.as_deref(),
                    case_sensitive,
                )
                .then(compare_optional_text(
                    a.metadata.artist.as_deref(),
                    b.metadata.artist.as_deref(),
                    case_sensitive,
                ))
                .then(compare_optional_text(
                    a.metadata.album.as_deref(),
                    b.metadata.album.as_deref(),
                    case_sensitive,
                ))
                .then_with(|| compare_title(a, b, title_sort, case_sensitive)),

                SortBy::FilePath => compare_path(&a.path, &b.path, case_sensitive),

                SortBy::Duration => compare_optional_f32(a.metadata.duration, b.metadata.duration)
                    .then_with(|| compare_title(a, b, title_sort, case_sensitive)),
            };

            // Possible tie breaker fix
            let ordering = ordering.then_with(|| compare_path(&a.path, &b.path, case_sensitive));

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

    pub fn instance_id(&self) -> String {
        self.entry_id.to_string()
    }
}

fn compare_title(
    a: &Track,
    b: &Track,
    title_sort: TitleSortMode,
    case_sensitive: bool,
) -> Ordering {
    match title_sort {
        TitleSortMode::Alphabetical => compare_optional_text(
            a.metadata.title.as_deref(),
            b.metadata.title.as_deref(),
            case_sensitive,
        ),
        TitleSortMode::TrackNumber => a
            .metadata
            .album_disc_number
            .cmp(&b.metadata.album_disc_number)
            .then(a.metadata.track_number.cmp(&b.metadata.track_number))
            .then_with(|| {
                compare_optional_text(
                    a.metadata.title.as_deref(),
                    b.metadata.title.as_deref(),
                    case_sensitive,
                )
            }),
    }
}

fn compare_optional_text(a: Option<&str>, b: Option<&str>, case_sensitive: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => compare_text(a, b, case_sensitive),
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
    }
}

fn compare_text(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    if case_sensitive {
        a.cmp(b)
    } else {
        a.chars()
            .flat_map(char::to_lowercase)
            .cmp(b.chars().flat_map(char::to_lowercase))
            .then_with(|| a.cmp(b))
    }
}

fn compare_path(a: &PathBuf, b: &PathBuf, case_sensitive: bool) -> Ordering {
    if case_sensitive {
        a.cmp(b)
    } else {
        let a_path = a.to_string_lossy();
        let b_path = b.to_string_lossy();

        compare_text(a_path.as_ref(), b_path.as_ref(), false).then_with(|| a.cmp(b))
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

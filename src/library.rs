// SPDX-License-Identifier: GPL-3.0

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use xdg::BaseDirectories;

use crate::constants::LIBRARY_FILENAME;

#[derive(Debug, Clone)]
pub struct Library {
    pub media: HashMap<PathBuf, MediaMetaData>,
}

impl Library {
    pub fn new() -> Library {
        Self {
            media: HashMap::new(),
        }
    }

    // Save the current media to the home data directory
    pub fn save(&self, app_xdg_dirs: &BaseDirectories) -> Result<(), Box<dyn Error>> {
        let file_path = app_xdg_dirs.place_data_file(LIBRARY_FILENAME).unwrap();
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self.media)?;
        writer.flush()?;
        Ok(())
    }

    pub fn from_id(&self, id: &String) -> Option<(&PathBuf, &MediaMetaData)> {
        if let Some(entry) = self.media.iter().find(|(_, v)| v.id == Some(id.clone())) {
            return Some(entry);
        }
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaMetaData {
    pub id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub album_disc_number: Option<u32>,
    pub album_disc_count: Option<u32>,
    pub track_number: Option<u32>,
    pub track_count: Option<u32>,
    pub duration: Option<f32>,
    pub artwork_filename: Option<String>,
}

impl MediaMetaData {
    pub fn new() -> Self {
        Self {
            id: None,
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            genre: None,
            album_disc_number: None,
            album_disc_count: None,
            track_number: None,
            track_count: None,
            duration: None,
            artwork_filename: None,
        }
    }
}

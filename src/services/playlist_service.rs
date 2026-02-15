use crate::app::PlaylistId;
use crate::constants::PLAYLISTS_DIR;
//use crate::library::MediaMetaData;
use crate::playlist::{Playlist, Track};
use anyhow::{Result, anyhow};
//use std::collections::HashMap;
use std::fs;
//use std::path::PathBuf;
use std::sync::Arc;
use xdg::BaseDirectories;

// #[derive(Debug)]
// pub enum PlaylistError {
//     NotFound(u32),
//     CannotModifyLibrary,
//     AlreadyExists(String),
// }

pub struct PlaylistService {
    playlists: Vec<Playlist>,
    xdg_dirs: Arc<BaseDirectories>,
}

impl PlaylistService {
    pub fn new(xdg_dirs: Arc<BaseDirectories>) -> Self {
        Self {
            playlists: Vec::new(),
            xdg_dirs,
        }
    }

    /// Load all playlists from the filesystem and the library
    pub fn load_all(&mut self, library_tracks: Vec<Track>) -> Result<()> {
        let mut library = Playlist::library();
        for track in library_tracks {
            library.push(track);
        }
        self.playlists.push(library);

        // Load user playlists
        let playlist_dir = self.xdg_dirs.create_data_directory(PLAYLISTS_DIR)?;

        for entry in fs::read_dir(playlist_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                let playlist: Playlist = serde_json::from_str(&content)?;
                self.playlists.push(playlist);
            }
        }

        Ok(())
    }

    /// Create new playlist
    pub fn create(&mut self, name: String) -> Result<PlaylistId> {
        // Check for duplicate names
        if self.playlists.iter().any(|p| p.name() == name) {
            return Err(anyhow!("Playlist '{}' already exists", name));
        }

        let playlist = Playlist::new(name);
        let id = playlist.id();

        self.playlists.push(playlist);
        self.save(id)?;

        Ok(id)
    }

    /// Rename playlist
    pub fn rename(&mut self, id: PlaylistId, new_name: String) -> Result<()> {
        let playlist = self.get_mut(id)?;

        if playlist.is_library() {
            return Err(anyhow!("Cannot rename library"));
        }

        playlist.set_name(new_name);
        self.save(id)?;

        Ok(())
    }

    /// Delete playlist
    pub fn delete(&mut self, id: PlaylistId) -> Result<()> {
        // Make sure it isn't the library
        let playlist = self.get(id)?;
        if playlist.is_library() {
            return Err(anyhow!("Cannot delete library"));
        }

        // Remove file
        let filename = format!("{}.json", id);
        let mut file_path = self.xdg_dirs.create_data_directory("playlists")?;
        file_path.push(filename);
        fs::remove_file(file_path)?;

        // Remove from memory
        self.playlists.retain(|p| p.id() != id);

        Ok(())
    }

    /// Add tracks
    pub fn add_tracks(&mut self, playlist_id: PlaylistId, tracks: Vec<Track>) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;

        for track in tracks {
            playlist.push(track);
        }

        if !playlist.is_library() {
            self.save(playlist_id)?;
        }

        Ok(())
    }

    /// Remove tracks
    pub fn remove_selected(&mut self, playlist_id: PlaylistId) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;

        if playlist.is_library() {
            return Err(anyhow!("Cannot remove tracks from library"));
        }

        playlist.remove_selected();

        playlist.remove_selected();
        self.save(playlist_id)?;

        Ok(())
    }

    /// Get playlist by ID
    pub fn get(&self, id: PlaylistId) -> Result<&Playlist> {
        self.playlists
            .iter()
            .find(|p| p.id() == id)
            .ok_or_else(|| anyhow!("Playlist {} not found", id))
    }

    /// Get mutable reference to playlist
    pub fn get_mut(&mut self, id: PlaylistId) -> Result<&mut Playlist> {
        self.playlists
            .iter_mut()
            .find(|p| p.id() == id)
            .ok_or_else(|| anyhow!("Playlist {} not found", id))
    }

    /// Get the library playlist
    pub fn get_library(&self) -> Result<&Playlist> {
        self.playlists
            .iter()
            .find(|p| p.is_library())
            .ok_or_else(|| anyhow!("Library playlist not found"))
    }

    /// Get a mutable reference to the library playlist
    pub fn get_library_mut(&mut self) -> Result<&mut Playlist> {
        self.playlists
            .iter_mut()
            .find(|p| p.is_library())
            .ok_or_else(|| anyhow!("Library not found"))
    }

    /// Get all playlists
    pub fn all(&self) -> &[Playlist] {
        &self.playlists
    }

    /// Get all user playlists
    pub fn user_playlists(&self) -> impl Iterator<Item = &Playlist> {
        self.playlists.iter().filter(|p| !p.is_library())
    }

    /// Save playlist to disk
    pub fn save(&self, id: PlaylistId) -> Result<()> {
        let playlist = self.get(id)?;

        if playlist.is_library() {
            return Ok(());
        }

        let filename = format!("{}.json", id);
        let mut file_path = self.xdg_dirs.create_data_directory("playlists")?;
        file_path.push(filename);

        let content = serde_json::to_string_pretty(playlist)?;
        fs::write(file_path, content)?;

        Ok(())
    }

    /// Select all tracks in a playlist
    pub fn select_all(&mut self, playlist_id: PlaylistId) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;
        playlist.select_all();
        Ok(())
    }

    /// Clear all selected tracks in a playlist
    pub fn clear_selection(&mut self, playlist_id: PlaylistId) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;
        playlist.clear_selected();
        Ok(())
    }

    /// Select a specific track
    pub fn select_track(&mut self, playlist_id: PlaylistId, index: usize) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;
        if index < playlist.len() {
            playlist.select(index);
            Ok(())
        } else {
            Err(anyhow!("Track index {} out of bounds", index))
        }
    }

    /// Deselect a specific track
    pub fn deselect_track(&mut self, playlist_id: PlaylistId, index: usize) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;
        if index < playlist.len() {
            playlist.deselect(index);
            Ok(())
        } else {
            Err(anyhow!("Track index {} out of bounds", index))
        }
    }

    /// Select track range
    pub fn select_range(
        &mut self,
        playlist_id: PlaylistId,
        start: usize,
        end: usize,
    ) -> Result<()> {
        let playlist = self.get_mut(playlist_id)?;
        playlist.select_range(start, end);
        Ok(())
    }
}

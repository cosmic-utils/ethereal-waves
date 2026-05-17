use crate::app::PlaylistId;
use crate::constants::PLAYLISTS_DIR;
use crate::library::Library;
use crate::playlist::{Playlist, Track};
use anyhow::{Result, anyhow};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;
use xdg::BaseDirectories;

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
        let playlist_dir = self.playlist_dir()?;

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
        let file_path = self.playlist_file_path(id)?;
        fs::remove_file(file_path)?;

        // Remove from memory
        self.playlists.retain(|p| p.id() != id);

        Ok(())
    }

    /// Import an M3U playlist as a new user playlist.
    pub fn import_m3u(&mut self, path: &Path, library: &Library) -> Result<PlaylistId> {
        let playlist_name = self.unique_playlist_name(Self::playlist_name_from_path(path));
        let mut playlist = Playlist::new(playlist_name);
        let playlist_id = playlist.id();

        for track in Self::read_m3u_tracks(path, library)? {
            playlist.push(track);
        }

        self.playlists.push(playlist);

        if let Err(err) = self.save(playlist_id) {
            self.playlists.retain(|p| p.id() != playlist_id);
            return Err(err);
        }

        Ok(playlist_id)
    }

    /// Export a user playlist to an extended M3U playlist file.
    pub fn export_m3u(&self, playlist_id: PlaylistId, path: &Path) -> Result<()> {
        let playlist = self.get(playlist_id)?;

        let mut content = String::from("#EXTM3U\n");

        for track in playlist.tracks() {
            let duration = track
                .metadata
                .duration
                .map(|duration| duration.round() as i64)
                .unwrap_or(-1);
            let title = Self::m3u_track_title(track);

            content.push_str(&format!("#EXTINF:{duration},{title}\n"));
            content.push_str(&format!("{}\n", track.path.to_string_lossy()));
        }

        fs::write(path, content)?;
        Ok(())
    }

    /// Split tracks, existing and new
    pub fn split_tracks_by_duplicate(
        &self,
        playlist_id: PlaylistId,
        tracks: Vec<Track>,
    ) -> Result<(Vec<Track>, Vec<Track>)> {
        let playlist = self.get(playlist_id)?;
        let mut seen: HashSet<_> = playlist
            .tracks()
            .iter()
            .map(|track| track.path.clone())
            .collect();

        let mut new_tracks = Vec::new();
        let mut duplicates = Vec::new();

        for track in tracks {
            if seen.insert(track.path.clone()) {
                new_tracks.push(track);
            } else {
                duplicates.push(track);
            }
        }

        Ok((new_tracks, duplicates))
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

        let file_path = self.playlist_file_path(id)?;

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

    fn playlist_dir(&self) -> Result<PathBuf> {
        Ok(self.xdg_dirs.create_data_directory(PLAYLISTS_DIR)?)
    }

    fn playlist_file_path(&self, id: PlaylistId) -> Result<PathBuf> {
        let mut file_path = self.playlist_dir()?;
        file_path.push(format!("{id}.json"));
        Ok(file_path)
    }

    fn playlist_name_from_path(path: &Path) -> &str {
        path.file_stem()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Imported Playlist")
    }

    fn unique_playlist_name(&self, base_name: &str) -> String {
        let base_name = base_name.trim();
        let base_name = if base_name.is_empty() {
            "Imported Playlist"
        } else {
            base_name
        };

        if !self.playlists.iter().any(|p| p.name() == base_name) {
            return base_name.to_string();
        }

        for suffix in 2.. {
            let candidate = format!("{base_name} {suffix}");
            if !self
                .playlists
                .iter()
                .any(|p| p.name() == candidate.as_str())
            {
                return candidate;
            }
        }

        unreachable!()
    }

    fn read_m3u_tracks(path: &Path, library: &Library) -> Result<Vec<Track>> {
        let content = String::from_utf8_lossy(&fs::read(path)?).into_owned();
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tracks = Vec::new();

        for line in content.lines() {
            let entry = line.trim().trim_start_matches('\u{feff}').trim();

            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }

            let Some(path) = Self::m3u_entry_path(base_dir, entry) else {
                continue;
            };

            tracks.push(Self::track_from_path(path, library));
        }

        Ok(tracks)
    }

    fn m3u_entry_path(base_dir: &Path, entry: &str) -> Option<PathBuf> {
        if let Ok(url) = Url::parse(entry) {
            if url.scheme() == "file" {
                return url.to_file_path().ok();
            }

            if !Self::is_windows_drive_path(entry) {
                return None;
            }
        }

        let path = PathBuf::from(entry);
        Some(if path.is_absolute() {
            path
        } else {
            base_dir.join(path)
        })
    }

    fn is_windows_drive_path(entry: &str) -> bool {
        let bytes = entry.as_bytes();

        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/')
    }

    fn track_from_path(path: PathBuf, library: &Library) -> Track {
        let track_path = if library.media.contains_key(&path) {
            path.clone()
        } else {
            path.canonicalize()
                .ok()
                .filter(|canonical_path| library.media.contains_key(canonical_path))
                .unwrap_or_else(|| path.clone())
        };

        let mut track = Track::new();
        track.path = track_path.clone();

        if let Some(metadata) = library.media.get(&track_path) {
            track.metadata = metadata.clone();
        }

        track
    }

    fn m3u_track_title(track: &Track) -> String {
        let title = track
            .metadata
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_string)
            .or_else(|| {
                track
                    .path
                    .file_stem()
                    .or_else(|| track.path.file_name())
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| track.path.to_string_lossy().to_string());

        let artist = track
            .metadata
            .artist
            .as_deref()
            .or(track.metadata.album_artist.as_deref())
            .map(str::trim)
            .filter(|artist| !artist.is_empty());

        let display_title = match artist {
            Some(artist) => format!("{artist} - {title}"),
            None => title,
        };

        display_title.replace('\r', " ").replace('\n', " ")
    }
}

// SPDX-License-Identifier: GPL-3.0
// src/services/library_service.rs

use crate::constants::*;
use crate::helpers::artwork_variant_filename;
use crate::library::{Library, MediaMetaData};
use gstreamer as gst;
use gstreamer_pbutils as pbutils;
use image::{DynamicImage, ImageFormat};
use sha256::digest;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use url::Url;
use walkdir::WalkDir;
use xdg::BaseDirectories;

/// Progress updates during library scanning
#[derive(Debug, Clone)]
pub enum LibraryProgress {
    /// Progress update with current/total/percent
    Progress {
        current: f32,
        total: f32,
        percent: f32,
    },
    /// Partial library update with completed entries
    PartialUpdate(HashMap<PathBuf, MediaMetaData>),
    /// Final complete library
    Complete(Library),
    Cancelled,
}

#[derive(Debug)]
pub enum LibraryError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidData(String),
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibraryError::Io(e) => write!(f, "IO error: {}", e),
            LibraryError::Json(e) => write!(f, "JSON error: {}", e),
            LibraryError::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}

impl std::error::Error for LibraryError {}

impl From<std::io::Error> for LibraryError {
    fn from(err: std::io::Error) -> Self {
        LibraryError::Io(err)
    }
}

impl From<serde_json::Error> for LibraryError {
    fn from(err: serde_json::Error) -> Self {
        LibraryError::Json(err)
    }
}

/// Service for managing the music library
pub struct LibraryService {
    xdg_dirs: Arc<BaseDirectories>,
}

impl LibraryService {
    pub fn new(xdg_dirs: Arc<BaseDirectories>) -> Self {
        Self { xdg_dirs }
    }

    /// Load library from disk
    pub fn load(&self) -> Result<Library, LibraryError> {
        let mut media: HashMap<PathBuf, MediaMetaData> = self
            .xdg_dirs
            .find_data_file(LIBRARY_FILENAME)
            .map(|path| {
                let content = fs::read_to_string(path)?;
                Ok::<_, LibraryError>(serde_json::from_str(&content)?)
            })
            .transpose()?
            .unwrap_or_default();

        // Remove any entry without an id
        media.retain(|_, v| v.id.is_some());

        Ok(Library { media })
    }

    /// Save library to disk
    pub fn save(&self, library: &Library) -> Result<(), LibraryError> {
        library
            .save(&self.xdg_dirs)
            .map_err(|e| LibraryError::InvalidData(format!("Failed to save library: {}", e)))
    }

    /// Scan library paths and extract metadata in a background thread
    ///
    /// This spawns a thread that:
    /// 1. Walks all provided paths to find audio files
    /// 2. Extracts metadata using GStreamer
    /// 3. Caches artwork
    /// 4. Sends progress updates via the channel
    pub fn scan_library(
        paths: HashSet<String>,
        xdg_dirs: Arc<BaseDirectories>,
        progress_tx: UnboundedSender<LibraryProgress>,
        cancel_token: CancellationToken,
        regenerate_thumbnails: bool,
    ) {
        std::thread::spawn(move || {
            let mut library = Library::new();

            // Step 1: Collect all audio file paths
            for path in paths {
                if cancel_token.is_cancelled() {
                    log::info!("Library scan cancelled by user");
                    let _ = progress_tx.send(LibraryProgress::Cancelled);
                    return;
                }

                for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                    let extension = entry
                        .file_name()
                        .to_str()
                        .unwrap_or("")
                        .split('.')
                        .last()
                        .unwrap_or("")
                        .to_lowercase();

                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                    if VALID_AUDIO_EXTENSIONS.contains(&extension.as_str()) && size > MIN_FILE_SIZE
                    {
                        library
                            .media
                            .insert(entry.into_path(), MediaMetaData::new());
                    }
                }
            }

            // Step 2: Extract metadata from each file
            if let Err(err) = gst::init() {
                eprintln!("Failed to initialize GStreamer: {}", err);
                if progress_tx
                    .send(LibraryProgress::Progress {
                        current: 0.0,
                        total: 0.0,
                        percent: 0.0,
                    })
                    .is_err()
                {
                    log::warn!("Failed to send progress update")
                };
                if progress_tx
                    .send(LibraryProgress::Complete(library))
                    .is_err()
                {
                    log::warn!("Failed to send completion update")
                };
                return;
            }

            let mut update_progress: f32 = 0.0;
            let update_total: f32 = library.media.len() as f32;

            let mut last_progress_update = Instant::now();
            let update_progress_interval = Duration::from_millis(PROGRESS_UPDATE_INTERVAL_MS);

            let mut last_library_update = Instant::now();
            let update_library_interval = Duration::from_secs(LIBRARY_UPDATE_INTERVAL_SECS);

            let mut entries: Vec<(PathBuf, MediaMetaData)> = library.media.into_iter().collect();

            let mut completed_entries: HashMap<PathBuf, MediaMetaData> = HashMap::new();

            let discoverer = match pbutils::Discoverer::new(gst::ClockTime::from_seconds(
                GSTREAMER_TIMEOUT_SECS,
            ))
            .map_err(|e| format!("Failed to create discoverer: {:?}", e))
            .ok()
            {
                Some(discoverer) => discoverer,
                None => {
                    eprintln!("Failed to create discoverer");
                    let _ = progress_tx.send(LibraryProgress::Cancelled);
                    return;
                }
            };
            let mut refreshed_artwork_hashes = HashSet::new();

            for (file, track_metadata) in entries.iter_mut() {
                if cancel_token.is_cancelled() {
                    log::info!("Library scan cancelled during metadata extraction");
                    let _ = progress_tx.send(LibraryProgress::Cancelled);
                    return;
                }

                // Always count this file as processed (attempted)
                update_progress += 1.0;

                let ok = match Self::extract_metadata(
                    file,
                    track_metadata,
                    &xdg_dirs,
                    &discoverer,
                    regenerate_thumbnails,
                    &mut refreshed_artwork_hashes,
                ) {
                    Ok(_) => true,
                    Err(e) => {
                        eprintln!("Failed to extract metadata from {:?}: {}", file, e);
                        false
                    }
                };

                if ok {
                    completed_entries.insert(file.clone(), track_metadata.clone());
                }

                let now = Instant::now();

                if now.duration_since(last_progress_update) >= update_progress_interval {
                    last_progress_update = now;
                    let _ = progress_tx.send(LibraryProgress::Progress {
                        current: update_progress,
                        total: update_total,
                        percent: if update_total > 0.0 {
                            update_progress / update_total * 100.0
                        } else {
                            100.0
                        },
                    });
                }

                if now.duration_since(last_library_update) >= update_library_interval {
                    last_library_update = now;
                    let _ =
                        progress_tx.send(LibraryProgress::PartialUpdate(completed_entries.clone()));
                }
            }

            // Ensure UI reaches 100% and finishes
            let _ = progress_tx.send(LibraryProgress::Progress {
                current: update_total,
                total: update_total,
                percent: 100.0,
            });

            let mut out = Library::new();
            out.media = completed_entries;

            let _ = progress_tx.send(LibraryProgress::Complete(out));
        });
    }

    /// Extract metadata from a single audio file using GStreamer
    fn extract_metadata(
        file: &PathBuf,
        track_metadata: &mut MediaMetaData,
        xdg_dirs: &BaseDirectories,
        discoverer: &pbutils::Discoverer,
        regenerate_thumbnails: bool,
        refreshed_artwork_hashes: &mut HashSet<String>,
    ) -> Result<(), String> {
        let file_str = file
            .to_str()
            .ok_or_else(|| "Invalid file path".to_string())?;

        let uri = Url::from_file_path(file_str).map_err(|_| "Failed to create URI".to_string())?;

        let info = discoverer
            .discover_uri(uri.as_str())
            .map_err(|e| format!("Failed to discover: {}", e))?;

        // Set the unique ID
        track_metadata.id = Some(digest(file_str));

        // Extract tags if available
        if let Some(tags) = info.tags() {
            track_metadata.title = tags.get::<gst::tags::Title>().map(|t| t.get().to_owned());
            track_metadata.artist = tags.get::<gst::tags::Artist>().map(|t| t.get().to_owned());
            track_metadata.album = tags.get::<gst::tags::Album>().map(|t| t.get().to_owned());
            track_metadata.album_artist = tags
                .get::<gst::tags::AlbumArtist>()
                .map(|t| t.get().to_owned());
            track_metadata.genre = tags.get::<gst::tags::Genre>().map(|t| t.get().to_owned());
            track_metadata.track_number = tags
                .get::<gst::tags::TrackNumber>()
                .map(|t| t.get().to_owned());
            track_metadata.track_count = tags
                .get::<gst::tags::TrackCount>()
                .map(|t| t.get().to_owned());
            track_metadata.album_disc_number = tags
                .get::<gst::tags::AlbumVolumeNumber>()
                .map(|t| t.get().to_owned());
            track_metadata.album_disc_count = tags
                .get::<gst::tags::AlbumVolumeCount>()
                .map(|t| t.get().to_owned());

            // Duration
            if let Some(duration) = info.duration() {
                track_metadata.duration = Some(duration.seconds() as f32);
            }

            // Cache artwork
            if let Some(sample) = tags.get::<gst::tags::Image>() {
                track_metadata.artwork_filename = Self::cache_artwork(
                    sample.get(),
                    xdg_dirs.clone(),
                    regenerate_thumbnails,
                    refreshed_artwork_hashes,
                );
            } else if let Some(sample) = tags.get::<gst::tags::PreviewImage>() {
                track_metadata.artwork_filename = Self::cache_artwork(
                    sample.get(),
                    xdg_dirs.clone(),
                    regenerate_thumbnails,
                    refreshed_artwork_hashes,
                );
            }
        } else {
            // No metadata - use filename
            track_metadata.title = Some(file.to_string_lossy().to_string());
        }

        Ok(())
    }

    /// Cache album artwork to disk, avoiding duplicates
    fn cache_artwork(
        sample: gst::Sample,
        xdg_dirs: BaseDirectories,
        regenerate_thumbnails: bool,
        refreshed_artwork_hashes: &mut HashSet<String>,
    ) -> Option<String> {
        let buffer = sample.buffer()?;
        let caps = sample.caps()?;

        let extension = caps
            .structure(0)
            .and_then(|s| s.name().split('/').nth(1))
            .map(Self::mime_extension)
            .unwrap_or("jpg");

        let map = buffer.map_readable().ok()?;
        let bytes = map.as_slice();
        let hash = digest(bytes);
        let file_name = format!("{}.{}", hash, extension);

        let full_path = xdg_dirs
            .place_cache_file(format!("{}/{}", ARTWORK_DIR, file_name))
            .ok()?;

        // When regeneration is enabled, rewrite each unique artwork image once per scan.
        // Reusing the same flag for all variants keeps Original, Medium, and Small in sync
        // without repeatedly rewriting duplicate album art shared by many tracks.
        let force_artwork_cache = regenerate_thumbnails && refreshed_artwork_hashes.insert(hash);

        if force_artwork_cache || !Path::new(&full_path).exists() {
            let mut file = File::create(&full_path).ok()?;
            if let Err(err) = file.write_all(bytes) {
                eprintln!("Cannot save album artwork: {:?}", err);
                return None;
            }
        }

        if let Err(err) = Self::cache_artwork_thumbnails(
            bytes,
            extension,
            &file_name,
            &xdg_dirs,
            force_artwork_cache,
        ) {
            eprintln!(
                "Cannot save album artwork thumbnails for {:?}: {}",
                file_name, err
            );
        }

        Some(file_name)
    }

    fn cache_artwork_thumbnails(
        bytes: &[u8],
        extension: &str,
        original_file_name: &str,
        xdg_dirs: &BaseDirectories,
        force: bool,
    ) -> Result<(), String> {
        let Some(format) = ImageFormat::from_extension(extension) else {
            return Ok(());
        };

        if !force
            && Self::artwork_thumbnail_exists(original_file_name, ARTWORK_MEDIUM_SUFFIX, xdg_dirs)?
            && Self::artwork_thumbnail_exists(original_file_name, ARTWORK_SMALL_SUFFIX, xdg_dirs)?
        {
            return Ok(());
        }

        let image = image::load_from_memory_with_format(bytes, format)
            .or_else(|_| image::load_from_memory(bytes))
            .map_err(|err| format!("failed to decode artwork: {err}"))?;

        Self::cache_artwork_thumbnail(
            &image,
            format.clone(),
            original_file_name,
            ARTWORK_MEDIUM_SUFFIX,
            ARTWORK_MEDIUM_SIZE,
            xdg_dirs,
            force,
        )?;
        Self::cache_artwork_thumbnail(
            &image,
            format,
            original_file_name,
            ARTWORK_SMALL_SUFFIX,
            ARTWORK_SMALL_SIZE,
            xdg_dirs,
            force,
        )?;

        Ok(())
    }

    fn artwork_thumbnail_exists(
        original_file_name: &str,
        suffix: &str,
        xdg_dirs: &BaseDirectories,
    ) -> Result<bool, String> {
        let file_name = artwork_variant_filename(original_file_name, suffix);
        let full_path = xdg_dirs
            .place_cache_file(format!("{}/{}", ARTWORK_DIR, file_name))
            .map_err(|err| format!("failed to place cache thumbnail: {err}"))?;

        Ok(Path::new(&full_path).exists())
    }

    fn cache_artwork_thumbnail(
        image: &DynamicImage,
        format: ImageFormat,
        original_file_name: &str,
        suffix: &str,
        size: u32,
        xdg_dirs: &BaseDirectories,
        force: bool,
    ) -> Result<(), String> {
        let file_name = artwork_variant_filename(original_file_name, suffix);
        let full_path = xdg_dirs
            .place_cache_file(format!("{}/{}", ARTWORK_DIR, file_name))
            .map_err(|err| format!("failed to place cache thumbnail: {err}"))?;

        if !force && Path::new(&full_path).exists() {
            return Ok(());
        }

        let thumbnail = image.thumbnail(size, size);
        let mut encoded = Cursor::new(Vec::new());
        thumbnail
            .write_to(&mut encoded, format)
            .map_err(|err| format!("failed to encode thumbnail: {err}"))?;

        let mut file =
            File::create(&full_path).map_err(|err| format!("failed to create thumbnail: {err}"))?;
        file.write_all(encoded.get_ref())
            .map_err(|err| format!("failed to write thumbnail: {err}"))?;

        Ok(())
    }

    fn mime_extension(mime_subtype: &str) -> &'static str {
        match mime_subtype {
            "jpeg" | "jpg" | "pjpeg" => "jpg",
            "png" | "x-png" => "png",
            "gif" => "gif",
            "webp" => "webp",
            "bmp" | "x-ms-bmp" => "bmp",
            "tiff" | "tif" => "tiff",
            _ => "jpg",
        }
    }
}

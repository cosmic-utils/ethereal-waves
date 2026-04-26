// SPDX-License-Identifier: GPL-3.0

use crate::constants::{ARTWORK_MEDIUM_THUMBNAIL_MAX_SIZE, ARTWORK_SMALL_THUMBNAIL_MAX_SIZE};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArtworkVariant {
    Original,
    Medium,
    Small,
}

impl ArtworkVariant {
    pub const ALL: [Self; 3] = [Self::Original, Self::Medium, Self::Small];

    fn cache_suffix(self) -> Option<&'static str> {
        match self {
            Self::Original => None,
            Self::Medium => Some("medium"),
            Self::Small => Some("small"),
        }
    }
}

pub fn variant_filename(original_filename: &str, variant: ArtworkVariant) -> String {
    match variant.cache_suffix() {
        Some(suffix) => suffixed_filename(original_filename, suffix),
        None => original_filename.to_string(),
    }
}

pub fn variant_filenames(original_filename: &str) -> [String; 3] {
    ArtworkVariant::ALL.map(|variant| variant_filename(original_filename, variant))
}

pub fn original_filename_for_variant(filename: &str) -> String {
    let Some((stem, extension)) = filename.rsplit_once('.') else {
        return filename.to_string();
    };

    for suffix in ["medium", "small"] {
        let marker = format!(".{suffix}");
        if let Some(original_stem) = stem.strip_suffix(&marker) {
            return format!("{original_stem}.{extension}");
        }
    }

    filename.to_string()
}

pub fn preferred_artwork_filename(original_filename: &str, requested_size: f32) -> String {
    if requested_size <= ARTWORK_SMALL_THUMBNAIL_MAX_SIZE as f32 {
        variant_filename(original_filename, ArtworkVariant::Small)
    } else if requested_size <= ARTWORK_MEDIUM_THUMBNAIL_MAX_SIZE as f32 {
        variant_filename(original_filename, ArtworkVariant::Medium)
    } else {
        original_filename.to_string()
    }
}

fn suffixed_filename(filename: &str, suffix: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            format!("{stem}.{suffix}.{extension}")
        }
        _ => format!("{filename}.{suffix}"),
    }
}

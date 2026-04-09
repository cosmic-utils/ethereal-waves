use std::{
    path::Path,
    time::{Duration, Instant},
};

/// Format seconds as MM:SS
pub fn format_time(seconds: f32) -> String {
    let total_seconds = seconds.max(0.0).floor() as u32;
    let minutes = total_seconds / 60;
    let secs = total_seconds % 60;

    format!("{minutes}:{secs:02}")
}

/// Format seconds as H:MM:SS when needed, otherwise M:SS
pub fn format_duration(seconds: f32) -> String {
    let total_seconds = seconds.max(0.0).floor() as u32;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

/// Format an optional duration as H:MM:SS when needed, otherwise M:SS
pub fn format_optional_duration(duration: Option<f32>) -> String {
    duration.map(format_duration).unwrap_or_default()
}

/// Format remaining time as -MM:SS
pub fn format_time_left(current: f32, duration: f32) -> String {
    let time_left = clamp(duration - current, 0.0, duration);

    format!("-{}", format_time(time_left))
}

/// Check if two instants represent a double-click
pub fn is_double_click(last: Instant, threshold_ms: u64) -> bool {
    Instant::now().duration_since(last) <= Duration::from_millis(threshold_ms)
}

/// Calculate row stride (height + divider)
pub fn calculate_row_stride(size_multiplier: f32, base_height: f32, divider_height: f32) -> f32 {
    (base_height * size_multiplier) + divider_height
}

/// Convert an optional value into a display string
pub fn optional_display<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// Return trimmed text when present and non-empty
pub fn non_empty_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Return trimmed text or a fallback label
pub fn fallback_text(value: Option<&str>, fallback: &str) -> String {
    non_empty_text(value).unwrap_or_else(|| fallback.to_string())
}

/// Return a readable display name for a file path
pub fn path_display_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Join non-empty strings with a separator
pub fn join_non_empty(parts: &[&str], separator: &str) -> String {
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}

/// Clamp a value between min and max
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

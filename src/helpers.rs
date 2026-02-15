use std::time::{Duration, Instant};

/// Format seconds as MM:SS
pub fn format_time(seconds: f32) -> String {
    let minutes = (seconds / 60.0) as u32;
    let secs = f32::trunc(seconds) as u32 - (minutes * 60);
    format!("{}:{:02}", minutes, secs)
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

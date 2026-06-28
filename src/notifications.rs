// SPDX-License-Identifier: GPL-3.0

use crate::app::APP_ID;
use std::collections::HashMap;
use zbus::{proxy, zvariant::Value};

const APP_NAME: &str = "Ethereal Waves";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationSlot {
    Playback,
    Library,
    Playlist,
}

#[derive(Debug, Clone)]
pub enum AppNotification {
    NowPlaying {
        title: String,
        artist: Option<String>,
        album: Option<String>,
        image_uri: Option<String>,
    },

    LibraryUpdateComplete {
        total: usize,
        added: usize,
        removed: usize,
    },

    LibraryUpdateCancelled,

    PlaylistCreated {
        name: String,
    },

    PlaylistRenamed {
        old_name: String,
        new_name: String,
    },

    PlaylistDeleted {
        name: String,
    },

    PlaylistTracksAdded {
        playlist_name: String,
        added: usize,
        skipped_duplicates: usize,
    },

    PlaylistTracksRemoved {
        playlist_name: String,
        removed: usize,
    },
}

struct NotificationPayload {
    summary: String,
    body: String,
    category: &'static str,
    image_uri: Option<String>,
    expire_timeout: i32,
}

impl AppNotification {
    pub fn slot(&self) -> NotificationSlot {
        match self {
            AppNotification::NowPlaying { .. } => NotificationSlot::Playback,
            AppNotification::LibraryUpdateComplete { .. }
            | AppNotification::LibraryUpdateCancelled => NotificationSlot::Library,
            AppNotification::PlaylistCreated { .. }
            | AppNotification::PlaylistRenamed { .. }
            | AppNotification::PlaylistDeleted { .. }
            | AppNotification::PlaylistTracksAdded { .. }
            | AppNotification::PlaylistTracksRemoved { .. } => NotificationSlot::Playlist,
        }
    }

    fn payload(&self) -> NotificationPayload {
        match self {
            AppNotification::NowPlaying {
                title,
                artist,
                album,
                image_uri,
            } => {
                let body = [artist.as_deref(), album.as_deref()]
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.trim().is_empty())
                    .map(escape_notification_text)
                    .collect::<Vec<_>>()
                    .join("\n");

                NotificationPayload {
                    summary: escape_notification_text(title),
                    body,
                    category: "x-ethereal-waves.now-playing",
                    image_uri: image_uri.clone(),
                    expire_timeout: 5_000,
                }
            }

            AppNotification::LibraryUpdateComplete {
                total,
                added,
                removed,
            } => NotificationPayload {
                summary: "Library updated".to_string(),
                body: format!(
                    "{}\n{} added, {} removed",
                    track_count(*total),
                    added,
                    removed
                ),
                category: "x-ethereal-waves.library",
                image_uri: None,
                expire_timeout: 8_000,
            },

            AppNotification::LibraryUpdateCancelled => NotificationPayload {
                summary: "Library update cancelled".to_string(),
                body: String::new(),
                category: "x-ethereal-waves.library",
                image_uri: None,
                expire_timeout: 5_000,
            },

            AppNotification::PlaylistCreated { name } => NotificationPayload {
                summary: "Playlist created".to_string(),
                body: escape_notification_text(name),
                category: "x-ethereal-waves.playlist",
                image_uri: None,
                expire_timeout: 5_000,
            },

            AppNotification::PlaylistRenamed { old_name, new_name } => NotificationPayload {
                summary: "Playlist renamed".to_string(),
                body: format!(
                    "{}\n→ {}",
                    escape_notification_text(old_name),
                    escape_notification_text(new_name)
                ),
                category: "x-ethereal-waves.playlist",
                image_uri: None,
                expire_timeout: 5_000,
            },

            AppNotification::PlaylistDeleted { name } => NotificationPayload {
                summary: "Playlist deleted".to_string(),
                body: escape_notification_text(name),
                category: "x-ethereal-waves.playlist",
                image_uri: None,
                expire_timeout: 5_000,
            },

            AppNotification::PlaylistTracksAdded {
                playlist_name,
                added,
                skipped_duplicates,
            } => {
                let mut body = format!(
                    "{} added to {}",
                    track_count(*added),
                    escape_notification_text(playlist_name)
                );

                if *skipped_duplicates > 0 {
                    body.push('\n');
                    body.push_str(&format!(
                        "{} skipped as duplicates",
                        track_count(*skipped_duplicates)
                    ));
                }

                NotificationPayload {
                    summary: "Playlist updated".to_string(),
                    body,
                    category: "x-ethereal-waves.playlist",
                    image_uri: None,
                    expire_timeout: 5_000,
                }
            }

            AppNotification::PlaylistTracksRemoved {
                playlist_name,
                removed,
            } => NotificationPayload {
                summary: "Playlist updated".to_string(),
                body: format!(
                    "{} removed from {}",
                    track_count(*removed),
                    escape_notification_text(playlist_name)
                ),
                category: "x-ethereal-waves.playlist",
                image_uri: None,
                expire_timeout: 5_000,
            },
        }
    }
}

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: &HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

pub async fn send(notification: AppNotification, replaces_id: u32) -> Result<u32, String> {
    let payload = notification.payload();

    let connection = zbus::Connection::session()
        .await
        .map_err(|err| format!("failed to connect to session bus: {err}"))?;

    let proxy = NotificationsProxy::new(&connection)
        .await
        .map_err(|err| format!("failed to create notification proxy: {err}"))?;

    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();

    hints.insert("desktop-entry", Value::new(APP_ID));
    hints.insert("category", Value::new(payload.category));
    hints.insert("urgency", Value::new(1u8));

    if let Some(image_uri) = payload.image_uri.as_deref() {
        hints.insert("image-path", Value::new(image_uri));
    }

    let actions: [&str; 0] = [];

    proxy
        .notify(
            APP_NAME,
            replaces_id,
            APP_ID,
            &payload.summary,
            &payload.body,
            &actions,
            &hints,
            payload.expire_timeout,
        )
        .await
        .map_err(|err| format!("failed to send notification: {err}"))
}

fn track_count(count: usize) -> String {
    match count {
        1 => "1 track".to_string(),
        n => format!("{n} tracks"),
    }
}

fn escape_notification_text(value: &str) -> String {
    value
        .trim()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

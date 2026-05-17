// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, MenuAction, Message, ViewMode};
use crate::constants::MENU_WIDGET_ID;
use crate::fl;
use crate::playback_state::RepeatMode;
use cosmic::Application;
use cosmic::{
    Element,
    widget::{
        menu::{self, ItemHeight, ItemWidth},
        responsive_menu_bar,
    },
};
use std::sync::LazyLock;

static MENU_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new(MENU_WIDGET_ID));

pub fn menu_bar<'a>(app: &AppModel) -> Element<'a, Message> {
    let selected_playlist = app
        .view_playlist
        .and_then(|id| app.playlist_service.get(id).ok());
    let has_playlist = selected_playlist.is_some();
    let selected_playlist_is_library = selected_playlist
        .as_ref()
        .is_some_and(|playlist| playlist.is_library());

    let selected_count = selected_playlist
        .as_ref()
        .map(|playlist| playlist.selected_iter().count())
        .unwrap_or(0);

    let repeat_one = app.state.repeat_mode == RepeatMode::One;
    let repeat_all = app.state.repeat_mode == RepeatMode::All;
    let list_view = app.config.view_mode == ViewMode::List;
    let grid_view = app.config.view_mode == ViewMode::Grid;

    let mut selected_playlist_list = Vec::new();
    let mut now_playing_playlist_list = Vec::new();

    // Add ordered playlists
    app.state.playlist_nav_order.iter().for_each(|p| {
        if let Ok(playlist) = app.playlist_service.get(*p) {
            selected_playlist_list.push(menu::Item::Button(
                playlist.name().to_string(),
                None,
                MenuAction::AddSelectedToPlaylist(playlist.id()),
            ));
            if app.playback_service.now_playing().is_some() {
                now_playing_playlist_list.push(menu::Item::Button(
                    playlist.name().to_string(),
                    None,
                    MenuAction::AddNowPlayingToPlaylist(playlist.id()),
                ));
            }
        }
    });
    // Add unordered playlists
    app.playlist_service
        .user_playlists()
        .filter(|p| !app.state.playlist_nav_order.contains(&p.id()))
        .for_each(|p| {
            selected_playlist_list.push(menu::Item::Button(
                p.name().to_string(),
                None,
                MenuAction::AddSelectedToPlaylist(p.id()),
            ));
            if app.playback_service.now_playing().is_some() {
                now_playing_playlist_list.push(menu::Item::Button(
                    p.name().to_string(),
                    None,
                    MenuAction::AddNowPlayingToPlaylist(p.id()),
                ));
            }
        });

    let file_items = vec![
        menu_button_optional(
            fl!("track-info"),
            MenuAction::TrackInfoPanel,
            selected_count > 0,
        ),
        menu::Item::Divider,
        menu::Item::Button(
            fl!("import-playlist-menu"),
            None,
            MenuAction::ImportPlaylist,
        ),
        menu_button_optional(
            fl!("export-playlist-menu"),
            MenuAction::ExportPlaylist,
            has_playlist,
        ),
        menu::Item::Divider,
        menu_button_optional(
            fl!("update-library"),
            MenuAction::UpdateLibrary,
            !app.is_updating,
        ),
        menu::Item::Divider,
        menu::Item::Button(fl!("quit"), None, MenuAction::Quit),
    ];

    let playlist_items = vec![
        menu::Item::Button(fl!("new-playlist-menu"), None, MenuAction::NewPlaylist),
        menu_button_optional(
            fl!("rename-playlist-menu"),
            MenuAction::RenamePlaylist,
            has_playlist && !selected_playlist_is_library,
        ),
        menu_button_optional(
            fl!("delete-playlist-menu"),
            MenuAction::DeletePlaylist,
            has_playlist && !selected_playlist_is_library,
        ),
        menu::Item::Divider,
        menu::Item::Folder(fl!("add-selected-to"), selected_playlist_list),
        menu_button_optional(
            fl!("remove-selected"),
            MenuAction::RemoveSelectedFromPlaylist,
            has_playlist && !selected_playlist_is_library,
        ),
        menu::Item::Divider,
        menu::Item::Folder(fl!("add-now-playing-to"), now_playing_playlist_list),
        menu::Item::Divider,
        menu::Item::Button(fl!("select-all"), None, MenuAction::SelectAll),
        menu::Item::Divider,
        menu_button_optional(fl!("move-up"), MenuAction::MoveNavUp, has_playlist),
        menu_button_optional(fl!("move-down"), MenuAction::MoveNavDown, has_playlist),
    ];

    let mute_label = if app.state.muted {
        fl!("unmute")
    } else {
        fl!("mute")
    };

    let playback_items = vec![
        menu::Item::Button(fl!("volume-up"), None, MenuAction::VolumeUp),
        menu::Item::Button(fl!("volume-down"), None, MenuAction::VolumeDown),
        menu::Item::Button(mute_label, None, MenuAction::ToggleMute),
        menu::Item::Divider,
        menu::Item::CheckBox(
            fl!("shuffle"),
            None,
            app.state.shuffle,
            MenuAction::ToggleShuffle,
        ),
        menu::Item::CheckBox(
            fl!("repeat"),
            None,
            app.state.repeat,
            MenuAction::ToggleRepeat,
        ),
        menu::Item::Divider,
        menu::Item::CheckBox(
            fl!("repeat-one"),
            None,
            repeat_one,
            MenuAction::ToggleRepeatMode,
        ),
        menu::Item::CheckBox(
            fl!("repeat-all"),
            None,
            repeat_all,
            MenuAction::ToggleRepeatMode,
        ),
    ];

    let view_items = vec![
        menu::Item::CheckBox(
            fl!("list-view"),
            None,
            list_view,
            MenuAction::SetViewMode(ViewMode::List),
        ),
        menu::Item::CheckBox(
            fl!("grid-view"),
            None,
            grid_view,
            MenuAction::SetViewMode(ViewMode::Grid),
        ),
        menu::Item::Divider,
        menu::Item::Button(fl!("zoom-in"), None, MenuAction::ZoomIn),
        menu::Item::Button(fl!("zoom-out"), None, MenuAction::ZoomOut),
        menu::Item::Divider,
        menu::Item::Button(fl!("settings-menu"), None, MenuAction::Settings),
        menu::Item::Divider,
        menu::Item::Button(fl!("about-ethereal-waves"), None, MenuAction::About),
    ];

    responsive_menu_bar()
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Uniform(250))
        .spacing(1.0)
        .into_element(
            app.core(),
            &app.key_binds,
            MENU_ID.clone(),
            Message::Surface,
            vec![
                (fl!("file"), file_items),
                (fl!("playlist"), playlist_items),
                (fl!("playback"), playback_items),
                (fl!("view"), view_items),
            ],
        )
}

const fn menu_button_optional(
    label: String,
    action: MenuAction,
    enabled: bool,
) -> menu::Item<MenuAction, String> {
    if enabled {
        menu::Item::Button(label, None, action)
    } else {
        menu::Item::ButtonDisabled(label, None, action)
    }
}

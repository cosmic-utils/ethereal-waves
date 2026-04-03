// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, GridViewModel, Message, SortBy, SortDirection, TrackDropData};
use crate::constants::*;
use crate::fl;
use crate::playlist::Track;
use cosmic::{
    Element, cosmic_theme,
    iced::{
        Alignment, Color, Length, Size,
        clipboard::dnd::DndAction,
        font::{Font, Weight},
    },
    iced_core::{text::Wrapping, widget::Tree},
    theme, widget,
};
use std::sync::Arc;

pub fn content<'a>(app: &'a AppModel) -> Element<'a, Message> {
    let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

    let sort_controls = widget::row()
        .width(Length::Fill)
        .padding([space_xxs, GRID_VIEW_PADDING as u16])
        .spacing(space_xxs)
        .align_y(Alignment::Center)
        .push(widget::dropdown(
            grid_sort_options(),
            grid_sort_selected(&app.state.sort_by),
            grid_sort_message,
        ))
        .push(grid_sort_direction_toggle(&app.state.sort_direction));

    widget::column()
        .push(sort_controls)
        .push(widget::divider::horizontal::default())
        .push(
            widget::container(widget::responsive(move |size| {
                scroll_content_responsive(app, size)
            }))
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn scroll_content_responsive<'a>(app: &'a AppModel, size: Size) -> Element<'a, Message> {
    let Some(view_model) = app.calculate_grid_view(size) else {
        return empty_scroller(app);
    };

    let Some(active_playlist) = app
        .view_playlist
        .and_then(|playlist_id| app.playlist_service.get(playlist_id).ok())
    else {
        return empty_scroller(app);
    };

    let active_tracks = active_playlist.tracks();
    let mut rows = widget::column();

    if view_model.top_spacer_height > 0.0 {
        rows = rows
            .push(widget::space::vertical().height(Length::Fixed(view_model.top_spacer_height)));
    }

    let visible_row_total = if view_model.visible_track_indices.is_empty() {
        0
    } else {
        (view_model.visible_track_indices.len() - 1) / view_model.column_count + 1
    };

    for (row_index, row_chunk) in view_model
        .visible_track_indices
        .chunks(view_model.column_count)
        .enumerate()
    {
        let mut grid_row = widget::row()
            .padding([0, view_model.view_padding as u16])
            .width(Length::Fill)
            .align_y(Alignment::Start);

        for column_index in 0..view_model.column_count {
            if let Some(&playlist_index) = row_chunk.get(column_index) {
                let track = &active_tracks[playlist_index];
                grid_row = grid_row.push(track_card(app, track, &view_model, playlist_index));
            } else {
                grid_row = grid_row
                    .push(widget::space::horizontal().width(Length::Fixed(view_model.card_width)));
            }

            if column_index + 1 < view_model.column_count {
                grid_row = grid_row.push(widget::space::horizontal().width(Length::Fill));
            }
        }

        rows = rows.push(grid_row);

        if row_index + 1 < visible_row_total {
            rows =
                rows.push(widget::space::vertical().height(Length::Fixed(view_model.item_spacing)));
        }
    }

    if view_model.bottom_spacer_height > 0.0 {
        rows = rows.push(
            widget::space::vertical().height(Length::Fixed(view_model.bottom_spacer_height)),
        );
    }

    widget::scrollable(rows)
        .id(app.list_scroll_id.clone())
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|viewport| Message::GridViewScroll(viewport))
        .into()
}

fn empty_scroller<'a>(app: &'a AppModel) -> Element<'a, Message> {
    widget::scrollable(widget::column())
        .id(app.list_scroll_id.clone())
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll(|viewport| Message::GridViewScroll(viewport))
        .into()
}

fn grid_sort_options() -> Vec<String> {
    vec![
        fl!("title"),
        fl!("album"),
        fl!("artist"),
        fl!("album-artist"),
    ]
}

fn grid_sort_selected(sort_by: &SortBy) -> Option<usize> {
    match sort_by {
        SortBy::Title => Some(0),
        SortBy::Album => Some(1),
        SortBy::Artist => Some(2),
        SortBy::AlbumArtist => Some(3),
        _ => None,
    }
}

fn grid_sort_message(index: usize) -> Message {
    Message::GridViewSort(match index {
        0 => SortBy::Title,
        1 => SortBy::Album,
        2 => SortBy::Artist,
        3 => SortBy::AlbumArtist,
        _ => SortBy::Title,
    })
}

fn grid_sort_direction_toggle<'a>(
    active_direction: &SortDirection,
) -> Element<'a, Message> {
    widget::button::icon(
        widget::icon::from_name(grid_sort_direction_icon_name(active_direction)).size(16),
    )
    .extra_small()
    .on_press(Message::GridViewSortDirection(
        grid_sort_direction_toggled(active_direction),
    ))
    .into()
}

fn grid_sort_direction_icon_name(direction: &SortDirection) -> &'static str {
    match direction {
        SortDirection::Ascending => "view-sort-ascending-symbolic",
        SortDirection::Descending => "view-sort-descending-symbolic",
    }
}

fn grid_sort_direction_toggled(direction: &SortDirection) -> SortDirection {
    match direction {
        SortDirection::Ascending => SortDirection::Descending,
        SortDirection::Descending => SortDirection::Ascending,
    }
}

fn track_card<'a>(
    app: &'a AppModel,
    track: &Track,
    view_model: &GridViewModel,
    playlist_index: usize,
) -> Element<'a, Message> {
    let track_id = track.instance_id();
    let is_in_library = app.library.media.contains_key(&track.path);
    let is_playing_track = app.is_track_playing(track, view_model.is_playing_playlist);
    let title = track_title(track);
    let subtitle = track_subtitle(track, is_in_library);
    let duration = format_duration(track.metadata.duration);

    let artwork = artwork_element(app, track, view_model.artwork_size, is_in_library);

    let status_icon: Element<'a, Message> = if is_playing_track {
        widget::container(
            widget::icon::from_name("media-playback-start-symbolic").size(GRID_STATUS_ICON_SIZE),
        )
        .width(Length::Fixed(GRID_STATUS_ICON_SLOT))
        .height(Length::Fixed(GRID_INFO_HEIGHT))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
    } else if !is_in_library {
        widget::container(
            widget::icon::from_name("help-about-symbolic").size(GRID_STATUS_ICON_SIZE),
        )
        .width(Length::Fixed(GRID_STATUS_ICON_SLOT))
        .height(Length::Fixed(GRID_INFO_HEIGHT))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
    } else {
        widget::space::horizontal()
            .width(Length::Fixed(GRID_STATUS_ICON_SLOT))
            .into()
    };

    let info_row = widget::row()
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(status_icon)
        .push(widget::space::horizontal().width(Length::Fill))
        .push(widget::text(duration));

    let card_contents = widget::column()
        .width(Length::Fixed(view_model.card_width))
        .spacing(GRID_CARD_CONTENT_SPACING as u16)
        .align_x(Alignment::Center)
        .push(artwork)
        .push(
            widget::container(
                widget::text(title)
                    .wrapping(Wrapping::WordOrGlyph)
                    .font(Font {
                        weight: Weight::Bold,
                        ..Font::default()
                    })
                    .width(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fixed(GRID_TITLE_HEIGHT))
            .clip(true),
        )
        .push(
            widget::container(
                widget::text(subtitle)
                    .wrapping(Wrapping::WordOrGlyph)
                    .width(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fixed(GRID_SUBTITLE_HEIGHT))
            .clip(true),
        )
        .push(
            widget::container(info_row)
                .width(Length::Fill)
                .height(Length::Fixed(GRID_INFO_HEIGHT)),
        );

    let card = widget::container(card_contents)
        .width(Length::Fixed(view_model.card_width))
        .height(Length::Fixed(view_model.card_height))
        .padding(view_model.card_padding as u16)
        .clip(true);

    let row_button = widget::button::custom(card)
        .class(button_style(track.selected))
        .on_press_down(Message::ChangeTrack(playlist_index))
        .padding(0)
        .width(Length::Fixed(view_model.card_width));

    let row_mouse =
        widget::mouse_area(row_button).on_release(Message::ListSelectRow(playlist_index));

    let selected_count = view_model.selected_track_ids.len();
    let drag_count = if track.selected && selected_count > 0 {
        selected_count
    } else {
        1
    };
    let drag_label = if drag_count == 1 {
        fl!("one-track-selected")
    } else {
        format!("{drag_count} {}", fl!("tracks-selected"))
    };

    let on_start = if track.selected {
        None
    } else {
        Some(Message::ListSelectRow(playlist_index))
    };

    let drag_ids = if track.selected && selected_count > 0 {
        Arc::clone(&view_model.selected_track_ids)
    } else {
        Arc::new(vec![track_id.clone()])
    };

    widget::dnd_source::DndSource::new(row_mouse)
        .drag_content(move || TrackDropData::new((*drag_ids).clone()))
        .action(DndAction::Copy)
        .on_start(on_start)
        .drag_icon(move |_offset| {
            let badge: cosmic::Element<'static, ()> = widget::layer_container(
                widget::column().push(
                    widget::row()
                        .push(widget::text::body(drag_label.clone()))
                        .padding([6, 10]),
                ),
            )
            .layer(cosmic_theme::Layer::Primary)
            .into();
            let state = Tree::new(&badge).state;
            let new_offset = cosmic::iced::Vector::new(20.0, 20.0);
            (badge, state, new_offset)
        })
        .drag_threshold(1.0)
        .into()
}

fn artwork_element<'a>(
    app: &'a AppModel,
    track: &Track,
    artwork_size: f32,
    is_in_library: bool,
) -> Element<'a, Message> {
    if let Some(artwork_filename) = &track.metadata.artwork_filename {
        app.image_store.request(artwork_filename.clone());
        if let Some(handle) = app.image_store.get(artwork_filename) {
            return widget::container(
                widget::image(handle.as_ref())
                    .width(Length::Fixed(artwork_size))
                    .height(Length::Fixed(artwork_size)),
            )
            .width(Length::Fixed(artwork_size))
            .height(Length::Fixed(artwork_size))
            .into();
        }
    }

    let placeholder_icon = if is_in_library {
        "audio-x-generic-symbolic"
    } else {
        "help-about-symbolic"
    };

    widget::layer_container(
        widget::container(
            widget::icon::from_name(placeholder_icon).size((artwork_size * 0.4) as u16),
        )
        .width(Length::Fixed(artwork_size))
        .height(Length::Fixed(artwork_size))
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .layer(cosmic_theme::Layer::Secondary)
    .width(Length::Fixed(artwork_size))
    .height(Length::Fixed(artwork_size))
    .into()
}

fn track_title(track: &Track) -> String {
    track
        .metadata
        .title
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| {
            track
                .path
                .file_stem()
                .or_else(|| track.path.file_name())
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}

fn track_subtitle(track: &Track, is_in_library: bool) -> String {
    if !is_in_library {
        return fl!("not-in-library");
    }

    let artist = track
        .metadata
        .artist
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .or_else(|| {
            track
                .metadata
                .album_artist
                .as_ref()
                .filter(|value| !value.trim().is_empty())
                .cloned()
        });
    let album = track
        .metadata
        .album
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .cloned();

    match (artist, album) {
        (Some(artist), Some(album)) => format!("{artist} • {album}"),
        (Some(artist), None) => artist,
        (None, Some(album)) => album,
        (None, None) => String::new(),
    }
}

fn format_duration(duration: Option<f32>) -> String {
    let Some(duration) = duration else {
        return String::new();
    };

    let total_seconds = duration.floor() as u32;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn button_style(selected: bool) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_focus, theme| button_appearance(theme, selected, false)),
        disabled: Box::new(move |theme| button_appearance(theme, selected, false)),
        hovered: Box::new(move |_focus, theme| button_appearance(theme, selected, true)),
        pressed: Box::new(move |_focus, theme| button_appearance(theme, selected, false)),
    }
}

fn button_appearance(theme: &theme::Theme, selected: bool, hovered: bool) -> widget::button::Style {
    let cosmic = theme.cosmic();
    let mut appearance = widget::button::Style::new();

    if selected {
        appearance.background = Some(Color::from(cosmic.accent_color()).into());
        appearance.icon_color = Some(Color::from(cosmic.on_accent_color()));
        appearance.text_color = Some(Color::from(cosmic.on_accent_color()));
    } else if hovered {
        appearance.background = Some(Color::from(cosmic.bg_component_color()).into());
        appearance.icon_color = Some(Color::from(cosmic.on_bg_component_color()));
        appearance.text_color = Some(Color::from(cosmic.on_bg_component_color()));
    } else {
        appearance.background = Some(Color::TRANSPARENT.into());
        appearance.icon_color = Some(Color::from(cosmic.on_bg_color()));
        appearance.text_color = Some(Color::from(cosmic.on_bg_color()));
    }

    appearance.outline_width = 0.0;
    appearance.border_width = 0.0;
    appearance.border_radius = cosmic.radius_xs().into();
    appearance
}

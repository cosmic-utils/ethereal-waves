// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, GridCardModel, Message, SortBy, SortDirection, TrackDropData};
use crate::config::GridGroupBy;
use crate::constants::*;
use crate::fl;
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
        .push(widget::text(fl!("group-by")))
        .push(widget::dropdown(
            grid_group_options(),
            grid_group_selected(&app.config.grid_group_by),
            grid_group_message,
        ))
        .push(widget::divider::vertical::default().height(Length::Fixed(20.0)))
        .push(widget::text(fl!("sort-by")))
        .push(widget::dropdown(
            grid_sort_options(),
            grid_sort_selected(&app.state.sort_by),
            grid_sort_message,
        ))
        .push(widget::divider::vertical::default().height(Length::Fixed(20.0)))
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

    let column_count = view_model.column_count;
    let card_width = view_model.card_width;
    let card_height = view_model.card_height;
    let artwork_size = view_model.artwork_size;
    let card_padding = view_model.card_padding;
    let item_spacing = view_model.item_spacing;
    let view_padding = view_model.view_padding;
    let top_spacer_height = view_model.top_spacer_height;
    let bottom_spacer_height = view_model.bottom_spacer_height;
    let visible_cards = view_model.visible_cards.clone();
    let selected_track_ids = Arc::clone(&view_model.selected_track_ids);

    let mut rows = widget::column();

    if top_spacer_height > 0.0 {
        rows = rows.push(widget::space::vertical().height(Length::Fixed(top_spacer_height)));
    }

    let visible_row_total = if visible_cards.is_empty() {
        0
    } else {
        (visible_cards.len() - 1) / column_count + 1
    };

    for (row_index, row_chunk) in visible_cards.chunks(column_count).enumerate() {
        let mut grid_row = widget::row()
            .padding([0, view_padding as u16])
            .width(Length::Fill)
            .align_y(Alignment::Start);

        for column_index in 0..column_count {
            if let Some(card) = row_chunk.get(column_index) {
                grid_row = grid_row.push(grid_card(
                    app,
                    card.clone(),
                    Arc::clone(&selected_track_ids),
                    card_width,
                    card_height,
                    artwork_size,
                    card_padding,
                ));
            } else {
                grid_row =
                    grid_row.push(widget::space::horizontal().width(Length::Fixed(card_width)));
            }

            if column_index + 1 < column_count {
                grid_row = grid_row.push(widget::space::horizontal().width(Length::Fill));
            }
        }

        rows = rows.push(grid_row);

        if row_index + 1 < visible_row_total {
            rows = rows.push(widget::space::vertical().height(Length::Fixed(item_spacing)));
        }
    }

    if bottom_spacer_height > 0.0 {
        rows = rows.push(widget::space::vertical().height(Length::Fixed(bottom_spacer_height)));
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

fn grid_group_options() -> Vec<String> {
    vec![
        "Track".to_string(),
        fl!("album"),
        fl!("artist"),
        fl!("album-artist"),
    ]
}

fn grid_group_selected(group_by: &GridGroupBy) -> Option<usize> {
    match group_by {
        GridGroupBy::Track => Some(0),
        GridGroupBy::Album => Some(1),
        GridGroupBy::Artist => Some(2),
        GridGroupBy::AlbumArtist => Some(3),
    }
}

fn grid_group_message(index: usize) -> Message {
    Message::GridViewGroupBy(match index {
        1 => GridGroupBy::Album,
        2 => GridGroupBy::Artist,
        3 => GridGroupBy::AlbumArtist,
        _ => GridGroupBy::Track,
    })
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

fn grid_sort_direction_toggle<'a>(active_direction: &SortDirection) -> Element<'a, Message> {
    let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

    let icon = widget::icon::from_name(grid_sort_direction_icon_name(active_direction))
        .icon()
        .size(16)
        .class(theme::Svg::custom(|theme| {
            cosmic::iced::widget::svg::Style {
                color: Some(Color::from(theme.cosmic().on_bg_color())),
            }
        }));

    widget::button::custom(
        widget::container(icon)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding(space_xxs)
    .class(grid_sort_button_style())
    .on_press(Message::GridViewSortDirection(grid_sort_direction_toggled(
        active_direction,
    )))
    .into()
}

fn grid_sort_button_style() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|_focus, theme| grid_sort_button_appearance(theme, false)),
        disabled: Box::new(|theme| grid_sort_button_appearance(theme, false)),
        hovered: Box::new(|_focus, theme| grid_sort_button_appearance(theme, true)),
        pressed: Box::new(|_focus, theme| grid_sort_button_appearance(theme, true)),
    }
}

fn grid_sort_button_appearance(theme: &theme::Theme, hovered: bool) -> widget::button::Style {
    let cosmic = theme.cosmic();
    let mut appearance = widget::button::Style::new();

    if hovered {
        appearance.background = Some(Color::from(cosmic.bg_component_color()).into());
        appearance.icon_color = Some(Color::from(cosmic.on_bg_component_color()));
        appearance.text_color = Some(Color::from(cosmic.on_bg_component_color()));
    } else {
        appearance.background = Some(Color::TRANSPARENT.into());
        appearance.icon_color = Some(Color::from(cosmic.on_bg_color()));
        appearance.text_color = Some(Color::from(cosmic.on_bg_color()));
    }

    // appearance.outline_width = 0.0;
    // appearance.border_width = 0.0;
    appearance
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

fn grid_card<'a>(
    app: &'a AppModel,
    card: GridCardModel,
    selected_track_ids: Arc<Vec<String>>,
    card_width: f32,
    card_height: f32,
    artwork_size: f32,
    card_padding: f32,
) -> Element<'a, Message> {
    let GridCardModel {
        title,
        subtitle,
        info_text,
        duration_text,
        artwork_filename,
        playlist_indices,
        track_ids,
        selected,
        is_playing,
        has_available_track,
        has_missing_tracks,
    } = card;

    if playlist_indices.is_empty() {
        return widget::space::horizontal()
            .width(Length::Fixed(card_width))
            .into();
    }

    let primary_index = playlist_indices[0];
    let press_message = if playlist_indices.len() == 1 {
        Message::ChangeTrack(primary_index)
    } else {
        Message::ChangeTracks(Arc::clone(&playlist_indices))
    };
    let release_message = if playlist_indices.len() == 1 {
        Message::ListSelectRow(primary_index)
    } else {
        Message::ListSelectRows(Arc::clone(&playlist_indices))
    };

    let artwork = artwork_element(
        app,
        artwork_filename.as_ref(),
        artwork_size,
        has_available_track,
    );

    let status_icon: Element<'a, Message> = if is_playing {
        widget::container(
            widget::icon::from_name("media-playback-start-symbolic").size(GRID_STATUS_ICON_SIZE),
        )
        .width(Length::Fixed(GRID_STATUS_ICON_SLOT))
        .height(Length::Fixed(GRID_INFO_HEIGHT))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
    } else if has_missing_tracks {
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

    let mut info_row = widget::row()
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(status_icon);

    if !info_text.is_empty() {
        info_row = info_row.push(widget::text(info_text.clone()));
    }

    info_row = info_row.push(widget::space::horizontal().width(Length::Fill));

    if !duration_text.is_empty() {
        info_row = info_row.push(widget::text(duration_text.clone()));
    }

    let card_contents = widget::column()
        .width(Length::Fixed(card_width))
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
        .width(Length::Fixed(card_width))
        .height(Length::Fixed(card_height))
        .padding(card_padding as u16)
        .clip(true);

    let row_button = widget::button::custom(card)
        .class(button_style(selected))
        .on_press_down(press_message)
        .padding(0)
        .width(Length::Fixed(card_width));

    let row_mouse = widget::mouse_area(row_button).on_release(release_message.clone());

    let selected_count = selected_track_ids.len();
    let drag_count = if selected && selected_count > 0 {
        selected_count
    } else {
        track_ids.len()
    };
    let drag_label = if drag_count == 1 {
        fl!("one-track-selected")
    } else {
        format!("{drag_count} {}", fl!("tracks-selected"))
    };

    let on_start = if selected {
        None
    } else {
        Some(release_message)
    };

    let drag_ids = if selected && selected_count > 0 {
        selected_track_ids
    } else {
        track_ids
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
    artwork_filename: Option<&String>,
    artwork_size: f32,
    has_available_track: bool,
) -> Element<'a, Message> {
    if let Some(artwork_filename) = artwork_filename {
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

    let placeholder_icon = if has_available_track {
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

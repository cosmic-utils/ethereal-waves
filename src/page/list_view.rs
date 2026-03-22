// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, Message, SortBy, TrackDropData};
use crate::config::ListColumn;
use crate::constants::*;
use crate::fl;
use crate::playlist::Track;
use cosmic::{
    cosmic_theme,
    iced::{Alignment, Color, Length, clipboard::dnd::DndAction},
    iced_core::widget::Tree,
    theme, widget,
};

pub fn content<'a>(app: &AppModel) -> widget::Column<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xxxs,
        ..
    } = theme::active().cosmic().spacing;

    // Get pre-calculated view model with all list view data
    let Some(view_model) = app.calculate_list_view() else {
        return widget::column();
    };

    let mut content = widget::column();
    let visible_columns: Vec<ListColumn> = app
        .config
        .normalized_list_column_order()
        .into_iter()
        .filter(|column| column.is_visible(&app.config))
        .collect();

    let track_number_label = fl!("track-number-short");
    let max_track_number_chars = view_model
        .visible_tracks
        .iter()
        .filter_map(|(_, track)| track.metadata.track_number)
        .max()
        .map(|track_number| track_number.to_string().len())
        .unwrap_or(0);
    let track_number_column_width = max_track_number_chars
        .max(track_number_label.chars().count())
        .max(2) as f32
        * 11.0
        + 8.0;

    // Header row
    let mut header_row = widget::row()
        .spacing(space_xxs)
        .push(widget::space::horizontal().width(space_xxxs))
        .push(widget::space::horizontal().width(Length::Fixed(view_model.icon_column_width)))
        .push(
            widget::text::heading("#")
                .align_x(Alignment::End)
                .width(Length::Fixed(view_model.number_column_width)),
        );

    for column in &visible_columns {
        header_row = header_row.push(list_column_header(
            app,
            &view_model,
            *column,
            &track_number_label,
            track_number_column_width,
            space_xxs,
        ));
    }

    content = content.push(header_row.push(widget::space::horizontal().width(space_xxs)));
    content = content.push(widget::divider::horizontal::default());

    // Build rows
    let mut rows = widget::column();
    rows = rows.push(widget::space::vertical().height(Length::Fixed(
        view_model.list_start as f32 * view_model.row_stride,
    )));

    let mut count: u32 = view_model.list_start as u32 + 1;

    let selected_track_ids: Vec<String> = view_model
        .visible_tracks
        .iter()
        .filter(|(_, t)| t.selected)
        .map(|(_, t)| t.instance_id())
        .collect();

    let selected_count = selected_track_ids.len();

    for (index, track) in view_model
        .visible_tracks
        .iter()
        .skip(view_model.list_start)
        .take(view_model.take)
        .enumerate()
    {
        let track_id = track.1.instance_id();
        let is_playing_track = app.is_track_playing(&track.1, &view_model);

        let mut row_element = widget::row()
            .spacing(space_xxs)
            .height(Length::Fixed(view_model.row_height));

        // Play icon column
        if is_playing_track {
            row_element = row_element.push(
                widget::container(
                    widget::icon::from_name("media-playback-start-symbolic").size(16),
                )
                .width(Length::Fixed(view_model.icon_column_width))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .height(view_model.row_height),
            );
        } else {
            let is_in_library = app.library.media.contains_key(&track.1.path);

            if !is_in_library {
                // Track is not in library, show indicator
                let icon_with_indicator = widget::row()
                    .spacing(2)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("help-about-symbolic").size(16));

                row_element = row_element.push(
                    widget::container(icon_with_indicator)
                        .width(Length::Fixed(view_model.icon_column_width))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .height(view_model.row_height),
                );
            } else {
                row_element = row_element.push(
                    widget::space::horizontal().width(Length::Fixed(view_model.icon_column_width)),
                );
            }
        }

        // Row number
        row_element = row_element.push(
            widget::container(
                widget::text(count.to_string())
                    .width(Length::Fixed(view_model.number_column_width))
                    .align_x(Alignment::End)
                    .align_y(view_model.row_align)
                    .height(view_model.row_height),
            )
            .clip(true),
        );

        for column in &visible_columns {
            row_element = row_element.push(list_column_cell(
                &track.1,
                &view_model,
                *column,
                track_number_column_width,
            ));
        }

        row_element = row_element.width(Length::Fill);

        let row_button = widget::button::custom(row_element)
            .class(button_style(track.1.selected, false))
            .on_press_down(Message::ChangeTrack(track.0))
            .padding(0)
            .width(Length::Fill);

        let row_mouse = widget::mouse_area(row_button).on_release(Message::ListSelectRow(track.0));

        let drag_count = if track.1.selected && selected_count > 0 {
            selected_count
        } else {
            1
        };

        let drag_label = if drag_count == 1 {
            fl!("one-track-selected")
        } else {
            format!("{drag_count} {}", fl!("tracks-selected"))
        };

        // If user drags an unselected row, select it when drag begins.
        // If they drag a selected row, preserve multi-selection.
        let on_start = if track.1.selected {
            None
        } else {
            Some(Message::ListSelectRow(track.0))
        };

        // Drag all selected row ids or just the current row id
        let drag_ids: Vec<String> = if track.1.selected && !selected_track_ids.is_empty() {
            selected_track_ids.clone()
        } else {
            vec![track_id.clone()]
        };

        let draggable_row = widget::dnd_source::DndSource::new(row_mouse)
            .drag_content(move || TrackDropData::new(drag_ids.clone()))
            .action(DndAction::Copy)
            .on_start(on_start)
            .drag_icon(move |_offset| {
                // Visual elements next to the cursor
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
            .drag_threshold(1.0);

        rows = rows.push(draggable_row);

        let visible_count = view_model.list_end.saturating_sub(view_model.list_start);
        let is_last_visible = index + 1 == visible_count;
        if !is_last_visible {
            rows = rows.push(
                widget::container(widget::divider::horizontal::default())
                    .height(Length::Fixed(DIVIDER_HEIGHT))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .clip(true),
            );
        }

        count += 1;
    }

    let scrollable_contents = widget::row()
        .push(widget::space::vertical().height(Length::Fixed(view_model.viewport_height)))
        .push(widget::space::horizontal().width(space_xxs))
        .push(rows)
        .push(widget::space::horizontal().width(space_xxs));

    let scroller = widget::scrollable(scrollable_contents)
        .id(app.list_scroll_id.clone())
        .width(Length::Fill)
        .on_scroll(|viewport| Message::ListViewScroll(viewport));

    content = content.push(scroller);

    content
}

fn list_column_header<'a>(
    app: &AppModel,
    view_model: &crate::app::ListViewModel,
    column: ListColumn,
    track_number_label: &str,
    track_number_column_width: f32,
    spacing: u16,
) -> cosmic::Element<'a, Message> {
    let label = list_column_heading(column, track_number_label);
    let width = list_column_width(column, track_number_column_width);

    match column.sort_by() {
        Some(sort_by) => create_sort_button(
            label,
            sort_by,
            &app.state,
            &view_model.sort_direction_icon,
            spacing,
            width,
        )
        .into(),
        None => widget::text::heading(label)
            .align_x(Alignment::End)
            .width(width)
            .into(),
    }
}

fn list_column_cell<'a>(
    track: &Track,
    view_model: &crate::app::ListViewModel,
    column: ListColumn,
    track_number_column_width: f32,
) -> cosmic::Element<'a, Message> {
    match column {
        ListColumn::TrackNumber => compact_text_cell(
            track
                .metadata
                .track_number
                .map(|track_number| track_number.to_string())
                .unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::Title => fill_text_cell(
            track
                .metadata
                .title
                .clone()
                .unwrap_or_else(|| track.path.to_string_lossy().to_string()),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::Album => fill_text_cell(
            track.metadata.album.clone().unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::Artist => fill_text_cell(
            track.metadata.artist.clone().unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::AlbumArtist => fill_text_cell(
            track.metadata.album_artist.clone().unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::TrackTotal => compact_text_cell(
            track
                .metadata
                .track_count
                .map(|track_total| track_total.to_string())
                .unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::DiscNumber => compact_text_cell(
            track
                .metadata
                .album_disc_number
                .map(|disc_number| disc_number.to_string())
                .unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::DiscTotal => compact_text_cell(
            track
                .metadata
                .album_disc_count
                .map(|disc_total| disc_total.to_string())
                .unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::Genre => fill_text_cell(
            track.metadata.genre.clone().unwrap_or_default(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::FilePath => fill_text_cell(
            track.path.to_string_lossy().to_string(),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
        ListColumn::Duration => compact_text_cell(
            format_duration(track.metadata.duration),
            list_column_width(column, track_number_column_width),
            view_model,
        ),
    }
}

fn list_column_heading(column: ListColumn, track_number_label: &str) -> String {
    match column {
        ListColumn::TrackNumber => track_number_label.to_string(),
        ListColumn::Title => fl!("title"),
        ListColumn::Album => fl!("album"),
        ListColumn::Artist => fl!("artist"),
        ListColumn::AlbumArtist => fl!("album-artist"),
        ListColumn::TrackTotal => fl!("track-total-short"),
        ListColumn::DiscNumber => fl!("disc-number-short"),
        ListColumn::DiscTotal => fl!("disc-total-short"),
        ListColumn::Genre => fl!("genre"),
        ListColumn::FilePath => fl!("file-path"),
        ListColumn::Duration => fl!("duration"),
    }
}

fn list_column_width(column: ListColumn, track_number_column_width: f32) -> Length {
    match column {
        ListColumn::TrackNumber => Length::Fixed(track_number_column_width),
        ListColumn::Title | ListColumn::FilePath => Length::FillPortion(2),
        ListColumn::TrackTotal | ListColumn::DiscNumber | ListColumn::DiscTotal => {
            Length::Fixed(COMPACT_COLUMN_WIDTH)
        }
        ListColumn::Duration => Length::Fixed(DURATION_COLUMN_WIDTH),
        ListColumn::Album | ListColumn::Artist | ListColumn::AlbumArtist | ListColumn::Genre => {
            Length::FillPortion(1)
        }
    }
}

fn fill_text_cell<'a>(
    value: String,
    width: Length,
    view_model: &crate::app::ListViewModel,
) -> cosmic::Element<'a, Message> {
    widget::container(
        widget::text(value)
            .align_y(view_model.row_align)
            .height(view_model.row_height)
            .wrapping(view_model.wrapping)
            .width(Length::Fill),
    )
    .width(width)
    .clip(true)
    .into()
}

fn compact_text_cell<'a>(
    value: String,
    width: Length,
    view_model: &crate::app::ListViewModel,
) -> cosmic::Element<'a, Message> {
    widget::container(
        widget::text(value)
            .align_x(Alignment::End)
            .align_y(view_model.row_align)
            .height(view_model.row_height)
            .width(Length::Fill),
    )
    .width(width)
    .clip(true)
    .into()
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

// Helper function for sort buttons
fn create_sort_button<'a>(
    label: String,
    sort_by: SortBy,
    state: &crate::config::State,
    sort_icon: &str,
    spacing: u16,
    width: Length,
) -> widget::Button<'a, Message> {
    let mut row = widget::row()
        .align_y(Alignment::Center)
        .spacing(spacing)
        .push(widget::text::heading(label));

    if state.sort_by == sort_by {
        row = row.push(widget::icon::from_name(sort_icon));
    }

    widget::button::custom(row)
        .class(button_style(false, true))
        .on_press(Message::ListViewSort(sort_by))
        .padding(0)
        .width(width)
}

// Row theming
fn button_style(selected: bool, heading: bool) -> theme::Button {
    theme::Button::Custom {
        active: Box::new(move |_focus, theme| button_appearance(theme, selected, heading, false)),
        disabled: Box::new(move |theme| button_appearance(theme, selected, heading, false)),
        hovered: Box::new(move |_focus, theme| button_appearance(theme, selected, heading, true)),
        pressed: Box::new(move |_focus, theme| button_appearance(theme, selected, heading, false)),
    }
}

fn button_appearance(
    theme: &theme::Theme,
    selected: bool,
    heading: bool,
    hovered: bool,
) -> widget::button::Style {
    let cosmic = theme.cosmic();
    let mut appearance = widget::button::Style::new();

    if heading {
        appearance.background = Some(Color::TRANSPARENT.into());
        appearance.icon_color = Some(Color::from(cosmic.on_bg_color()));
        appearance.text_color = Some(Color::from(cosmic.on_bg_color()));
    } else if selected {
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

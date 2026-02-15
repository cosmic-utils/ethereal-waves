// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, Message, SortBy};
use crate::constants::*;
use crate::fl;
use cosmic::{
    cosmic_theme,
    iced::{Alignment, Color, Length},
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

    // Header row
    content = content.push(
        widget::row()
            .spacing(space_xxs)
            .push(widget::horizontal_space().width(space_xxxs))
            .push(widget::horizontal_space().width(Length::Fixed(view_model.icon_column_width)))
            .push(
                widget::text::heading("#")
                    .align_x(Alignment::End)
                    .width(Length::Fixed(view_model.number_column_width)),
            )
            .push(create_sort_button(
                fl!("title"),
                SortBy::Title,
                &app.state,
                &view_model.sort_direction_icon,
                space_xxs,
            ))
            .push(create_sort_button(
                fl!("album"),
                SortBy::Album,
                &app.state,
                &view_model.sort_direction_icon,
                space_xxs,
            ))
            .push(create_sort_button(
                fl!("artist"),
                SortBy::Artist,
                &app.state,
                &view_model.sort_direction_icon,
                space_xxs,
            ))
            .push(widget::horizontal_space().width(space_xxs)),
    );
    content = content.push(widget::divider::horizontal::default());

    // Build rows
    let mut rows = widget::column();
    rows = rows.push(widget::vertical_space().height(Length::Fixed(
        view_model.list_start as f32 * view_model.row_stride,
    )));

    let mut count: u32 = view_model.list_start as u32 + 1;

    for (index, track) in view_model
        .visible_tracks
        .iter()
        .skip(view_model.list_start)
        .take(view_model.take)
        .enumerate()
    {
        let id = track.1.metadata.id.clone().unwrap();
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
            // Check if track is in library
            let is_in_library = track.1.metadata.id.as_ref().map_or(false, |track_id| {
                app.library.media.values().any(|metadata| {
                    metadata
                        .id
                        .as_ref()
                        .map_or(false, |lib_id| lib_id == track_id)
                })
            });

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
                    widget::horizontal_space().width(Length::Fixed(view_model.icon_column_width)),
                );
            }
        }

        // Track number
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

        // Title, Album, Artist columns
        row_element = row_element
            .push(
                widget::container(
                    widget::text(
                        track
                            .1
                            .metadata
                            .title
                            .clone()
                            .unwrap_or_else(|| track.1.path.to_string_lossy().to_string()),
                    )
                    .align_y(view_model.row_align)
                    .height(view_model.row_height)
                    .wrapping(view_model.wrapping)
                    .width(Length::FillPortion(1)),
                )
                .clip(true),
            )
            .push(
                widget::container(
                    widget::text(track.1.metadata.album.clone().unwrap_or_default())
                        .align_y(view_model.row_align)
                        .height(view_model.row_height)
                        .wrapping(view_model.wrapping)
                        .width(Length::FillPortion(1)),
                )
                .clip(true),
            )
            .push(
                widget::container(
                    widget::text(track.1.metadata.artist.clone().unwrap_or_default())
                        .align_y(view_model.row_align)
                        .height(view_model.row_height)
                        .wrapping(view_model.wrapping)
                        .width(Length::FillPortion(1)),
                )
                .clip(true),
            )
            .width(Length::Fill);

        let row_button = widget::button::custom(row_element)
            .class(button_style(track.1.selected, false))
            .on_press_down(Message::ChangeTrack(id, track.0))
            .padding(0);

        rows =
            rows.push(widget::mouse_area(row_button).on_release(Message::ListSelectRow(track.0)));

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
        .push(widget::vertical_space().height(Length::Fixed(view_model.viewport_height)))
        .push(widget::horizontal_space().width(space_xxs))
        .push(rows)
        .push(widget::horizontal_space().width(space_xxs));

    let scroller = widget::scrollable(scrollable_contents)
        .id(app.list_scroll_id.clone())
        .width(Length::Fill)
        .on_scroll(|viewport| Message::ListViewScroll(viewport));

    content = content.push(scroller);

    content
}

// Helper function for sort buttons
fn create_sort_button<'a>(
    label: String,
    sort_by: SortBy,
    state: &crate::config::State,
    sort_icon: &str,
    spacing: u16,
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
        .width(Length::FillPortion(1))
}

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

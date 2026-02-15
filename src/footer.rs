// SPDX-License-Identifier: GPL-3.0

use crate::app::PlaybackStatus;
use crate::app::{AppModel, Message};
use crate::fl;
use crate::helpers::*;
use crate::library::MediaMetaData;
use cosmic::widget::tooltip::Position;
use cosmic::{
    Element, cosmic_theme,
    iced::{
        Alignment, Font, Length,
        font::{self, Weight},
    },
    theme, widget,
    widget::image::Handle,
};
use std::sync::Arc;

pub fn footer<'a>(app: &AppModel) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xs,
        space_m,
        space_l,
        ..
    } = theme::active().cosmic().spacing;

    let progress_bar_height = Length::Fixed(4.0);
    let artwork_size = 85;
    let now_playing = app.now_playing.clone().unwrap_or(MediaMetaData::new());

    // Main content container
    let mut content = widget::column().padding(space_xs);

    // Update progress area
    if app.is_updating {
        let updating_col = widget::column()
            .spacing(space_xxs)
            .push(widget::progress_bar(0.0..=100.0, app.update_percent).height(progress_bar_height))
            .push(widget::text(if app.update_progress == 0.0 {
                fl!("scanning-paths")
            } else {
                app.update_progress_display.to_string()
            }))
            .push(widget::vertical_space().height(space_xs));

        content = content.push(updating_col);
    }

    let mut handle: Option<Arc<Handle>> = None;

    if let Some(now_playing) = &app.now_playing {
        if let Some(artwork_filename) = &now_playing.artwork_filename {
            app.image_store.request(artwork_filename.clone());
            handle = app.image_store.get(&artwork_filename.clone());
        }
    }

    // Now playing column
    let artwork: Element<Message> = handle
        .as_ref()
        .map(|handle| {
            widget::row()
                .align_y(Alignment::Center)
                .width(Length::Fixed(artwork_size as f32))
                .height(Length::Fixed(artwork_size as f32))
                .push(
                    widget::image(handle.as_ref())
                        .height(artwork_size)
                        .width(artwork_size),
                )
                .into()
        })
        .unwrap_or_else(|| {
            widget::layer_container(widget::row())
                .layer(cosmic_theme::Layer::Secondary)
                .width(Length::Fixed(artwork_size as f32))
                .height(Length::Fixed(artwork_size as f32))
                .into()
        });

    let mut now_playing_text = widget::column();
    if app.now_playing.is_some() {
        now_playing_text = now_playing_text
            .push(
                widget::text(now_playing.title.unwrap_or(String::new()))
                    .wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph)
                    .font(Font {
                        weight: Weight::Bold,
                        ..Font::default()
                    }),
            )
            .push(
                widget::text(now_playing.album.unwrap_or(String::new())).font(Font {
                    style: font::Style::Italic,
                    ..Font::default()
                }),
            )
            .push(widget::text(now_playing.artist.unwrap_or(String::new())))
    }

    let now_playing_column = widget::column().width(Length::FillPortion(1)).push(
        widget::row()
            .spacing(space_xxs)
            .push(artwork)
            .push(now_playing_text),
    );

    let play_icon = match app.playback_status {
        PlaybackStatus::Stopped => "media-playback-start-symbolic",
        PlaybackStatus::Paused => "media-playback-start-symbolic",
        _ => "media-playback-pause-symbolic",
    };

    // Playback controls column
    let playback_control_column = widget::column()
        .width(Length::FillPortion(2))
        // Slider row
        .push(
            widget::row()
                .align_y(Alignment::Center)
                .spacing(space_xxs)
                .width(Length::Fill)
                .push(widget::text(format_time(app.playback_progress)))
                .push(
                    widget::slider(
                        0.0..=now_playing.duration.unwrap_or(0.0),
                        app.playback_progress,
                        Message::SliderSeek,
                    )
                    .on_release(Message::ReleaseSlider),
                )
                .push(widget::text(format_time_left(
                    app.playback_progress,
                    now_playing.duration.unwrap_or(0.0),
                ))),
        )
        // Spacer above controls
        .push(widget::vertical_space().height(space_xxs))
        // Controls row
        .push(
            widget::row()
                .align_y(Alignment::Center)
                .spacing(space_xxs)
                .width(Length::Fill)
                .push(widget::horizontal_space().width(Length::Fill))
                .push(widget::tooltip(
                    widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
                        .on_press(Message::Previous)
                        .padding(space_xs)
                        .icon_size(space_m),
                    widget::text(fl!("previous")),
                    Position::Bottom,
                ))
                .push(widget::tooltip(
                    widget::button::icon(widget::icon::from_name(play_icon))
                        .on_press(Message::PlayPause)
                        .padding(space_xs)
                        .icon_size(space_l),
                    widget::text(fl!("play")),
                    Position::Bottom,
                ))
                .push(widget::tooltip(
                    widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
                        .on_press(Message::Next)
                        .padding(space_xs)
                        .icon_size(space_m),
                    widget::text(fl!("next")),
                    Position::Bottom,
                ))
                .push(widget::horizontal_space().width(Length::Fill)),
        );

    // Other controls column
    let volume_icon = if app.state.muted {
        "audio-volume-muted-symbolic"
    } else {
        match app.state.volume {
            0..=33 => "audio-volume-low-symbolic",
            34..=66 => "audio-volume-medium-symbolic",
            67..=100 => "audio-volume-high-symbolic",
            _ => "audio-volume-high-symbolic",
        }
    };

    let other_controls_column = widget::column().width(Length::FillPortion(1)).push(
        widget::row()
            .align_y(Alignment::Center)
            .spacing(space_xxs)
            .push(widget::horizontal_space().width(Length::FillPortion(1)))
            .push(
                widget::button::icon(widget::icon::from_name(volume_icon))
                    .on_press(Message::ToggleMute),
            )
            .push(
                widget::column().push(
                    widget::slider(0..=100, app.state.volume, Message::SetVolume)
                        .width(Length::Fixed(150.0)),
                ),
            ),
    );

    let control_row = widget::row()
        .spacing(space_xxs)
        .push(now_playing_column)
        .push(playback_control_column)
        .push(other_controls_column);

    widget::layer_container(content.push(control_row))
        .layer(cosmic_theme::Layer::Primary)
        .into()
}

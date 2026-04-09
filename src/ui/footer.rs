// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, Message};
use crate::constants::FOOTER_CONDENSED_BREAKPOINT;
use crate::fl;
use crate::helpers::*;
use crate::library::MediaMetaData;
use crate::playback_state::PlaybackStatus;
use cosmic::widget::tooltip::Position;
use cosmic::{
    Element, cosmic_theme,
    iced::{
        Alignment, Font, Length,
        font::{self, Weight},
    },
    theme, widget,
};
use std::sync::Arc;

pub fn footer<'a>(app: &AppModel) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xs,
        ..
    } = theme::active().cosmic().spacing;

    let is_condensed = app.state.window_width < FOOTER_CONDENSED_BREAKPOINT;
    let progress_bar_height = Length::Fixed(4.0);
    let now_playing = if app.playback_service.now_playing().is_some() {
        app.playback_service.now_playing().unwrap()
    } else {
        &MediaMetaData::new()
    };

    // Main content container
    let mut content = widget::column().padding(space_xs);

    // Update progress area
    if app.is_updating {
        let updating_col = widget::column()
            .spacing(space_xxs)
            .push(widget::row().push(
                widget::progress_bar(0.0..=100.0, app.update_percent).girth(progress_bar_height),
            ))
            .push(
                widget::row()
                    .push(widget::text(if app.update_progress == 0.0 {
                        fl!("scanning-paths")
                    } else {
                        app.update_progress_display.to_string()
                    }))
                    .push(widget::space::horizontal())
                    .push(widget::tooltip(
                        widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
                            .on_press(Message::CancelLibraryUpdate),
                        widget::text(fl!("cancel-update")),
                        Position::Bottom,
                    ))
                    .align_y(Alignment::Center),
            )
            .push(widget::space::vertical().height(space_xs));

        content = content.push(updating_col);
    }

    let mut handle: Option<Arc<cosmic::widget::image::Handle>> = None;

    if let Some(now_playing) = &app.playback_service.now_playing() {
        if let Some(artwork_filename) = &now_playing.artwork_filename {
            app.image_store.request(artwork_filename.clone());
            handle = app.image_store.get(&artwork_filename.clone());
        }
    }

    let play_icon = match app.playback_service.status() {
        PlaybackStatus::Stopped | PlaybackStatus::Paused => "media-playback-start-symbolic",
        _ => "media-playback-pause-symbolic",
    };

    let volume_icon = if app.state.muted {
        "audio-volume-muted-symbolic"
    } else {
        match app.state.volume {
            0..=33 => "audio-volume-low-symbolic",
            34..=66 => "audio-volume-medium-symbolic",
            _ => "audio-volume-high-symbolic",
        }
    };

    let controls: Element<_> = if is_condensed {
        condensed_footer(app, now_playing, play_icon, volume_icon, handle)
    } else {
        full_footer(app, now_playing, play_icon, volume_icon, handle)
    };

    widget::layer_container(content.push(controls))
        .layer(cosmic_theme::Layer::Primary)
        .into()
}

fn condensed_footer<'a>(
    app: &AppModel,
    now_playing: &MediaMetaData,
    play_icon: &str,
    volume_icon: &str,
    handle: Option<Arc<cosmic::widget::image::Handle>>,
) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xs,
        space_m,
        space_l,
        ..
    } = theme::active().cosmic().spacing;

    let artwork_size = 48;

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

    let title_text = now_playing.title.as_deref().unwrap_or_default().to_string();
    let by_text = join_non_empty(
        &[
            now_playing.album.as_deref().unwrap_or_default(),
            now_playing.artist.as_deref().unwrap_or_default(),
        ],
        " - ",
    );

    let mut meta = widget::column().width(Length::Fill);
    if !title_text.is_empty() {
        meta = meta.push(
            widget::text(title_text)
                .wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph)
                .font(Font {
                    weight: Weight::Bold,
                    ..Font::default()
                }),
        )
    }
    if !by_text.is_empty() {
        meta = meta
            .push(widget::text(by_text).wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph));
    }

    let seek_row = widget::row()
        .align_y(Alignment::Center)
        .spacing(space_xxs)
        .width(Length::Fill)
        .push(widget::text(format_time(app.playback_service.progress())))
        .push(
            widget::slider(
                0.0..=now_playing.duration.unwrap_or(0.0),
                app.playback_service.progress(),
                Message::SliderSeek,
            )
            .on_release(Message::ReleaseSlider),
        )
        .push(widget::text(format_time_left(
            app.playback_service.progress(),
            now_playing.duration.unwrap_or(0.0),
        )));

    let control_row = widget::row()
        .align_y(Alignment::Center)
        .spacing(space_xxs)
        .push(artwork)
        .push(widget::space::horizontal())
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
        .push(widget::space::horizontal())
        .push(
            widget::button::icon(widget::icon::from_name(volume_icon))
                .on_press(Message::ToggleMute),
        );

    widget::column()
        .spacing(space_xxs)
        .push(meta)
        .push(seek_row)
        .push(control_row)
        .into()
}

fn full_footer<'a>(
    app: &AppModel,
    now_playing: &MediaMetaData,
    play_icon: &str,
    volume_icon: &str,
    handle: Option<Arc<cosmic::widget::image::Handle>>,
) -> Element<'a, Message> {
    let cosmic_theme::Spacing {
        space_xxs,
        space_xs,
        space_m,
        space_l,
        ..
    } = theme::active().cosmic().spacing;

    let artwork_size = 85;

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
    if app.playback_service.now_playing().is_some() {
        now_playing_text = now_playing_text
            .push(
                widget::text(now_playing.clone().title.unwrap_or(String::new()))
                    .wrapping(cosmic::iced_core::text::Wrapping::WordOrGlyph)
                    .font(Font {
                        weight: Weight::Bold,
                        ..Font::default()
                    }),
            )
            .push(
                widget::text(now_playing.clone().album.unwrap_or(String::new())).font(Font {
                    style: font::Style::Italic,
                    ..Font::default()
                }),
            )
            .push(widget::text(
                now_playing.clone().artist.unwrap_or(String::new()),
            ))
    }

    let now_playing_column = widget::column().width(Length::FillPortion(1)).push(
        widget::row()
            .spacing(space_xxs)
            .push(artwork)
            .push(now_playing_text),
    );

    // Playback controls column
    let playback_control_column = widget::column()
        .width(Length::FillPortion(2))
        // Slider row
        .push(
            widget::row()
                .align_y(Alignment::Center)
                .spacing(space_xxs)
                .width(Length::Fill)
                .push(widget::text(format_time(app.playback_service.progress())))
                .push(
                    widget::slider(
                        0.0..=now_playing.duration.unwrap_or(0.0),
                        app.playback_service.progress(),
                        Message::SliderSeek,
                    )
                    .on_release(Message::ReleaseSlider),
                )
                .push(widget::text(format_time_left(
                    app.playback_service.progress(),
                    now_playing.duration.unwrap_or(0.0),
                ))),
        )
        // Spacer above controls
        .push(widget::space::vertical().height(space_xxs))
        // Controls row
        .push(
            widget::row()
                .align_y(Alignment::Center)
                .spacing(space_xxs)
                .width(Length::Fill)
                .push(widget::space::horizontal().width(Length::Fill))
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
                .push(widget::space::horizontal().width(Length::Fill)),
        );

    let other_controls_column = widget::column().width(Length::FillPortion(1)).push(
        widget::row()
            .align_y(Alignment::Center)
            .spacing(space_xxs)
            .push(widget::space::horizontal().width(Length::FillPortion(1)))
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
        .width(Length::Fill)
        .push(now_playing_column.width(Length::FillPortion(1)))
        .push(
            playback_control_column
                .align_x(Alignment::Center)
                .width(Length::FillPortion(1)),
        )
        .push(
            other_controls_column
                .align_x(Alignment::End)
                .width(Length::FillPortion(1)),
        );

    let content = widget::column().width(Length::Fill);

    widget::layer_container(content.push(control_row))
        .layer(cosmic_theme::Layer::Primary)
        .into()
}

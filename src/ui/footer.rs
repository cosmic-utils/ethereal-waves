// SPDX-License-Identifier: GPL-3.0

use crate::app::{AppModel, Message, TrackDropData};
use crate::constants::{
    DEFAULT_FOOTER_VISUALIZER_COLOR, FOOTER_CONDENSED_BREAKPOINT,
    FOOTER_VISUALIZER_ASSUMED_SAMPLE_RATE_HZ, FOOTER_VISUALIZER_LOWER_CUTOFF_HZ,
    FOOTER_VISUALIZER_UPPER_CUTOFF_HZ, LIBRARY_TRACK_DROP_PREFIX, MAX_FOOTER_VISUALIZER_BAR_COUNT,
    MIN_FOOTER_VISUALIZER_BAR_COUNT,
};
use crate::fl;
use crate::helpers::*;
use crate::library::MediaMetaData;
use crate::playback_state::PlaybackStatus;
use crate::player::VisualizerSampleBuffer;
use cosmic::widget::tooltip::Position;
use cosmic::{
    Element, Renderer, Theme, cosmic_theme,
    iced::{
        Alignment, Color, Font, Length, Point, Rectangle, Size,
        clipboard::dnd::DndAction,
        font::{self, Weight},
        mouse, time, window,
    },
    iced_core::widget::Tree,
    theme, widget,
};
use std::sync::{Arc, Mutex};

const VISUALIZER_FRAME_INTERVAL_MS: u64 = 16;
const VISUALIZER_BAR_OFFSET_PERCENT: f32 = 10.0;
const VISUALIZER_NOISE_FLOOR_AMPLITUDE: f32 = 0.01;
const VISUALIZER_DISPLAY_EXPONENT: f32 = 0.35;
const VISUALIZER_INPUT_GAIN: f32 = 4.0;
const VISUALIZER_HIGH_FREQUENCY_GAIN: f32 = 4.0;
const VISUALIZER_HIGH_FREQUENCY_GAIN_CURVE: f32 = 1.2;
const VISUALIZER_IDLE_EPSILON: f32 = 0.002;
const VISUALIZER_ATTACK_RATE: f32 = 45.0;
const VISUALIZER_DECAY_RATE: f32 = 7.0;

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
    let controls = footer_controls_with_visualizer(app, controls);

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

    let artwork_drag_data = current_track_drop_data(app);
    let artwork_drag_label = now_playing
        .clone()
        .title
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fl!("one-track-selected"));
    let artwork = footer_artwork(handle, artwork_size, artwork_drag_data, artwork_drag_label);

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
    let artwork_drag_data = current_track_drop_data(app);
    let artwork_drag_label = now_playing
        .clone()
        .title
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fl!("one-track-selected"));
    let artwork = footer_artwork(handle, artwork_size, artwork_drag_data, artwork_drag_label);

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

    widget::column()
        .width(Length::Fill)
        .push(control_row)
        .into()
}

fn footer_controls_with_visualizer<'a>(
    app: &AppModel,
    controls: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.config.footer_visualizer_enabled {
        return controls;
    }

    let bar_count = app.config.footer_visualizer_bar_count.clamp(
        MIN_FOOTER_VISUALIZER_BAR_COUNT,
        MAX_FOOTER_VISUALIZER_BAR_COUNT,
    ) as usize;
    let fallback = parse_footer_visualizer_color(DEFAULT_FOOTER_VISUALIZER_COLOR)
        .unwrap_or_else(|| Color::from(theme::active().cosmic().accent_color()));
    let color =
        parse_footer_visualizer_color(&app.config.footer_visualizer_color).unwrap_or(fallback);

    let visualizer: Element<_> = widget::canvas(FooterVisualizer {
        bar_count,
        color,
        playing: app.playback_service.status() == PlaybackStatus::Playing,
        samples: app.playback_service.visualizer_samples(),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    cosmic::iced::widget::stack(vec![controls])
        .push_under(visualizer)
        .width(Length::Fill)
        .clip(true)
        .into()
}

#[derive(Debug, Clone)]
struct FooterVisualizer {
    bar_count: usize,
    color: Color,
    playing: bool,
    samples: Arc<Mutex<VisualizerSampleBuffer>>,
}

#[derive(Debug, Default)]
struct FooterVisualizerState {
    bars: Vec<f32>,
    last_frame: Option<time::Instant>,
}

impl FooterVisualizerState {
    fn resize(&mut self, bar_count: usize) {
        self.bars.resize(bar_count, 0.0);
        self.last_frame = None;
    }
}

impl<Message> widget::canvas::Program<Message, Theme, Renderer> for FooterVisualizer {
    type State = FooterVisualizerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &widget::canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<widget::canvas::Action<Message>> {
        if state.bars.len() != self.bar_count {
            state.resize(self.bar_count);
        }

        let widget::canvas::Event::Window(window::Event::RedrawRequested(now)) = event else {
            return None;
        };

        let elapsed_secs = state
            .last_frame
            .map(|last_frame| now.duration_since(last_frame).as_secs_f32())
            .unwrap_or(VISUALIZER_FRAME_INTERVAL_MS as f32 / 1000.0)
            .clamp(0.001, 0.1);
        state.last_frame = Some(*now);

        let samples = self
            .samples
            .lock()
            .map(|mut samples| samples.visible_samples())
            .unwrap_or_default();

        for index in 0..self.bar_count {
            let target = if self.playing {
                visualizer_bar_amplitude(&samples, index, self.bar_count)
            } else {
                0.0
            };
            let current = state.bars[index];
            let rate = if target > current {
                VISUALIZER_ATTACK_RATE
            } else {
                VISUALIZER_DECAY_RATE
            };
            let alpha = 1.0 - (-rate * elapsed_secs).exp();

            state.bars[index] = (current + (target - current) * alpha).clamp(0.0, 1.0);
        }

        if self.playing
            || state
                .bars
                .iter()
                .any(|value| *value > VISUALIZER_IDLE_EPSILON)
        {
            Some(widget::canvas::Action::request_redraw_at(
                *now + time::Duration::from_millis(VISUALIZER_FRAME_INTERVAL_MS),
            ))
        } else {
            None
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<widget::canvas::Geometry> {
        if bounds.width < 1.0 || bounds.height < 1.0 || self.bar_count == 0 {
            return Vec::new();
        }

        let mut frame = widget::canvas::Frame::new(renderer, bounds.size());
        let step = bounds.width / self.bar_count as f32;
        let offset_px = step * VISUALIZER_BAR_OFFSET_PERCENT / 100.0;
        let bar_width = (step - offset_px * 2.0).max(1.0);

        for index in 0..self.bar_count {
            let amplitude = state.bars.get(index).copied().unwrap_or(0.0);
            let bar_height = bounds.height * amplitude;
            if bar_height <= 0.0 {
                continue;
            }

            let x = step * index as f32 + offset_px;
            let y = bounds.height - bar_height;
            let bar =
                widget::canvas::Path::rectangle(Point::new(x, y), Size::new(bar_width, bar_height));
            let mut color = self.color;
            color.a = 0.22 + amplitude * 0.34;

            frame.fill(&bar, color);
        }

        vec![frame.into_geometry()]
    }
}

fn visualizer_bar_amplitude(samples: &[f32], index: usize, bar_count: usize) -> f32 {
    if samples.is_empty() || bar_count == 0 {
        return 0.0;
    }

    let sample_count = samples.len();
    let start = logarithmic_sample_edge(index, bar_count, sample_count);
    let end = logarithmic_sample_edge(index + 1, bar_count, sample_count)
        .max(start + 1.0)
        .min(sample_count as f32);

    let amplitude = weighted_sample_average(samples, start, end);
    display_amplitude(amplitude * visualizer_frequency_gain(index, bar_count))
}

fn visualizer_frequency_gain(index: usize, bar_count: usize) -> f32 {
    let position = if bar_count <= 1 {
        1.0
    } else {
        (index as f32 / (bar_count - 1) as f32).clamp(0.0, 1.0)
    };
    let high_frequency_boost = 1.0
        + (VISUALIZER_HIGH_FREQUENCY_GAIN - 1.0)
            * position.powf(VISUALIZER_HIGH_FREQUENCY_GAIN_CURVE);

    VISUALIZER_INPUT_GAIN * high_frequency_boost
}

fn logarithmic_sample_edge(edge_index: usize, bar_count: usize, sample_count: usize) -> f32 {
    let mut previous = 0.0;
    let mut current = 0.0;

    for edge in 0..=edge_index {
        let desired =
            frequency_to_sample_position(logarithmic_edge_frequency(edge, bar_count), sample_count);

        current = if edge > 0 && desired <= previous + 1.0 {
            previous + 1.0
        } else {
            desired
        }
        .min(sample_count as f32);
        previous = current;
    }

    current
}

fn logarithmic_edge_frequency(edge_index: usize, bar_count: usize) -> f32 {
    let nyquist = FOOTER_VISUALIZER_ASSUMED_SAMPLE_RATE_HZ / 2.0;
    let lower = FOOTER_VISUALIZER_LOWER_CUTOFF_HZ.clamp(1.0, nyquist - 1.0);
    let upper = FOOTER_VISUALIZER_UPPER_CUTOFF_HZ.clamp(lower + 1.0, nyquist);
    let position = edge_index as f32 / bar_count.max(1) as f32;

    lower * (upper / lower).powf(position)
}

fn frequency_to_sample_position(frequency_hz: f32, sample_count: usize) -> f32 {
    let nyquist = FOOTER_VISUALIZER_ASSUMED_SAMPLE_RATE_HZ / 2.0;
    (frequency_hz / nyquist * sample_count as f32).clamp(0.0, sample_count as f32)
}

fn weighted_sample_average(samples: &[f32], start: f32, end: f32) -> f32 {
    let sample_count = samples.len();
    let start_sample = start.floor().clamp(0.0, sample_count as f32) as usize;
    let end_sample = end.ceil().clamp(0.0, sample_count as f32) as usize;
    let mut total = 0.0;
    let mut weight_total = 0.0;

    for sample_index in start_sample..end_sample {
        let sample_start = sample_index as f32;
        let sample_end = sample_start + 1.0;
        let weight = (sample_end.min(end) - sample_start.max(start)).max(0.0);

        total += samples[sample_index] * weight;
        weight_total += weight;
    }

    if weight_total > 0.0 {
        (total / weight_total).clamp(0.0, 1.0)
    } else {
        samples[start_sample.min(sample_count - 1)].clamp(0.0, 1.0)
    }
}

fn display_amplitude(amplitude: f32) -> f32 {
    let amplitude = ((amplitude - VISUALIZER_NOISE_FLOOR_AMPLITUDE)
        / (1.0 - VISUALIZER_NOISE_FLOOR_AMPLITUDE))
        .clamp(0.0, 1.0);

    amplitude.powf(VISUALIZER_DISPLAY_EXPONENT)
}

fn parse_footer_visualizer_color(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());

    match hex.len() {
        3 => {
            let r = parse_repeated_hex_digit(&hex[0..1])?;
            let g = parse_repeated_hex_digit(&hex[1..2])?;
            let b = parse_repeated_hex_digit(&hex[2..3])?;
            Some(rgb8(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(rgb8(r, g, b))
        }
        _ => None,
    }
}

fn parse_repeated_hex_digit(value: &str) -> Option<u8> {
    let digit = u8::from_str_radix(value, 16).ok()?;
    Some(digit * 17)
}

fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    )
}

fn current_track_drop_data(app: &AppModel) -> Option<TrackDropData> {
    app.playback_service
        .now_playing()
        .and_then(|now_playing| now_playing.id.as_ref())
        .filter(|id| !id.is_empty())
        .cloned()
        .map(|id| TrackDropData::new(vec![format!("{LIBRARY_TRACK_DROP_PREFIX}{id}")]))
}

fn draggable_artwork<'a>(
    artwork: Element<'a, Message>,
    drag_data: Option<TrackDropData>,
    drag_label: String,
) -> Element<'a, Message> {
    let Some(drag_data) = drag_data else {
        return artwork;
    };

    widget::dnd_source::DndSource::new(widget::mouse_area(artwork))
        .drag_content(move || drag_data.clone())
        .action(DndAction::Copy)
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

fn footer_artwork<'a>(
    handle: Option<Arc<cosmic::widget::image::Handle>>,
    artwork_size: u16,
    drag_data: Option<TrackDropData>,
    drag_label: String,
) -> Element<'a, Message> {
    let artwork: Element<'a, Message> = handle
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

    draggable_artwork(artwork, drag_data, drag_label)
}

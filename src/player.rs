// SPDX-License-Identifier: GPL-3.0

use crate::constants::FOOTER_VISUALIZER_ANALYZER_BANDS;
use crate::helpers::clamp;
use gst::prelude::*;
use gstreamer::{self as gst};
use std::{
    collections::VecDeque,
    sync::mpsc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const SPECTRUM_THRESHOLD_DB: f32 = -80.0;
//const SPECTRUM_INTERVAL_NS: u64 = 16_666_667;
const SPECTRUM_INTERVAL_NS: u64 = 10_000_000;
const VISUALIZER_SYNC_DELAY_MS: u64 = 1_200;
const VISUALIZER_HISTORY_MS: u64 = 3_000;

#[derive(Debug)]
pub struct VisualizerSampleBuffer {
    visible: Vec<f32>,
    frames: VecDeque<VisualizerSampleFrame>,
}

#[derive(Debug)]
struct VisualizerSampleFrame {
    captured_at: Instant,
    samples: Vec<f32>,
}

impl VisualizerSampleBuffer {
    fn new(sample_count: usize) -> Self {
        Self {
            visible: vec![0.0; sample_count],
            frames: VecDeque::new(),
        }
    }

    fn push(&mut self, samples: Vec<f32>) {
        let now = Instant::now();
        self.frames.push_back(VisualizerSampleFrame {
            captured_at: now,
            samples,
        });

        let retention = Duration::from_millis(VISUALIZER_SYNC_DELAY_MS + VISUALIZER_HISTORY_MS);
        while self
            .frames
            .front()
            .is_some_and(|frame| now.duration_since(frame.captured_at) > retention)
        {
            let _ = self.frames.pop_front();
        }
    }

    pub fn visible_samples(&mut self) -> Vec<f32> {
        let display_time = Instant::now()
            .checked_sub(Duration::from_millis(VISUALIZER_SYNC_DELAY_MS))
            .unwrap_or_else(Instant::now);

        while self
            .frames
            .front()
            .is_some_and(|frame| frame.captured_at <= display_time)
        {
            let Some(frame) = self.frames.pop_front() else {
                break;
            };
            self.visible = frame.samples;
        }

        self.visible.clone()
    }

    fn clear(&mut self) {
        self.visible.fill(0.0);
        self.frames.clear();
    }
}

pub struct Player {
    pub playbin: gst::Element,
    queued_uri: Arc<Mutex<Option<String>>>,
    about_to_finish_rx: mpsc::Receiver<()>,
    spectrum: Option<gst::Element>,
    visualizer_samples: Arc<Mutex<VisualizerSampleBuffer>>,
}

impl Player {
    pub fn new() -> Self {
        match gst::init() {
            Ok(_) => {}
            Err(error) => {
                panic!("Failed to initialize GStreamer: {:?}", error)
            }
        }

        let playbin = gst::ElementFactory::make("playbin")
            .build()
            .expect("Failed to create playbin.");
        let spectrum = Self::spectrum_filter();
        let visualizer_samples = Arc::new(Mutex::new(VisualizerSampleBuffer::new(
            FOOTER_VISUALIZER_ANALYZER_BANDS,
        )));

        if let Some(spectrum) = &spectrum {
            playbin.set_property("audio-filter", spectrum);
        }

        Self::install_spectrum_bus_handler(&playbin, visualizer_samples.clone());

        let queued_uri: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let (about_to_finish_tx, about_to_finish_rx) = mpsc::sync_channel::<()>(8);

        // Connect the about-to-finish signal for gapless playback.
        let queued_uri_clone = queued_uri.clone();
        playbin.connect("about-to-finish", false, move |args| {
            let playbin_elem = args[0]
                .get::<gst::Element>()
                .expect("about-to-finish: invalid element arg");

            // If a next URI has been queued, set it now for seamless transition.
            if let Ok(guard) = queued_uri_clone.lock() {
                if let Some(ref uri) = *guard {
                    playbin_elem.set_property("uri", uri);
                    // Notify the main thread that a gapless transition was queued.
                    let _ = about_to_finish_tx.try_send(());
                }
            }

            None
        });

        Self {
            playbin,
            queued_uri,
            about_to_finish_rx,
            spectrum,
            visualizer_samples,
        }
    }

    fn spectrum_filter() -> Option<gst::Element> {
        match gst::ElementFactory::make("spectrum")
            .property("bands", FOOTER_VISUALIZER_ANALYZER_BANDS as u32)
            .property("threshold", SPECTRUM_THRESHOLD_DB as i32)
            .property("interval", SPECTRUM_INTERVAL_NS)
            .property("post-messages", true)
            .property("message-magnitude", true)
            .property("message-phase", false)
            .build()
        {
            Ok(spectrum) => Some(spectrum),
            Err(err) => {
                log::warn!("failed to create GStreamer spectrum filter: {err}");
                None
            }
        }
    }

    fn install_spectrum_bus_handler(
        playbin: &gst::Element,
        visualizer_samples: Arc<Mutex<VisualizerSampleBuffer>>,
    ) {
        let Some(bus) = playbin.bus() else {
            return;
        };

        bus.set_sync_handler(move |_bus, msg| {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    if structure.name() == "spectrum" {
                        if let Some(samples) = Self::spectrum_samples_from_structure(structure) {
                            if let Ok(mut visualizer_samples) = visualizer_samples.lock() {
                                visualizer_samples.push(samples);
                            }
                        }
                    }
                }
            }

            gst::BusSyncReply::Pass
        });
    }

    fn spectrum_samples_from_structure(structure: &gst::StructureRef) -> Option<Vec<f32>> {
        let magnitudes = structure.get::<gst::List>("magnitude").ok()?;
        let raw_samples: Vec<f32> = magnitudes
            .iter()
            .filter_map(|value| {
                let db = value
                    .get::<f32>()
                    .ok()
                    .or_else(|| value.get::<f64>().ok().map(|db| db as f32))?;

                Some(Self::spectrum_amplitude_from_db(db))
            })
            .collect();

        if raw_samples.is_empty() {
            return None;
        }

        Some(Self::mono_samples(raw_samples))
    }

    fn mono_samples(raw_samples: Vec<f32>) -> Vec<f32> {
        if raw_samples.len() <= FOOTER_VISUALIZER_ANALYZER_BANDS {
            return raw_samples;
        }

        let channel_count = raw_samples.len() / FOOTER_VISUALIZER_ANALYZER_BANDS;
        if channel_count == 0 || raw_samples.len() % FOOTER_VISUALIZER_ANALYZER_BANDS != 0 {
            return raw_samples;
        }

        let mut samples = vec![0.0; FOOTER_VISUALIZER_ANALYZER_BANDS];
        for channel in 0..channel_count {
            let channel_offset = channel * FOOTER_VISUALIZER_ANALYZER_BANDS;
            for band in 0..FOOTER_VISUALIZER_ANALYZER_BANDS {
                samples[band] += raw_samples[channel_offset + band] / channel_count as f32;
            }
        }

        samples
    }

    fn spectrum_amplitude_from_db(db: f32) -> f32 {
        if db.is_finite() {
            10.0_f32.powf(db / 20.0).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn load(&self, uri: &str) {
        self.playbin.set_property("uri", &uri);
    }

    pub fn play(&mut self) {
        match self.playbin.set_state(gst::State::Playing) {
            Ok(_) => {}
            Err(error) => {
                panic!("Failed to play: {:?}", error);
            }
        }
    }

    pub fn pause(&mut self) {
        match self.playbin.set_state(gst::State::Paused) {
            Ok(_) => {}
            Err(error) => {
                panic!("Failed to pause: {:?}", error);
            }
        }
    }

    pub fn stop(&mut self) {
        match self.playbin.set_state(gst::State::Null) {
            Ok(_) => {}
            Err(error) => {
                panic!("Failed to stop: {:?}", error);
            }
        }
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.playbin.set_property("volume", clamp(volume, 0.0, 1.0));
    }

    pub fn visualizer_samples(&self) -> Arc<Mutex<VisualizerSampleBuffer>> {
        self.visualizer_samples.clone()
    }

    pub fn set_visualizer_enabled(&self, enabled: bool) {
        if let Some(spectrum) = &self.spectrum {
            spectrum.set_property("post-messages", enabled);
        }

        if !enabled {
            self.clear_visualizer_samples();
        }
    }

    pub fn clear_visualizer_samples(&self) {
        if let Ok(mut samples) = self.visualizer_samples.lock() {
            samples.clear();
        }
    }

    /// Set (or clear) the URI to be played gaplessly after the current track.
    pub fn set_queued_uri(&self, uri: Option<String>) {
        if let Ok(mut guard) = self.queued_uri.lock() {
            *guard = uri;
        }
    }

    /// Returns `true` if the about-to-finish callback fired since the last call,
    /// meaning a gapless transition was queued. Drains all pending notifications.
    pub fn take_about_to_finish(&self) -> bool {
        let mut fired = false;

        while self.about_to_finish_rx.try_recv().is_ok() {
            fired = true;
        }

        fired
    }
}

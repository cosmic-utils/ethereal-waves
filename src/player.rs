// SPDX-License-Identifier: GPL-3.0

use crate::helpers::clamp;
use gst::prelude::*;
use gstreamer::{self as gst};

pub struct Player {
    pub playbin: gst::Element,
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

        Self { playbin }
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
}

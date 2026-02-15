// SPDX-License-Identifier: GPL-3.0

mod app;
mod config;
mod constants;
mod footer;
mod helpers;
mod i18n;
mod image_store;
mod key_bind;
mod library;
mod menu;
mod mpris;
mod page;
mod player;
mod playlist;
mod services;

use app::Flags;
use config::{Config, State};
use cosmic::{
    app::Settings,
    iced::{Limits, Size},
};

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    let (config_handler, config) = Config::load();
    let (state_handler, state) = State::load();

    // Settings for configuring the application window and iced runtime.
    let mut settings: Settings = Settings::default();
    settings = settings.size_limits(Limits::NONE.min_width(360.0).min_height(180.0));
    settings = settings.theme(config.app_theme.theme());
    settings = settings.size(Size::new(state.window_width, state.window_height));

    let flags = Flags {
        config_handler,
        state_handler,
        state,
    };

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, flags)
}

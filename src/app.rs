// SPDX-License-Identifier: GPL-3.0

use crate::config::{AppTheme, CONFIG_VERSION, Config, State};
use crate::constants::*;
use crate::fl;
use crate::footer::footer;
use crate::helpers::*;
use crate::image_store::ImageStore;
use crate::key_bind::key_binds;
use crate::library::Library;
use crate::menu::menu_bar;
use crate::mpris::{MediaPlayer2, MediaPlayer2Player, MprisCommand};
use crate::page::empty_library;
use crate::page::list_view;
use crate::page::loading;
use crate::playback_state::{PlaybackStatus, RepeatMode};
use crate::playlist::{Playlist, Track};
use crate::services::library_service::{LibraryProgress, LibraryService};
use crate::services::playback_service::{PlaybackEvent, PlaybackService};
use crate::services::playlist_service::PlaylistService;
use cosmic::iced_widget::scrollable::{self, AbsoluteOffset};
use cosmic::prelude::*;
use cosmic::{
    Action,
    app::context_drawer,
    cosmic_config::{self, CosmicConfigEntry},
    cosmic_theme,
    dialog::file_chooser,
    iced::{
        self, Alignment, Length, Size, Subscription,
        alignment::{Horizontal, Vertical},
        event::{self, Event},
        font::{Font, Weight},
        keyboard::{Event as KeyEvent, Key, Modifiers, key::Named},
        window::Event as WindowEvent,
    },
    iced_core::text::Wrapping,
    theme,
    widget::{
        self, Column,
        about::About,
        menu::{self, Action as WidgetMenuAction},
        nav_bar, row, settings, text, toggler,
    },
};

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;
use urlencoding::decode;
use xdg::BaseDirectories;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!(
    "../resources/icons/hicolor/scalable/apps/com.github.LotusPetal392.ethereal-waves.svg"
);

pub type PlaylistId = u32;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page this app.
    about: About,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    pub key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    pub config: Config,
    /// Settings page / app theme dropdown labels
    app_theme_labels: Vec<String>,
    pub is_condensed: bool,

    config_handler: Option<cosmic_config::Config>,
    state_handler: Option<cosmic_config::Config>,
    pub state: crate::config::State,

    app_xdg_dirs: Arc<BaseDirectories>,

    pub library_service: LibraryService,
    library_update_cancel: Option<CancellationToken>,
    pub playback_service: PlaybackService,

    pub library: Library,

    pub is_updating: bool,
    pub update_progress: f32,
    pub update_total: f32,
    pub update_percent: f32,
    pub update_progress_display: String,

    initial_load_complete: bool,

    dialog_pages: DialogPages,

    view_mode: ViewMode,

    size_multiplier: f32,
    pub list_scroll_id: widget::Id,
    pub list_start: usize,
    pub list_visible_row_count: usize,
    list_last_clicked: Option<Instant>,
    list_last_selected_id: Option<usize>,

    control_pressed: u8,
    shift_pressed: u8,

    pub view_playlist: Option<u32>,

    search_id: widget::Id,
    pub search_term: Option<String>,

    pub image_store: ImageStore,

    pub playlist_service: PlaylistService,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    AddLibraryDialog,
    AddSelectedToPlaylist(PlaylistId),
    AddNowPlayingToPlaylist(PlaylistId),
    AppTheme(AppTheme),
    CancelLibraryUpdate,
    ChangeTrack(String, usize),
    DeletePlaylist,
    DialogCancel,
    DialogComplete,
    KeyPressed(Modifiers, Key),
    KeyReleased(Key),
    LaunchUrl(String),
    LibraryPathOpenError(Arc<file_chooser::Error>),
    LibraryProgress(LibraryProgress),
    ListSelectRow(usize),
    ListViewScroll(scrollable::Viewport),
    ListViewSort(SortBy),
    MoveNavDown,
    MoveNavUp,
    NewPlaylist,
    Next,
    Noop,
    PlayPause,
    Previous,
    Quit,
    ReleaseSlider,
    RemoveLibraryPath(String),
    RemoveSelectedFromPlaylist,
    RenamePlaylist,
    SearchActivate,
    SearchClear,
    SearchInput(String),
    SelectAll,
    SelectedPaths(Vec<String>),
    SetVolume(i32),
    SliderSeek(f32),
    Tick,
    ToggleContextPage(ContextPage),
    ToggleListRowAlignTop(bool),
    ToggleListTextWrap(bool),
    ToggleMute,
    ToggleRepeat,
    ToggleRepeatMode,
    ToggleShuffle,
    UpdateConfig(Config),
    UpdateDialog(DialogPage),
    UpdateLibrary,
    WindowResized(Size),
    ZoomIn,
    ZoomOut,
}

/// Unique identifier in RDNN (reverse domain name notation) format.
pub const APP_ID: &'static str = "com.github.LotusPetal392.ethereal-waves";

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = Flags;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = APP_ID;

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with three page items.
        let nav = nav_bar::Model::default();

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Initialize MPRIS
        let (mpris_tx, mpris_rx) = tokio::sync::mpsc::unbounded_channel();
        let (conn_tx, _) = std::sync::mpsc::sync_channel(1);

        tokio::spawn(async move {
            let connection = zbus::Connection::session().await.unwrap();

            connection
                .object_server()
                .at("/org/mpris/MediaPlayer2", MediaPlayer2)
                .await
                .unwrap();

            connection
                .object_server()
                .at(
                    "/org/mpris/MediaPlayer2",
                    MediaPlayer2Player {
                        tx: mpris_tx,
                        playback_status: Arc::new(Mutex::new(PlaybackStatus::Stopped)),
                    },
                )
                .await
                .unwrap();

            connection
                .request_name("org.mpris.MediaPlayer2.ethereal-waves")
                .await
                .unwrap();

            // Send clone back to the app
            let _ = conn_tx.send(connection.clone());

            // Keep alive
            futures::future::pending::<()>().await;
        });

        let app_xdg_dirs = xdg::BaseDirectories::with_prefix("ethereal-waves");

        // Build out artwork cache directory
        let artwork_dir = app_xdg_dirs
            .get_cache_home()
            .map(|p| p.join(ARTWORK_DIR))
            .unwrap_or(PathBuf::new());

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: key_binds(),
            config: cosmic_config::Config::new(APP_ID, CONFIG_VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            app_theme_labels: vec![fl!("match-desktop"), fl!("dark"), fl!("light")],
            is_condensed: false,
            config_handler: _flags.config_handler,
            state_handler: _flags.state_handler,
            state: _flags.state.clone(),
            app_xdg_dirs: Arc::new(app_xdg_dirs.clone()),
            library_service: LibraryService::new(Arc::new(app_xdg_dirs.clone())),
            library_update_cancel: None,
            playback_service: PlaybackService::new(mpris_rx),
            initial_load_complete: false,
            library: Library::new(),
            is_updating: false,
            update_progress: 0.0,
            update_total: 0.0,
            update_percent: 0.0,
            update_progress_display: "0".into(),
            dialog_pages: DialogPages::new(),
            view_mode: ViewMode::List,
            size_multiplier: _flags.state.size_multiplier,
            list_scroll_id: widget::Id::unique(),
            list_start: 0,
            list_visible_row_count: 0,
            list_last_clicked: None,
            list_last_selected_id: None,
            control_pressed: 0,
            shift_pressed: 0,
            view_playlist: None,
            search_id: widget::Id::new(SEARCH_INPUT_ID),
            search_term: None,
            image_store: ImageStore::new(artwork_dir.clone()),
            playlist_service: PlaylistService::new(Arc::new(app_xdg_dirs.clone())),
        };

        // Create a startup command that sets the window title.
        let update_title = app.update_title();

        // Load the master library and playlists
        let load_data = app.load_data();

        (app, Task::batch([update_title, load_data]))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu_bar(self);
        vec![menu_bar.into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::with_capacity(1);

        if self.search_term.is_some() {
            elements.push(
                widget::text_input::search_input("", self.search_term.clone().unwrap())
                    .width(Length::Fixed(SEARCH_INPUT_WIDTH))
                    .id(self.search_id.clone())
                    .on_clear(Message::SearchClear)
                    .on_input(Message::SearchInput)
                    .into(),
            );
        } else {
            elements.push(
                widget::button::icon(widget::icon::from_name("system-search-symbolic"))
                    .on_press(Message::SearchActivate)
                    .padding(8)
                    .into(),
            );
        }

        elements
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
            ContextPage::Settings => context_drawer::context_drawer(
                self.settings(),
                Message::ToggleContextPage(ContextPage::Settings),
            )
            .title(fl!("settings")),
            ContextPage::TrackInfo => context_drawer::context_drawer(
                self.track_info_panel(),
                Message::ToggleContextPage(ContextPage::TrackInfo),
            )
            .title(fl!("track-info")),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        if self.initial_load_complete == false {
            return loading::content().into();
        }

        let playlist = self.playlist_service.get(self.view_playlist.unwrap()).ok();

        let content: Column<_> = match playlist {
            Some(p) if p.is_library() && p.tracks().is_empty() => empty_library::content(),
            Some(_) => list_view::content(self),
            None => empty_library::content(),
        };

        widget::container(widget::column().push(content))
            .apply(widget::container)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Top)
            .into()
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        let dialog_page = self.dialog_pages.front()?;

        let dialog = match dialog_page {
            DialogPage::NewPlaylist(name) => {
                let complete_maybe = if name.is_empty() {
                    None
                } else if name.trim().is_empty() {
                    None
                } else {
                    Some(Message::DialogComplete)
                };

                let dialog = widget::dialog()
                    .title(fl!("new-playlist"))
                    .primary_action(
                        widget::button::suggested(fl!("create")).on_press_maybe(complete_maybe),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel")).on_press(Message::DialogCancel),
                    )
                    .control(widget::column::with_children(vec![
                        widget::text_input(fl!("untitled-playlist"), name)
                            .id(widget::Id::new(NEW_PLAYLIST_INPUT_ID))
                            .on_input(move |name| {
                                Message::UpdateDialog(DialogPage::NewPlaylist(name))
                            })
                            .into(),
                    ]));

                dialog
            }

            DialogPage::RenamePlaylist { id, name } => {
                let complete_maybe = if name.is_empty() {
                    None
                } else if name.trim().is_empty() {
                    None
                } else {
                    Some(Message::DialogComplete)
                };

                let dialog = widget::dialog()
                    .title(fl!("rename-playlist"))
                    .primary_action(
                        widget::button::suggested(fl!("rename")).on_press_maybe(complete_maybe),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel")).on_press(Message::DialogCancel),
                    )
                    .control(widget::column::with_children(vec![
                        widget::text_input("", name)
                            .on_input(move |name| {
                                Message::UpdateDialog(DialogPage::RenamePlaylist {
                                    id: *id,
                                    name: name,
                                })
                            })
                            .id(widget::Id::new(RENAME_PLAYLIST_INPUT_ID))
                            .into(),
                    ]));
                dialog
            }

            DialogPage::DeletePlaylist(id) => {
                let playlist = self.playlist_service.get(*id).ok();

                let dialog = widget::dialog()
                    .title(fl!("delete-playlist"))
                    .icon(widget::icon::from_name("dialog-warning").size(64))
                    .body(format!("{} {}?", fl!("delete"), playlist.unwrap().name()))
                    .primary_action(
                        widget::button::suggested(fl!("yes")).on_press(Message::DialogComplete),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel")).on_press(Message::DialogCancel),
                    )
                    .control(widget::column::with_children(vec![
                        widget::text(fl!("delete-warning")).into(),
                    ]));
                dialog
            }

            DialogPage::DeleteSelectedFromPlaylist => {
                let view_playlist = self
                    .playlist_service
                    .get(self.view_playlist.unwrap())
                    .ok()
                    .unwrap();

                let dialog = widget::dialog()
                    .title(fl!("remove-selected-from-playlist"))
                    .icon(widget::icon::from_name("dialog-warning").size(64))
                    .body(format!(
                        "{} {} {} {}?",
                        fl!("remove"),
                        view_playlist.selected_iter().count(),
                        fl!("tracks-from"),
                        view_playlist.name()
                    ))
                    .primary_action(
                        widget::button::suggested(fl!("yes")).on_press(Message::DialogComplete),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel")).on_press(Message::DialogCancel),
                    );

                dialog
            }
        };

        Some(dialog.into())
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        // Add subscriptions which are always active.
        let mut subscriptions = vec![
            event::listen_with(|event, _status, _window_id| match event {
                Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. }) => {
                    Some(Message::KeyPressed(modifiers, key))
                }
                Event::Keyboard(KeyEvent::KeyReleased { key, .. }) => {
                    Some(Message::KeyReleased(key))
                }
                Event::Window(WindowEvent::CloseRequested) => Some(Message::Quit),
                Event::Window(WindowEvent::Closed) => Some(Message::Quit),
                Event::Window(WindowEvent::Resized(size)) => Some(Message::WindowResized(size)),
                _ => None,
            }),
            // Watch for application configuration changes.
            self.core().watch_config::<Config>(APP_ID).map(|update| {
                // for why in update.errors {
                //     tracing::error!(?why, "app config error");
                // }

                Message::UpdateConfig(update.config)
            }),
        ];

        // Tick
        subscriptions.push(
            iced::time::every(Duration::from_millis(TICK_INTERVAL_MS)).map(|_| Message::Tick),
        );

        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> cosmic::Task<cosmic::Action<Self::Message>> {
        self.is_condensed = self.core().is_condensed();

        // Helper for updating configuration
        macro_rules! config_set {
            ($name: ident, $value: expr) => {
                match &self.config_handler {
                    Some(config_handler) => {
                        match paste::paste! { self.config.[<set_ $name>](&config_handler, $value) }
                        {
                            Ok(_) => {}
                            Err(err) => {
                                log::warn!(
                                    "failed to save config {:?}: {}",
                                    stringify!($name),
                                    err
                                );
                            }
                        }
                    }
                    None => {
                        self.config.$name = $value;
                        log::warn!(
                            "failed to save config {:?}: no config handler",
                            stringify!($name)
                        );
                    }
                }
            };
        }

        // Helper for updating application state
        macro_rules! state_set {
            ($name: ident, $value: expr) => {
                match &self.state_handler {
                    Some(state_handler) => {
                        match paste::paste! { self.state.[<set_ $name>](&state_handler, $value) } {
                            Ok(_) => {}
                            Err(err) => {
                                log::warn!("failed to save state {:?}: {}", stringify!($name), err);
                            }
                        }
                    }
                    None => {
                        self.state.$name = $value;
                        log::warn!(
                            "failed to save state {:?}: no config handler",
                            stringify!($name)
                        );
                    }
                }
            };
        }

        match message {
            // Open dialog for adding library locations
            Message::AddLibraryDialog => {
                return cosmic::task::future(async move {
                    let dialog = file_chooser::open::Dialog::new().title(fl!("add-location"));

                    match dialog.open_folders().await {
                        Ok(response) => {
                            let mut paths: Vec<String> = Vec::new();

                            for u in response.urls() {
                                if let Ok(decoded) = decode(u.path()) {
                                    paths.push(decoded.into_owned());
                                } else {
                                    eprintln!("Can't decode URL.");
                                }
                            }
                            Message::SelectedPaths(paths)
                        }
                        Err(file_chooser::Error::Cancelled) => Message::Noop,
                        Err(why) => Message::LibraryPathOpenError(Arc::new(why)),
                    }
                });
            }

            Message::AppTheme(app_theme) => {
                config_set!(app_theme, app_theme);
                return self.update_config();
            }

            Message::AddSelectedToPlaylist(destination_id) => {
                let source_id = match self.view_playlist {
                    Some(id) => id,
                    None => return Task::none(),
                };

                // Get selected tracks from source playlist
                let selected_tracks: Vec<Track> = match self.playlist_service.get(source_id) {
                    Ok(playlist) => playlist
                        .selected()
                        .iter()
                        .map(|t| {
                            let mut track = (*t).clone();
                            track.selected = false;
                            track
                        })
                        .collect(),
                    Err(_) => return Task::none(),
                };

                if let Err(err) = self
                    .playlist_service
                    .add_tracks(destination_id, selected_tracks)
                {
                    eprintln!("Error adding tracks: {}", err);
                }
            }

            Message::AddNowPlayingToPlaylist(destination_id) => {
                if let Some(now_playing) = self.playback_service.now_playing().clone() {
                    if let Some(now_playing_data) = self
                        .library
                        .from_id(&now_playing.clone().id.unwrap_or_default())
                    {
                        let track = Track {
                            path: now_playing_data.0.clone(),
                            metadata: now_playing_data.1.clone(),
                            ..Default::default()
                        };

                        if let Err(err) = self
                            .playlist_service
                            .add_tracks(destination_id, vec![track])
                        {
                            eprintln!("Error adding now playing to playlist: {}", err);
                        }
                    }
                }
            }

            Message::CancelLibraryUpdate => {
                if let Some(cancel_token) = self.library_update_cancel.take() {
                    cancel_token.cancel();
                    self.is_updating = false;
                }
            }

            Message::ChangeTrack(id, index) => {
                if self.library.from_id(&id).is_none() {
                    return Task::none();
                }

                let now = Instant::now();

                if let Some(last) = self.list_last_clicked {
                    if is_double_click(last, DOUBLE_CLICK_THRESHOLD_MS) {
                        // Double-click: play track
                        if let Some(playlist_id) = self.view_playlist {
                            if let Ok(playlist) = self.playlist_service.get(playlist_id) {
                                self.playback_service.start_session(
                                    playlist,
                                    index,
                                    self.state.shuffle,
                                );
                                self.playback_service.play();
                            }
                        }
                    }
                }

                self.list_last_clicked = Some(now);
            }

            Message::DialogCancel => {
                let _ = self.dialog_pages.pop_front();
            }

            Message::DialogComplete => {
                if let Some(dialog_page) = self.dialog_pages.pop_front() {
                    match dialog_page {
                        DialogPage::NewPlaylist(name) => {
                            match self.playlist_service.create(name) {
                                Ok(id) => {
                                    self.view_playlist = Some(id);

                                    // Rebuild nav preserving order
                                    let items = self.build_ordered_nav_items();
                                    self.rebuild_nav_from_order(items, id);
                                }
                                Err(err) => {
                                    eprintln!("Error creating playlist: {}", err);
                                }
                            }
                        }

                        DialogPage::RenamePlaylist { id, name } => {
                            match self.playlist_service.rename(id, name) {
                                Ok(_) => {
                                    self.view_playlist = Some(id);

                                    // Rebuild nav preserving order
                                    let items = self.build_ordered_nav_items();
                                    self.rebuild_nav_from_order(items, id);
                                }
                                Err(err) => {
                                    eprintln!("Error renaming playlist: {}", err);
                                }
                            }
                        }

                        DialogPage::DeletePlaylist(id) => {
                            match self.playlist_service.delete(id) {
                                Ok(_) => {
                                    // Switch to library view
                                    let library_id =
                                        if let Ok(library) = self.playlist_service.get_library() {
                                            library.id()
                                        } else {
                                            return Task::none();
                                        };

                                    self.view_playlist = Some(library_id);

                                    // Rebuild nav preserving order
                                    let items = self.build_ordered_nav_items();
                                    self.rebuild_nav_from_order(items, library_id);
                                }
                                Err(err) => {
                                    eprintln!("Error deleting playlist: {}", err);
                                }
                            }
                        }

                        DialogPage::DeleteSelectedFromPlaylist => {
                            let playlist_id = match self.view_playlist {
                                Some(id) => id,
                                None => return Task::none(),
                            };

                            if let Err(err) = self.playlist_service.remove_selected(playlist_id) {
                                eprintln!("Error removing tracks: {}", err);
                            }
                        }
                    };
                };
            }

            Message::KeyPressed(modifiers, key) => {
                return self.handle_key_pressed(modifiers, key);
            }

            Message::KeyReleased(key) => {
                if key == Key::Named(Named::Control) {
                    self.control_pressed = self.control_pressed.saturating_sub(1);
                }
                if key == Key::Named(Named::Shift) {
                    self.shift_pressed = self.shift_pressed.saturating_sub(1);
                }
            }

            Message::LibraryPathOpenError(why) => {
                eprintln!("{why}");
            }

            Message::LibraryProgress(progress) => match progress {
                LibraryProgress::Progress {
                    current,
                    total,
                    percent,
                } => {
                    self.update_progress = current;
                    self.update_total = total;
                    self.update_percent = percent;
                    self.update_progress_display = format!(
                        "{} {}/{} ({:.2}%)",
                        fl!("updating-library"),
                        current,
                        total,
                        percent
                    );
                }

                LibraryProgress::PartialUpdate(media) => {
                    self.library.media = media;
                    if let Err(e) = self.library_service.save(&self.library) {
                        eprintln!("Error saving partial library update: {}", e);
                    }
                    self.update_library_playlist();
                }

                LibraryProgress::Complete(library) => {
                    self.library = library;
                    if let Err(e) = self.library_service.save(&self.library) {
                        eprintln!("Error saving library: {}", e);
                    }
                    self.is_updating = false;
                    self.update_library_playlist();
                }

                LibraryProgress::Cancelled => {
                    self.is_updating = false;
                    self.update_library_playlist();
                    log::info!("Library update cancelled")
                }
            },

            Message::ListSelectRow(index) => {
                let Some(playlist_id) = self.view_playlist else {
                    return Task::none();
                };

                if self.control_pressed > 0 {
                    // Ctrl + Click: toggle selection
                    let Ok(playlist) = self.playlist_service.get(playlist_id) else {
                        return Task::none();
                    };

                    if playlist.tracks().get(index).map_or(false, |t| t.selected) {
                        let _ = self.playlist_service.deselect_track(playlist_id, index);
                    } else {
                        let _ = self.playlist_service.select_track(playlist_id, index);
                    }

                    self.list_last_selected_id = Some(index);
                } else if self.control_pressed > 0 && self.shift_pressed > 0 {
                    // Ctrl + Shift + Click: select range
                    if let Some(last_id) = self.list_last_selected_id {
                        let _ = self
                            .playlist_service
                            .select_range(playlist_id, last_id, index);
                    }
                } else {
                    // Click: clear all and select one
                    let _ = self.playlist_service.clear_selection(playlist_id);
                    let _ = self.playlist_service.select_track(playlist_id, index);

                    // TODO: Handle double click for playback

                    self.list_last_selected_id = Some(index);
                }
            }

            // Handle scroll events from scrollable widgets
            Message::ListViewScroll(viewport) => {
                let scroll_offset = viewport.absolute_offset().y;
                let viewport_height = viewport.bounds().height;

                let row_stride =
                    calculate_row_stride(self.size_multiplier, BASE_ROW_HEIGHT, DIVIDER_HEIGHT);

                // Update scroll position
                if scroll_offset == 0.0 || row_stride == 0.0 {
                    self.list_start = 0;
                } else {
                    self.list_start = (scroll_offset / row_stride).floor() as usize;
                }

                // Update visible row count
                self.list_visible_row_count = (viewport_height / row_stride).ceil() as usize;

                // Clamp to valid range
                let tracks_len = self
                    .view_playlist
                    .and_then(|id| self.playlist_service.get(id).ok())
                    .map(|p| p.len())
                    .unwrap_or(0);

                let max_start = tracks_len.saturating_sub(self.list_visible_row_count);
                self.list_start = self.list_start.min(max_start);
            }

            Message::ListViewSort(new_sort_by) => {
                let new_direction = if self.state.sort_by == new_sort_by {
                    match self.state.sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    }
                } else {
                    SortDirection::Ascending
                };

                state_set!(sort_by, new_sort_by.clone());
                state_set!(sort_direction, new_direction.clone());

                // Sort ALL playlists (including library)
                let playlist_ids: Vec<u32> =
                    self.playlist_service.all().iter().map(|p| p.id()).collect();

                for id in playlist_ids {
                    if let Ok(playlist) = self.playlist_service.get_mut(id) {
                        playlist.sort(new_sort_by.clone(), new_direction.clone());
                    }
                }
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            // Kick off the New Playlist dialog
            Message::NewPlaylist => {
                self.dialog_pages
                    .push_back(DialogPage::NewPlaylist(String::new()));
                return widget::text_input::focus(widget::Id::new(NEW_PLAYLIST_INPUT_ID));
            }

            Message::Noop => {}

            // Kick off the Rename Playlist dialog
            Message::RenamePlaylist => match self.nav.data(self.nav.active()) {
                Some(Page::Playlist(id)) => {
                    if self.playlist_service.get(*id).ok().unwrap().is_library() {
                        return Task::none();
                    }
                    let name = self.nav.text(self.nav.active()).unwrap_or("");
                    self.dialog_pages.push_back(DialogPage::RenamePlaylist {
                        id: *id,
                        name: name.into(),
                    });
                    return widget::text_input::focus(widget::Id::new(RENAME_PLAYLIST_INPUT_ID));
                }
                _ => {}
            },

            // Kick off the delete playlist dialog
            Message::DeletePlaylist => {
                if let Some(Page::Playlist(id)) = self.nav.data(self.nav.active()) {
                    if let Some(p) = self.playlist_service.get(*id).ok() {
                        if !p.is_library() {
                            self.dialog_pages.push_back(DialogPage::DeletePlaylist(*id));
                        }
                    }
                }
            }

            Message::MoveNavUp | Message::MoveNavDown => {
                self.move_active_nav(if matches!(message, Message::MoveNavUp) {
                    -1
                } else {
                    1
                });

                let order = self.nav_order();

                state_set!(playlist_nav_order, order);
            }

            Message::Next => {
                self.playback_service
                    .next(self.state.repeat_mode.clone(), self.state.repeat);
            }

            Message::PlayPause => {
                match self.playback_service.status() {
                    PlaybackStatus::Stopped => {
                        // Start playback from current view
                        if let Some(playlist_id) = self.view_playlist {
                            if let Ok(playlist) = self.playlist_service.get(playlist_id) {
                                // Pick random starting index if shuffle is enabled
                                let start_index =
                                    if self.state.shuffle && !playlist.tracks().is_empty() {
                                        use rand::Rng;
                                        rand::rng().random_range(0..playlist.tracks().len())
                                    } else {
                                        0
                                    };

                                self.playback_service.start_session(
                                    playlist,
                                    start_index,
                                    self.state.shuffle,
                                );
                                self.playback_service.play();
                            }
                        }
                    }
                    PlaybackStatus::Paused => {
                        self.playback_service.play();
                    }
                    PlaybackStatus::Playing => {
                        self.playback_service.pause();
                    }
                }
            }

            Message::Previous => {
                self.playback_service.prev(self.state.repeat_mode.clone());
            }

            Message::Quit => {
                self.playback_service.stop();
                process::exit(0);
            }

            Message::ReleaseSlider => {
                let time = self.playback_service.progress();
                self.playback_service.set_dragging_slider(false);
                self.playback_service.seek(time);
            }

            Message::RemoveLibraryPath(path) => {
                let mut library_paths = self.config.library_paths.clone();
                library_paths.remove(&path);
                config_set!(library_paths, library_paths);
            }

            Message::RemoveSelectedFromPlaylist => {
                // Show confirmation dialog
                self.dialog_pages
                    .push_back(DialogPage::DeleteSelectedFromPlaylist);
            }

            Message::SearchActivate => {
                self.search_term = Some(String::new());
                return widget::text_input::focus(self.search_id.clone());
            }

            Message::SearchClear => {
                self.search_term = None;

                // Reset viewport scroll to top
                self.list_start = 0;
                return scrollable::scroll_to(
                    self.list_scroll_id.clone(),
                    AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }

            Message::SearchInput(term) => {
                self.search_term = Some(term);

                // Reset viewport scroll to top
                self.list_start = 0;
                return scrollable::scroll_to(
                    self.list_scroll_id.clone(),
                    AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }

            Message::SelectAll => {
                if let Some(playlist_id) = self.view_playlist {
                    if let Err(err) = self.playlist_service.select_all(playlist_id) {
                        eprintln!("Error selecting all tracks: {}", err);
                    }
                }
            }

            // Add selected paths from the Open dialog
            Message::SelectedPaths(paths) => {
                let mut library_paths = self.config.library_paths.clone();

                for path in paths {
                    library_paths.insert(path);
                }

                config_set!(library_paths, library_paths);
            }

            Message::SetVolume(volume) => {
                state_set!(volume, volume);
                self.playback_service.set_volume(volume as f64 / 100.0);
            }

            Message::SliderSeek(time) => {
                self.playback_service.set_dragging_slider(true);
                self.playback_service.set_progress(time);
            }

            Message::Tick => {
                self.playback_service.validate_session();

                // Process playback events
                let events = self.playback_service.tick();
                for event in events {
                    match event {
                        PlaybackEvent::TrackEnded => {
                            self.playback_service
                                .next(self.state.repeat_mode.clone(), self.state.repeat);
                        }
                        PlaybackEvent::Error(err) => {
                            eprintln!("Playback error: {}", err);
                            self.playback_service
                                .next(self.state.repeat_mode.clone(), self.state.repeat);
                        }
                        PlaybackEvent::PositionUpdate(_) => {
                            // Position already updated in service
                        }
                    }
                }

                // Handle MPRIS commands
                let commands = self.playback_service.process_mpris_commands();
                for cmd in commands {
                    println!("mpris message: {:?}", cmd);
                    match cmd {
                        MprisCommand::Play => self.playback_service.play(),
                        MprisCommand::Pause => self.playback_service.pause(),
                        MprisCommand::PlayPause => self.playback_service.play_pause(),
                        MprisCommand::Stop => self.playback_service.stop(),
                        MprisCommand::Next => self
                            .playback_service
                            .next(self.state.repeat_mode.clone(), self.state.repeat),
                        MprisCommand::Previous => {
                            self.playback_service.prev(self.state.repeat_mode.clone())
                        }
                        _ => {}
                    }
                }
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                // Restore scroll position from view model
                if let Some(view_model) = self.calculate_list_view() {
                    return scrollable::scroll_to::<Action<Message>>(
                        self.list_scroll_id.clone(),
                        AbsoluteOffset {
                            x: 0.0,
                            y: view_model.scroll_offset,
                        },
                    );
                }

                return Task::none();
            }

            Message::ToggleListTextWrap(list_text_wrap) => {
                config_set!(list_text_wrap, list_text_wrap);
            }

            Message::ToggleListRowAlignTop(list_row_align_top) => {
                config_set!(list_row_align_top, list_row_align_top);
            }

            Message::ToggleMute => {
                let muted = !self.state.muted;
                if muted {
                    self.playback_service.set_volume(0.0);
                } else {
                    self.playback_service
                        .set_volume(self.state.volume as f64 / 100.0);
                }
                state_set!(muted, muted);
            }

            Message::ToggleRepeat => {
                let repeat = !self.state.repeat;
                state_set!(repeat, repeat);
            }

            Message::ToggleRepeatMode => {
                let repeat_mode = if self.state.repeat_mode == RepeatMode::All {
                    RepeatMode::One
                } else {
                    RepeatMode::All
                };

                state_set!(repeat_mode, repeat_mode);
            }

            Message::ToggleShuffle => {
                let shuffle = !self.state.shuffle;
                state_set!(shuffle, shuffle);

                // Update current session with new shuffle setting
                if let Some(session) = self.playback_service.session() {
                    let playlist_id = session.playlist_id;
                    if let Ok(playlist) = self.playlist_service.get(playlist_id) {
                        self.playback_service
                            .update_session_shuffle(playlist, shuffle);
                    }
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::UpdateDialog(dialog_page) => match dialog_page {
                DialogPage::NewPlaylist(name) => {
                    self.dialog_pages
                        .update_front(DialogPage::NewPlaylist(name));
                }

                DialogPage::RenamePlaylist { id, name } => {
                    self.dialog_pages
                        .update_front(DialogPage::RenamePlaylist { id: id, name: name });
                }

                DialogPage::DeletePlaylist(id) => {
                    self.dialog_pages
                        .update_front(DialogPage::DeletePlaylist(id));
                }

                DialogPage::DeleteSelectedFromPlaylist => {}
            },

            Message::UpdateLibrary => {
                if self.is_updating {
                    return Task::none();
                }
                self.is_updating = true;
                self.update_progress = 0.0;

                let library_paths = self.config.library_paths.clone();
                let xdg_dirs = self.app_xdg_dirs.clone();

                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

                // Create cancellation token
                let cancel_token = CancellationToken::new();
                self.library_update_cancel = Some(cancel_token.clone()); // ← Store it

                // Spawn the scan with cancellation support
                LibraryService::scan_library(library_paths, xdg_dirs, tx, cancel_token);

                return cosmic::Task::stream(UnboundedReceiverStream::new(rx))
                    .map(|progress| cosmic::Action::App(Message::LibraryProgress(progress)));
            }

            Message::WindowResized(size) => {
                let window_width = size.width;
                let window_height = size.height;
                state_set!(window_width, window_width);
                state_set!(window_height, window_height);
            }

            Message::ZoomIn => {
                self.size_multiplier = clamp(
                    self.size_multiplier + ZOOM_STEP,
                    MIN_SIZE_MULTIPLIER,
                    MAX_SIZE_MULTIPLIER,
                );
                state_set!(size_multiplier, self.size_multiplier);
            }

            Message::ZoomOut => {
                self.size_multiplier = clamp(
                    self.size_multiplier - ZOOM_STEP,
                    MIN_SIZE_MULTIPLIER,
                    MAX_SIZE_MULTIPLIER,
                );
                state_set!(size_multiplier, self.size_multiplier);
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        if let Some(Page::Playlist(pid)) = self.nav.data(id) {
            // Check if we're switching to a different playlist
            let is_switching = self.view_playlist != Some(*pid);

            self.view_playlist = Some(*pid);

            if is_switching {
                // Reset state when switching playlists
                self.list_last_selected_id = None;
                self.list_start = 0; // Reset scroll position state

                return Task::batch([
                    self.update_title(),
                    scrollable::scroll_to(
                        self.list_scroll_id.clone(),
                        AbsoluteOffset { x: 0.0, y: 0.0 },
                    ),
                ]);
            }
        }

        self.update_title()
    }

    /// Footer area
    fn footer(&self) -> Option<Element<'_, Message>> {
        Some(footer(self).into())
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        let page = self.nav.text(self.nav.active());

        if page.is_some() {
            window_title.push_str(" — ");
            window_title.push_str(page.unwrap());
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }

    /// Settings page content
    fn settings(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;
        let app_theme_selected = match self.config.app_theme {
            AppTheme::Dark => 1,
            AppTheme::Light => 2,
            AppTheme::System => 0,
        };

        let mut library_column = widget::column();

        library_column = library_column.push(
            row()
                .push(
                    widget::column()
                        .push(
                            widget::button::text(fl!("add-location"))
                                .on_press(Message::AddLibraryDialog),
                        )
                        .width(Length::FillPortion(1))
                        .align_x(Alignment::Start),
                )
                .push(
                    widget::column()
                        .push(
                            widget::button::text(fl!("update-library"))
                                .on_press(Message::UpdateLibrary),
                        )
                        .width(Length::FillPortion(1))
                        .align_x(Alignment::End),
                )
                .width(Length::Fill),
        );

        let library_paths_length = self.config.library_paths.len().saturating_sub(1);

        // Create library path rows
        for (i, path) in self.config.library_paths.iter().enumerate() {
            library_column = library_column.push(
                row()
                    .width(Length::Fill)
                    .padding(space_xxs)
                    // Adds text
                    .push(text::text(path.clone()).width(Length::FillPortion(1)))
                    // Adds delete button
                    .push(
                        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                            .on_press(Message::RemoveLibraryPath(path.clone())),
                    ),
            );

            if i < library_paths_length {
                library_column = library_column.push(widget::divider::horizontal::light());
            }
        }

        settings::view_column(vec![
            settings::section()
                .title(fl!("appearance"))
                .add({
                    widget::settings::item::builder(fl!("theme")).control(widget::dropdown(
                        &self.app_theme_labels,
                        Some(app_theme_selected),
                        move |index| {
                            Message::AppTheme(match index {
                                1 => AppTheme::Dark,
                                2 => AppTheme::Light,
                                _ => AppTheme::System,
                            })
                        },
                    ))
                })
                .into(),
            settings::section()
                .title(fl!("list-view"))
                .add({
                    settings::item::builder(fl!("wrap-text")).control(
                        toggler(self.config.list_text_wrap).on_toggle(Message::ToggleListTextWrap),
                    )
                })
                .add({
                    settings::item::builder(fl!("align-rows-top")).control(
                        toggler(self.config.list_row_align_top)
                            .on_toggle(Message::ToggleListRowAlignTop),
                    )
                })
                .into(),
            settings::section()
                .title(fl!("library"))
                .add(library_column)
                .into(),
        ])
        .into()
    }

    /// Track info panel
    fn track_info_panel(&self) -> Element<'_, Message> {
        let cosmic_theme::Spacing { space_xs, .. } = theme::active().cosmic().spacing;

        let active_playlist = self.get_active_playlist();

        let tracks: Vec<&Track> = match active_playlist {
            Some(playlist) => playlist.selected(),
            None => vec![],
        };
        let take = TRACK_INFO_LIST_TOTAL;

        let mut column = widget::column().spacing(space_xs);

        for (i, t) in tracks.iter().enumerate().take(take) {
            let duration = t.metadata.duration.clone().unwrap_or(0.0);
            let minutes = (duration / 60.0) as u32;
            let seconds = f32::trunc(duration) as u32 - (minutes * 60);
            let display_duration = format!("{}:{:02}", minutes, seconds);

            let container = widget::container(
                widget::column()
                    .push(track_info_row(
                        fl!("title"),
                        t.metadata.title.clone().unwrap_or_default(),
                    ))
                    .push(track_info_row(
                        fl!("album"),
                        t.metadata.album.clone().unwrap_or_default(),
                    ))
                    .push(track_info_row(
                        fl!("artist"),
                        t.metadata.artist.clone().unwrap_or_default(),
                    ))
                    .push(track_info_row(
                        fl!("album-artist"),
                        t.metadata.album_artist.clone().unwrap_or_default(),
                    ))
                    .push(track_info_row(
                        fl!("genre"),
                        t.metadata.genre.clone().unwrap_or_default(),
                    ))
                    .push(track_info_row(
                        fl!("album-disc-number"),
                        t.metadata
                            .album_disc_number
                            .clone()
                            .unwrap_or_default()
                            .to_string(),
                    ))
                    .push(track_info_row(
                        fl!("album-disc-count"),
                        t.metadata
                            .album_disc_count
                            .clone()
                            .unwrap_or_default()
                            .to_string(),
                    ))
                    .push(track_info_row(
                        fl!("track-number"),
                        t.metadata
                            .track_number
                            .clone()
                            .unwrap_or_default()
                            .to_string(),
                    ))
                    .push(track_info_row(
                        fl!("track-count"),
                        t.metadata
                            .track_count
                            .clone()
                            .unwrap_or_default()
                            .to_string(),
                    ))
                    .push(track_info_row(fl!("duration"), display_duration))
                    .push(
                        widget::row()
                            .width(Length::Fill)
                            .push(widget::text(t.path.to_string_lossy())),
                    ),
            );

            if i > 0 {
                column = column.push(widget::divider::horizontal::light())
            }

            column = column.push(container);
        }

        if tracks.len() > take {
            column = column.push(widget::text("...".to_string()));
        }

        column.into()
    }

    /// Updates the cosmic config, in particular the theme
    fn update_config(&mut self) -> Task<cosmic::Action<Message>> {
        cosmic::command::set_theme(self.config.app_theme.theme())
    }

    /// Load library and playlists
    pub fn load_data(&mut self) -> Task<cosmic::Action<Message>> {
        // Load library from disk
        self.library = self.library_service.load().unwrap_or_else(|e| {
            eprintln!("Error loading library: {}", e);
            Library::new()
        });
        let library_media = self.library.media.clone();

        // Convert library to tracks
        let library_tracks: Vec<Track> = library_media
            .iter()
            .map(|(path, metadata)| {
                let mut track = Track::new();
                track.path = path.clone();
                track.metadata = metadata.clone();
                track
            })
            .collect();

        // Load all playlists through the service
        if let Err(e) = self.playlist_service.load_all(library_tracks) {
            eprintln!("Error loading playlists: {}", e);
            self.initial_load_complete = false;
            return Task::none();
        }

        let playlist_ids: Vec<u32> = self.playlist_service.all().iter().map(|p| p.id()).collect();

        for id in playlist_ids {
            if let Ok(playlist) = self.playlist_service.get_mut(id) {
                playlist.sort(
                    self.state.sort_by.clone(),
                    self.state.sort_direction.clone(),
                );
            }
        }

        // Decide nav order
        let items: Vec<NavPlaylistItem> = if !self.state.playlist_nav_order.is_empty() {
            // Start with saved order
            let mut ordered_items: Vec<NavPlaylistItem> = self
                .state
                .playlist_nav_order
                .iter()
                .filter_map(|pid| {
                    self.playlist_service
                        .get(*pid)
                        .ok()
                        .map(|p| NavPlaylistItem {
                            id: *pid,
                            name: p.name().to_string(),
                        })
                })
                .collect();

            // Add any playlists that aren't in the saved order
            let ordered_ids: HashSet<_> = ordered_items.iter().map(|item| item.id).collect();
            for playlist in self.playlist_service.user_playlists() {
                if !ordered_ids.contains(&playlist.id()) {
                    ordered_items.push(NavPlaylistItem {
                        id: playlist.id(),
                        name: playlist.name().to_string(),
                    });
                }
            }

            ordered_items
        } else {
            self.playlist_service
                .user_playlists()
                .map(|p| NavPlaylistItem {
                    id: p.id(),
                    name: p.name().to_string(),
                })
                .collect()
        };

        // Decide what should be active
        let active_id = match self.view_playlist {
            Some(id) => id,
            None => match self.playlist_service.get_library() {
                Ok(lib) => lib.id(),
                Err(e) => {
                    eprintln!("Failed to get library playlist: {}", e);
                    self.initial_load_complete = false; // Stay in loading state
                    return Task::none();
                }
            },
        };

        // Rebuild nav once
        self.rebuild_nav_from_order(items, active_id);

        self.initial_load_complete = true;
        Task::none()
    }

    fn rebuild_nav_from_order(&mut self, items: Vec<NavPlaylistItem>, activate_id: u32) {
        self.nav.clear();

        // Add the library first
        let library_id = self.playlist_service.get_library().map(|p| p.id()).unwrap();

        self.nav
            .insert()
            .text(fl!("library"))
            .data(Page::Playlist(library_id))
            .icon(widget::icon::from_name("folder-music-symbolic"));

        // Add the playlists
        for (i, item) in items.iter().enumerate() {
            if self.playlist_service.get(item.id).is_err() {
                continue;
            }

            self.nav
                .insert()
                .text(item.name.clone())
                .icon(widget::icon::from_name("playlist-symbolic"))
                .data(Page::Playlist(item.id))
                .divider_above(i == 0);
        }

        let nav_id_to_activate = self
            .nav
            .iter()
            .find_map(|id| match self.nav.data::<Page>(id) {
                Some(Page::Playlist(pid)) if *pid == activate_id => Some(id),
                _ => None,
            });

        if let Some(id) = nav_id_to_activate {
            self.nav.activate(id);
            self.view_playlist = Some(activate_id);
        }

        self.nav_order();
    }

    /// Swap positions of nav items
    fn move_active_nav(&mut self, direction: i32) {
        let active = self.nav.active();

        let Some(Page::Playlist(active_id)) = self.nav.data::<Page>(active) else {
            return;
        };

        let active_playlist = match self.playlist_service.get(*active_id) {
            Ok(p) => p,
            Err(_) => return,
        };

        if active_playlist.is_library() {
            return;
        }

        let mut items: Vec<_> = self
            .nav
            .iter()
            .filter_map(|nav_id| {
                self.nav.data::<Page>(nav_id).and_then(|p| match p {
                    Page::Playlist(pid) => self
                        .playlist_service
                        .get(*pid)
                        .ok()
                        .filter(|pl| !pl.is_library())
                        .map(|pl| NavPlaylistItem {
                            id: *pid,
                            name: pl.name().to_string(),
                        }),
                })
            })
            .collect();

        let idx = items.iter().position(|p| p.id == *active_id).unwrap();

        let new_idx = match direction {
            -1 if idx > 0 => idx - 1,
            1 if idx + 1 < items.len() => idx + 1,
            _ => return,
        };

        items.swap(idx, new_idx);

        self.rebuild_nav_from_order(items, *active_id);
    }

    fn nav_order(&mut self) -> Vec<u32> {
        self.nav
            .iter()
            .filter_map(|id| {
                self.nav.data::<Page>(id).and_then(|page| match page {
                    Page::Playlist(pid) => self
                        .playlist_service
                        .get(*pid)
                        .ok()
                        .filter(|p| !p.is_library())
                        .map(|_| *pid),
                })
            })
            .collect()
    }

    pub fn calculate_list_view(&self) -> Option<ListViewModel> {
        let active_playlist = self.playlist_service.get(self.view_playlist?).ok()?;

        let search = self.search_term.as_deref().unwrap_or("").to_lowercase();

        let visible_tracks: Vec<(usize, Track)> = if self.search_term.is_some() {
            active_playlist
                .tracks()
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, t)| {
                    [
                        t.metadata.title.as_deref(),
                        t.metadata.album.as_deref(),
                        t.metadata.artist.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|v| v.to_lowercase().contains(&search))
                })
                .collect()
        } else {
            active_playlist
                .tracks()
                .iter()
                .cloned()
                .enumerate()
                .collect()
        };

        let mut list_start = self.list_start;
        let tracks_len = visible_tracks.len();

        let row_height = self.size_multiplier * BASE_ROW_HEIGHT;

        let row_stride =
            calculate_row_stride(self.size_multiplier, BASE_ROW_HEIGHT, DIVIDER_HEIGHT);

        let list_end = (list_start + self.list_visible_row_count + 1).min(tracks_len);

        if list_start >= list_end {
            list_start = 0;
        }

        let take = list_end.saturating_sub(list_start);
        let chars = tracks_len.to_string().len() as f32;
        let number_column_width = chars * 11.0;
        let icon_column_width = 24.0;
        let viewport_height = tracks_len as f32 * row_stride;

        let is_playing_playlist = self
            .playback_service
            .session()
            .map(|session| session.playlist_id == active_playlist.id())
            .unwrap_or(false);

        // Determine UI settings from config
        let wrapping = if self.config.list_text_wrap {
            Wrapping::Word
        } else {
            Wrapping::None
        };

        let row_align = if self.config.list_row_align_top {
            Alignment::Start
        } else {
            Alignment::Center
        };

        let sort_direction_icon = match self.state.sort_direction {
            SortDirection::Ascending => "pan-down-symbolic".to_string(),
            SortDirection::Descending => "pan-up-symbolic".to_string(),
        };

        let scroll_offset = list_start as f32 * row_stride;

        Some(ListViewModel {
            visible_tracks,
            list_start,
            list_end,
            take,
            number_column_width,
            icon_column_width,
            row_stride,
            viewport_height,
            is_playing_playlist,
            row_height,
            scroll_offset: scroll_offset,
            wrapping,
            row_align,
            sort_direction_icon,
        })
    }

    pub fn is_track_playing(&self, track: &Track, view_model: &ListViewModel) -> bool {
        view_model.is_playing_playlist
            && self
                .playback_service
                .session()
                .and_then(|session| {
                    session.order.get(session.index).and_then(|playing_track| {
                        let playing_id = playing_track.metadata.id.clone()?;
                        let current_id = track.metadata.id.clone()?;
                        Some(playing_id == current_id)
                    })
                })
                .unwrap_or(false)
    }

    /// Safely get the active playlist by ID
    fn get_active_playlist(&self) -> Option<&Playlist> {
        self.view_playlist
            .and_then(|id| self.playlist_service.get(id).ok())
    }

    /// Build nav items respecting the saved order
    fn build_ordered_nav_items(&self) -> Vec<NavPlaylistItem> {
        if !self.state.playlist_nav_order.is_empty() {
            // Start with saved order
            let mut ordered_items: Vec<NavPlaylistItem> = self
                .state
                .playlist_nav_order
                .iter()
                .filter_map(|pid| {
                    self.playlist_service
                        .get(*pid)
                        .ok()
                        .map(|p| NavPlaylistItem {
                            id: *pid,
                            name: p.name().to_string(),
                        })
                })
                .collect();

            // Add any playlists that aren't in the saved order (newly created ones)
            let ordered_ids: HashSet<_> = ordered_items.iter().map(|item| item.id).collect();
            for playlist in self.playlist_service.user_playlists() {
                if !ordered_ids.contains(&playlist.id()) {
                    ordered_items.push(NavPlaylistItem {
                        id: playlist.id(),
                        name: playlist.name().to_string(),
                    });
                }
            }

            ordered_items
        } else {
            // No saved order, just use all playlists
            self.playlist_service
                .user_playlists()
                .map(|p| NavPlaylistItem {
                    id: p.id(),
                    name: p.name().to_string(),
                })
                .collect()
        }
    }

    fn update_library_playlist(&mut self) {
        if let Ok(lib_playlist) = self.playlist_service.get_library_mut() {
            lib_playlist.clear();
            for (path, metadata) in &self.library.media {
                let mut track = Track::new();
                track.path = path.clone();
                track.metadata = metadata.clone();
                lib_playlist.push(track);
            }
            lib_playlist.sort(
                self.state.sort_by.clone(),
                self.state.sort_direction.clone(),
            );

            let library = lib_playlist.clone();
            self.playback_service.update_session_for_library(&library);
        }
    }

    fn handle_key_pressed(&mut self, modifiers: Modifiers, key: Key) -> Task<Action<Message>> {
        // Check key bindings first
        for (key_bind, action) in self.key_binds.iter() {
            if key_bind.matches(modifiers, &key) {
                // Don't call self.update() - return a task that processes the message
                let message = action.message();
                return Task::done(cosmic::Action::App(message));
            }
        }

        if key == Key::Named(Named::Control) && self.control_pressed < 2 {
            self.control_pressed += 1;
        }
        if key == Key::Named(Named::Shift) && self.shift_pressed < 2 {
            self.shift_pressed += 1;
        }

        if self.dialog_pages.front().is_some() {
            if key == Key::Named(Named::Escape) {
                return Task::done(cosmic::Action::App(Message::DialogCancel));
            }

            match self.dialog_pages.front().unwrap() {
                DialogPage::NewPlaylist(name) => {
                    if key == Key::Named(Named::Enter) && name.len() > 0 {
                        return Task::done(cosmic::Action::App(Message::DialogComplete));
                    }
                }
                DialogPage::RenamePlaylist { id, name } => {
                    let _ = id;
                    if key == Key::Named(Named::Enter) && name.len() > 0 {
                        return Task::done(cosmic::Action::App(Message::DialogComplete));
                    }
                }
                DialogPage::DeletePlaylist(_) => {}
                DialogPage::DeleteSelectedFromPlaylist => {}
            }

            if key == Key::Named(Named::Enter) {
                return Task::done(cosmic::Action::App(Message::DialogComplete));
            }
        }

        if matches!(self.view_mode, ViewMode::List) {
            if let Some(view_model) = self.calculate_list_view() {
                let scroll_amount = self.list_visible_row_count as f32 * view_model.row_stride;

                match key {
                    Key::Named(Named::PageUp) => {
                        return scrollable::scroll_by::<Action<Message>>(
                            self.list_scroll_id.clone(),
                            scrollable::AbsoluteOffset {
                                x: 0.0,
                                y: -scroll_amount,
                            },
                        );
                    }
                    Key::Named(Named::PageDown) => {
                        return scrollable::scroll_by::<Action<Message>>(
                            self.list_scroll_id.clone(),
                            scrollable::AbsoluteOffset {
                                x: 0.0,
                                y: scroll_amount,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        Task::none()
    }
}

#[derive(Clone)]
struct NavPlaylistItem {
    id: u32,
    name: String,
}

/// Flags passed into the app
#[derive(Clone, Debug)]
pub struct Flags {
    pub config_handler: Option<cosmic_config::Config>,
    pub state_handler: Option<cosmic_config::Config>,
    pub state: State,
}

/// The page to display in the application.
#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    Playlist(u32),
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    Settings,
    TrackInfo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    AddSelectedToPlaylist(PlaylistId),
    AddNowPlayingToPlaylist(PlaylistId),
    RemoveSelectedFromPlaylist,
    DeletePlaylist,
    MoveNavDown,
    MoveNavUp,
    NewPlaylist,
    Quit,
    RenamePlaylist,
    SelectAll,
    Settings,
    ToggleRepeat,
    ToggleRepeatMode,
    ToggleShuffle,
    TrackInfoPanel,
    UpdateLibrary,
    ZoomIn,
    ZoomOut,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::AddSelectedToPlaylist(id) => Message::AddSelectedToPlaylist(*id),
            MenuAction::AddNowPlayingToPlaylist(id) => Message::AddNowPlayingToPlaylist(*id),
            MenuAction::RemoveSelectedFromPlaylist => Message::RemoveSelectedFromPlaylist,
            MenuAction::DeletePlaylist => Message::DeletePlaylist,
            MenuAction::MoveNavDown => Message::MoveNavDown,
            MenuAction::MoveNavUp => Message::MoveNavUp,
            MenuAction::NewPlaylist => Message::NewPlaylist,
            MenuAction::RenamePlaylist => Message::RenamePlaylist,
            MenuAction::Quit => Message::Quit,
            MenuAction::SelectAll => Message::SelectAll,
            MenuAction::Settings => Message::ToggleContextPage(ContextPage::Settings),
            MenuAction::ToggleRepeat => Message::ToggleRepeat,
            MenuAction::ToggleRepeatMode => Message::ToggleRepeatMode,
            MenuAction::ToggleShuffle => Message::ToggleShuffle,
            MenuAction::TrackInfoPanel => Message::ToggleContextPage(ContextPage::TrackInfo),
            MenuAction::UpdateLibrary => Message::UpdateLibrary,
            MenuAction::ZoomIn => Message::ZoomIn,
            MenuAction::ZoomOut => Message::ZoomOut,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DialogPage {
    NewPlaylist(String),
    RenamePlaylist { id: u32, name: String },
    DeletePlaylist(u32),
    DeleteSelectedFromPlaylist,
}

pub struct DialogPages {
    pages: VecDeque<DialogPage>,
}

impl Default for DialogPages {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogPages {
    pub const fn new() -> Self {
        Self {
            pages: VecDeque::new(),
        }
    }

    pub fn front(&self) -> Option<&DialogPage> {
        self.pages.front()
    }

    pub fn push_back(&mut self, page: DialogPage) {
        self.pages.push_back(page);
    }

    #[must_use]
    pub fn pop_front(&mut self) -> Option<DialogPage> {
        let page = self.pages.pop_front()?;
        Some(page)
    }

    pub fn update_front(&mut self, page: DialogPage) {
        if !self.pages.is_empty() {
            self.pages[0] = page;
        }
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum SortBy {
    Artist,
    Album,
    Title,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaylistKind {
    Library,
    User,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViewMode {
    List,
}

fn track_info_row<'a>(title: String, data: String) -> widget::Row<'a, Message> {
    let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

    widget::row()
        .push(
            widget::text(title)
                .width(Length::FillPortion(1))
                .align_x(Alignment::End)
                .font(Font {
                    weight: Weight::Bold,
                    ..Font::default()
                }),
        )
        .push(widget::text(data).width(Length::FillPortion(1)))
        .spacing(space_xxs)
        .width(Length::Fill)
}

pub struct ListViewModel {
    pub visible_tracks: Vec<(usize, Track)>,
    pub list_start: usize,
    pub list_end: usize,
    pub take: usize,
    pub number_column_width: f32,
    pub icon_column_width: f32,
    pub row_stride: f32,
    pub viewport_height: f32,
    pub is_playing_playlist: bool,
    pub row_height: f32,
    pub scroll_offset: f32,
    pub wrapping: Wrapping,
    pub row_align: Alignment,
    pub sort_direction_icon: String,
}

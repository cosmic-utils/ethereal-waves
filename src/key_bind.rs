// SPDX-License-Identifier: GPL-3.0

use cosmic::{
    iced::keyboard::{Key, key::Named},
    widget::menu::key_bind::{KeyBind, Modifier},
};
use std::collections::HashMap;

use crate::app::{MenuAction, ViewMode};

pub fn key_binds() -> HashMap<KeyBind, MenuAction> {
    let mut key_binds = HashMap::new();

    macro_rules! bind {
        ([$($modifier:ident),* $(,)?], $key:expr, $action:ident) => {{
            key_binds.insert(
                KeyBind {
                    modifiers: vec![$(Modifier::$modifier),*],
                    key: $key,
                },
                MenuAction::$action
            );
        }};
    }

    bind!([Ctrl], Key::Character("u".into()), UpdateLibrary);
    bind!([Ctrl], Key::Character("q".into()), Quit);
    bind!([Ctrl], Key::Character(",".into()), Settings);
    bind!([Ctrl], Key::Character("-".into()), ZoomOut);
    bind!([Ctrl], Key::Character("=".into()), ZoomIn);
    bind!([Ctrl], Key::Character("n".into()), NewPlaylist);
    bind!([Ctrl], Key::Character("a".into()), SelectAll);
    bind!([], Key::Named(Named::F2), RenamePlaylist);
    bind!([Ctrl], Key::Named(Named::ArrowUp), MoveNavUp);
    bind!([Ctrl], Key::Named(Named::ArrowDown), MoveNavDown);
    bind!([], Key::Named(Named::F1), TrackInfoPanel);
    bind!([], Key::Character("m".into()), ToggleMute);
    bind!([], Key::Character("-".into()), VolumeDown);
    bind!([], Key::Character("=".into()), VolumeUp);

    key_binds.insert(
        KeyBind {
            modifiers: vec![Modifier::Ctrl],
            key: Key::Character("1".into()),
        },
        MenuAction::SetViewMode(ViewMode::List),
    );
    key_binds.insert(
        KeyBind {
            modifiers: vec![Modifier::Ctrl],
            key: Key::Character("2".into()),
        },
        MenuAction::SetViewMode(ViewMode::Grid),
    );

    key_binds
}

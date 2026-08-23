// SPDX-License-Identifier: GPL-3.0

use crate::app::{ContextPage, Message};
use crate::fl;
use cosmic::{
    Element, cosmic_theme,
    iced::{Alignment, Length},
    theme, widget,
};
pub fn content<'a>() -> Element<'a, Message> {
    let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

    widget::column![]
        .push(
            widget::row::with_children(vec![
                widget::text(fl!("go-to-view")).into(),
                widget::button::link(fl!("settings"))
                    .on_press(Message::ToggleContextPage(ContextPage::Settings))
                    .padding(0)
                    .into(),
                widget::text(fl!("then-update-library")).into(),
            ])
            .spacing(4),
        )
        .padding(space_xxs)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

// SPDX-License-Identifier: GPL-3.0

use crate::app::Message;
use crate::fl;
use cosmic::{
    Element, cosmic_theme,
    iced::{Alignment, Length},
    theme, widget,
};
pub fn content<'a>() -> Element<'a, Message> {
    let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

    let content = widget::column![]
        .push(widget::row![].push(widget::text(fl!("loading"))).spacing(4))
        .padding(space_xxs)
        .width(Length::Fill)
        .align_x(Alignment::Center);

    content.into()
}

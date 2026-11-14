use ducky_app::{Modal, ModalMsg, Msg};

use crate::mouse::{MouseTarget, ScrollArea, scroll_lines};
use crate::tui_state::TuiState;

pub(super) fn click(
    open_modal: &Modal,
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    is_inside: bool,
) -> Vec<Msg> {
    if !is_inside {
        if matches!(open_modal, Modal::Help) {
            tui.help_scroll = 0;
        }
        return vec![Msg::Modal(ModalMsg::Cancel)];
    }
    match target {
        Some(MouseTarget::ModalItem(item_index)) => vec![
            Msg::Modal(ModalMsg::Select(item_index)),
            Msg::Modal(ModalMsg::Submit),
        ],
        _ => Vec::new(),
    }
}

pub(super) fn wheel(
    tui: &mut TuiState,
    target: Option<MouseTarget>,
    is_inside: bool,
    is_down: bool,
) -> Vec<Msg> {
    if !is_inside {
        return Vec::new();
    }
    if target == Some(MouseTarget::Scroll(ScrollArea::Help)) {
        scroll_lines(&mut tui.help_scroll, is_down);
        return Vec::new();
    }
    let modal_msg = match is_down {
        true => ModalMsg::Next,
        false => ModalMsg::Previous,
    };
    vec![Msg::Modal(modal_msg)]
}

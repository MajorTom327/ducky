use crate::msg::ProfilesMsg;
use crate::state::{AppState, CellEditor, CellInput, Modal};
use crate::update::{clamp_index, parameters, profiles, step_index};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionMove {
    Step { is_forward: bool },
    To(usize),
}

impl SelectionMove {
    fn apply(self, index: usize, length: usize) -> usize {
        match self {
            SelectionMove::Step { is_forward } => step_index(index, length, is_forward),
            SelectionMove::To(target_index) => clamp_index(target_index, length),
        }
    }
}

pub(crate) fn move_selection(state: &mut AppState, selection_move: SelectionMove) {
    let schema_count = state
        .workspace()
        .and_then(|workspace| workspace.catalog.ready())
        .map_or(0, |catalog| catalog.schemas.len());
    let palette_item_count = match (&state.modal, state.workspace()) {
        (Some(Modal::Palette(command_palette)), Some(workspace)) => {
            workspace.palette_items(command_palette.input.value()).len()
        }
        _ => 0,
    };
    match &mut state.modal {
        Some(Modal::Palette(command_palette)) => {
            command_palette.selected_item_index =
                selection_move.apply(command_palette.selected_item_index, palette_item_count);
        }
        Some(Modal::SchemaPicker {
            selected_schema_index,
        }) => *selected_schema_index = selection_move.apply(*selected_schema_index, schema_count),
        Some(Modal::Parameters(form)) => {
            if let SelectionMove::Step { is_forward } = selection_move {
                parameters::step(form, is_forward);
            }
        }
        Some(Modal::ReferencePicker {
            links,
            selected_link_index,
        }) => *selected_link_index = selection_move.apply(*selected_link_index, links.len()),
        Some(Modal::RowDetail(row_detail)) => {
            row_detail.selected_field_index =
                selection_move.apply(row_detail.selected_field_index, row_detail.columns.len());
        }
        Some(Modal::CellEditor(CellEditor {
            input:
                CellInput::Choice {
                    options,
                    selected_option_index,
                },
            ..
        })) => *selected_option_index = selection_move.apply(*selected_option_index, options.len()),
        Some(Modal::ProfilePicker(picker)) => {
            let profiles_msg = match selection_move {
                SelectionMove::Step { is_forward: true } => ProfilesMsg::Next,
                SelectionMove::Step { is_forward: false } => ProfilesMsg::Previous,
                SelectionMove::To(profile_index) => ProfilesMsg::Select(profile_index),
            };
            profiles::apply(picker, &state.profiles, profiles_msg);
        }
        Some(
            Modal::Prompt(_)
            | Modal::Confirm(_)
            | Modal::Help
            | Modal::CellEditor(CellEditor {
                input: CellInput::Text(_),
                ..
            }),
        )
        | None => {}
    }
}

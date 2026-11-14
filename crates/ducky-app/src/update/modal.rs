use crate::command::Command;
use crate::msg::ModalMsg;
use crate::state::{
    AppState, CellEditor, CellInput, Confirm, ConfirmKind, JobState, Modal, NoticeLevel,
    PendingSave, Prompt, PromptKind,
};
use crate::update::modal_selection::{SelectionMove, move_selection};
use crate::update::table::apply_filter;
use crate::update::{
    cell_editor, cell_edits, notice, palette, parameters, profiles, references, run_confirmation,
    workspace_mut,
};

pub(crate) fn update(state: &mut AppState, msg: ModalMsg) -> Vec<Command> {
    match msg {
        ModalMsg::Input(request) => {
            let prompt = match &mut state.modal {
                Some(Modal::Prompt(prompt)) => prompt,
                Some(Modal::Palette(command_palette)) => {
                    command_palette.input.handle(request);
                    command_palette.selected_item_index = 0;
                    return Vec::new();
                }
                Some(Modal::Parameters(form)) => {
                    parameters::input(form, request);
                    return Vec::new();
                }
                Some(Modal::CellEditor(CellEditor {
                    input: CellInput::Text(input),
                    ..
                })) => {
                    input.handle(request);
                    return Vec::new();
                }
                _ => return Vec::new(),
            };
            prompt.input.handle(request);
            if prompt.kind == PromptKind::SidebarFilter
                && let Some(workspace) = workspace_mut(&mut state.screen)
            {
                workspace.sidebar.filter = prompt.input.value().to_owned();
                workspace.sidebar.selected_item_index = 0;
            }
            Vec::new()
        }
        ModalMsg::Submit => match state.modal.take() {
            Some(Modal::Prompt(prompt)) => submit_prompt(state, prompt),
            Some(Modal::Confirm(confirm)) => submit_confirm(state, confirm),
            Some(Modal::SchemaPicker {
                selected_schema_index,
            }) => {
                if let Some(workspace) = workspace_mut(&mut state.screen)
                    && let Some(schema_name) = workspace
                        .schema_names()
                        .into_iter()
                        .nth(selected_schema_index)
                {
                    workspace.schema_name = schema_name;
                    workspace.sidebar.selected_item_index = 0;
                    workspace.sidebar.filter.clear();
                }
                Vec::new()
            }
            Some(Modal::Palette(command_palette)) => palette::submit(state, &command_palette),
            Some(Modal::Parameters(form)) => parameters::submit(state, form),
            Some(Modal::ProfilePicker(picker)) => {
                state.modal = Some(Modal::ProfilePicker(picker));
                profiles::connect_from_modal(state)
            }
            Some(Modal::ReferencePicker {
                links,
                selected_link_index,
            }) => references::submit_picker(state, links, selected_link_index),
            Some(Modal::CellEditor(editor)) => cell_editor::submit(state, editor),
            Some(Modal::RowDetail(_) | Modal::Help) | None => Vec::new(),
        },
        ModalMsg::Cancel => {
            match state.modal.take() {
                Some(Modal::Prompt(prompt)) if prompt.kind == PromptKind::SidebarFilter => {
                    if let Some(workspace) = workspace_mut(&mut state.screen) {
                        workspace.sidebar.filter.clear();
                        workspace.sidebar.selected_item_index = 0;
                    }
                }
                Some(Modal::Confirm(Confirm {
                    kind: ConfirmKind::OverwriteFile,
                    ..
                })) => {
                    if let Some(workspace) = workspace_mut(&mut state.screen) {
                        workspace.pending_save = None;
                    }
                }
                Some(Modal::Confirm(Confirm {
                    kind: ConfirmKind::RunScript { .. },
                    ..
                })) => run_confirmation::cancelled(state),
                Some(Modal::Confirm(Confirm {
                    kind: ConfirmKind::SaveCellEdits { .. },
                    ..
                })) => cell_edits::cancelled(state),
                _ => {}
            }
            Vec::new()
        }
        ModalMsg::Next | ModalMsg::Previous => {
            let is_forward = msg == ModalMsg::Next;
            move_selection(state, SelectionMove::Step { is_forward });
            Vec::new()
        }
        ModalMsg::Select(item_index) => {
            move_selection(state, SelectionMove::To(item_index));
            Vec::new()
        }
        ModalMsg::ToggleRaw => {
            if let Some(Modal::Parameters(form)) = &mut state.modal {
                parameters::toggle_raw(form);
            }
            Vec::new()
        }
        ModalMsg::CopyValue => {
            let Some(Modal::RowDetail(row_detail)) = &state.modal else {
                return Vec::new();
            };
            let Some(cell) = row_detail.values.get(row_detail.selected_field_index) else {
                return Vec::new();
            };
            let value = cell.clone().unwrap_or_default();
            state.notice = notice(NoticeLevel::Info, "Copied");
            vec![Command::CopyToClipboard(value)]
        }
    }
}

fn submit_prompt(state: &mut AppState, prompt: Prompt) -> Vec<Command> {
    let trimmed_value = prompt.input.value().trim().to_owned();
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let optional_value = Some(trimmed_value.clone()).filter(|value| !value.is_empty());
    match prompt.kind {
        PromptKind::SidebarFilter => Vec::new(),
        PromptKind::TableFilter => match workspace.active_table_mut() {
            Some(table) => apply_filter(table, optional_value),
            None => Vec::new(),
        },
        PromptKind::SaveFileAs { content } => {
            if trimmed_value.is_empty() {
                state.notice = notice(NoticeLevel::Error, "File name is required");
                state.modal = Some(Modal::Prompt(Prompt {
                    kind: PromptKind::SaveFileAs { content },
                    ..prompt
                }));
                return Vec::new();
            }
            let Some(tab_id) = workspace.active_tab_id() else {
                return Vec::new();
            };
            workspace.pending_save = Some(PendingSave {
                tab_id,
                name: trimmed_value.clone(),
                content: content.clone(),
            });
            vec![Command::SaveFile {
                name: trimmed_value,
                content,
                overwrite: false,
            }]
        }
        PromptKind::DumpFileName => {
            workspace.dump.custom_file_name = optional_value;
            Vec::new()
        }
    }
}

fn submit_confirm(state: &mut AppState, confirm: Confirm) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    match confirm.kind {
        ConfirmKind::OverwriteFile => match &workspace.pending_save {
            Some(pending_save) => vec![Command::SaveFile {
                name: pending_save.name.clone(),
                content: pending_save.content.clone(),
                overwrite: true,
            }],
            None => Vec::new(),
        },
        ConfirmKind::RunScript { tab_id, sql } => run_confirmation::confirmed(state, tab_id, sql),
        ConfirmKind::SaveCellEdits { tab_id, sql } => cell_edits::run(state, tab_id, sql),
        ConfirmKind::Restore { file_name } => {
            let restore = &mut workspace.restore;
            restore.job = JobState::Running;
            restore.log_lines.clear();
            vec![Command::StartRestore {
                file_name,
                options: restore.options,
            }]
        }
    }
}

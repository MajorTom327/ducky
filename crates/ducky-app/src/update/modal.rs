use crate::command::Command;
use crate::msg::ModalMsg;
use crate::state::{
    AppState, Confirm, ConfirmKind, JobState, Modal, NoticeLevel, PendingSave, Prompt, PromptKind,
};
use crate::update::table::apply_filter;
use crate::update::{notice, step_index, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: ModalMsg) -> Vec<Command> {
    match msg {
        ModalMsg::Input(request) => {
            let Some(Modal::Prompt(prompt)) = &mut state.modal else {
                return Vec::new();
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
                        workspace.query.pending_save = None;
                    }
                }
                _ => {}
            }
            Vec::new()
        }
        ModalMsg::Next | ModalMsg::Previous => {
            let is_forward = msg == ModalMsg::Next;
            let schema_count = state
                .workspace()
                .and_then(|workspace| workspace.catalog.ready())
                .map_or(0, |catalog| catalog.schemas.len());
            match &mut state.modal {
                Some(Modal::SchemaPicker {
                    selected_schema_index,
                }) => {
                    *selected_schema_index =
                        step_index(*selected_schema_index, schema_count, is_forward);
                }
                Some(Modal::RowDetail(row_detail)) => {
                    row_detail.selected_field_index = step_index(
                        row_detail.selected_field_index,
                        row_detail.columns.len(),
                        is_forward,
                    );
                }
                _ => {}
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
        PromptKind::TableFilter => match &mut workspace.table {
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
            workspace.query.pending_save = Some(PendingSave {
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
        ConfirmKind::OverwriteFile => match &workspace.query.pending_save {
            Some(pending_save) => vec![Command::SaveFile {
                name: pending_save.name.clone(),
                content: pending_save.content.clone(),
                overwrite: true,
            }],
            None => Vec::new(),
        },
        ConfirmKind::ExecuteFile { name } => vec![Command::OpenFile {
            name,
            run_after_open: true,
        }],
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

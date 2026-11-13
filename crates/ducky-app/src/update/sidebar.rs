use crate::command::Command;
use crate::msg::SidebarMsg;
use crate::state::{AppState, Modal, Prompt, PromptKind, Workspace};
use crate::update::tabs::open_relation;
use crate::update::{clamp_index, step_index, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: SidebarMsg) -> Vec<Command> {
    let page_size = state.page_size;
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let item_count = workspace.sidebar_items().len();
    let sidebar = &mut workspace.sidebar;
    match msg {
        SidebarMsg::Next | SidebarMsg::Previous => {
            sidebar.selected_item_index = step_index(
                sidebar.selected_item_index,
                item_count,
                msg == SidebarMsg::Next,
            );
            Vec::new()
        }
        SidebarMsg::First => {
            sidebar.selected_item_index = 0;
            Vec::new()
        }
        SidebarMsg::Last => {
            sidebar.selected_item_index = clamp_index(usize::MAX, item_count);
            Vec::new()
        }
        SidebarMsg::StartFilter => {
            state.modal = Some(Modal::Prompt(Prompt::new(
                PromptKind::SidebarFilter,
                "Filter tables",
                sidebar.filter.clone(),
            )));
            Vec::new()
        }
        SidebarMsg::ClearFilter => {
            sidebar.filter.clear();
            sidebar.selected_item_index = 0;
            Vec::new()
        }
        SidebarMsg::Activate => activate(workspace, page_size),
    }
}

fn activate(workspace: &mut Workspace, page_size: usize) -> Vec<Command> {
    let Some(item) = workspace
        .sidebar_items()
        .into_iter()
        .nth(workspace.sidebar.selected_item_index)
    else {
        return Vec::new();
    };
    open_relation(workspace, item.relation, page_size)
}

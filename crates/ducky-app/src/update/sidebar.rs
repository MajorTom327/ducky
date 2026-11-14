use crate::command::Command;
use crate::msg::SidebarMsg;
use crate::state::{
    AppState, Focus, MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH, Modal, Prompt, PromptKind,
    SIDEBAR_WIDTH_STEP, Sidebar, Workspace,
};
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
        }
        SidebarMsg::First => sidebar.selected_item_index = 0,
        SidebarMsg::Last => sidebar.selected_item_index = clamp_index(usize::MAX, item_count),
        SidebarMsg::Select(item_index) => {
            sidebar.selected_item_index = clamp_index(item_index, item_count);
            workspace.set_focus(Focus::Sidebar);
        }
        SidebarMsg::StartFilter => {
            state.modal = Some(Modal::Prompt(Prompt::new(
                PromptKind::SidebarFilter,
                "Filter tables",
                sidebar.filter.clone(),
            )));
        }
        SidebarMsg::ClearFilter => {
            sidebar.filter.clear();
            sidebar.selected_item_index = 0;
        }
        SidebarMsg::ToggleCollapsed => match sidebar.is_collapsed {
            true => workspace.set_focus(Focus::Sidebar),
            false => {
                sidebar.is_collapsed = true;
                workspace.focus = Focus::Main;
            }
        },
        SidebarMsg::Widen | SidebarMsg::Narrow => resize(sidebar, msg == SidebarMsg::Widen),
        SidebarMsg::SetWidth(width) => {
            sidebar.width = width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
            sidebar.is_collapsed = false;
        }
        SidebarMsg::Activate => return activate(workspace, page_size),
    }
    Vec::new()
}

fn resize(sidebar: &mut Sidebar, is_widening: bool) {
    if sidebar.is_collapsed {
        sidebar.is_collapsed = false;
        return;
    }
    let requested_width = match is_widening {
        true => sidebar.width.saturating_add(SIDEBAR_WIDTH_STEP),
        false => sidebar.width.saturating_sub(SIDEBAR_WIDTH_STEP),
    };
    sidebar.width = requested_width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
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

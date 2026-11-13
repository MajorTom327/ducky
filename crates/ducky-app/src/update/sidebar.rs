use ducky_core::PageRequest;

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::SidebarMsg;
use crate::state::{
    AppState, Focus, GridCursor, MainView, Modal, PAGE_SIZE, Prompt, PromptKind, QueryPane,
    SidebarAction, SidebarItem, TableTab, TableView, Workspace,
};
use crate::update::{clamp_index, step_index, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: SidebarMsg) -> Vec<Command> {
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
        SidebarMsg::Activate => activate(workspace),
    }
}

fn activate(workspace: &mut Workspace) -> Vec<Command> {
    let Some(item) = workspace
        .sidebar_items()
        .into_iter()
        .nth(workspace.sidebar.selected_item_index)
    else {
        return Vec::new();
    };
    workspace.focus = Focus::Main;
    match item {
        SidebarItem::Action(SidebarAction::SqlCommand) => {
            workspace.view = MainView::Query;
            workspace.query.pane = QueryPane::Editor;
            Vec::new()
        }
        SidebarItem::Action(SidebarAction::SqlFiles) => {
            workspace.view = MainView::Files;
            workspace.files.entries = Loadable::Loading;
            vec![Command::ListSqlFiles]
        }
        SidebarItem::Action(SidebarAction::Dump) => {
            workspace.view = MainView::Dump;
            Vec::new()
        }
        SidebarItem::Action(SidebarAction::Restore) => {
            workspace.view = MainView::Restore;
            workspace.restore.entries = Loadable::Loading;
            vec![Command::ListDumps]
        }
        SidebarItem::Relation { relation, .. } => {
            let request = PageRequest {
                relation: relation.clone(),
                page_index: 0,
                page_size: PAGE_SIZE,
                sort: None,
                filter: None,
            };
            workspace.table = Some(TableView {
                relation,
                tab: TableTab::Data,
                request: request.clone(),
                page: Loadable::Loading,
                cursor: GridCursor::default(),
                details: Loadable::Idle,
            });
            workspace.view = MainView::Table;
            vec![Command::LoadPage(request)]
        }
    }
}

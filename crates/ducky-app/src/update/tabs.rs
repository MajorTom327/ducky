use ducky_core::RelationRef;

use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::TabsMsg;
use crate::state::{
    AppState, Document, Focus, NoticeLevel, QueryPane, TabContent, TabId, TableView, Workspace,
};
use crate::update::cell_edits::unsaved_drafts_refusal;
use crate::update::{notice, workspace_mut};

pub(crate) fn update(state: &mut AppState, msg: TabsMsg) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let tab_count = workspace.tabs.len();
    match msg {
        TabsMsg::Next | TabsMsg::Previous => {
            if tab_count > 0 {
                let step = if msg == TabsMsg::Next {
                    1
                } else {
                    tab_count - 1
                };
                workspace.active_tab_index = (workspace.active_tab_index + step) % tab_count;
            }
            Vec::new()
        }
        TabsMsg::Select(tab_index) => {
            if tab_index < tab_count {
                workspace.active_tab_index = tab_index;
            }
            Vec::new()
        }
        TabsMsg::Close => {
            let Some(tab_id) = workspace.active_tab_id() else {
                return Vec::new();
            };
            if workspace.running_script_tab_id == Some(tab_id) {
                state.notice = notice(
                    NoticeLevel::Error,
                    "Cancel the running script before closing this tab",
                );
                return Vec::new();
            }
            if let Some(refusal) = workspace
                .active_table_mut()
                .and_then(|table| unsaved_drafts_refusal(table))
            {
                state.notice = Some(refusal);
                return Vec::new();
            }
            close_active_tab(workspace);
            Vec::new()
        }
        TabsMsg::NewQuery => {
            workspace.push_query(Document::default());
            workspace.focus = Focus::Main;
            Vec::new()
        }
        TabsMsg::OpenFiles => {
            activate_or_push(workspace, TabContent::Files);
            if workspace.files.entries.ready().is_some() {
                return Vec::new();
            }
            workspace.files.entries = Loadable::Loading;
            vec![Command::ListSqlFiles]
        }
        TabsMsg::OpenDump => {
            activate_or_push(workspace, TabContent::Dump);
            Vec::new()
        }
        TabsMsg::OpenRestore => {
            activate_or_push(workspace, TabContent::Restore);
            workspace.restore.entries = Loadable::Loading;
            vec![Command::ListDumps]
        }
    }
}

fn close_active_tab(workspace: &mut Workspace) {
    let tab_index = workspace.active_tab_index;
    let tab_id = workspace.tabs.remove(tab_index).id;
    if workspace
        .pending_save
        .as_ref()
        .is_some_and(|pending_save| pending_save.tab_id == tab_id)
    {
        workspace.pending_save = None;
    }
    workspace.active_tab_index = tab_index.min(workspace.tabs.len().saturating_sub(1));
    if workspace.tabs.is_empty() {
        workspace.focus = Focus::Sidebar;
    }
}

fn activate_or_push(workspace: &mut Workspace, content: TabContent) {
    let existing_tab_index = workspace
        .tabs
        .iter()
        .position(|tab| std::mem::discriminant(&tab.content) == std::mem::discriminant(&content));
    match existing_tab_index {
        Some(tab_index) => workspace.active_tab_index = tab_index,
        None => {
            workspace.push_tab(content);
        }
    }
    workspace.focus = Focus::Main;
}

pub(crate) fn reveal_schema(workspace: &mut Workspace, schema_name: &str) {
    if workspace.schema_name != schema_name {
        workspace.schema_name = schema_name.to_owned();
        workspace.sidebar.selected_item_index = 0;
        workspace.sidebar.filter.clear();
    }
}

pub(crate) fn open_relation(
    workspace: &mut Workspace,
    relation: RelationRef,
    page_size: usize,
) -> Vec<Command> {
    workspace.focus = Focus::Main;
    let existing_tab_index = workspace.tabs.iter().position(
        |tab| matches!(&tab.content, TabContent::Table(table) if table.relation == relation),
    );
    if let Some(tab_index) = existing_tab_index {
        workspace.active_tab_index = tab_index;
        return Vec::new();
    }
    let table = TableView::first_page(relation, page_size);
    let request = table.request.clone();
    workspace.push_tab(TabContent::Table(table));
    vec![Command::LoadPage(request)]
}

pub(crate) fn open_document(workspace: &mut Workspace, name: String, text: String) -> TabId {
    let existing_tab_id = workspace.tabs.iter().find_map(|tab| match &tab.content {
        TabContent::Query(query) if query.document.name.as_deref() == Some(name.as_str()) => {
            Some(tab.id)
        }
        _ => None,
    });
    let tab_id = match existing_tab_id {
        Some(tab_id) => {
            workspace.activate_tab(tab_id);
            tab_id
        }
        None => workspace.push_query(Document {
            name: Some(name),
            text: String::new(),
            revision: 0,
        }),
    };
    if let Some(query) = workspace.query_mut(tab_id) {
        query.document.text = text;
        query.document.revision += 1;
        query.pane = QueryPane::Editor;
    }
    workspace.focus = Focus::Main;
    tab_id
}

mod support;

use ducky_app::{
    Command, Focus, Loadable, MainView, Modal, Msg, PAGE_SIZE, PromptKind, QueryPane,
    SidebarAction, SidebarItem, SidebarMsg, TableTab,
};
use ducky_core::{PageRequest, RelationRef};
use support::{
    activate_relation, select_sidebar_item, send, workspace, workspace_mut, workspace_state,
};

fn relation_names(state: &ducky_app::AppState) -> Vec<String> {
    workspace(state)
        .sidebar_items()
        .into_iter()
        .filter_map(|item| match item {
            SidebarItem::Relation { relation, .. } => Some(relation.name),
            SidebarItem::Action(_) => None,
        })
        .collect()
}

fn activate_action(state: &mut ducky_app::AppState, action: SidebarAction) -> Vec<Command> {
    select_sidebar_item(state, |item| *item == SidebarItem::Action(action));
    send(state, Msg::Sidebar(SidebarMsg::Activate))
}

#[test]
fn items_list_actions_then_sorted_relations_of_current_schema() {
    let state = workspace_state();
    let items = workspace(&state).sidebar_items();
    assert_eq!(
        items[..4],
        [
            SidebarItem::Action(SidebarAction::SqlCommand),
            SidebarItem::Action(SidebarAction::SqlFiles),
            SidebarItem::Action(SidebarAction::Dump),
            SidebarItem::Action(SidebarAction::Restore),
        ]
    );
    assert_eq!(
        relation_names(&state),
        ["customers", "order_totals", "orders"]
    );
}

#[test]
fn filter_narrows_items_case_insensitively() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.filter = "ORDER".to_owned();
    assert_eq!(relation_names(&state), ["order_totals", "orders"]);
}

#[test]
fn navigation_clamps_inside_items() {
    let mut state = workspace_state();
    send(&mut state, Msg::Sidebar(SidebarMsg::Previous));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
    send(&mut state, Msg::Sidebar(SidebarMsg::Last));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 6);
    send(&mut state, Msg::Sidebar(SidebarMsg::Next));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 6);
    send(&mut state, Msg::Sidebar(SidebarMsg::First));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
}

#[test]
fn start_filter_opens_prefilled_prompt() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.filter = "ord".to_owned();
    send(&mut state, Msg::Sidebar(SidebarMsg::StartFilter));
    let Some(Modal::Prompt(prompt)) = &state.modal else {
        panic!("expected prompt");
    };
    assert_eq!(prompt.kind, PromptKind::SidebarFilter);
    assert_eq!(prompt.title, "Filter tables");
    assert_eq!(prompt.input.value(), "ord");
}

#[test]
fn clear_filter_resets_filter_and_index() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.filter = "ord".to_owned();
    workspace_mut(&mut state).sidebar.selected_item_index = 5;
    send(&mut state, Msg::Sidebar(SidebarMsg::ClearFilter));
    assert_eq!(workspace(&state).sidebar.filter, "");
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
}

#[test]
fn activate_sql_command_opens_editor() {
    let mut state = workspace_state();
    assert!(activate_action(&mut state, SidebarAction::SqlCommand).is_empty());
    let workspace = workspace(&state);
    assert_eq!(
        (workspace.view, workspace.focus, workspace.query.pane),
        (MainView::Query, Focus::Main, QueryPane::Editor)
    );
}

#[test]
fn activate_sql_files_lists_files() {
    let mut state = workspace_state();
    assert_eq!(
        activate_action(&mut state, SidebarAction::SqlFiles),
        vec![Command::ListSqlFiles]
    );
    assert_eq!(workspace(&state).view, MainView::Files);
    assert!(workspace(&state).files.entries.is_loading());
}

#[test]
fn activate_dump_shows_dump_view() {
    let mut state = workspace_state();
    assert!(activate_action(&mut state, SidebarAction::Dump).is_empty());
    assert_eq!(workspace(&state).view, MainView::Dump);
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn activate_restore_lists_dumps() {
    let mut state = workspace_state();
    assert_eq!(
        activate_action(&mut state, SidebarAction::Restore),
        vec![Command::ListDumps]
    );
    assert_eq!(workspace(&state).view, MainView::Restore);
    assert!(workspace(&state).restore.entries.is_loading());
}

#[test]
fn activate_relation_loads_first_page() {
    let mut state = workspace_state();
    let commands = activate_relation(&mut state, "customers");
    let expected_request = PageRequest {
        relation: RelationRef {
            schema: "public".to_owned(),
            name: "customers".to_owned(),
        },
        page_index: 0,
        page_size: PAGE_SIZE,
        sort: None,
        filter: None,
    };
    assert_eq!(commands, vec![Command::LoadPage(expected_request.clone())]);
    let workspace = workspace(&state);
    let table = workspace.table.as_ref().expect("table");
    assert_eq!(table.request, expected_request);
    assert_eq!(table.tab, TableTab::Data);
    assert!(table.page.is_loading());
    assert_eq!(table.details, Loadable::Idle);
    assert_eq!(
        (workspace.view, workspace.focus),
        (MainView::Table, Focus::Main)
    );
}

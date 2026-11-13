mod support;

use ducky_app::{
    Command, Focus, Loadable, Modal, Msg, PromptKind, SidebarItem, SidebarMsg, TableTab, TabsMsg,
};
use ducky_core::{DEFAULT_PAGE_SIZE, PageRequest, RelationKind, RelationRef};
use support::{activate_relation, active_table, send, workspace, workspace_mut, workspace_state};

fn relation_names(state: &ducky_app::AppState) -> Vec<String> {
    workspace(state)
        .sidebar_items()
        .into_iter()
        .map(|item| item.relation.name)
        .collect()
}

#[test]
fn items_list_sorted_relations_of_current_schema() {
    let state = workspace_state();
    assert_eq!(
        relation_names(&state),
        ["customers", "order_totals", "orders"]
    );
    assert_eq!(
        workspace(&state).sidebar_items()[1],
        SidebarItem {
            relation: RelationRef {
                schema: "public".to_owned(),
                name: "order_totals".to_owned(),
            },
            kind: RelationKind::View,
            estimated_row_count: Some(10),
        }
    );
}

#[test]
fn items_are_empty_without_ready_catalog() {
    let state = support::connected_state(support::tools_available());
    assert!(workspace(&state).sidebar_items().is_empty());
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
    assert_eq!(workspace(&state).sidebar.selected_item_index, 2);
    send(&mut state, Msg::Sidebar(SidebarMsg::Next));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 2);
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
    workspace_mut(&mut state).sidebar.selected_item_index = 2;
    send(&mut state, Msg::Sidebar(SidebarMsg::ClearFilter));
    assert_eq!(workspace(&state).sidebar.filter, "");
    assert_eq!(workspace(&state).sidebar.selected_item_index, 0);
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
        page_size: DEFAULT_PAGE_SIZE,
        sort: None,
        filter: None,
    };
    assert_eq!(commands, vec![Command::LoadPage(expected_request.clone())]);
    let table = active_table(&state);
    assert_eq!(table.request, expected_request);
    assert_eq!(table.tab, TableTab::Data);
    assert!(table.page.is_loading());
    assert_eq!(table.details, Loadable::Idle);
    let workspace = workspace(&state);
    assert_eq!(workspace.tabs.len(), 1);
    assert_eq!(workspace.tabs[0].id, 1);
    assert_eq!(workspace.tabs[0].title(), "customers");
    assert_eq!(workspace.next_tab_id, 2);
    assert_eq!(workspace.focus, Focus::Main);
}

#[test]
fn activating_an_open_relation_reuses_its_tab() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    activate_relation(&mut state, "orders");
    workspace_mut(&mut state).focus = Focus::Sidebar;
    assert!(activate_relation(&mut state, "customers").is_empty());
    let workspace = workspace(&state);
    assert_eq!(workspace.tabs.len(), 2);
    assert_eq!(workspace.active_tab_index, 0);
    assert_eq!(workspace.focus, Focus::Main);
}

#[test]
fn activating_a_relation_pushes_after_other_tabs() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    activate_relation(&mut state, "orders");
    let workspace = workspace(&state);
    assert_eq!(workspace.tabs.len(), 2);
    assert_eq!(workspace.active_tab_index, 1);
    assert_eq!(active_table(&state).relation.name, "orders");
}

#[test]
fn activating_a_relation_uses_the_configured_page_size() {
    let mut state = workspace_state().with_page_size(25);
    let commands = activate_relation(&mut state, "customers");
    match commands.as_slice() {
        [Command::LoadPage(request)] => assert_eq!(request.page_size, 25),
        other => panic!("expected one LoadPage, got {other:?}"),
    }
}

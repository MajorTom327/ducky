mod support;

use ducky_app::{
    AppState, Command, Focus, Modal, ModalMsg, Msg, PaletteAction, PaletteItem, TabContent, TabsMsg,
};
use ducky_core::{Catalog, RelationKind, RelationRef};
use support::{
    activate_relation, active_tab_id, active_table, connected_state, new_query, relation, schema,
    send, tools_missing, workspace, workspace_mut, workspace_state,
};
use tui_input::InputRequest;

fn type_query(state: &mut AppState, text: &str) {
    for typed_char in text.chars() {
        send(
            state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(typed_char))),
        );
    }
}

fn labels(items: &[PaletteItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

fn selected_palette_index(state: &AppState) -> usize {
    match &state.modal {
        Some(Modal::Palette(command_palette)) => command_palette.selected_item_index,
        _ => panic!("expected palette"),
    }
}

#[test]
fn open_palette_requires_a_workspace() {
    let mut state = AppState::new(support::profiles(), support::storage());
    send(&mut state, Msg::OpenPalette);
    assert!(state.modal.is_none());
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    let Some(Modal::Palette(command_palette)) = &state.modal else {
        panic!("expected palette");
    };
    assert_eq!(command_palette.input.value(), "");
    assert_eq!(command_palette.selected_item_index, 0);
}

#[test]
fn empty_query_lists_tabs_then_actions_then_relations() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    let items = workspace(&state).palette_items("");
    assert_eq!(
        labels(&items),
        [
            "customers",
            "New SQL query",
            "SQL files",
            "Dump database",
            "Restore dump",
            "Switch schema",
            "Refresh catalog",
            "Reconnect",
            "Disconnect",
            "Help",
            "customers",
            "order_totals",
            "orders",
            "analytics.events",
        ]
    );
    assert_eq!(items[0].detail.as_deref(), Some("open tab"));
    assert_eq!(items[0].action, PaletteAction::SwitchTab(1));
    assert_eq!(items[3].detail, None);
    assert_eq!(items[10].detail.as_deref(), Some("table · ~10 rows"));
    assert_eq!(items[11].detail.as_deref(), Some("view"));
    assert_eq!(
        items[13].action,
        PaletteAction::OpenRelation(RelationRef {
            schema: "analytics".to_owned(),
            name: "events".to_owned(),
        })
    );
}

#[test]
fn missing_tools_mark_actions_unavailable() {
    let mut state = connected_state(tools_missing());
    send(&mut state, Msg::CatalogLoaded(Ok(support::catalog())));
    let items = workspace(&state).palette_items("");
    let unavailable_labels: Vec<&str> = items
        .iter()
        .filter(|item| item.detail.as_deref() == Some("unavailable"))
        .map(|item| item.label.as_str())
        .collect();
    assert_eq!(unavailable_labels, ["Dump database", "Restore dump"]);
}

#[test]
fn relation_details_cover_every_kind() {
    let mut state = workspace_state();
    let mut unknown_count = relation("big", RelationKind::Table);
    unknown_count.estimated_row_count = None;
    let catalog = Catalog {
        schemas: vec![schema(
            "public",
            vec![
                unknown_count,
                relation("mv", RelationKind::MaterializedView),
                relation("remote", RelationKind::ForeignTable),
            ],
        )],
    };
    send(&mut state, Msg::CatalogLoaded(Ok(catalog)));
    let details: Vec<Option<String>> = workspace(&state)
        .palette_items("")
        .into_iter()
        .skip(9)
        .map(|item| item.detail)
        .collect();
    assert_eq!(
        details,
        [
            Some("table".to_owned()),
            Some("materialized view".to_owned()),
            Some("foreign table".to_owned()),
        ]
    );
}

#[test]
fn ranking_puts_prefix_before_substring_before_subsequence() {
    let mut state = workspace_state();
    let catalog = Catalog {
        schemas: vec![
            schema(
                "public",
                vec![
                    relation("order_lines", RelationKind::Table),
                    relation("back_orders", RelationKind::Table),
                    relation("ok_round_data", RelationKind::Table),
                ],
            ),
            schema(
                "analytics",
                vec![relation("orders_archive", RelationKind::Table)],
            ),
        ],
    };
    send(&mut state, Msg::CatalogLoaded(Ok(catalog)));
    assert_eq!(
        labels(&workspace(&state).palette_items("ORD")),
        [
            "order_lines",
            "analytics.orders_archive",
            "back_orders",
            "Restore dump",
            "ok_round_data",
        ]
    );
}

#[test]
fn input_resets_selection_and_navigation_clamps() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    send(&mut state, Msg::Modal(ModalMsg::Previous));
    assert_eq!(selected_palette_index(&state), 0);
    for _ in 0..20 {
        send(&mut state, Msg::Modal(ModalMsg::Next));
    }
    assert_eq!(selected_palette_index(&state), 12);
    type_query(&mut state, "o");
    assert_eq!(selected_palette_index(&state), 0);
}

#[test]
fn cross_schema_relation_submit_switches_schema_and_opens_tab() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.filter = "x".to_owned();
    workspace_mut(&mut state).sidebar.selected_item_index = 1;
    workspace_mut(&mut state).focus = Focus::Sidebar;
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "events");
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(state.modal.is_none());
    assert_eq!(
        commands,
        vec![Command::LoadPage(active_table(&state).request.clone())]
    );
    assert_eq!(active_table(&state).relation.name, "events");
    let workspace = workspace(&state);
    assert_eq!(workspace.schema_name, "analytics");
    assert_eq!(workspace.sidebar.filter, "");
    assert_eq!(workspace.sidebar.selected_item_index, 0);
    assert_eq!(workspace.focus, Focus::Main);
}

#[test]
fn relation_submit_reuses_an_open_tab() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    new_query(&mut state);
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "customers");
    send(&mut state, Msg::Modal(ModalMsg::Next));
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert_eq!(workspace(&state).tabs.len(), 2);
    assert_eq!(active_tab_id(&state), 1);
}

#[test]
fn action_submit_opens_files() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "sql files");
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(commands, vec![Command::ListSqlFiles]);
    assert!(state.modal.is_none());
    assert!(matches!(
        workspace(&state).active_tab().map(|tab| &tab.content),
        Some(TabContent::Files)
    ));
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn switch_tab_submit_activates_tab_and_focuses_main() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    send(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    workspace_mut(&mut state).focus = Focus::Sidebar;
    send(&mut state, Msg::OpenPalette);
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(workspace(&state).active_tab_index, 0);
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn modal_actions_replace_the_palette() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "switch schema");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(matches!(state.modal, Some(Modal::SchemaPicker { .. })));
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "help");
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(matches!(state.modal, Some(Modal::Help)));
}

#[test]
fn submit_without_items_only_closes() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    type_query(&mut state, "zzzz");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
    assert!(workspace(&state).tabs.is_empty());
}

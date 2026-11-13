mod support;

use ducky_app::{
    AppState, Command, Focus, Loadable, Msg, NoticeLevel, ProfilesMsg, Screen, TabsMsg,
};
use ducky_core::RelationKind;
use support::{
    activate_relation, active_tab_id, active_table, catalog, connection, open_loaded_table,
    profiles, relation, schema, send, storage, tools_available, tools_missing, workspace,
    workspace_mut, workspace_state,
};

#[test]
fn connect_success_opens_workspace_and_loads_catalog() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(connection(tools_available())),
        },
    );
    assert_eq!(commands, vec![Command::LoadCatalog]);
    let workspace = workspace(&state);
    assert_eq!(workspace.profile.label, "Local");
    assert!(workspace.catalog.is_loading());
    assert!(workspace.tabs.is_empty());
    assert_eq!(workspace.active_tab_index, 0);
    assert_eq!(workspace.next_tab_id, 1);
    assert_eq!(workspace.next_query_number, 1);
    assert_eq!(workspace.focus, Focus::Sidebar);
    assert_eq!(workspace.pending_save, None);
    assert_eq!(workspace.running_script_tab_id, None);
}

#[test]
fn connect_failure_stays_on_profiles_with_error() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Err("refused".to_owned()),
        },
    );
    assert!(commands.is_empty());
    let Screen::Profiles(picker) = &state.screen else {
        panic!("expected profiles");
    };
    assert_eq!(picker.connecting_profile_index, None);
    assert_eq!(picker.connection_error.as_deref(), Some("refused"));
}

#[test]
fn connected_without_pending_connect_is_ignored() {
    let mut state = AppState::new(profiles(), storage());
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(connection(tools_available())),
        },
    );
    assert!(commands.is_empty());
    assert!(state.workspace().is_none());
}

#[test]
fn reconnect_emits_connect_with_notice() {
    let mut state = workspace_state();
    let commands = send(&mut state, Msg::Reconnect);
    assert_eq!(
        commands,
        vec![Command::Connect {
            profile_index: 0,
            profile: profiles()[0].clone(),
        }]
    );
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (NoticeLevel::Info, "Reconnecting…")
    );
}

#[test]
fn reconnect_reloads_every_table_tab() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let customers_request = active_table(&state).request.clone();
    activate_relation(&mut state, "orders");
    let orders_request = active_table(&state).request.clone();
    support::new_query(&mut state);
    let commands = send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(connection(tools_missing())),
        },
    );
    assert_eq!(
        commands,
        vec![
            Command::LoadCatalog,
            Command::LoadPage(customers_request),
            Command::LoadPage(orders_request)
        ]
    );
    assert_eq!(workspace(&state).tools, tools_missing());
    send(&mut state, Msg::Tabs(TabsMsg::Select(0)));
    assert!(active_table(&state).page.is_loading());
    assert_eq!(
        state.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("Reconnected")
    );
}

#[test]
fn reconnect_failure_sets_error_notice() {
    let mut state = workspace_state();
    send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Err("down".to_owned()),
        },
    );
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (NoticeLevel::Error, "down")
    );
}

#[test]
fn disconnect_returns_to_profiles() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenHelp);
    assert_eq!(send(&mut state, Msg::Disconnect), vec![Command::Disconnect]);
    assert!(state.modal.is_none());
    let Screen::Profiles(picker) = &state.screen else {
        panic!("expected profiles");
    };
    assert_eq!(picker.selected_profile_index, 0);
}

#[test]
fn catalog_schema_fallback_order() {
    let mut state = workspace_state();
    assert_eq!(workspace(&state).schema_name, "public");
    workspace_mut(&mut state).schema_name = "analytics".to_owned();
    send(&mut state, Msg::CatalogLoaded(Ok(catalog())));
    assert_eq!(workspace(&state).schema_name, "analytics");
    let without_public = ducky_core::Catalog {
        schemas: vec![schema("zeta", Vec::new()), schema("alpha", Vec::new())],
    };
    workspace_mut(&mut state).schema_name = "gone".to_owned();
    send(&mut state, Msg::CatalogLoaded(Ok(without_public)));
    assert_eq!(workspace(&state).schema_name, "zeta");
    send(
        &mut state,
        Msg::CatalogLoaded(Ok(ducky_core::Catalog {
            schemas: Vec::new(),
        })),
    );
    assert_eq!(workspace(&state).schema_name, "");
}

#[test]
fn catalog_failure_is_reported() {
    let mut state = support::connected_state(tools_available());
    send(&mut state, Msg::CatalogLoaded(Err("nope".to_owned())));
    assert_eq!(
        workspace(&state).catalog,
        Loadable::Failed("nope".to_owned())
    );
}

#[test]
fn catalog_reload_clamps_sidebar_and_closes_vanished_table_tab() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    activate_relation(&mut state, "orders");
    let orders_tab_id = active_tab_id(&state);
    send(&mut state, Msg::Tabs(TabsMsg::Select(0)));
    workspace_mut(&mut state).sidebar.selected_item_index = 2;
    let shrunk_catalog = ducky_core::Catalog {
        schemas: vec![schema(
            "public",
            vec![relation("orders", RelationKind::Table)],
        )],
    };
    send(&mut state, Msg::CatalogLoaded(Ok(shrunk_catalog)));
    let workspace = workspace(&state);
    assert_eq!(workspace.sidebar.selected_item_index, 0);
    assert_eq!(workspace.tabs.len(), 1);
    assert_eq!(workspace.tabs[0].id, orders_tab_id);
    assert_eq!(workspace.active_tab_index, 0);
    assert_eq!(workspace.focus, Focus::Main);
}

#[test]
fn catalog_reload_keeps_active_tab_when_an_earlier_tab_vanishes() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    let query_tab_id = support::new_query(&mut state);
    let shrunk_catalog = ducky_core::Catalog {
        schemas: vec![schema("public", Vec::new())],
    };
    send(&mut state, Msg::CatalogLoaded(Ok(shrunk_catalog)));
    assert_eq!(workspace(&state).tabs.len(), 1);
    assert_eq!(active_tab_id(&state), query_tab_id);
}

#[test]
fn catalog_reload_returns_focus_to_sidebar_when_last_tab_vanishes() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(
        &mut state,
        Msg::CatalogLoaded(Ok(ducky_core::Catalog {
            schemas: Vec::new(),
        })),
    );
    assert!(workspace(&state).tabs.is_empty());
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

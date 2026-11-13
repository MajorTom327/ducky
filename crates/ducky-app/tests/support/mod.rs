#![allow(dead_code)]

use std::path::PathBuf;
use std::time::Duration;

use ducky_app::{
    AppState, Command, Connection, Msg, ProfilesMsg, QueryView, Screen, SidebarItem, SidebarMsg,
    TabContent, TabId, TableView, TabsMsg, Workspace, update,
};
use ducky_core::{
    Catalog, Column, FileEntry, Page, PgTools, Profile, ProfileKind, Relation, RelationKind,
    ResultSet, Schema, ScriptReport, SessionInfo, StatementOutcome, StatementResult, StoragePaths,
    ToolStatus, TransactionMode,
};

pub fn profiles() -> Vec<Profile> {
    vec![
        Profile {
            label: "Local".to_owned(),
            url: "postgres://localhost/a".to_owned(),
            kind: ProfileKind::Local,
        },
        Profile {
            label: "Prod".to_owned(),
            url: "postgres://localhost/b".to_owned(),
            kind: ProfileKind::Prod,
        },
    ]
}

pub fn storage() -> StoragePaths {
    StoragePaths {
        sql_dir: PathBuf::from("sql"),
        dump_dir: PathBuf::from("dumps"),
    }
}

pub fn tools_available() -> PgTools {
    PgTools {
        pg_dump: ToolStatus::Available(PathBuf::from("/bin/pg_dump")),
        pg_restore: ToolStatus::Available(PathBuf::from("/bin/pg_restore")),
    }
}

pub fn tools_missing() -> PgTools {
    PgTools {
        pg_dump: ToolStatus::Missing("`pg_dump` was not found in PATH".to_owned()),
        pg_restore: ToolStatus::Missing("`pg_restore` was not found in PATH".to_owned()),
    }
}

pub fn connection(tools: PgTools) -> Connection {
    Connection {
        session: SessionInfo {
            database: "ducky".to_owned(),
            user: "ducky".to_owned(),
            server_version: "18.0".to_owned(),
        },
        tools,
    }
}

fn column(name: &str) -> Column {
    Column {
        name: name.to_owned(),
        data_type: "text".to_owned(),
        is_nullable: true,
        default_expression: None,
        is_primary_key: false,
    }
}

pub fn relation(name: &str, kind: RelationKind) -> Relation {
    Relation {
        name: name.to_owned(),
        kind,
        estimated_row_count: Some(10),
        columns: vec![column("id"), column("name")],
    }
}

pub fn schema(name: &str, relations: Vec<Relation>) -> Schema {
    Schema {
        name: name.to_owned(),
        relations,
    }
}

pub fn catalog() -> Catalog {
    Catalog {
        schemas: vec![
            schema("analytics", vec![relation("events", RelationKind::Table)]),
            schema(
                "public",
                vec![
                    relation("orders", RelationKind::Table),
                    relation("customers", RelationKind::Table),
                    relation("order_totals", RelationKind::View),
                ],
            ),
        ],
    }
}

pub fn send(state: &mut AppState, msg: Msg) -> Vec<Command> {
    update(state, msg)
}

pub fn connected_state(tools: PgTools) -> AppState {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    send(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(connection(tools)),
        },
    );
    state
}

pub fn workspace_state() -> AppState {
    let mut state = connected_state(tools_available());
    send(&mut state, Msg::CatalogLoaded(Ok(catalog())));
    state
}

pub fn workspace(state: &AppState) -> &Workspace {
    match &state.screen {
        Screen::Workspace(workspace) => workspace.as_ref(),
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

pub fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace.as_mut(),
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

pub fn select_sidebar_item(state: &mut AppState, predicate: impl Fn(&SidebarItem) -> bool) {
    let item_index = workspace(state)
        .sidebar_items()
        .iter()
        .position(predicate)
        .expect("sidebar item present");
    workspace_mut(state).sidebar.selected_item_index = item_index;
}

pub fn activate_relation(state: &mut AppState, relation_name: &str) -> Vec<Command> {
    select_sidebar_item(state, |item| item.relation.name == relation_name);
    send(state, Msg::Sidebar(SidebarMsg::Activate))
}

pub fn active_tab_id(state: &AppState) -> TabId {
    workspace(state).active_tab().expect("active tab").id
}

pub fn new_query(state: &mut AppState) -> TabId {
    send(state, Msg::Tabs(TabsMsg::NewQuery));
    active_tab_id(state)
}

pub fn query_tab(state: &AppState, tab_id: TabId) -> &QueryView {
    let tab_index = workspace(state).tab_index(tab_id).expect("tab open");
    match &workspace(state).tabs[tab_index].content {
        TabContent::Query(query) => query,
        other => panic!("expected a query tab, got {other:?}"),
    }
}

pub fn query_tab_mut(state: &mut AppState, tab_id: TabId) -> &mut QueryView {
    let tab_index = workspace(state).tab_index(tab_id).expect("tab open");
    match &mut workspace_mut(state).tabs[tab_index].content {
        TabContent::Query(query) => query,
        other => panic!("expected a query tab, got {other:?}"),
    }
}

pub fn active_query(state: &AppState) -> &QueryView {
    query_tab(state, active_tab_id(state))
}

pub fn table_tab(state: &AppState, tab_id: TabId) -> &TableView {
    let tab_index = workspace(state).tab_index(tab_id).expect("tab open");
    match &workspace(state).tabs[tab_index].content {
        TabContent::Table(table) => table,
        other => panic!("expected a table tab, got {other:?}"),
    }
}

pub fn table_tab_mut(state: &mut AppState, tab_id: TabId) -> &mut TableView {
    let tab_index = workspace(state).tab_index(tab_id).expect("tab open");
    match &mut workspace_mut(state).tabs[tab_index].content {
        TabContent::Table(table) => table,
        other => panic!("expected a table tab, got {other:?}"),
    }
}

pub fn active_table(state: &AppState) -> &TableView {
    table_tab(state, active_tab_id(state))
}

pub fn result_set(columns: &[&str], row_count: usize) -> ResultSet {
    ResultSet {
        columns: columns.iter().map(|column| (*column).to_owned()).collect(),
        rows: (0..row_count)
            .map(|row_index| {
                columns
                    .iter()
                    .enumerate()
                    .map(|(column_index, _)| {
                        if column_index == 1 && row_index == 0 {
                            None
                        } else {
                            Some(format!("r{row_index}c{column_index}"))
                        }
                    })
                    .collect()
            })
            .collect(),
        is_truncated: false,
    }
}

pub fn page(row_count: usize, has_next_page: bool) -> Page {
    Page {
        result_set: result_set(&["id", "name"], row_count),
        has_next_page,
    }
}

pub fn open_loaded_table(state: &mut AppState, has_next_page: bool) {
    let commands = activate_relation(state, "customers");
    let Some(Command::LoadPage(request)) = commands.into_iter().next() else {
        panic!("expected LoadPage");
    };
    send(
        state,
        Msg::PageLoaded {
            request,
            result: Ok(page(3, has_next_page)),
        },
    );
}

pub fn rows_result(sql: &str, row_count: usize) -> StatementResult {
    StatementResult {
        sql: sql.to_owned(),
        outcome: StatementOutcome::Rows(result_set(&["a", "b"], row_count)),
        elapsed: Duration::from_millis(1),
    }
}

pub fn affected_result(sql: &str, affected: u64) -> StatementResult {
    StatementResult {
        sql: sql.to_owned(),
        outcome: StatementOutcome::Affected(affected),
        elapsed: Duration::from_millis(1),
    }
}

pub fn report(mode: TransactionMode, results: Vec<StatementResult>) -> ScriptReport {
    ScriptReport {
        mode,
        results,
        failure: None,
        elapsed: Duration::from_millis(42),
    }
}

pub fn file_entry(name: &str) -> FileEntry {
    FileEntry {
        name: name.to_owned(),
        size_bytes: 10,
        modified_at: None,
    }
}

mod support;

use ducky_app::{Command, Focus, Msg, NoticeLevel, PendingSave, QueryMsg, TabContent, TabsMsg};
use support::{
    activate_relation, active_tab_id, new_query, query_tab, send, workspace, workspace_mut,
    workspace_state,
};

fn active_index(state: &ducky_app::AppState) -> usize {
    workspace(state).active_tab_index
}

fn three_tabs_state() -> ducky_app::AppState {
    let mut state = workspace_state();
    new_query(&mut state);
    new_query(&mut state);
    new_query(&mut state);
    state
}

#[test]
fn new_query_tabs_are_numbered_with_fresh_ids() {
    let mut state = workspace_state();
    activate_relation(&mut state, "customers");
    workspace_mut(&mut state).focus = ducky_app::Focus::Sidebar;
    let first_query_tab_id = new_query(&mut state);
    let second_query_tab_id = new_query(&mut state);
    assert_eq!((first_query_tab_id, second_query_tab_id), (2, 3));
    assert_eq!(query_tab(&state, first_query_tab_id).number, 1);
    assert_eq!(query_tab(&state, second_query_tab_id).number, 2);
    let workspace = workspace(&state);
    assert_eq!(workspace.tabs[1].title(), "Query 1");
    assert_eq!(workspace.tabs[2].title(), "Query 2");
    assert_eq!(workspace.active_tab_index, 2);
    assert_eq!(workspace.next_tab_id, 4);
    assert_eq!(workspace.next_query_number, 3);
    assert_eq!(workspace.focus, Focus::Main);
}

#[test]
fn next_and_previous_wrap_around() {
    let mut state = three_tabs_state();
    send(&mut state, Msg::Tabs(TabsMsg::Next));
    assert_eq!(active_index(&state), 0);
    send(&mut state, Msg::Tabs(TabsMsg::Previous));
    assert_eq!(active_index(&state), 2);
    send(&mut state, Msg::Tabs(TabsMsg::Previous));
    assert_eq!(active_index(&state), 1);
}

#[test]
fn tab_navigation_keeps_focus() {
    let mut state = three_tabs_state();
    workspace_mut(&mut state).focus = Focus::Sidebar;
    send(&mut state, Msg::Tabs(TabsMsg::Next));
    send(&mut state, Msg::Tabs(TabsMsg::Select(1)));
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn navigation_without_tabs_does_nothing() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::Next));
    send(&mut state, Msg::Tabs(TabsMsg::Previous));
    assert!(send(&mut state, Msg::Tabs(TabsMsg::Close)).is_empty());
    assert_eq!(active_index(&state), 0);
    assert!(workspace(&state).tabs.is_empty());
}

#[test]
fn select_out_of_range_is_ignored() {
    let mut state = three_tabs_state();
    send(&mut state, Msg::Tabs(TabsMsg::Select(0)));
    assert_eq!(active_index(&state), 0);
    send(&mut state, Msg::Tabs(TabsMsg::Select(3)));
    assert_eq!(active_index(&state), 0);
}

#[test]
fn close_clamps_index_and_returns_focus_to_sidebar_when_empty() {
    let mut state = three_tabs_state();
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert_eq!(workspace(&state).tabs.len(), 2);
    assert_eq!(active_index(&state), 1);
    send(&mut state, Msg::Tabs(TabsMsg::Select(0)));
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert_eq!(active_index(&state), 0);
    assert_eq!(active_tab_id(&state), 2);
    assert_eq!(workspace(&state).focus, Focus::Main);
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert!(workspace(&state).tabs.is_empty());
    assert_eq!(active_index(&state), 0);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn closing_the_running_tab_is_refused() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(
        &mut state,
        Msg::Query(QueryMsg::Run {
            sql: "select 1".to_owned(),
        }),
    );
    assert!(send(&mut state, Msg::Tabs(TabsMsg::Close)).is_empty());
    assert_eq!(workspace(&state).tabs.len(), 1);
    let notice = state.notice.clone().expect("notice");
    assert_eq!(
        (notice.level, notice.text.as_str()),
        (
            NoticeLevel::Error,
            "Cancel the running script before closing this tab"
        )
    );
    new_query(&mut state);
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert_eq!(workspace(&state).tabs.len(), 1);
}

#[test]
fn close_drops_the_pending_save_of_that_tab() {
    let mut state = workspace_state();
    let tab_id = new_query(&mut state);
    workspace_mut(&mut state).pending_save = Some(PendingSave {
        tab_id,
        name: "a".to_owned(),
        content: "x".to_owned(),
    });
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert_eq!(workspace(&state).pending_save, None);
}

#[test]
fn tool_tabs_are_singletons() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Tabs(TabsMsg::OpenDump)).is_empty());
    assert_eq!(
        send(&mut state, Msg::Tabs(TabsMsg::OpenRestore)),
        vec![Command::ListDumps]
    );
    assert!(workspace(&state).restore.entries.is_loading());
    assert_eq!(
        send(&mut state, Msg::Tabs(TabsMsg::OpenFiles)),
        vec![Command::ListSqlFiles]
    );
    workspace_mut(&mut state).focus = Focus::Sidebar;
    send(&mut state, Msg::Tabs(TabsMsg::OpenDump));
    let workspace = workspace(&state);
    assert_eq!(workspace.tabs.len(), 3);
    assert_eq!(workspace.active_tab_index, 0);
    assert_eq!(workspace.focus, Focus::Main);
    let titles: Vec<String> = workspace.tabs.iter().map(|tab| tab.title()).collect();
    assert_eq!(titles, ["Dump", "Restore", "SQL files"]);
    assert!(matches!(workspace.tabs[2].content, TabContent::Files));
}

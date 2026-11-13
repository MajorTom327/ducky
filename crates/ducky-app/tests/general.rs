mod support;

use ducky_app::{
    Command, Focus, Loadable, Modal, Msg, Notice, NoticeLevel, PaneDirection, QueryPane,
    SidebarMsg, TabsMsg,
};
use support::{
    active_query, assert_ignored_after_disconnect, new_query, send, workspace, workspace_mut,
    workspace_state,
};

#[test]
fn quit_sets_should_quit() {
    let mut state = workspace_state();
    assert!(send(&mut state, Msg::Quit).is_empty());
    assert!(state.should_quit);
}

#[test]
fn notified_then_dismissed() {
    let mut state = workspace_state();
    let notice = Notice {
        level: NoticeLevel::Info,
        text: "hello".to_owned(),
    };
    send(&mut state, Msg::Notified(notice.clone()));
    assert_eq!(state.notice, Some(notice));
    send(&mut state, Msg::DismissNotice);
    assert_eq!(state.notice, None);
}

#[test]
fn open_help_shows_help_modal() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenHelp);
    assert!(matches!(state.modal, Some(Modal::Help)));
}

#[test]
fn focus_cycles_through_editor_and_results_in_query_tab() {
    let mut state = workspace_state();
    new_query(&mut state);
    let focus_and_pane =
        |state: &ducky_app::AppState| (workspace(state).focus, active_query(state).pane);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Editor));
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Results));
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Editor));
}

#[test]
fn focus_stays_on_sidebar_without_tabs() {
    let mut state = workspace_state();
    assert!(workspace(&state).tabs.is_empty());
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn focus_toggles_in_other_tabs() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::OpenDump));
    assert_eq!(workspace(&state).focus, Focus::Main);
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn pane_directions_move_between_sidebar_editor_and_results() {
    let mut state = workspace_state();
    new_query(&mut state);
    let focus_and_pane =
        |state: &ducky_app::AppState| (workspace(state).focus, active_query(state).pane);
    send(&mut state, Msg::FocusPane(PaneDirection::Down));
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Results));
    send(&mut state, Msg::FocusPane(PaneDirection::Left));
    assert_eq!(focus_and_pane(&state), (Focus::Sidebar, QueryPane::Results));
    send(&mut state, Msg::FocusPane(PaneDirection::Up));
    assert_eq!(focus_and_pane(&state), (Focus::Sidebar, QueryPane::Results));
    send(&mut state, Msg::FocusPane(PaneDirection::Right));
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Results));
    send(&mut state, Msg::FocusPane(PaneDirection::Up));
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Editor));
}

#[test]
fn pane_right_without_tabs_keeps_the_sidebar() {
    let mut state = workspace_state();
    send(&mut state, Msg::FocusPane(PaneDirection::Right));
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn schema_picker_opens_on_current_schema() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenSchemaPicker);
    assert!(matches!(
        state.modal,
        Some(Modal::SchemaPicker {
            selected_schema_index: 1
        })
    ));
}

#[test]
fn schema_picker_requires_ready_catalog() {
    let mut state = support::connected_state(support::tools_available());
    send(&mut state, Msg::OpenSchemaPicker);
    assert!(state.modal.is_none());
}

#[test]
fn refresh_keeps_ready_catalog_visible() {
    let mut state = workspace_state();
    assert_eq!(
        send(&mut state, Msg::RefreshCatalog),
        vec![Command::LoadCatalog]
    );
    assert!(workspace(&state).catalog.ready().is_some());
}

#[test]
fn refresh_after_failure_shows_loading() {
    let mut state = workspace_state();
    workspace_mut(&mut state).catalog = Loadable::Failed("boom".to_owned());
    assert_eq!(
        send(&mut state, Msg::RefreshCatalog),
        vec![Command::LoadCatalog]
    );
    assert!(workspace(&state).catalog.is_loading());
}

#[test]
fn workspace_messages_are_ignored_on_profiles_screen() {
    let mut state = ducky_app::AppState::new(support::profiles(), support::storage());
    assert!(send(&mut state, Msg::RefreshCatalog).is_empty());
    assert!(send(&mut state, Msg::Reconnect).is_empty());
    assert!(send(&mut state, Msg::Sidebar(SidebarMsg::Activate)).is_empty());
    assert!(send(&mut state, Msg::Tabs(TabsMsg::NewQuery)).is_empty());
    assert!(send(&mut state, Msg::OpenPalette).is_empty());
    assert!(state.modal.is_none());
    assert!(state.workspace().is_none());
}

#[test]
fn workspace_wide_messages_are_ignored_on_the_profile_screen() {
    assert_ignored_after_disconnect(vec![
        Msg::OpenSchemaPicker,
        Msg::ToggleFocus,
        Msg::FocusPane(PaneDirection::Left),
        Msg::FocusPane(PaneDirection::Right),
    ]);
}

mod support;

use ducky_app::{
    Command, Focus, Loadable, MainView, Modal, Msg, Notice, NoticeLevel, QueryPane, SidebarAction,
    SidebarItem, SidebarMsg,
};
use support::{send, workspace, workspace_mut, workspace_state};

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
fn focus_cycles_through_editor_and_results_in_query_view() {
    let mut state = workspace_state();
    support::select_sidebar_item(&mut state, |item| {
        *item == SidebarItem::Action(SidebarAction::SqlCommand)
    });
    send(&mut state, Msg::Sidebar(SidebarMsg::Activate));
    let focus_and_pane =
        |state: &ducky_app::AppState| (workspace(state).focus, workspace(state).query.pane);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Editor));
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Results));
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(focus_and_pane(&state), (Focus::Main, QueryPane::Editor));
}

#[test]
fn focus_toggles_in_other_views() {
    let mut state = workspace_state();
    assert_eq!(workspace(&state).view, MainView::Welcome);
    send(&mut state, Msg::ToggleFocus);
    assert_eq!(workspace(&state).focus, Focus::Main);
    send(&mut state, Msg::ToggleFocus);
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
    assert!(state.workspace().is_none());
}

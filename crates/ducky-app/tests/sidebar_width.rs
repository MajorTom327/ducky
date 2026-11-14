mod support;

use ducky_app::{
    AppState, DEFAULT_SIDEBAR_WIDTH, Focus, MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH, ModalMsg, Msg,
    PaneDirection, SIDEBAR_WIDTH_STEP, Sidebar, SidebarMsg, TabsMsg,
};
use support::{new_query, send, workspace, workspace_mut, workspace_state};
use tui_input::InputRequest;

fn width(state: &AppState) -> u16 {
    workspace(state).sidebar.width
}

fn is_collapsed(state: &AppState) -> bool {
    workspace(state).sidebar.is_collapsed
}

fn sidebar(state: &mut AppState, msg: SidebarMsg) {
    send(state, Msg::Sidebar(msg));
}

fn collapsed_query_state() -> AppState {
    let mut state = workspace_state();
    new_query(&mut state);
    sidebar(&mut state, SidebarMsg::ToggleCollapsed);
    state
}

#[test]
fn default_sidebar_is_expanded_at_the_default_width() {
    let default_sidebar = Sidebar::default();
    assert_eq!(default_sidebar.width, DEFAULT_SIDEBAR_WIDTH);
    assert!(!default_sidebar.is_collapsed);
    let state = workspace_state();
    assert_eq!(width(&state), 32);
    assert!(!is_collapsed(&state));
}

#[test]
fn widen_and_narrow_step_by_four_and_clamp() {
    let mut state = workspace_state();
    sidebar(&mut state, SidebarMsg::Widen);
    assert_eq!(width(&state), DEFAULT_SIDEBAR_WIDTH + SIDEBAR_WIDTH_STEP);
    sidebar(&mut state, SidebarMsg::Narrow);
    sidebar(&mut state, SidebarMsg::Narrow);
    assert_eq!(width(&state), 28);
    for _ in 0..10 {
        sidebar(&mut state, SidebarMsg::Narrow);
    }
    assert_eq!(width(&state), MIN_SIDEBAR_WIDTH);
    for _ in 0..30 {
        sidebar(&mut state, SidebarMsg::Widen);
    }
    assert_eq!(width(&state), MAX_SIDEBAR_WIDTH);
}

#[test]
fn set_width_clamps_and_expands() {
    let mut state = collapsed_query_state();
    sidebar(&mut state, SidebarMsg::SetWidth(5));
    assert_eq!(width(&state), MIN_SIDEBAR_WIDTH);
    assert!(!is_collapsed(&state));
    sidebar(&mut state, SidebarMsg::SetWidth(500));
    assert_eq!(width(&state), MAX_SIDEBAR_WIDTH);
    sidebar(&mut state, SidebarMsg::SetWidth(45));
    assert_eq!(width(&state), 45);
}

#[test]
fn collapsing_moves_the_focus_to_main_and_expanding_focuses_the_sidebar() {
    let mut state = workspace_state();
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    sidebar(&mut state, SidebarMsg::ToggleCollapsed);
    assert!(is_collapsed(&state));
    assert_eq!(workspace(&state).focus, Focus::Main);
    sidebar(&mut state, SidebarMsg::ToggleCollapsed);
    assert!(!is_collapsed(&state));
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    assert_eq!(width(&state), DEFAULT_SIDEBAR_WIDTH);
}

#[test]
fn widen_or_narrow_on_a_collapsed_sidebar_only_expands_it() {
    let mut state = collapsed_query_state();
    sidebar(&mut state, SidebarMsg::Widen);
    assert!(!is_collapsed(&state));
    assert_eq!(width(&state), DEFAULT_SIDEBAR_WIDTH);
    sidebar(&mut state, SidebarMsg::ToggleCollapsed);
    sidebar(&mut state, SidebarMsg::Narrow);
    assert!(!is_collapsed(&state));
    assert_eq!(width(&state), DEFAULT_SIDEBAR_WIDTH);
}

#[test]
fn focusing_the_sidebar_expands_it() {
    let mut state = collapsed_query_state();
    send(&mut state, Msg::FocusPane(PaneDirection::Left));
    assert!(!is_collapsed(&state));
    assert_eq!(workspace(&state).focus, Focus::Sidebar);

    let mut state = collapsed_query_state();
    send(&mut state, Msg::ToggleFocus);
    assert!(is_collapsed(&state));
    send(&mut state, Msg::ToggleFocus);
    assert!(!is_collapsed(&state));
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn closing_the_last_tab_expands_the_collapsed_sidebar() {
    let mut state = collapsed_query_state();
    send(&mut state, Msg::Tabs(TabsMsg::Close));
    assert!(workspace(&state).tabs.is_empty());
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    assert!(!is_collapsed(&state));
}

#[test]
fn select_clamps_the_index_and_focuses_the_sidebar() {
    let mut state = collapsed_query_state();
    sidebar(&mut state, SidebarMsg::Select(1));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 1);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
    assert!(!is_collapsed(&state));
    sidebar(&mut state, SidebarMsg::Select(99));
    assert_eq!(workspace(&state).sidebar.selected_item_index, 2);
}

#[test]
fn select_then_activate_opens_the_clicked_relation() {
    let mut state = workspace_state();
    sidebar(&mut state, SidebarMsg::Select(2));
    sidebar(&mut state, SidebarMsg::Activate);
    let active_title = workspace(&state)
        .active_tab()
        .map(|tab| tab.title())
        .expect("a tab is open");
    assert_eq!(active_title, "orders");
    assert_eq!(workspace(&state).focus, Focus::Main);
}

#[test]
fn palette_toggle_sidebar_collapses_and_expands() {
    let mut state = workspace_state();
    workspace_mut(&mut state).focus = Focus::Main;
    for (is_expected_collapsed, expected_focus) in [(true, Focus::Main), (false, Focus::Sidebar)] {
        send(&mut state, Msg::OpenPalette);
        for character in "toggle sidebar".chars() {
            send(
                &mut state,
                Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(character))),
            );
        }
        send(&mut state, Msg::Modal(ModalMsg::Submit));
        assert!(state.modal.is_none());
        assert_eq!(is_collapsed(&state), is_expected_collapsed);
        assert_eq!(workspace(&state).focus, expected_focus);
    }
}

#[test]
fn sidebar_messages_are_ignored_on_the_profile_screen() {
    let mut state = AppState::new(support::profiles(), support::storage());
    assert!(send(&mut state, Msg::Sidebar(SidebarMsg::ToggleCollapsed)).is_empty());
    assert!(state.workspace().is_none());
}

mod mouse_support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{Focus, GridMove, Msg, Sidebar, SidebarMsg, TableMsg};
use ducky_tui::keys::handle_key;
use ducky_tui::keys::help::HELP_ROWS;
use ducky_tui::keys::hints::hints;
use ducky_tui::keys::sequence::{KeyPrefix, pending_hints};
use ducky_tui::layout::{FrameAreas, MIN_MAIN_WIDTH, sidebar_width};
use ducky_tui::tui_state::TuiState;
use mouse_support::{
    draw_rows, position_of, query_state_with_results, send, table_state, workspace_mut,
    workspace_state,
};
use ratatui::layout::Rect;

const SCREEN: Rect = Rect::new(0, 0, 120, 36);

fn press_all(state: &ducky_app::AppState, tui: &mut TuiState, characters: &str) -> Vec<Msg> {
    characters
        .chars()
        .flat_map(|character| {
            handle_key(
                state,
                tui,
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
            )
        })
        .collect()
}

#[test]
fn frame_areas_give_the_sidebar_its_width_or_nothing_when_collapsed() {
    let wide = FrameAreas::new(SCREEN, 50);
    assert_eq!((wide.sidebar.x, wide.sidebar.width), (0, 50));
    assert_eq!((wide.main.x, wide.main.width), (50, 70));
    assert_eq!(wide.tab_bar.x, 50);
    let collapsed = FrameAreas::new(SCREEN, 0);
    assert_eq!(collapsed.sidebar.width, 0);
    assert_eq!((collapsed.main.x, collapsed.main.width), (0, 120));
    assert_eq!(collapsed.body(), wide.body());
}

#[test]
fn sidebar_width_keeps_forty_main_columns_and_is_zero_when_collapsed() {
    let mut sidebar = Sidebar::default();
    assert_eq!(sidebar_width(&sidebar, 120), 32);
    sidebar.width = 80;
    assert_eq!(sidebar_width(&sidebar, 100), 100 - MIN_MAIN_WIDTH);
    assert_eq!(sidebar_width(&sidebar, 30), 0);
    sidebar.is_collapsed = true;
    assert_eq!(sidebar_width(&sidebar, 200), 0);
}

#[test]
fn space_e_toggles_the_sidebar_from_the_sidebar_and_the_editor() {
    let state = workspace_state();
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, " e"),
        vec![Msg::Sidebar(SidebarMsg::ToggleCollapsed)]
    );
    let editor_state = query_state_with_results();
    assert_eq!(
        press_all(&editor_state, &mut tui, " e"),
        vec![Msg::Sidebar(SidebarMsg::ToggleCollapsed)]
    );
    let leader_hints = pending_hints(KeyPrefix::Leader);
    assert!(leader_hints.contains(&("e", "sidebar")));
}

#[test]
fn angle_brackets_resize_only_the_focused_sidebar() {
    let state = workspace_state();
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, "<>"),
        vec![
            Msg::Sidebar(SidebarMsg::Narrow),
            Msg::Sidebar(SidebarMsg::Widen),
        ]
    );
    let table = table_state();
    assert!(
        press_all(&table, &mut tui, "<>")
            .iter()
            .all(|message| !matches!(message, Msg::Sidebar(_)))
    );
    assert!(hints(&state).contains(&("< >", "width")));
}

#[test]
fn a_collapsed_sidebar_gives_the_whole_width_to_the_main_pane() {
    let mut state = table_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    assert!(rows[1].contains(" public "));
    let expanded_ada_column = position_of(&rows, "Ada", 0).0;
    send(&mut state, [Msg::Sidebar(SidebarMsg::ToggleCollapsed)]);
    let rows = draw_rows(&state, &mut tui);
    assert!(!rows[1].contains(" public "));
    assert!(rows[1].starts_with(" 1 customers"));
    assert!(rows[2].starts_with('┌'));
    let collapsed_ada_column = position_of(&rows, "Ada", 0).0;
    assert_eq!(
        expanded_ada_column - collapsed_ada_column,
        ducky_app::DEFAULT_SIDEBAR_WIDTH
    );
    send(&mut state, [Msg::Table(TableMsg::Move(GridMove::Down))]);
    send(&mut state, [Msg::ToggleFocus]);
    let rows = draw_rows(&state, &mut tui);
    assert!(rows[1].contains(" public "));
}

#[test]
fn a_wider_sidebar_pushes_the_main_pane_right() {
    let mut state = workspace_state();
    workspace_mut(&mut state).sidebar.width = 60;
    workspace_mut(&mut state).focus = Focus::Main;
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    assert_eq!(rows[2].chars().nth(59), Some('│'));
    workspace_mut(&mut state).sidebar.width = 80;
    let rows = draw_rows(&state, &mut tui);
    let main_column = 120 - MIN_MAIN_WIDTH;
    assert_eq!(rows[2].chars().nth(usize::from(main_column - 1)), Some('│'));
}

#[test]
fn the_palette_shows_the_toggle_shortcut_and_help_lists_the_new_rows() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenPalette]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let toggle_row = rows
        .iter()
        .find(|row| row.contains("Toggle sidebar"))
        .expect("the palette lists Toggle sidebar");
    assert!(toggle_row.contains("Space e"));
    for expected_row in [
        ("Space e", "show / hide the sidebar"),
        ("< / >", "narrow / widen the sidebar"),
        ("", "Mouse"),
    ] {
        assert!(HELP_ROWS.contains(&expected_row), "{expected_row:?}");
    }
}

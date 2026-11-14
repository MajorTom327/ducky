mod mouse_support;

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEventKind};
use ducky_app::{
    DEFAULT_SIDEBAR_WIDTH, Focus, GridCursor, GridMove, Msg, Notice, NoticeLevel, PaneDirection,
    QueryMsg, QueryPane, SidebarMsg, TabContent, TableMsg, TableTab, TabsMsg,
};
use ducky_tui::keys::focused_editor_tab;
use ducky_tui::mouse::{DOUBLE_CLICK_INTERVAL, MouseTarget, handle_mouse};
use ducky_tui::tui_state::TuiState;
use edtui::{EditorState, Lines};
use mouse_support::{
    active_tab_id, click, draw_rows, mouse_event, position_of, query_state_with_results, send,
    table_state, wheel, workspace, workspace_mut, workspace_state,
};

const MAIN_COLUMN: u16 = DEFAULT_SIDEBAR_WIDTH;

fn cell(row_index: usize, column_index: usize) -> GridCursor {
    GridCursor {
        row_index,
        column_index,
    }
}

#[test]
fn clicking_a_sidebar_table_selects_and_opens_it() {
    let mut state = workspace_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let orders = position_of(&rows, "orders", 0);
    assert!(orders.0 < MAIN_COLUMN);
    let messages = click(&state, &mut tui, orders);
    assert_eq!(
        messages,
        vec![
            Msg::Sidebar(SidebarMsg::Select(2)),
            Msg::Sidebar(SidebarMsg::Activate),
        ]
    );
    send(&mut state, messages);
    let active_tab = workspace(&state).active_tab().expect("a tab opened");
    assert_eq!(active_tab.title(), "orders");
}

#[test]
fn clicking_the_empty_sidebar_area_focuses_it_and_the_wheel_moves_three_items() {
    let mut state = table_state();
    let mut tui = TuiState::new(false);
    draw_rows(&state, &mut tui);
    let below_items = (4, 20);
    assert_eq!(
        click(&state, &mut tui, below_items),
        vec![Msg::FocusPane(PaneDirection::Left)]
    );
    let next = Msg::Sidebar(SidebarMsg::Next);
    assert_eq!(
        wheel(&state, &mut tui, below_items, true),
        vec![next.clone(), next.clone(), next]
    );
    let previous = Msg::Sidebar(SidebarMsg::Previous);
    assert_eq!(
        wheel(&state, &mut tui, (4, 2), false),
        vec![previous.clone(), previous.clone(), previous]
    );
    send(&mut state, [Msg::FocusPane(PaneDirection::Left)]);
    assert_eq!(workspace(&state).focus, Focus::Sidebar);
}

#[test]
fn clicking_a_tab_label_selects_that_tab() {
    let mut state = table_state();
    send(&mut state, [Msg::Tabs(TabsMsg::NewQuery)]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let first_tab = position_of(&rows, "1 customers", MAIN_COLUMN);
    assert_eq!(first_tab.1, 1);
    let messages = click(&state, &mut tui, first_tab);
    assert_eq!(messages, vec![Msg::Tabs(TabsMsg::Select(0))]);
    send(&mut state, messages);
    assert_eq!(workspace(&state).active_tab_index, 0);
    let gap_after_the_last_tab = (WIDTH_BEFORE_EDGE, 1);
    assert!(click(&state, &mut tui, gap_after_the_last_tab).is_empty());
}

const WIDTH_BEFORE_EDGE: u16 = mouse_support::WIDTH - 2;

#[test]
fn clicking_a_cell_moves_the_cursor_and_a_double_click_changes_the_cell() {
    let mut state = table_state();
    workspace_mut(&mut state).focus = Focus::Sidebar;
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let bea = position_of(&rows, "Bea", MAIN_COLUMN);
    let select_bea = Msg::Table(TableMsg::SelectCell(cell(1, 1)));
    let messages = click(&state, &mut tui, bea);
    assert_eq!(messages, vec![select_bea.clone()]);
    send(&mut state, messages);
    assert_eq!(workspace(&state).focus, Focus::Main);
    draw_rows(&state, &mut tui);
    assert_eq!(
        click(&state, &mut tui, bea),
        vec![select_bea.clone(), Msg::Table(TableMsg::ChangeCell)]
    );
    assert_eq!(click(&state, &mut tui, bea), vec![select_bea]);
}

#[test]
fn a_slow_second_click_is_not_a_double_click() {
    let state = table_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let ada = position_of(&rows, "Ada", MAIN_COLUMN);
    let target = MouseTarget::GridCell {
        pane: ducky_tui::keys::sequence::GridPane::TableData,
        row_index: 0,
        column_index: 1,
    };
    let long_ago = Instant::now()
        .checked_sub(DOUBLE_CLICK_INTERVAL + Duration::from_millis(50))
        .expect("an earlier instant");
    tui.last_click = Some((target, long_ago));
    assert_eq!(
        click(&state, &mut tui, ada),
        vec![Msg::Table(TableMsg::SelectCell(cell(0, 1)))]
    );
}

#[test]
fn clicking_a_header_sorts_on_that_column_from_the_cursor_row() {
    let mut state = table_state();
    send(&mut state, [Msg::Table(TableMsg::Move(GridMove::Down))]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let header = position_of(&rows, "name", MAIN_COLUMN);
    let messages = click(&state, &mut tui, header);
    assert_eq!(
        messages,
        vec![
            Msg::Table(TableMsg::SelectCell(cell(1, 1))),
            Msg::Table(TableMsg::CycleSort),
        ]
    );
    send(&mut state, messages);
    let TabContent::Table(table) = &workspace(&state).active_tab().expect("tab").content else {
        panic!("expected a table tab");
    };
    let sort = table.request.sort.as_ref().expect("a sort");
    assert_eq!(sort.column, "name");
}

#[test]
fn the_wheel_on_the_grid_moves_three_rows() {
    let state = table_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let ada = position_of(&rows, "Ada", MAIN_COLUMN);
    let down = Msg::Table(TableMsg::Move(GridMove::Down));
    assert_eq!(
        wheel(&state, &mut tui, ada, true),
        vec![down.clone(), down.clone(), down]
    );
    let below_rows = (ada.0, ada.1 + 10);
    let up = Msg::Table(TableMsg::Move(GridMove::Up));
    assert_eq!(
        wheel(&state, &mut tui, below_rows, false),
        vec![up.clone(), up.clone(), up]
    );
}

#[test]
fn results_cells_focus_the_results_and_a_double_click_opens_the_row() {
    let mut state = query_state_with_results();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let cyd = position_of(&rows, "Cyd", MAIN_COLUMN);
    let select_cyd = Msg::Query(QueryMsg::SelectCell(cell(2, 1)));
    let messages = click(&state, &mut tui, cyd);
    assert_eq!(messages, vec![select_cyd.clone()]);
    send(&mut state, messages);
    let TabContent::Query(query) = &workspace(&state).active_tab().expect("tab").content else {
        panic!("expected a query tab");
    };
    assert_eq!(query.pane, QueryPane::Results);
    assert_eq!(query.cursor, cell(2, 1));
    draw_rows(&state, &mut tui);
    assert_eq!(
        click(&state, &mut tui, cyd),
        vec![select_cyd, Msg::Query(QueryMsg::OpenRowDetail)]
    );
    let down = Msg::Query(QueryMsg::Move(GridMove::Down));
    assert_eq!(
        wheel(&state, &mut tui, cyd, true),
        vec![down.clone(), down.clone(), down]
    );
}

#[test]
fn clicking_the_results_header_or_empty_area_focuses_the_results() {
    let state = query_state_with_results();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let header = position_of(&rows, "name", MAIN_COLUMN);
    let focus_results = vec![
        Msg::FocusPane(PaneDirection::Right),
        Msg::FocusPane(PaneDirection::Down),
    ];
    assert_eq!(click(&state, &mut tui, header), focus_results);
    let results_bottom = (header.0, mouse_support::HEIGHT - 4);
    assert_eq!(click(&state, &mut tui, results_bottom), focus_results);
}

#[test]
fn clicking_the_editor_focuses_it_and_places_the_cursor() {
    let mut state = query_state_with_results();
    send(&mut state, [Msg::FocusPane(PaneDirection::Down)]);
    let tab_id = active_tab_id(&state);
    let mut tui = TuiState::new(false);
    draw_rows(&state, &mut tui);
    tui.editor(tab_id).state = EditorState::new(Lines::from("select 1\nfrom orders"));
    let rows = draw_rows(&state, &mut tui);
    let orders = position_of(&rows, "orders", MAIN_COLUMN);
    let messages = click(&state, &mut tui, orders);
    assert_eq!(
        messages,
        vec![
            Msg::FocusPane(PaneDirection::Right),
            Msg::FocusPane(PaneDirection::Up),
        ]
    );
    let editor_cursor = tui.editor(tab_id).state.cursor;
    assert_eq!((editor_cursor.row, editor_cursor.col), (1, 5));
    send(&mut state, messages);
    assert_eq!(focused_editor_tab(workspace(&state)), Some(tab_id));
    draw_rows(&state, &mut tui);
    let select_word = position_of(&rows, "select", MAIN_COLUMN);
    assert!(click(&state, &mut tui, select_word).is_empty());
    let drag_end = (select_word.0 + 3, select_word.1);
    let drag = mouse_event(MouseEventKind::Drag(MouseButton::Left), drag_end);
    assert!(handle_mouse(&state, &mut tui, drag).is_empty());
    let release = mouse_event(MouseEventKind::Up(MouseButton::Left), drag_end);
    assert!(handle_mouse(&state, &mut tui, release).is_empty());
    let editor = tui.editor(tab_id);
    assert!(editor.state.selection.is_some());
    assert_eq!((editor.state.cursor.row, editor.state.cursor.col), (0, 3));
}

#[test]
fn dragging_the_sidebar_border_resizes_the_sidebar() {
    let mut state = workspace_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let border_column = MAIN_COLUMN - 1;
    assert_eq!(rows[5].chars().nth(usize::from(border_column)), Some('│'));
    assert!(click(&state, &mut tui, (border_column, 5)).is_empty());
    assert!(tui.is_dragging_sidebar);
    let drag = mouse_event(MouseEventKind::Drag(MouseButton::Left), (45, 9));
    let messages = handle_mouse(&state, &mut tui, drag);
    assert_eq!(messages, vec![Msg::Sidebar(SidebarMsg::SetWidth(46))]);
    send(&mut state, messages);
    let release = mouse_event(MouseEventKind::Up(MouseButton::Left), (45, 9));
    assert!(handle_mouse(&state, &mut tui, release).is_empty());
    assert!(!tui.is_dragging_sidebar);
    let rows = draw_rows(&state, &mut tui);
    assert_eq!(rows[5].chars().nth(45), Some('│'));
    assert_eq!(workspace(&state).sidebar.width, 46);
}

#[test]
fn the_wheel_scrolls_the_structure_tab() {
    let mut state = table_state();
    let tab_id = active_tab_id(&state);
    send(&mut state, [Msg::Table(TableMsg::ToggleTab)]);
    let TabContent::Table(table) = &workspace(&state).active_tab().expect("tab").content else {
        panic!("expected a table tab");
    };
    assert_eq!(table.tab, TableTab::Structure);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let columns_title = position_of(&rows, "Columns", MAIN_COLUMN);
    assert!(wheel(&state, &mut tui, columns_title, true).is_empty());
    assert_eq!(tui.structure_scrolls.get(&tab_id), Some(&3));
    wheel(&state, &mut tui, columns_title, false);
    assert_eq!(tui.structure_scrolls.get(&tab_id), Some(&0));
    assert_eq!(
        click(&state, &mut tui, columns_title),
        vec![Msg::FocusPane(PaneDirection::Right)]
    );
}

#[test]
fn a_press_dismisses_the_notice_but_the_wheel_and_other_buttons_do_not() {
    let mut state = workspace_state();
    state.notice = Some(Notice {
        level: NoticeLevel::Info,
        text: "Copied".to_owned(),
    });
    let mut tui = TuiState::new(false);
    draw_rows(&state, &mut tui);
    let header_row = (60, 0);
    assert_eq!(
        click(&state, &mut tui, header_row),
        vec![Msg::DismissNotice]
    );
    let right_press = mouse_event(MouseEventKind::Down(MouseButton::Right), header_row);
    assert_eq!(
        handle_mouse(&state, &mut tui, right_press),
        vec![Msg::DismissNotice]
    );
    assert!(wheel(&state, &mut tui, header_row, true).is_empty());
    state.notice = None;
    let right_on_sidebar = mouse_event(MouseEventKind::Down(MouseButton::Right), (4, 3));
    assert!(handle_mouse(&state, &mut tui, right_on_sidebar).is_empty());
    let moved = mouse_event(MouseEventKind::Moved, (4, 3));
    assert!(handle_mouse(&state, &mut tui, moved).is_empty());
}

#[test]
fn a_collapsed_sidebar_has_no_targets() {
    let mut state = table_state();
    send(&mut state, [Msg::Sidebar(SidebarMsg::ToggleCollapsed)]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    assert!(tui.mouse_targets.iter().all(|(_, target)| !matches!(
        target,
        MouseTarget::SidebarItem(_) | MouseTarget::SidebarList | MouseTarget::SidebarBorder
    )));
    let ada = position_of(&rows, "Ada", 0);
    assert!(ada.0 < MAIN_COLUMN);
    assert_eq!(
        click(&state, &mut tui, ada),
        vec![Msg::Table(TableMsg::SelectCell(cell(0, 1)))]
    );
}

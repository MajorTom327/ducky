use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Loadable, Modal, ModalMsg, Msg, Screen, SidebarMsg, TabsMsg, Workspace,
    update,
};
use ducky_core::{
    Catalog, PgTools, Profile, ProfileKind, Relation, RelationKind, Schema, SessionInfo,
    StoragePaths, ToolStatus,
};
use ducky_tui::keys::handle_key;
use ducky_tui::keys::help::HELP_ROWS;
use ducky_tui::theme::palette;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use edtui::{EditorState, Lines};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use tui_input::InputRequest;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;
const TAB_BAR_ROW: u16 = 1;

fn relation(name: &str, kind: RelationKind, estimated_row_count: Option<i64>) -> Relation {
    Relation {
        name: name.to_owned(),
        kind,
        estimated_row_count,
        columns: Vec::new(),
    }
}

fn workspace_state(tools: PgTools) -> AppState {
    let profile = Profile {
        label: "Dev local".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
    };
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    };
    let mut state = AppState::new(vec![profile.clone()], storage);
    let mut workspace = Workspace::new(
        0,
        profile,
        Connection {
            session: SessionInfo {
                database: "ducky".to_owned(),
                user: "ducky".to_owned(),
                server_version: "18.1".to_owned(),
            },
            tools,
        },
    );
    workspace.catalog = Loadable::Ready(Catalog {
        schemas: vec![
            Schema {
                name: "analytics".to_owned(),
                relations: vec![relation("events", RelationKind::Table, Some(20))],
            },
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    relation("customers", RelationKind::Table, Some(250)),
                    relation("order_totals", RelationKind::View, None),
                ],
            },
        ],
    });
    workspace.schema_name = "public".to_owned();
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn available_tools() -> PgTools {
    PgTools {
        pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
        pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
    }
}

fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

fn workspace(state: &AppState) -> &Workspace {
    state.workspace().expect("workspace")
}

fn send(state: &mut AppState, messages: impl IntoIterator<Item = Msg>) {
    for message in messages {
        update(state, message);
    }
}

fn render(state: &AppState, tui: &mut TuiState, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
        .expect("draw");
    terminal
}

fn rows(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

fn column_of(row: &str, needle: &str) -> u16 {
    let byte_offset = row.find(needle).expect(needle);
    u16::try_from(row[..byte_offset].chars().count()).expect("column")
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

fn state_with_three_tabs() -> AppState {
    let mut state = workspace_state(available_tools());
    send(
        &mut state,
        [
            Msg::Sidebar(SidebarMsg::Activate),
            Msg::Tabs(TabsMsg::NewQuery),
            Msg::Tabs(TabsMsg::OpenFiles),
        ],
    );
    state
}

#[test]
fn tab_bar_lists_titles_and_highlights_the_active_tab() {
    let state = state_with_three_tabs();
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
    let tab_row = &rows(&terminal)[usize::from(TAB_BAR_ROW)];
    for expected in [" 1 customers ", " 2 Query 1 ", " 3 SQL files "] {
        assert!(
            tab_row.contains(expected),
            "missing {expected} in {tab_row}"
        );
    }
    let accent = palette(Some(ProfileKind::Local)).accent;
    let buffer = terminal.backend().buffer();
    let active_x = column_of(tab_row, "3 SQL files");
    assert_eq!(buffer[(active_x, TAB_BAR_ROW)].bg, accent);
    let inactive_x = column_of(tab_row, "1 customers");
    assert_eq!(buffer[(inactive_x, TAB_BAR_ROW)].fg, Color::DarkGray);
    assert_ne!(buffer[(inactive_x, TAB_BAR_ROW)].bg, accent);
}

#[test]
fn tab_bar_marks_running_and_modified_query_tabs() {
    let mut state = state_with_three_tabs();
    let query_tab_id = workspace(&state).tabs[1].id;
    workspace_mut(&mut state).running_script_tab_id = Some(query_tab_id);
    let mut tui = TuiState::new(false);
    let tab_row = rows(&render(&state, &mut tui, WIDTH, HEIGHT))[usize::from(TAB_BAR_ROW)].clone();
    assert!(tab_row.contains(" 2 Query 1 ● "), "{tab_row}");
    workspace_mut(&mut state).running_script_tab_id = None;
    tui.editor(query_tab_id).state = EditorState::new(Lines::from("select 1"));
    let tab_row = rows(&render(&state, &mut tui, WIDTH, HEIGHT))[usize::from(TAB_BAR_ROW)].clone();
    assert!(tab_row.contains(" 2 Query 1 • "), "{tab_row}");
    assert!(!tab_row.contains('●'));
}

#[test]
fn tab_bar_scrolls_to_the_active_tab_with_overflow_markers() {
    let mut state = workspace_state(available_tools());
    send(&mut state, (0..12).map(|_| Msg::Tabs(TabsMsg::NewQuery)));
    let mut tui = TuiState::new(false);
    let tab_row = rows(&render(&state, &mut tui, 80, 24))[usize::from(TAB_BAR_ROW)].clone();
    assert!(tab_row.contains(" 12 Query 12 "), "{tab_row}");
    assert!(tab_row.contains('‹'));
    assert!(!tab_row.contains('›'));
    send(&mut state, [Msg::Tabs(TabsMsg::Select(0))]);
    let tab_row = rows(&render(&state, &mut tui, 80, 24))[usize::from(TAB_BAR_ROW)].clone();
    assert!(tab_row.contains(" 1 Query 1 "), "{tab_row}");
    assert!(tab_row.contains('›'));
    assert!(!tab_row.contains('‹'));
}

#[test]
fn tab_bar_without_tabs_shows_the_shortcut_hint() {
    let state = workspace_state(available_tools());
    let mut tui = TuiState::new(false);
    let screen = rows(&render(&state, &mut tui, WIDTH, HEIGHT));
    assert!(screen[usize::from(TAB_BAR_ROW)].contains("Ctrl+n new query · Ctrl+p go to"));
    let text = screen.join("\n");
    assert!(text.contains("go to anything"));
    assert!(text.contains("all key bindings"));
}

#[test]
fn status_bar_styles_keys_of_the_hints() {
    let state = workspace_state(available_tools());
    let mut tui = TuiState::new(false);
    let terminal = render(&state, &mut tui, WIDTH, HEIGHT);
    let status_row = rows(&terminal)[usize::from(HEIGHT - 1)].clone();
    assert!(
        status_row.contains("Ctrl+p go to · Enter open"),
        "{status_row}"
    );
    assert!(status_row.trim_end().ends_with("Sidebar"));
    let key_x = column_of(&status_row, "Ctrl+p");
    let accent = palette(Some(ProfileKind::Local)).accent;
    assert_eq!(terminal.backend().buffer()[(key_x, HEIGHT - 1)].fg, accent);
}

#[test]
fn palette_lists_actions_and_cross_schema_relations() {
    let tools = PgTools {
        pg_dump: ToolStatus::Missing("`pg_dump` was not found in PATH".to_owned()),
        pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
    };
    let mut state = workspace_state(tools);
    send(&mut state, [Msg::OpenPalette]);
    let mut tui = TuiState::new(false);
    let text = rows(&render(&state, &mut tui, WIDTH, HEIGHT)).join("\n");
    for expected in [
        "New SQL query",
        "Ctrl+n",
        "unavailable",
        "customers",
        "table · ~250 rows",
        "analytics.events",
        "Enter open · ↑↓ move · Esc close",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
}

#[test]
fn palette_filters_items_by_the_typed_query() {
    let mut state = workspace_state(available_tools());
    send(&mut state, [Msg::OpenPalette]);
    send(
        &mut state,
        "evts"
            .chars()
            .map(|character| Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(character)))),
    );
    let mut tui = TuiState::new(false);
    let screen_rows = rows(&render(&state, &mut tui, WIDTH, HEIGHT));
    let input_row = screen_rows
        .iter()
        .find(|row| row.contains("› evts"))
        .expect("the palette input is rendered");
    let popup_start = input_row
        .chars()
        .position(|character| character == '›')
        .expect("input marker");
    let popup_text: String = screen_rows
        .iter()
        .map(|row| row.chars().skip(popup_start).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(popup_text.contains("analytics.events"));
    assert!(!popup_text.contains("New SQL query"));
    assert!(!popup_text.contains("order_totals"));
}

#[test]
fn palette_keys_edit_move_submit_and_cancel() {
    let mut state = workspace_state(available_tools());
    send(&mut state, [Msg::OpenPalette]);
    let mut tui = TuiState::new(false);
    let input = |request| vec![Msg::Modal(ModalMsg::Input(request))];
    let modal = |message| vec![Msg::Modal(message)];
    let cases = [
        (
            key(KeyCode::Char('c')),
            input(InputRequest::InsertChar('c')),
        ),
        (
            key(KeyCode::Char('j')),
            input(InputRequest::InsertChar('j')),
        ),
        (key(KeyCode::Backspace), input(InputRequest::DeletePrevChar)),
        (ctrl('w'), input(InputRequest::DeletePrevWord)),
        (ctrl('u'), input(InputRequest::DeleteLine)),
        (key(KeyCode::Down), modal(ModalMsg::Next)),
        (key(KeyCode::Tab), modal(ModalMsg::Next)),
        (ctrl('n'), modal(ModalMsg::Next)),
        (key(KeyCode::Up), modal(ModalMsg::Previous)),
        (ctrl('p'), modal(ModalMsg::Previous)),
        (
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
            modal(ModalMsg::Previous),
        ),
        (key(KeyCode::Enter), modal(ModalMsg::Submit)),
        (key(KeyCode::Esc), modal(ModalMsg::Cancel)),
    ];
    for (pressed_key, expected) in cases {
        assert_eq!(
            handle_key(&state, &mut tui, pressed_key),
            expected,
            "{pressed_key:?}"
        );
    }
}

#[test]
fn help_shows_every_description_untruncated_while_scrolling() {
    for (width, height) in [(WIDTH, HEIGHT), (160, 40)] {
        let mut state = workspace_state(available_tools());
        state.modal = Some(Modal::Help);
        let mut tui = TuiState::new(false);
        let mut seen_text = String::new();
        for requested_scroll in 0..300 {
            tui.help_scroll = requested_scroll;
            seen_text.push_str(&rows(&render(&state, &mut tui, width, height)).join("\n"));
            if tui.help_scroll < requested_scroll {
                break;
            }
        }
        for (keys, description) in HELP_ROWS {
            assert!(seen_text.contains(description), "missing {description}");
            if !keys.is_empty() {
                assert!(seen_text.contains(keys), "missing {keys}");
            }
        }
        assert!(seen_text.contains("one transaction / autocommit"));
    }
}

#[test]
fn help_scroll_keys_move_reset_and_close() {
    let mut state = workspace_state(available_tools());
    state.modal = Some(Modal::Help);
    let mut tui = TuiState::new(false);
    assert!(handle_key(&state, &mut tui, key(KeyCode::Char('j'))).is_empty());
    assert!(handle_key(&state, &mut tui, key(KeyCode::Down)).is_empty());
    assert_eq!(tui.help_scroll, 2);
    handle_key(&state, &mut tui, key(KeyCode::Char('k')));
    assert_eq!(tui.help_scroll, 1);
    handle_key(&state, &mut tui, ctrl('d'));
    assert_eq!(tui.help_scroll, 11);
    handle_key(&state, &mut tui, ctrl('u'));
    assert_eq!(tui.help_scroll, 1);
    handle_key(&state, &mut tui, key(KeyCode::Char('g')));
    assert_eq!(tui.help_scroll, 0);
    handle_key(&state, &mut tui, key(KeyCode::Up));
    assert_eq!(tui.help_scroll, 0);
    tui.help_scroll = 5;
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
    assert_eq!(tui.help_scroll, 0);
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('?'))),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
}

fn press_all(state: &AppState, tui: &mut TuiState, characters: &str) -> Vec<Msg> {
    characters
        .chars()
        .flat_map(|character| handle_key(state, tui, key(KeyCode::Char(character))))
        .collect()
}

#[test]
fn bracket_b_sequences_switch_tabs_and_space_b_d_closes() {
    let state = state_with_three_tabs();
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, "]b"),
        vec![Msg::Tabs(TabsMsg::Next)]
    );
    assert_eq!(
        press_all(&state, &mut tui, "[b"),
        vec![Msg::Tabs(TabsMsg::Previous)]
    );
    assert_eq!(
        press_all(&state, &mut tui, " bd"),
        vec![Msg::Tabs(TabsMsg::Close)]
    );
    assert!(press_all(&state, &mut tui, "[x").is_empty());
    assert_eq!(tui.pending_prefix, None);
}

#[test]
fn a_pending_sequence_is_shown_in_the_status_bar() {
    let state = state_with_three_tabs();
    let mut tui = TuiState::new(false);
    press_all(&state, &mut tui, " b");
    let status_row = rows(&render(&state, &mut tui, WIDTH, HEIGHT))
        .last()
        .cloned()
        .unwrap_or_default();
    assert!(status_row.contains("d close tab"), "{status_row}");
}

#[test]
fn sequences_work_in_editor_normal_mode_but_not_in_insert_mode() {
    let mut state = state_with_three_tabs();
    send(&mut state, [Msg::Tabs(TabsMsg::Select(1))]);
    let mut tui = TuiState::new(false);
    assert_eq!(
        press_all(&state, &mut tui, "]b"),
        vec![Msg::Tabs(TabsMsg::Next)]
    );
    assert!(press_all(&state, &mut tui, "i]b").is_empty());
    let query_tab_id = workspace(&state).tabs[1].id;
    assert_eq!(tui.editor(query_tab_id).text(), "]b");
}

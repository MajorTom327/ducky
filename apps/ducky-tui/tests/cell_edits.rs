use std::collections::BTreeMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, CellEditor, CellInput, Command, Confirm, ConfirmKind, Connection, GridCursor,
    Loadable, Modal, ModalMsg, Msg, NoticeLevel, Screen, SidebarMsg, TabContent, TableMsg,
    TableView, Workspace, update,
};
use ducky_core::{
    Catalog, Cell, Column, Page, PgTools, Profile, ProfileKind, Relation, RelationKind, ResultSet,
    Schema, SessionInfo, StoragePaths, ToolStatus,
};
use ducky_tui::keys::hints::hints;
use ducky_tui::keys::{handle_key, modal};
use ducky_tui::theme::palette;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::{cell_editor, draw};
use ducky_tui::widgets::grid::{GridOptions, GridViewport, render_grid};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier};
use tui_input::{Input, InputRequest};

const NAME: usize = 1;
const STATUS: usize = 2;

fn cell(value: &str) -> Cell {
    Some(value.to_owned())
}

fn profile(kind: ProfileKind) -> Profile {
    Profile {
        label: "Dev".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind,
        project: None,
        tags: Vec::new(),
    }
}

fn column(name: &str, data_type: &str, enum_labels: &[&str]) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        is_nullable: name != "id",
        default_expression: None,
        is_primary_key: name == "id",
        enum_labels: enum_labels
            .iter()
            .map(|label| (*label).to_owned())
            .collect(),
    }
}

fn edit_state(kind: ProfileKind) -> AppState {
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    };
    let mut state = AppState::new(vec![profile(kind)], storage);
    let mut workspace = Workspace::new(
        0,
        profile(kind),
        Connection {
            session: SessionInfo {
                database: "ducky".to_owned(),
                user: "ducky".to_owned(),
                server_version: "18.1".to_owned(),
            },
            tools: PgTools {
                pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
                pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
            },
        },
    );
    workspace.catalog = Loadable::Ready(Catalog {
        schemas: vec![Schema {
            name: "public".to_owned(),
            relations: vec![Relation {
                name: "customers".to_owned(),
                kind: RelationKind::Table,
                estimated_row_count: Some(2),
                columns: vec![
                    column("id", "integer", &[]),
                    column("name", "text", &[]),
                    column("status", "order_status", &["new", "paid"]),
                ],
                foreign_keys: Vec::new(),
            }],
        }],
    });
    workspace.schema_name = "public".to_owned();
    state.screen = Screen::Workspace(Box::new(workspace));
    let commands = update(&mut state, Msg::Sidebar(SidebarMsg::Activate));
    let Some(Command::LoadPage(request)) = commands.into_iter().next() else {
        panic!("expected LoadPage");
    };
    update(
        &mut state,
        Msg::PageLoaded {
            request,
            result: Ok(Page {
                result_set: ResultSet {
                    columns: vec!["id".to_owned(), "name".to_owned(), "status".to_owned()],
                    rows: vec![
                        vec![cell("1"), cell("Ada"), cell("new")],
                        vec![cell("2"), None, cell("paid")],
                    ],
                    is_truncated: false,
                },
                has_next_page: false,
            }),
        },
    );
    state
}

fn active_table(state: &AppState) -> &TableView {
    match &state
        .workspace()
        .expect("workspace")
        .active_tab()
        .expect("tab")
        .content
    {
        TabContent::Table(table) => table,
        _ => panic!("expected a table tab"),
    }
}

fn active_table_mut(state: &mut AppState) -> &mut TableView {
    let Screen::Workspace(workspace) = &mut state.screen else {
        panic!("expected a workspace");
    };
    match &mut workspace.tabs[0].content {
        TabContent::Table(table) => table,
        _ => panic!("expected a table tab"),
    }
}

fn press(state: &mut AppState, tui: &mut TuiState, key: KeyEvent) {
    for msg in handle_key(state, tui, key) {
        update(state, msg);
    }
}

fn press_char(state: &mut AppState, tui: &mut TuiState, character: char) {
    press(
        state,
        tui,
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
    );
}

fn draw_screen(state: &AppState, tui: &mut TuiState) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(100, 20)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, tui))
        .expect("draw");
    terminal
}

fn buffer_text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| row_text(buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol().to_owned())
        .collect()
}

fn find_text(buffer: &Buffer, text: &str) -> Position {
    (0..buffer.area.height)
        .find_map(|y| {
            let line = row_text(buffer, y);
            line.find(text).map(|byte_index| {
                let x = line[..byte_index].chars().count();
                Position::new(u16::try_from(x).expect("fits"), y)
            })
        })
        .unwrap_or_else(|| panic!("missing {text:?} in\n{}", buffer_text(buffer)))
}

fn move_cursor(state: &mut AppState, row_index: usize, column_index: usize) {
    active_table_mut(state).cursor = GridCursor {
        row_index,
        column_index,
    };
}

#[test]
fn grid_shows_drafted_values_in_bold_yellow_and_returns_the_cursor_cell() {
    let result_set = ResultSet {
        columns: vec!["id".to_owned(), "name".to_owned()],
        rows: vec![vec![cell("1"), cell("Ada")], vec![cell("2"), cell("Bea")]],
        is_truncated: false,
    };
    let draft_cells = BTreeMap::from([
        (
            GridCursor {
                row_index: 0,
                column_index: NAME,
            },
            cell("Margaret"),
        ),
        (
            GridCursor {
                row_index: 1,
                column_index: NAME,
            },
            None,
        ),
    ]);
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).expect("terminal");
    let grid_palette = palette(Some(ProfileKind::Local));
    let mut cursor_cell_area = None;
    terminal
        .draw(|frame| {
            cursor_cell_area = render_grid(
                frame,
                frame.area(),
                &result_set,
                &mut GridViewport::default(),
                GridOptions {
                    cursor: GridCursor {
                        row_index: 1,
                        column_index: 0,
                    },
                    sort: None,
                    first_row_number: 1,
                    is_focused: true,
                    linked_columns: &[],
                    selected_row_indexes: &[],
                    draft_cells: &draft_cells,
                },
                &grid_palette,
            );
        })
        .expect("draw");
    let buffer = terminal.backend().buffer();
    let drafted_position = find_text(buffer, "Margaret");
    let drafted_style = buffer[(drafted_position.x, drafted_position.y)].style();
    assert_eq!(drafted_style.fg, Some(Color::Yellow));
    assert!(drafted_style.add_modifier.contains(Modifier::BOLD));
    let null_position = find_text(buffer, "NULL");
    assert_eq!(null_position.y, 3);
    assert_eq!(
        buffer[(null_position.x, null_position.y)].style().fg,
        Some(Color::Yellow)
    );
    assert!(!buffer_text(buffer).contains("Bea"));
    let cursor_cell_area = cursor_cell_area.expect("cursor cell shown");
    assert_eq!((cursor_cell_area.y, cursor_cell_area.height), (3, 1));
    let cursor_cell_text: String = (cursor_cell_area.x..cursor_cell_area.right())
        .map(|x| buffer[(x, cursor_cell_area.y)].symbol().to_owned())
        .collect();
    assert_eq!(cursor_cell_text, " 2");
}

#[test]
fn grid_without_rows_has_no_cursor_cell() {
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).expect("terminal");
    let grid_palette = palette(None);
    let mut cursor_cell_area = Some(Rect::default());
    terminal
        .draw(|frame| {
            cursor_cell_area = render_grid(
                frame,
                frame.area(),
                &ResultSet {
                    columns: vec!["id".to_owned()],
                    rows: Vec::new(),
                    is_truncated: false,
                },
                &mut GridViewport::default(),
                GridOptions {
                    cursor: GridCursor::default(),
                    sort: None,
                    first_row_number: 1,
                    is_focused: true,
                    linked_columns: &[],
                    selected_row_indexes: &[],
                    draft_cells: &BTreeMap::new(),
                },
                &grid_palette,
            );
        })
        .expect("draw");
    assert_eq!(cursor_cell_area, None);
}

#[test]
fn text_editor_renders_over_the_cursor_cell_with_the_terminal_cursor() {
    let mut state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    move_cursor(&mut state, 0, NAME);
    draw_screen(&state, &mut tui);
    let cell_area = tui.table_cursor_cells[&1];
    press_char(&mut state, &mut tui, 'c');
    assert!(matches!(state.modal, Some(Modal::CellEditor(_))));
    let mut terminal = draw_screen(&state, &mut tui);
    let buffer = terminal.backend().buffer().clone();
    let editor_text: String = (cell_area.x..cell_area.x + 24)
        .map(|x| buffer[(x, cell_area.y)].symbol().to_owned())
        .collect();
    assert_eq!(editor_text, format!("{:<24}", "Ada"));
    let accent = palette(Some(ProfileKind::Local)).accent;
    assert_eq!(
        buffer[(cell_area.x + 20, cell_area.y)].style().bg,
        Some(accent)
    );
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(cell_area.x + 3, cell_area.y)
    );
    assert_eq!(hints(&state)[0], ("Enter", "set"));
    press_char(&mut state, &mut tui, '!');
    press(
        &mut state,
        &mut tui,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert!(state.modal.is_none());
    let screen = buffer_text(draw_screen(&state, &mut tui).backend().buffer());
    assert!(screen.contains("Ada!"), "{screen}");
    assert!(screen.contains("1 unsaved"), "{screen}");
    assert_eq!(hints(&state)[0], ("Ctrl+s", "save"));
    assert_eq!(hints(&state)[1], ("U", "discard"));
}

#[test]
fn dropdown_lists_labels_and_null_under_the_cell() {
    let mut state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    move_cursor(&mut state, 0, STATUS);
    draw_screen(&state, &mut tui);
    let cell_area = tui.table_cursor_cells[&1];
    press_char(&mut state, &mut tui, 'c');
    assert_eq!(hints(&state)[0], ("j/k", "move"));
    let terminal = draw_screen(&state, &mut tui);
    let buffer = terminal.backend().buffer();
    let title_position = find_text(buffer, "┌ status ");
    assert_eq!(title_position.y, cell_area.y + 1);
    let selected_position = find_text(buffer, "│ new ");
    assert_eq!(selected_position.y, cell_area.y + 2);
    let accent = palette(Some(ProfileKind::Local)).accent;
    assert_eq!(
        buffer[(selected_position.x + 2, selected_position.y)]
            .style()
            .bg,
        Some(accent)
    );
    let null_position = find_text(buffer, "│ NULL ");
    assert_eq!(null_position.y, cell_area.y + 4);
    assert_eq!(
        buffer[(null_position.x + 2, null_position.y)].style().fg,
        Some(Color::DarkGray)
    );
    press_char(&mut state, &mut tui, 'j');
    press(
        &mut state,
        &mut tui,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_eq!(
        active_table(&state).cell_drafts,
        BTreeMap::from([(
            GridCursor {
                row_index: 0,
                column_index: STATUS,
            },
            cell("paid")
        )])
    );
}

#[test]
fn editor_falls_back_to_the_screen_center_and_opens_above_near_the_bottom() {
    let editor_palette = palette(None);
    let screen = Rect::new(0, 0, 60, 20);
    let text_editor = CellEditor {
        column_name: "name".to_owned(),
        column_index: NAME,
        row_indexes: vec![0],
        input: CellInput::Text(Input::new("Ada".to_owned())),
    };
    let choice_editor = CellEditor {
        column_name: "status".to_owned(),
        column_index: STATUS,
        row_indexes: vec![0],
        input: CellInput::Choice {
            options: (0..10)
                .map(|index| cell(&format!("label{index}")))
                .collect(),
            selected_option_index: 9,
        },
    };
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).expect("terminal");
    let mut areas = Vec::new();
    terminal
        .draw(|frame| {
            areas.push(cell_editor::render(
                frame,
                screen,
                &text_editor,
                None,
                &editor_palette,
            ));
            areas.push(cell_editor::render(
                frame,
                screen,
                &choice_editor,
                None,
                &editor_palette,
            ));
            areas.push(cell_editor::render(
                frame,
                screen,
                &choice_editor,
                Some(Rect::new(55, 18, 5, 1)),
                &editor_palette,
            ));
            areas.push(cell_editor::render(
                frame,
                screen,
                &text_editor,
                Some(Rect::new(50, 5, 5, 1)),
                &editor_palette,
            ));
        })
        .expect("draw");
    assert_eq!(areas[0], Rect::new(18, 10, 24, 1));
    assert_eq!(areas[1].height, 10);
    assert_eq!(areas[1].y, 5);
    assert_eq!(areas[2].y + areas[2].height, 18);
    assert_eq!(areas[2].x + areas[2].width, 60);
    assert_eq!(areas[3], Rect::new(36, 5, 24, 1));
    let buffer = terminal.backend().buffer();
    assert!(buffer_text(buffer).contains(" label9 "));
    assert!(!buffer_text(buffer).contains(" label0 "));
}

#[test]
fn summary_shows_saving_while_the_save_runs() {
    let mut state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    move_cursor(&mut state, 0, NAME);
    press_char(&mut state, &mut tui, 'c');
    press_char(&mut state, &mut tui, 'x');
    press(
        &mut state,
        &mut tui,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let commands = update(&mut state, Msg::Table(TableMsg::SaveDrafts));
    assert_eq!(commands.len(), 1);
    let screen = buffer_text(draw_screen(&state, &mut tui).backend().buffer());
    assert!(screen.contains("saving…"), "{screen}");
    assert!(!screen.contains("unsaved"), "{screen}");
}

#[test]
fn save_confirmation_previews_the_sql_without_a_mode_line() {
    let mut state = edit_state(ProfileKind::Prod);
    let mut tui = TuiState::new(false);
    state.modal = Some(Modal::Confirm(Confirm {
        kind: ConfirmKind::SaveCellEdits {
            tab_id: 1,
            sql: "UPDATE \"public\".\"customers\" SET \"name\" = 'x' WHERE \"id\" = '1';"
                .to_owned(),
        },
        message: "Save 1 change(s) in 1 row(s) on PROD?".to_owned(),
    }));
    let terminal = draw_screen(&state, &mut tui);
    let buffer = terminal.backend().buffer();
    let screen = buffer_text(buffer);
    assert!(screen.contains("SET \"name\" = 'x'"), "{screen}");
    assert!(!screen.contains("Mode:"), "{screen}");
    let title_position = find_text(buffer, " Confirm ");
    assert_eq!(
        buffer[(title_position.x + 1, title_position.y)].style().fg,
        Some(Color::Red)
    );
}

#[test]
fn table_keys_map_to_cell_edit_messages() {
    let state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Char('c'), KeyModifiers::NONE, TableMsg::ChangeCell),
        (
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            TableMsg::SetCellNull,
        ),
        (KeyCode::Char('u'), KeyModifiers::NONE, TableMsg::RevertCell),
        (
            KeyCode::Char('U'),
            KeyModifiers::SHIFT,
            TableMsg::DiscardDrafts,
        ),
        (
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
            TableMsg::SaveDrafts,
        ),
    ];
    for (code, modifiers, expected) in cases {
        assert_eq!(
            handle_key(&state, &mut tui, KeyEvent::new(code, modifiers)),
            vec![Msg::Table(expected)]
        );
    }
}

#[test]
fn cell_editor_modal_keys_follow_the_input_kind() {
    let mut tui = TuiState::new(false);
    let text_editor = Modal::CellEditor(CellEditor {
        column_name: "name".to_owned(),
        column_index: NAME,
        row_indexes: vec![0],
        input: CellInput::Text(Input::default()),
    });
    let choice_editor = Modal::CellEditor(CellEditor {
        column_name: "status".to_owned(),
        column_index: STATUS,
        row_indexes: vec![0],
        input: CellInput::Choice {
            options: vec![cell("new"), None],
            selected_option_index: 0,
        },
    });
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    let text_cases = [
        (key(KeyCode::Enter), ModalMsg::Submit),
        (key(KeyCode::Esc), ModalMsg::Cancel),
        (
            key(KeyCode::Char('j')),
            ModalMsg::Input(InputRequest::InsertChar('j')),
        ),
        (key(KeyCode::Home), ModalMsg::Input(InputRequest::GoToStart)),
        (
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
            ModalMsg::Input(InputRequest::DeletePrevWord),
        ),
    ];
    for (pressed_key, expected) in text_cases {
        assert_eq!(
            modal::handle(&text_editor, &mut tui, pressed_key),
            vec![Msg::Modal(expected)]
        );
    }
    let choice_cases = [
        (key(KeyCode::Char('j')), ModalMsg::Next),
        (key(KeyCode::Up), ModalMsg::Previous),
        (key(KeyCode::Enter), ModalMsg::Submit),
        (key(KeyCode::Esc), ModalMsg::Cancel),
    ];
    for (pressed_key, expected) in choice_cases {
        assert_eq!(
            modal::handle(&choice_editor, &mut tui, pressed_key),
            vec![Msg::Modal(expected)]
        );
    }
}

#[test]
fn warning_notices_are_yellow() {
    let warning_style = palette(None).notice(NoticeLevel::Warning);
    assert_eq!(warning_style.fg, Some(Color::Yellow));
    let mut state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    move_cursor(&mut state, 0, NAME);
    press_char(&mut state, &mut tui, 'c');
    press_char(&mut state, &mut tui, 'x');
    press(
        &mut state,
        &mut tui,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    press_char(&mut state, &mut tui, 'n');
    let notice = state.notice.as_ref().expect("refusal notice");
    assert_eq!(notice.level, NoticeLevel::Warning);
    assert_eq!(
        notice.text,
        "Save (Ctrl+s) or discard (U) the 1 unsaved change(s) first"
    );
    assert_eq!(active_table(&state).cell_drafts.len(), 1);
}

#[test]
fn an_open_modal_fades_the_rest_of_the_screen() {
    let mut state = edit_state(ProfileKind::Local);
    let mut tui = TuiState::new(false);
    move_cursor(&mut state, 0, STATUS);
    let unfaded_terminal = draw_screen(&state, &mut tui);
    let unfaded_name = find_text(unfaded_terminal.backend().buffer(), "Ada");
    assert!(
        !unfaded_terminal.backend().buffer()[(unfaded_name.x, unfaded_name.y)]
            .modifier
            .contains(Modifier::DIM)
    );
    press_char(&mut state, &mut tui, 'c');
    let terminal = draw_screen(&state, &mut tui);
    let buffer = terminal.backend().buffer();
    let name_position = find_text(buffer, "Ada");
    assert!(
        buffer[(name_position.x, name_position.y)]
            .modifier
            .contains(Modifier::DIM)
    );
    let option_position = find_text(buffer, "│ paid ");
    assert!(
        !buffer[(option_position.x + 2, option_position.y)]
            .modifier
            .contains(Modifier::DIM)
    );
}

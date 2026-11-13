use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Loadable, Modal, ModalMsg, Msg, QueryMsg, Screen, TabsMsg, Workspace,
    update,
};
use ducky_core::{Catalog, PgTools, Profile, ProfileKind, SessionInfo, StoragePaths, ToolStatus};
use ducky_tui::keys::handle_key;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tui_input::InputRequest;

fn form_state() -> AppState {
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
            tools: PgTools {
                pg_dump: ToolStatus::Available(PathBuf::from("/usr/bin/pg_dump")),
                pg_restore: ToolStatus::Available(PathBuf::from("/usr/bin/pg_restore")),
            },
        },
    );
    workspace.catalog = Loadable::Ready(Catalog {
        schemas: Vec::new(),
    });
    state.screen = Screen::Workspace(Box::new(workspace));
    update(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    update(
        &mut state,
        Msg::Query(QueryMsg::Run {
            sql: "select * from orders where id = :id and created_at < $1".to_owned(),
        }),
    );
    assert!(matches!(state.modal, Some(Modal::Parameters(_))));
    state
}

fn type_value(state: &mut AppState, text: &str) {
    for character in text.chars() {
        update(
            state,
            Msg::Modal(ModalMsg::Input(InputRequest::InsertChar(character))),
        );
    }
}

fn screen_rows(state: &AppState) -> Vec<String> {
    let mut tui = TuiState::new(false);
    let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, &mut tui))
        .expect("draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

fn row_with<'a>(rows: &'a [String], needle: &str) -> &'a str {
    rows.iter()
        .find(|row| row.contains(needle))
        .unwrap_or_else(|| panic!("no row contains {needle}:\n{}", rows.join("\n")))
}

#[test]
fn form_renders_names_previews_and_footer() {
    let mut state = form_state();
    type_value(&mut state, "42");
    update(&mut state, Msg::Modal(ModalMsg::Next));
    type_value(&mut state, "now()");
    update(&mut state, Msg::Modal(ModalMsg::ToggleRaw));
    let rows = screen_rows(&state);
    row_with(&rows, "Query parameters");
    let text_row = row_with(&rows, ":id");
    assert!(text_row.contains("42"));
    assert!(text_row.contains("→ '42'"));
    assert!(!text_row.contains("raw"));
    let raw_row = row_with(&rows, "$1");
    assert!(raw_row.contains("→ now()"));
    assert!(raw_row.contains("raw"));
    row_with(
        &rows,
        "Tab next · Shift+Tab previous · Ctrl+r raw SQL · Enter run · Esc cancel",
    );
}

#[test]
fn form_keys_map_to_modal_messages() {
    let state = form_state();
    let mut tui = TuiState::new(false);
    let mut press = |code: KeyCode, modifiers: KeyModifiers| {
        handle_key(&state, &mut tui, KeyEvent::new(code, modifiers))
    };
    assert_eq!(
        press(KeyCode::Tab, KeyModifiers::NONE),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        press(KeyCode::Down, KeyModifiers::NONE),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        press(KeyCode::BackTab, KeyModifiers::SHIFT),
        vec![Msg::Modal(ModalMsg::Previous)]
    );
    assert_eq!(
        press(KeyCode::Char('r'), KeyModifiers::CONTROL),
        vec![Msg::Modal(ModalMsg::ToggleRaw)]
    );
    assert_eq!(
        press(KeyCode::Char('r'), KeyModifiers::NONE),
        vec![Msg::Modal(ModalMsg::Input(InputRequest::InsertChar('r')))]
    );
    assert_eq!(
        press(KeyCode::Enter, KeyModifiers::NONE),
        vec![Msg::Modal(ModalMsg::Submit)]
    );
    assert_eq!(
        press(KeyCode::Esc, KeyModifiers::NONE),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
}

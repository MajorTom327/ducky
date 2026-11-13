use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Confirm, ConfirmKind, Connection, Loadable, Modal, ModalMsg, Msg, QueryMsg, Screen,
    TabsMsg, Workspace, update,
};
use ducky_core::{Catalog, PgTools, Profile, ProfileKind, SessionInfo, StoragePaths, ToolStatus};
use ducky_tui::keys::handle_key;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;
const MESSAGE: &str = "Run 4 statement(s) on PROD · Prod db (prod.example.com:5432/app)?";
const SQL: &str = "select 1;\nselect 2;\nselect 3;\nselect 4;";

fn prod_state_with_confirm() -> AppState {
    let prod_profile = Profile {
        label: "Prod db".to_owned(),
        url: "postgres://user@prod.example.com:5432/app".to_owned(),
        kind: ProfileKind::Prod,
        project: None,
        tags: Vec::new(),
    };
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    };
    let mut state = AppState::new(vec![prod_profile.clone()], storage);
    let mut workspace = Workspace::new(
        0,
        prod_profile,
        Connection {
            session: SessionInfo {
                database: "app".to_owned(),
                user: "user".to_owned(),
                server_version: "18.1".to_owned(),
            },
            tools: PgTools {
                pg_dump: ToolStatus::Missing("missing".to_owned()),
                pg_restore: ToolStatus::Missing("missing".to_owned()),
            },
        },
    );
    workspace.catalog = Loadable::Ready(Catalog {
        schemas: Vec::new(),
    });
    state.screen = Screen::Workspace(Box::new(workspace));
    update(&mut state, Msg::Tabs(TabsMsg::NewQuery));
    update(&mut state, Msg::Query(QueryMsg::ToggleTransactionMode));
    assert!(
        update(
            &mut state,
            Msg::Query(QueryMsg::Run {
                sql: SQL.to_owned()
            })
        )
        .is_empty()
    );
    let Some(Modal::Confirm(Confirm {
        kind: ConfirmKind::RunScript { .. },
        message,
    })) = &state.modal
    else {
        panic!("expected a run confirmation");
    };
    assert_eq!(message, MESSAGE);
    state
}

fn render(state: &AppState) -> Terminal<TestBackend> {
    let mut tui = TuiState::new(false);
    if let Some(workspace) = state.workspace() {
        tui.sync_workspace(workspace);
    }
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, &mut tui))
        .expect("draw");
    terminal
}

fn screen_rows(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn confirm_popup_shows_message_sql_preview_and_mode() {
    let terminal = render(&prod_state_with_confirm());
    let screen = screen_rows(&terminal).join("\n");
    assert!(screen.contains(MESSAGE), "{screen}");
    for preview_line in ["select 1;", "select 2;", "select 3;", "…"] {
        assert!(screen.contains(preview_line), "{preview_line} in {screen}");
    }
    assert!(!screen.contains("select 4;"), "{screen}");
    assert!(screen.contains("Mode: autocommit"), "{screen}");
    assert!(
        screen.contains("y / Enter confirm · n / Esc cancel"),
        "{screen}"
    );
}

#[test]
fn confirm_popup_border_uses_the_prod_accent() {
    let terminal = render(&prod_state_with_confirm());
    let rows = screen_rows(&terminal);
    let (message_row_index, message_row) = rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.contains("Run 4 statement(s)"))
        .expect("message row");
    let message_char_index = message_row
        .split("Run 4 statement(s)")
        .next()
        .map_or(0, |before| before.chars().count());
    let border_x = u16::try_from(message_char_index - 1).expect("x");
    let border_y = u16::try_from(message_row_index).expect("y");
    let border_cell = &terminal.backend().buffer()[(border_x, border_y)];
    assert_eq!(border_cell.symbol(), "│");
    assert_eq!(border_cell.fg, Color::Red);
}

#[test]
fn confirm_keys_submit_and_cancel() {
    let state = prod_state_with_confirm();
    let mut tui = TuiState::new(false);
    for (code, expected_msg) in [
        (KeyCode::Char('y'), ModalMsg::Submit),
        (KeyCode::Enter, ModalMsg::Submit),
        (KeyCode::Char('n'), ModalMsg::Cancel),
        (KeyCode::Esc, ModalMsg::Cancel),
    ] {
        assert_eq!(
            handle_key(&state, &mut tui, KeyEvent::new(code, KeyModifiers::NONE)),
            vec![Msg::Modal(expected_msg)]
        );
    }
}

#[test]
fn confirm_popup_names_the_transaction_mode() {
    let mut state = prod_state_with_confirm();
    update(&mut state, Msg::Modal(ModalMsg::Cancel));
    update(&mut state, Msg::Query(QueryMsg::ToggleTransactionMode));
    update(
        &mut state,
        Msg::Query(QueryMsg::Run {
            sql: SQL.to_owned(),
        }),
    );
    let rows = screen_rows(&render(&state));
    assert!(rows.iter().any(|row| row.contains("Mode: one transaction")));
}

#[test]
fn overwrite_confirm_shows_the_message_without_sql_or_mode() {
    let mut state = prod_state_with_confirm();
    state.modal = Some(Modal::Confirm(Confirm {
        kind: ConfirmKind::OverwriteFile,
        message: "report.sql already exists. Overwrite?".to_owned(),
    }));
    let rows = screen_rows(&render(&state));
    assert!(
        rows.iter()
            .any(|row| row.contains("report.sql already exists. Overwrite?"))
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("y / Enter confirm · n / Esc cancel"))
    );
    assert!(!rows.iter().any(|row| row.contains("Mode:")));
    assert!(!rows.iter().any(|row| row.contains("select 1;")));
}

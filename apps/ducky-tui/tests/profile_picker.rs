use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Connection, Loadable, ModalMsg, Msg, ProfilesMsg, Screen, Workspace, update,
};
use ducky_core::{Catalog, PgTools, Profile, ProfileKind, SessionInfo, StoragePaths, ToolStatus};
use ducky_tui::keys::handle_key;
use ducky_tui::theme::palette;
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::draw;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tui_input::InputRequest;

const WIDTH: u16 = 120;
const HEIGHT: u16 = 36;

fn profile(label: &str, project: Option<&str>, kind: ProfileKind, url: &str) -> Profile {
    Profile {
        label: label.to_owned(),
        url: url.to_owned(),
        kind,
        project: project.map(str::to_owned),
        tags: Vec::new(),
    }
}

fn profiles() -> Vec<Profile> {
    let mut shop_prod = profile(
        "Shop prod",
        Some("shop"),
        ProfileKind::Prod,
        "postgres://app:${PW}@prod.example.com:5432/shop?sslmode=require",
    );
    shop_prod.tags = vec!["eu".to_owned(), "readonly".to_owned()];
    vec![
        profile(
            "Shop local",
            Some("shop"),
            ProfileKind::Local,
            "postgres://app@localhost:5440/shop",
        ),
        shop_prod,
        profile(
            "B local",
            Some("analytics"),
            ProfileKind::Local,
            "postgres://b@localhost/b",
        ),
        profile(
            "Scratch",
            None,
            ProfileKind::Local,
            "postgres://localhost/scratch",
        ),
    ]
}

fn storage() -> StoragePaths {
    StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    }
}

fn workspace_state() -> AppState {
    let mut state = AppState::new(profiles(), storage());
    let mut workspace = Workspace::new(
        1,
        profiles()[1].clone(),
        Connection {
            session: SessionInfo {
                database: "shop".to_owned(),
                user: "app".to_owned(),
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
    state
}

fn send(state: &mut AppState, messages: impl IntoIterator<Item = Msg>) {
    for message in messages {
        update(state, message);
    }
}

fn type_filter(state: &mut AppState, text: &str) {
    send(
        state,
        text.chars().map(|typed_char| {
            Msg::Profiles(ProfilesMsg::Filter(InputRequest::InsertChar(typed_char)))
        }),
    );
}

fn render(state: &AppState) -> Terminal<TestBackend> {
    let mut tui = TuiState::new(false);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");
    terminal
        .draw(|frame| draw(frame, state, &mut tui))
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

fn row_containing<'a>(screen_rows: &'a [String], text: &str) -> (u16, &'a String) {
    let (row_index, row) = screen_rows
        .iter()
        .enumerate()
        .find(|(_, row)| row.contains(text))
        .unwrap_or_else(|| panic!("no row contains {text}:\n{}", screen_rows.join("\n")));
    (u16::try_from(row_index).expect("row"), row)
}

fn column_of(row: &str, text: &str) -> u16 {
    let byte_offset = row.find(text).expect("text in row");
    u16::try_from(row[..byte_offset].chars().count()).expect("column")
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

fn profiles_msg(profiles_msg: ProfilesMsg) -> Vec<Msg> {
    vec![Msg::Profiles(profiles_msg)]
}

#[test]
fn start_screen_groups_profiles_with_badges_tags_targets_and_count() {
    let state = AppState::new(profiles(), storage());
    let terminal = render(&state);
    let screen_rows = rows(&terminal);
    let screen_text = screen_rows.join("\n");
    for expected_text in [
        "Connect",
        "type to filter",
        "shop 2",
        "analytics 1",
        "Other 1",
        "#eu #readonly",
        "prod.example.com:5432/shop",
        "localhost:5440/shop",
        "4 of 4",
    ] {
        assert!(
            screen_text.contains(expected_text),
            "{expected_text}\n{screen_text}"
        );
    }
    let (prod_row_index, prod_row) = row_containing(&screen_rows, "Shop prod");
    assert!(prod_row.contains("PROD    Shop prod"), "{prod_row}");
    assert!(!prod_row.contains('●'), "{prod_row}");
    let badge_column = column_of(prod_row, "PROD");
    assert_eq!(
        terminal.backend().buffer()[(badge_column, prod_row_index)].fg,
        palette(Some(ProfileKind::Prod)).accent
    );
    let (local_row_index, local_row) = row_containing(&screen_rows, "Shop local");
    assert_eq!(
        terminal.backend().buffer()[(column_of(local_row, "Shop local"), local_row_index)].bg,
        palette(Some(ProfileKind::Local)).accent
    );
    let shop_row = row_containing(&screen_rows, "shop 2").0;
    let analytics_row = row_containing(&screen_rows, "analytics 1").0;
    let other_row = row_containing(&screen_rows, "Other 1").0;
    assert!(shop_row < prod_row_index && prod_row_index < analytics_row);
    assert!(analytics_row < other_row);
}

#[test]
fn filtering_narrows_rows_and_count() {
    let mut state = AppState::new(profiles(), storage());
    type_filter(&mut state, "shop eu");
    let screen_text = rows(&render(&state)).join("\n");
    assert!(screen_text.contains("1 of 4"), "{screen_text}");
    assert!(screen_text.contains("Shop prod"), "{screen_text}");
    assert!(!screen_text.contains("Shop local"), "{screen_text}");
    assert!(!screen_text.contains("analytics"), "{screen_text}");
    type_filter(&mut state, "zzz");
    let screen_text = rows(&render(&state)).join("\n");
    assert!(screen_text.contains("No matching profile"), "{screen_text}");
    assert!(screen_text.contains("0 of 4"), "{screen_text}");
}

#[test]
fn switch_modal_marks_the_connected_profile() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenProfilePicker]);
    let screen_rows = rows(&render(&state));
    let (title_row_index, _) = row_containing(&screen_rows, "Switch profile");
    let popup_rows = &screen_rows[usize::from(title_row_index)..];
    let (_, prod_row) = row_containing(popup_rows, "Shop prod");
    assert!(prod_row.contains("● PROD"), "{prod_row}");
    let (_, local_row) = row_containing(popup_rows, "Shop local");
    assert!(!local_row.contains('●'), "{local_row}");
}

#[test]
fn header_shows_the_project_of_the_connected_profile() {
    let state = workspace_state();
    let header = rows(&render(&state)).remove(0);
    assert!(
        header.contains("ducky │ shop │ [PROD] Shop prod │ app@shop"),
        "{header}"
    );
}

#[test]
fn filter_mode_keys_edit_the_filter_and_move() {
    let state = AppState::new(profiles(), storage());
    let mut tui = TuiState::new(false);
    let mut press = |key_event| handle_key(&state, &mut tui, key_event);
    assert_eq!(
        press(key(KeyCode::Char('j'))),
        profiles_msg(ProfilesMsg::Filter(InputRequest::InsertChar('j')))
    );
    assert_eq!(
        press(key(KeyCode::Char('q'))),
        profiles_msg(ProfilesMsg::Filter(InputRequest::InsertChar('q')))
    );
    assert_eq!(
        press(key(KeyCode::Backspace)),
        profiles_msg(ProfilesMsg::Filter(InputRequest::DeletePrevChar))
    );
    assert_eq!(
        press(ctrl('w')),
        profiles_msg(ProfilesMsg::Filter(InputRequest::DeletePrevWord))
    );
    assert_eq!(
        press(ctrl('u')),
        profiles_msg(ProfilesMsg::Filter(InputRequest::DeleteLine))
    );
    for next_key in [key(KeyCode::Down), key(KeyCode::Tab), ctrl('j'), ctrl('n')] {
        assert_eq!(press(next_key), profiles_msg(ProfilesMsg::Next));
    }
    for previous_key in [
        key(KeyCode::Up),
        key(KeyCode::BackTab),
        ctrl('k'),
        ctrl('p'),
    ] {
        assert_eq!(press(previous_key), profiles_msg(ProfilesMsg::Previous));
    }
    assert_eq!(
        press(key(KeyCode::Enter)),
        profiles_msg(ProfilesMsg::Connect)
    );
    assert_eq!(
        press(key(KeyCode::Esc)),
        profiles_msg(ProfilesMsg::FocusList)
    );
    assert_eq!(press(ctrl('q')), vec![Msg::Quit]);
}

#[test]
fn navigate_mode_keys_on_the_start_screen() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, [Msg::Profiles(ProfilesMsg::FocusList)]);
    let mut tui = TuiState::new(false);
    let mut press = |key_event| handle_key(&state, &mut tui, key_event);
    assert_eq!(
        press(key(KeyCode::Char('j'))),
        profiles_msg(ProfilesMsg::Next)
    );
    assert_eq!(
        press(key(KeyCode::Char('k'))),
        profiles_msg(ProfilesMsg::Previous)
    );
    assert_eq!(
        press(key(KeyCode::Char('g'))),
        profiles_msg(ProfilesMsg::First)
    );
    assert_eq!(
        press(key(KeyCode::Char('G'))),
        profiles_msg(ProfilesMsg::Last)
    );
    assert_eq!(
        press(key(KeyCode::Char('/'))),
        profiles_msg(ProfilesMsg::FocusFilter)
    );
    assert_eq!(
        press(key(KeyCode::Char('i'))),
        profiles_msg(ProfilesMsg::FocusFilter)
    );
    assert_eq!(
        press(key(KeyCode::Enter)),
        profiles_msg(ProfilesMsg::Connect)
    );
    assert!(press(key(KeyCode::Esc)).is_empty());
    assert_eq!(press(key(KeyCode::Char('q'))), vec![Msg::Quit]);
    send(
        &mut state,
        [Msg::Profiles(ProfilesMsg::Filter(
            InputRequest::InsertChar('s'),
        ))],
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        profiles_msg(ProfilesMsg::ClearFilter)
    );
}

#[test]
fn navigate_mode_keys_in_the_switch_modal() {
    let mut state = workspace_state();
    send(
        &mut state,
        [
            Msg::OpenProfilePicker,
            Msg::Profiles(ProfilesMsg::FocusList),
        ],
    );
    let mut tui = TuiState::new(false);
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('j'))),
        profiles_msg(ProfilesMsg::Next)
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Esc)),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
    assert_eq!(
        handle_key(&state, &mut tui, key(KeyCode::Char('q'))),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
}

#[test]
fn space_p_opens_the_profile_picker() {
    let mut state = workspace_state();
    let mut tui = TuiState::new(false);
    assert!(handle_key(&state, &mut tui, key(KeyCode::Char(' '))).is_empty());
    let messages = handle_key(&state, &mut tui, key(KeyCode::Char('p')));
    assert_eq!(messages, vec![Msg::OpenProfilePicker]);
    send(&mut state, messages);
    let screen_text = rows(&render(&state)).join("\n");
    assert!(screen_text.contains("Switch profile"), "{screen_text}");
    assert!(screen_text.contains("type to filter"), "{screen_text}");
}

fn unlabelled_prod_state() -> AppState {
    let mut state = workspace_state();
    state.profiles[1].label = String::new();
    if let Screen::Workspace(workspace) = &mut state.screen {
        workspace.profile.label = String::new();
    }
    state
}

#[test]
fn header_keeps_the_badge_when_the_label_is_empty() {
    let header = rows(&render(&unlabelled_prod_state())).remove(0);
    assert!(
        header.contains("ducky │ shop │ [PROD] │ app@shop"),
        "{header}"
    );
}

#[test]
fn picker_row_without_label_shows_badge_and_target() {
    let mut profiles_without_label = profiles();
    profiles_without_label[1].label = String::new();
    let state = AppState::new(profiles_without_label, storage());
    let screen_rows = rows(&render(&state));
    let (_, prod_row) = row_containing(&screen_rows, "prod.example.com:5432/shop");
    assert!(prod_row.contains("PROD"), "{prod_row}");
    assert!(!prod_row.contains("Shop prod"), "{prod_row}");
}

#[test]
fn start_screen_shows_the_wrapped_connection_error_and_connecting_state() {
    let mut state = AppState::new(profiles(), storage());
    let error_text = format!("FATAL: password authentication failed {}", "x".repeat(150));
    if let Screen::Profiles(picker) = &mut state.screen {
        picker.connection_error = Some(format!("{error_text}\nHINT: check the password"));
        picker.connecting_profile_index = Some(0);
    }
    let screen_rows = rows(&render(&state));
    let (error_row, _) = row_containing(&screen_rows, "FATAL: password authentication failed");
    assert!(screen_rows[usize::from(error_row) + 1].contains("xxx"));
    row_containing(&screen_rows, "HINT: check the password");
    row_containing(&screen_rows, " · Connecting…");
}

#[test]
fn navigate_mode_does_not_place_the_cursor_in_the_filter() {
    let mut state = AppState::new(profiles(), storage());
    let mut terminal = render(&state);
    let filter_cursor = terminal.get_cursor_position().expect("cursor");
    send(&mut state, [Msg::Profiles(ProfilesMsg::FocusList)]);
    let mut terminal_after = render(&state);
    let navigate_cursor = terminal_after.get_cursor_position().expect("cursor");
    assert_ne!(filter_cursor, navigate_cursor);
}

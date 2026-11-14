mod mouse_support;

use ducky_app::{
    AppState, DEFAULT_SIDEBAR_WIDTH, FilesMsg, JobState, Loadable, Modal, ModalMsg, Msg,
    PaneDirection, ProfilePicker, ProfilesMsg, RestoreMsg, Screen, TabsMsg,
};
use ducky_core::FileEntry;
use ducky_tui::tui_state::TuiState;
use mouse_support::{
    click, draw_rows, position_of, profile, send, storage, table_state, wheel, workspace,
    workspace_mut, workspace_state,
};

const MAIN_COLUMN: u16 = DEFAULT_SIDEBAR_WIDTH;

fn entry(name: &str) -> FileEntry {
    FileEntry {
        name: name.to_owned(),
        size_bytes: 10,
        modified_at: None,
    }
}

fn files_state() -> AppState {
    let mut state = workspace_state();
    send(&mut state, [Msg::Tabs(TabsMsg::OpenFiles)]);
    let workspace = workspace_mut(&mut state);
    workspace.files.entries = Loadable::Ready(vec![entry("first.sql"), entry("second.sql")]);
    workspace.files.preview = Loadable::Ready("select 1".to_owned());
    state
}

#[test]
fn clicking_a_palette_item_runs_it() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenPalette]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let help_index = workspace(&state)
        .palette_items("")
        .iter()
        .position(|item| item.label == "Help")
        .expect("help item");
    let help_item = position_of(&rows, "Help", 0);
    let messages = click(&state, &mut tui, help_item);
    assert_eq!(
        messages,
        vec![
            Msg::Modal(ModalMsg::Select(help_index)),
            Msg::Modal(ModalMsg::Submit),
        ]
    );
    send(&mut state, messages);
    assert!(matches!(state.modal, Some(Modal::Help)));
}

#[test]
fn clicks_inside_a_modal_do_nothing_and_clicks_outside_close_it() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenPalette]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let title = position_of(&rows, "Go to", 0);
    assert!(click(&state, &mut tui, title).is_empty());
    assert_eq!(
        wheel(&state, &mut tui, title, true),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        wheel(&state, &mut tui, title, false),
        vec![Msg::Modal(ModalMsg::Previous)]
    );
    let sidebar_item = (4, 3);
    assert!(wheel(&state, &mut tui, sidebar_item, true).is_empty());
    let messages = click(&state, &mut tui, sidebar_item);
    assert_eq!(messages, vec![Msg::Modal(ModalMsg::Cancel)]);
    send(&mut state, messages);
    assert!(state.modal.is_none());
}

#[test]
fn the_wheel_scrolls_the_help_and_a_click_outside_closes_it() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenHelp]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let global_title = position_of(&rows, "Global", 0);
    assert!(wheel(&state, &mut tui, global_title, true).is_empty());
    assert_eq!(tui.help_scroll, 3);
    assert!(click(&state, &mut tui, global_title).is_empty());
    assert_eq!(
        click(&state, &mut tui, (0, 0)),
        vec![Msg::Modal(ModalMsg::Cancel)]
    );
    assert_eq!(tui.help_scroll, 0);
}

#[test]
fn clicking_a_schema_switches_to_it() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenSchemaPicker]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let analytics = position_of(&rows, "analytics", 0);
    let messages = click(&state, &mut tui, analytics);
    assert_eq!(
        messages,
        vec![
            Msg::Modal(ModalMsg::Select(0)),
            Msg::Modal(ModalMsg::Submit)
        ]
    );
    send(&mut state, messages);
    assert_eq!(workspace(&state).schema_name, "analytics");
}

#[test]
fn clicking_a_profile_in_the_switch_popup_selects_and_submits_it() {
    let mut state = workspace_state();
    send(&mut state, [Msg::OpenProfilePicker]);
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let staging = position_of(&rows, "Staging copy", 0);
    assert_eq!(
        click(&state, &mut tui, staging),
        vec![
            Msg::Modal(ModalMsg::Select(1)),
            Msg::Modal(ModalMsg::Submit)
        ]
    );
    send(&mut state, [Msg::Modal(ModalMsg::Select(1))]);
    let Some(Modal::ProfilePicker(picker)) = &state.modal else {
        panic!("expected the profile picker");
    };
    assert_eq!(picker.selected_profile_index, 1);
}

#[test]
fn clicking_a_profile_on_the_start_screen_connects_it() {
    let mut state = AppState::new(
        vec![profile("Dev local"), profile("Staging copy")],
        storage(),
    );
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let staging = position_of(&rows, "Staging copy", 0);
    let messages = click(&state, &mut tui, staging);
    assert_eq!(
        messages,
        vec![
            Msg::Profiles(ProfilesMsg::Select(1)),
            Msg::Profiles(ProfilesMsg::Connect),
        ]
    );
    assert_eq!(
        wheel(&state, &mut tui, staging, true),
        vec![Msg::Profiles(ProfilesMsg::Next)]
    );
    assert_eq!(
        wheel(&state, &mut tui, staging, false),
        vec![Msg::Profiles(ProfilesMsg::Previous)]
    );
    assert!(click(&state, &mut tui, (0, 0)).is_empty());
    assert!(wheel(&state, &mut tui, (0, 0), true).is_empty());
    send(&mut state, [Msg::Profiles(ProfilesMsg::Select(1))]);
    let Screen::Profiles(ProfilePicker {
        selected_profile_index,
        ..
    }) = &state.screen
    else {
        panic!("expected the start screen");
    };
    assert_eq!(*selected_profile_index, 1);
}

#[test]
fn clicking_a_sql_file_selects_it_and_a_double_click_opens_it() {
    let mut state = files_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let second = position_of(&rows, "second.sql", MAIN_COLUMN);
    tui.preview_scroll = 4;
    let select_second = vec![
        Msg::FocusPane(PaneDirection::Right),
        Msg::Files(FilesMsg::Select(1)),
    ];
    let messages = click(&state, &mut tui, second);
    assert_eq!(messages, select_second);
    assert_eq!(tui.preview_scroll, 0);
    send(&mut state, messages);
    assert_eq!(workspace(&state).files.selected_entry_index, 1);
    let mut double_click_messages = select_second;
    double_click_messages.push(Msg::Files(FilesMsg::Open));
    assert_eq!(click(&state, &mut tui, second), double_click_messages);
    tui.preview_scroll = 4;
    assert_eq!(
        wheel(&state, &mut tui, second, false),
        vec![Msg::Files(FilesMsg::Previous)]
    );
    assert_eq!(tui.preview_scroll, 0);
}

#[test]
fn the_wheel_scrolls_the_file_preview() {
    let state = files_state();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let preview = position_of(&rows, "select 1", MAIN_COLUMN);
    assert!(wheel(&state, &mut tui, preview, true).is_empty());
    assert_eq!(tui.preview_scroll, 3);
    wheel(&state, &mut tui, preview, false);
    assert_eq!(tui.preview_scroll, 0);
}

#[test]
fn clicking_a_dump_selects_it_and_a_double_click_starts_the_restore() {
    let mut state = workspace_state();
    send(&mut state, [Msg::Tabs(TabsMsg::OpenRestore)]);
    let workspace = workspace_mut(&mut state);
    workspace.restore.entries = Loadable::Ready(vec![entry("old.dump"), entry("new.dump")]);
    workspace.restore.log_lines = (0..40)
        .map(|index| format!("restore line {index}"))
        .collect();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let new_dump = position_of(&rows, "new.dump", MAIN_COLUMN);
    let select_new = vec![
        Msg::FocusPane(PaneDirection::Right),
        Msg::Restore(RestoreMsg::Select(1)),
    ];
    assert_eq!(click(&state, &mut tui, new_dump), select_new);
    let mut double_click_messages = select_new;
    double_click_messages.push(Msg::Restore(RestoreMsg::Start));
    assert_eq!(click(&state, &mut tui, new_dump), double_click_messages);
    assert_eq!(
        wheel(&state, &mut tui, new_dump, true),
        vec![Msg::Restore(RestoreMsg::Next)]
    );
    let last_log_line = position_of(&rows, "restore line 39", MAIN_COLUMN);
    wheel(&state, &mut tui, last_log_line, false);
    assert_eq!(tui.job_log_scroll, Some(37));
    wheel(&state, &mut tui, last_log_line, true);
    assert_eq!(tui.job_log_scroll, None);
}

#[test]
fn the_wheel_scrolls_the_dump_log() {
    let mut state = workspace_state();
    send(&mut state, [Msg::Tabs(TabsMsg::OpenDump)]);
    let dump = &mut workspace_mut(&mut state).dump;
    dump.job = JobState::Running;
    dump.log_lines = (0..30).map(|index| format!("dump line {index}")).collect();
    let mut tui = TuiState::new(false);
    let rows = draw_rows(&state, &mut tui);
    let last_line = position_of(&rows, "dump line 29", MAIN_COLUMN);
    wheel(&state, &mut tui, last_line, false);
    assert_eq!(tui.job_log_scroll, Some(27));
}

#[test]
fn file_clicks_outside_the_file_tabs_only_focus_the_main_pane() {
    let state = table_state();
    let mut tui = TuiState::new(false);
    draw_rows(&state, &mut tui);
    tui.mouse_targets.push((
        ratatui::layout::Rect::new(0, 0, 10, 1),
        ducky_tui::mouse::MouseTarget::FileItem(0),
    ));
    assert_eq!(
        click(&state, &mut tui, (1, 0)),
        vec![Msg::FocusPane(PaneDirection::Right)]
    );
    assert!(wheel(&state, &mut tui, (1, 0), true).is_empty());
}

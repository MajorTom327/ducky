mod support;

use ducky_app::{
    AppState, Command, FilesMsg, Focus, GridCursor, Modal, ModalMsg, Msg, ProfilesMsg, QueryMsg,
    QueryPane, RestoreMsg, Screen, TableMsg, TabsMsg,
};
use ducky_core::TransactionMode;
use support::{
    active_query, active_tab_id, active_table, file_entry, new_query, open_loaded_table, profiles,
    report, rows_result, send, storage, workspace, workspace_mut, workspace_state,
};

fn cell(row_index: usize, column_index: usize) -> GridCursor {
    GridCursor {
        row_index,
        column_index,
    }
}

fn finished_query_state(row_count: usize) -> AppState {
    let mut state = workspace_state();
    new_query(&mut state);
    let tab_id = active_tab_id(&state);
    send(
        &mut state,
        Msg::ScriptFinished {
            tab_id,
            report: report(
                TransactionMode::Transaction,
                vec![rows_result("select", row_count)],
            ),
        },
    );
    state
}

#[test]
fn table_select_cell_moves_the_cursor_clamped_and_focuses_main() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    workspace_mut(&mut state).focus = Focus::Sidebar;
    send(&mut state, Msg::Table(TableMsg::SelectCell(cell(2, 1))));
    assert_eq!(active_table(&state).cursor, cell(2, 1));
    assert_eq!(workspace(&state).focus, Focus::Main);
    send(&mut state, Msg::Table(TableMsg::SelectCell(cell(40, 9))));
    assert_eq!(active_table(&state).cursor, cell(2, 1));
}

#[test]
fn query_select_cell_focuses_the_results_and_clamps() {
    let mut state = finished_query_state(3);
    workspace_mut(&mut state).focus = Focus::Sidebar;
    assert_eq!(active_query(&state).pane, QueryPane::Editor);
    send(&mut state, Msg::Query(QueryMsg::SelectCell(cell(1, 1))));
    assert_eq!(active_query(&state).cursor, cell(1, 1));
    assert_eq!(active_query(&state).pane, QueryPane::Results);
    assert_eq!(workspace(&state).focus, Focus::Main);
    send(&mut state, Msg::Query(QueryMsg::SelectCell(cell(50, 50))));
    assert_eq!(active_query(&state).cursor, cell(2, 1));
}

#[test]
fn query_select_cell_without_rows_only_focuses_the_results() {
    let mut state = workspace_state();
    new_query(&mut state);
    send(&mut state, Msg::Query(QueryMsg::SelectCell(cell(1, 1))));
    assert_eq!(active_query(&state).cursor, GridCursor::default());
    assert_eq!(active_query(&state).pane, QueryPane::Results);
}

#[test]
fn files_select_loads_the_preview_of_a_new_selection_only() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::OpenFiles));
    send(
        &mut state,
        Msg::SqlFilesListed(Ok(vec![file_entry("a.sql"), file_entry("b.sql")])),
    );
    assert_eq!(
        send(&mut state, Msg::Files(FilesMsg::Select(9))),
        vec![Command::LoadFilePreview {
            name: "b.sql".to_owned()
        }]
    );
    assert_eq!(workspace(&state).files.selected_entry_index, 1);
    assert!(send(&mut state, Msg::Files(FilesMsg::Select(1))).is_empty());
}

#[test]
fn restore_select_clamps_to_the_dump_list() {
    let mut state = workspace_state();
    send(&mut state, Msg::Tabs(TabsMsg::OpenRestore));
    assert!(send(&mut state, Msg::Restore(RestoreMsg::Select(1))).is_empty());
    assert_eq!(workspace(&state).restore.selected_entry_index, 0);
    send(
        &mut state,
        Msg::DumpsListed(Ok(vec![file_entry("a.dump"), file_entry("b.dump")])),
    );
    send(&mut state, Msg::Restore(RestoreMsg::Select(5)));
    assert_eq!(workspace(&state).restore.selected_entry_index, 1);
    send(&mut state, Msg::Restore(RestoreMsg::Select(0)));
    assert_eq!(workspace(&state).restore.selected_entry_index, 0);
}

#[test]
fn modal_select_then_submit_runs_the_clicked_palette_item() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenPalette);
    let help_index = workspace(&state)
        .palette_items("")
        .iter()
        .position(|item| item.label == "Help")
        .expect("help item");
    send(&mut state, Msg::Modal(ModalMsg::Select(help_index)));
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(matches!(state.modal, Some(Modal::Help)));
    send(&mut state, Msg::OpenPalette);
    send(&mut state, Msg::Modal(ModalMsg::Select(usize::MAX)));
    let item_count = workspace(&state).palette_items("").len();
    assert!(matches!(
        &state.modal,
        Some(Modal::Palette(command_palette)) if command_palette.selected_item_index == item_count - 1
    ));
}

#[test]
fn modal_select_picks_the_schema() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenSchemaPicker);
    send(&mut state, Msg::Modal(ModalMsg::Select(0)));
    send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert_eq!(workspace(&state).schema_name, "analytics");
}

#[test]
fn modal_select_moves_the_row_detail_field() {
    let mut state = workspace_state();
    open_loaded_table(&mut state, false);
    send(&mut state, Msg::Table(TableMsg::OpenRowDetail));
    send(&mut state, Msg::Modal(ModalMsg::Select(7)));
    assert!(matches!(
        &state.modal,
        Some(Modal::RowDetail(row_detail)) if row_detail.selected_field_index == 1
    ));
}

fn modal_picker_index(state: &AppState) -> usize {
    match &state.modal {
        Some(Modal::ProfilePicker(picker)) => picker.selected_profile_index,
        _ => panic!("expected the profile picker"),
    }
}

#[test]
fn modal_select_and_steps_drive_the_profile_switch_popup() {
    let mut state = workspace_state();
    send(&mut state, Msg::OpenProfilePicker);
    send(&mut state, Msg::Modal(ModalMsg::Next));
    assert_eq!(modal_picker_index(&state), 1);
    send(&mut state, Msg::Modal(ModalMsg::Previous));
    assert_eq!(modal_picker_index(&state), 0);
    send(&mut state, Msg::Modal(ModalMsg::Select(1)));
    assert_eq!(modal_picker_index(&state), 1);
    let commands = send(&mut state, Msg::Modal(ModalMsg::Submit));
    assert!(matches!(
        commands.as_slice(),
        [Command::Connect {
            profile_index: 1,
            ..
        }]
    ));
}

#[test]
fn profiles_select_ignores_hidden_profiles_and_connects_the_clicked_one() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(7)));
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(1)));
    let Screen::Profiles(picker) = &state.screen else {
        panic!("expected the profile screen");
    };
    assert_eq!(picker.selected_profile_index, 1);
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(7)));
    let commands = send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    assert!(matches!(
        commands.as_slice(),
        [Command::Connect {
            profile_index: 1,
            ..
        }]
    ));
}

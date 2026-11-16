use std::collections::BTreeMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ducky_app::{
    AppState, Confirm, ConfirmKind, Connection, DEFAULT_SIDEBAR_WIDTH, Focus, GridCursor, GridMove,
    Loadable, Modal, ModalMsg, Msg, Notice, NoticeLevel, PickerMode, ProfilePicker, ProfilesMsg,
    Prompt, PromptKind, QueryMsg, QueryPane, QueryView, RowDetail, Screen, SidebarMsg, Tab,
    TabContent, TabId, TableMsg, TableTab, TableView, TabsMsg, Workspace,
};
use ducky_core::{
    Catalog, PageRequest, PgTools, Profile, ProfileKind, RelationRef, SessionInfo, StoragePaths,
    ToolStatus,
};
use ducky_sql::{Completion, CompletionItem, CompletionKind, TokenKind};
use ducky_tui::editor::completion::{CompletionPopup, accept_completion};
use ducky_tui::editor::highlight::editor_highlights;
use ducky_tui::editor::panel::EditorPanel;
use ducky_tui::effects::UiEffects;
use ducky_tui::keys::help::KeyHint;
use ducky_tui::keys::hints::hints;
use ducky_tui::keys::modal::{HELP_HALF_PAGE, handle};
use ducky_tui::keys::sequence::{KeyPrefix, pending_hints};
use ducky_tui::keys::{handle_key, handle_paste};
use ducky_tui::layout::FrameAreas;
use ducky_tui::theme::{palette, token_style};
use ducky_tui::tui_state::TuiState;
use ducky_tui::widgets::loading::LoadSlot;
use edtui::{EditorState, Lines};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use tui_input::{Input, InputRequest};

fn press(modal: &Modal, tui: &mut TuiState, code: KeyCode) -> Vec<Msg> {
    handle(modal, tui, KeyEvent::new(code, KeyModifiers::NONE))
}

fn prompt() -> Modal {
    Modal::Prompt(Prompt {
        kind: PromptKind::SidebarFilter,
        title: "Filter tables".to_owned(),
        input: Input::default(),
    })
}

#[test]
fn prompt_navigation_keys_map_to_input_requests() {
    let modal = prompt();
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Delete, InputRequest::DeleteNextChar),
        (KeyCode::Left, InputRequest::GoToPrevChar),
        (KeyCode::Right, InputRequest::GoToNextChar),
        (KeyCode::Home, InputRequest::GoToStart),
        (KeyCode::End, InputRequest::GoToEnd),
    ];
    for (code, request) in cases {
        assert_eq!(
            press(&modal, &mut tui, code),
            vec![Msg::Modal(ModalMsg::Input(request))]
        );
    }
    assert!(press(&modal, &mut tui, KeyCode::F(3)).is_empty());
}

#[test]
fn confirm_ignores_unbound_keys() {
    let modal = Modal::Confirm(Confirm {
        kind: ConfirmKind::OverwriteFile,
        message: "Overwrite?".to_owned(),
    });
    let mut tui = TuiState::new(false);
    assert!(press(&modal, &mut tui, KeyCode::Char('z')).is_empty());
}

#[test]
fn delete_confirm_runs_only_on_y() {
    let modal = Modal::Confirm(Confirm {
        kind: ConfirmKind::DeleteRows {
            tab_id: 1,
            sql: "DELETE FROM t WHERE id IN ('1');".to_owned(),
        },
        message: "You are going to delete 1 row. Are you sure?".to_owned(),
    });
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Char('y'), ModalMsg::Submit),
        (KeyCode::Enter, ModalMsg::Cancel),
        (KeyCode::Char('Y'), ModalMsg::Cancel),
        (KeyCode::Char('z'), ModalMsg::Cancel),
        (KeyCode::Esc, ModalMsg::Cancel),
    ];
    for (code, modal_msg) in cases {
        assert_eq!(press(&modal, &mut tui, code), vec![Msg::Modal(modal_msg)]);
    }
}

#[test]
fn row_detail_keys_map_to_modal_messages() {
    let modal = Modal::RowDetail(RowDetail {
        columns: vec!["id".to_owned()],
        values: vec![Some("1".to_owned())],
        selected_field_index: 0,
    });
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Down, vec![Msg::Modal(ModalMsg::Next)]),
        (KeyCode::Up, vec![Msg::Modal(ModalMsg::Previous)]),
        (KeyCode::Esc, vec![Msg::Modal(ModalMsg::Cancel)]),
        (KeyCode::Char('j'), vec![Msg::Modal(ModalMsg::Next)]),
        (KeyCode::Char('k'), vec![Msg::Modal(ModalMsg::Previous)]),
        (KeyCode::Char('y'), vec![Msg::Modal(ModalMsg::CopyValue)]),
        (KeyCode::Char('q'), vec![Msg::Modal(ModalMsg::Cancel)]),
        (KeyCode::Char('z'), Vec::new()),
    ];
    for (code, expected) in cases {
        assert_eq!(press(&modal, &mut tui, code), expected, "{code:?}");
    }
}

#[test]
fn schema_picker_arrows_move_and_other_keys_are_ignored() {
    let modal = Modal::SchemaPicker {
        selected_schema_index: 0,
    };
    let mut tui = TuiState::new(false);
    assert_eq!(
        press(&modal, &mut tui, KeyCode::Down),
        vec![Msg::Modal(ModalMsg::Next)]
    );
    assert_eq!(
        press(&modal, &mut tui, KeyCode::Up),
        vec![Msg::Modal(ModalMsg::Previous)]
    );
    assert!(press(&modal, &mut tui, KeyCode::Char('z')).is_empty());
}

#[test]
fn help_ignores_unbound_keys_without_scrolling() {
    let mut tui = TuiState::new(false);
    tui.help_scroll = HELP_HALF_PAGE;
    assert!(press(&Modal::Help, &mut tui, KeyCode::Char('z')).is_empty());
    assert_eq!(tui.help_scroll, HELP_HALF_PAGE);
}

fn profile() -> Profile {
    Profile {
        label: "Dev local".to_owned(),
        url: "postgres://ducky@localhost/ducky".to_owned(),
        kind: ProfileKind::Local,
        project: None,
        tags: Vec::new(),
    }
}

fn profiles_state() -> AppState {
    let storage = StoragePaths {
        sql_dir: PathBuf::from("/tmp/ducky/sql"),
        dump_dir: PathBuf::from("/tmp/ducky/dumps"),
    };
    AppState::new(vec![profile()], storage)
}

fn workspace_state(tabs: Vec<TabContent>, focus: Focus) -> AppState {
    let mut state = profiles_state();
    let mut workspace = Workspace::new(
        0,
        profile(),
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
    workspace.next_tab_id = tabs.len() as TabId + 1;
    workspace.tabs = tabs
        .into_iter()
        .enumerate()
        .map(|(index, content)| Tab {
            id: index as TabId + 1,
            content,
        })
        .collect();
    workspace.focus = focus;
    state.screen = Screen::Workspace(Box::new(workspace));
    state
}

fn workspace_mut(state: &mut AppState) -> &mut Workspace {
    match &mut state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => panic!("expected a workspace"),
    }
}

fn table_view(tab: TableTab) -> TableView {
    let relation = RelationRef {
        schema: "public".to_owned(),
        name: "customers".to_owned(),
    };
    TableView {
        relation: relation.clone(),
        tab,
        request: PageRequest {
            relation,
            page_index: 0,
            page_size: 100,
            sort: None,
            filter: None,
        },
        page: Loadable::Loading,
        cursor: GridCursor::default(),
        details: Loadable::Idle,
        marked_row_indexes: Default::default(),
        visual_anchor_row_index: None,
        cell_drafts: BTreeMap::new(),
        is_saving_drafts: false,
    }
}

fn query_tab(pane: QueryPane) -> TabContent {
    let mut query = QueryView::new(1);
    query.pane = pane;
    TabContent::Query(query)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn route(state: &AppState, tui: &mut TuiState, code: KeyCode, modifiers: KeyModifiers) -> Vec<Msg> {
    handle_key(state, tui, key(code, modifiers))
}

#[test]
fn paste_reaches_only_a_focused_editor_without_modal() {
    let mut state = workspace_state(vec![query_tab(QueryPane::Editor)], Focus::Main);
    let mut tui = TuiState::new(false);
    assert!(handle_paste(&state, &mut tui, "select 1".to_owned()).is_empty());
    assert_eq!(tui.editor(1).text(), "select 1");
    state.modal = Some(Modal::Help);
    handle_paste(&state, &mut tui, " more".to_owned());
    assert_eq!(tui.editor(1).text(), "select 1");
    assert!(handle_paste(&profiles_state(), &mut tui, "x".to_owned()).is_empty());
}

#[test]
fn global_keys_open_help_switch_tabs_and_cancel_scripts() {
    let mut state = workspace_state(vec![TabContent::Files], Focus::Main);
    let mut tui = TuiState::new(false);
    assert_eq!(
        route(&state, &mut tui, KeyCode::F(1), KeyModifiers::NONE),
        vec![Msg::OpenHelp]
    );
    assert_eq!(
        route(&state, &mut tui, KeyCode::PageDown, KeyModifiers::CONTROL),
        vec![Msg::Tabs(TabsMsg::Next)]
    );
    assert_eq!(
        route(&state, &mut tui, KeyCode::PageUp, KeyModifiers::CONTROL),
        vec![Msg::Tabs(TabsMsg::Previous)]
    );
    assert!(route(&state, &mut tui, KeyCode::Char('c'), KeyModifiers::CONTROL).is_empty());
    workspace_mut(&mut state).running_script_tab_id = Some(1);
    assert_eq!(
        route(&state, &mut tui, KeyCode::Char('c'), KeyModifiers::CONTROL),
        vec![Msg::Query(QueryMsg::Cancel)]
    );
}

#[test]
fn navigation_keys_toggle_focus_quit_help_and_previous_tab() {
    let state = workspace_state(vec![TabContent::Files], Focus::Main);
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Tab, Msg::ToggleFocus),
        (KeyCode::BackTab, Msg::ToggleFocus),
        (KeyCode::Char('q'), Msg::Quit),
        (KeyCode::Char('?'), Msg::OpenHelp),
        (KeyCode::Char('H'), Msg::Tabs(TabsMsg::Previous)),
    ];
    for (code, expected) in cases {
        assert_eq!(
            route(&state, &mut tui, code, KeyModifiers::NONE),
            vec![expected]
        );
    }
}

#[test]
fn main_focus_without_tabs_ignores_keys() {
    let state = workspace_state(Vec::new(), Focus::Main);
    let mut tui = TuiState::new(false);
    assert!(route(&state, &mut tui, KeyCode::Char('z'), KeyModifiers::NONE).is_empty());
}

#[test]
fn sidebar_arrows_and_enter_map_to_sidebar_messages() {
    let state = workspace_state(Vec::new(), Focus::Sidebar);
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Down, SidebarMsg::Next),
        (KeyCode::Up, SidebarMsg::Previous),
        (KeyCode::Enter, SidebarMsg::Activate),
    ];
    for (code, expected) in cases {
        assert_eq!(
            route(&state, &mut tui, code, KeyModifiers::NONE),
            vec![Msg::Sidebar(expected)]
        );
    }
}

#[test]
fn table_handler_ignores_missing_or_foreign_tabs() {
    let mut tui = TuiState::new(false);
    let empty = workspace_state(Vec::new(), Focus::Main);
    let enter = key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(
        ducky_tui::keys::table::handle(empty.workspace().expect("ws"), &mut tui, enter).is_empty()
    );
    let files = workspace_state(vec![TabContent::Files], Focus::Main);
    assert!(
        ducky_tui::keys::table::handle(files.workspace().expect("ws"), &mut tui, enter).is_empty()
    );
}

#[test]
fn table_data_arrows_move_the_cursor() {
    let state = workspace_state(
        vec![TabContent::Table(table_view(TableTab::Data))],
        Focus::Main,
    );
    let mut tui = TuiState::new(false);
    let cases = [
        (KeyCode::Left, GridMove::Left),
        (KeyCode::Right, GridMove::Right),
        (KeyCode::Down, GridMove::Down),
        (KeyCode::Up, GridMove::Up),
    ];
    for (code, expected) in cases {
        assert_eq!(
            route(&state, &mut tui, code, KeyModifiers::NONE),
            vec![Msg::Table(TableMsg::Move(expected))]
        );
    }
}

#[test]
fn table_structure_keys_scroll_and_map_messages() {
    let state = workspace_state(
        vec![TabContent::Table(table_view(TableTab::Structure))],
        Focus::Main,
    );
    let mut tui = TuiState::new(false);
    let mut press = |code| route(&state, &mut tui, code, KeyModifiers::NONE);
    assert!(press(KeyCode::Down).is_empty());
    assert!(press(KeyCode::Down).is_empty());
    assert!(press(KeyCode::Up).is_empty());
    assert_eq!(press(KeyCode::Esc), vec![Msg::ToggleFocus]);
    assert_eq!(
        press(KeyCode::Char('t')),
        vec![Msg::Table(TableMsg::ToggleTab)]
    );
    assert_eq!(
        press(KeyCode::Char('r')),
        vec![Msg::Table(TableMsg::Reload)]
    );
    assert_eq!(tui.structure_scrolls.get(&1), Some(&1));
}

#[test]
fn profile_picker_keys_cover_navigation_and_unbound_keys() {
    let mut picker = ProfilePicker::new(&[profile()], None);
    picker.mode = PickerMode::Navigate;
    let navigate =
        |code| ducky_tui::keys::profiles::handle(&picker, key(code, KeyModifiers::NONE), false);
    assert_eq!(
        navigate(KeyCode::Down),
        vec![Msg::Profiles(ProfilesMsg::Next)]
    );
    assert_eq!(
        navigate(KeyCode::Up),
        vec![Msg::Profiles(ProfilesMsg::Previous)]
    );
    assert!(navigate(KeyCode::Char('z')).is_empty());
    let mut filtering = picker.clone();
    filtering.mode = PickerMode::Filter;
    assert!(
        ducky_tui::keys::profiles::handle(
            &filtering,
            key(KeyCode::F(3), KeyModifiers::NONE),
            false
        )
        .is_empty()
    );
}

fn first_hint(hints: &[KeyHint]) -> KeyHint {
    hints[0]
}

#[test]
fn hints_follow_the_active_context() {
    let mut picker_state = profiles_state();
    if let Screen::Profiles(picker) = &mut picker_state.screen {
        picker.mode = PickerMode::Navigate;
    }
    assert_eq!(first_hint(hints(&picker_state)), ("j/k", "move"));
    let no_tab = workspace_state(Vec::new(), Focus::Main);
    assert_eq!(first_hint(hints(&no_tab)), ("Ctrl+p", "go to"));
    assert_eq!(hints(&no_tab)[1], ("Ctrl+n", "new query"));
    let results = workspace_state(vec![query_tab(QueryPane::Results)], Focus::Main);
    assert_eq!(hints(&results)[1], ("p/n", "statement"));
}

#[test]
fn modal_hints_describe_each_modal() {
    let mut state = workspace_state(Vec::new(), Focus::Main);
    state.modal = Some(prompt());
    assert_eq!(hints(&state)[2], ("Ctrl+w", "delete word"));
    state.modal = Some(Modal::Confirm(Confirm {
        kind: ConfirmKind::OverwriteFile,
        message: String::new(),
    }));
    assert_eq!(first_hint(hints(&state)), ("y/Enter", "confirm"));
    state.modal = Some(Modal::RowDetail(RowDetail {
        columns: Vec::new(),
        values: Vec::new(),
        selected_field_index: 0,
    }));
    assert_eq!(hints(&state)[1], ("y", "copy value"));
    let mut picker = ProfilePicker::new(&[profile()], None);
    picker.mode = PickerMode::Navigate;
    state.modal = Some(Modal::ProfilePicker(picker));
    assert_eq!(hints(&state)[2], ("Enter", "switch"));
    assert_eq!(hints(&state)[3], ("Esc", "close"));
}

#[test]
fn pending_hints_name_the_previous_and_next_tab_bindings() {
    assert_eq!(pending_hints(KeyPrefix::Previous)[1], ("b", "previous tab"));
    assert_eq!(pending_hints(KeyPrefix::Next)[1], ("b", "next tab"));
}

fn popup(labels: &[&str]) -> CompletionPopup {
    CompletionPopup {
        completion: Completion {
            replace_start: 0,
            items: labels
                .iter()
                .map(|label| CompletionItem {
                    label: (*label).to_owned(),
                    kind: CompletionKind::Keyword,
                    detail: None,
                })
                .collect(),
        },
        selected_item_index: 0,
    }
}

#[test]
fn completion_selection_clamps_at_both_ends() {
    let mut completion_popup = popup(&["a", "b"]);
    completion_popup.select_previous();
    assert_eq!(completion_popup.selected_item_index, 0);
    completion_popup.select_next();
    completion_popup.select_next();
    assert_eq!(completion_popup.selected_item_index, 1);
    completion_popup.select_previous();
    assert_eq!(completion_popup.selected_item_index, 0);
}

#[test]
fn accept_completion_without_a_selectable_item_keeps_the_text() {
    let mut panel = EditorPanel::new();
    panel.state = EditorState::new(Lines::from("sel"));
    accept_completion(&mut panel);
    assert_eq!(panel.text(), "sel");
    panel.completion = Some(popup(&[]));
    accept_completion(&mut panel);
    assert_eq!(panel.text(), "sel");
    assert!(panel.completion.is_none());
}

#[test]
fn multi_line_comment_gets_one_highlight_per_line() {
    let highlights = editor_highlights("/* one\ntwo\nthree */");
    assert_eq!(highlights.len(), 3);
    assert_eq!(highlights[1].start.row, 1);
    assert_eq!(highlights[1].start.col, 0);
    assert_eq!(highlights[1].end.col, 2);
    assert!(
        highlights
            .iter()
            .all(|highlight| highlight.style == token_style(TokenKind::Comment))
    );
}

#[test]
fn comment_style_is_dark_gray_italic() {
    let style = token_style(TokenKind::Comment);
    assert_eq!(style.fg, Some(Color::DarkGray));
    assert!(style.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn effects_start_for_workspace_entry_notices_modals_and_loading() {
    let screen = Rect::new(0, 0, 100, 30);
    let areas = FrameAreas::new(screen, DEFAULT_SIDEBAR_WIDTH);
    let accent = palette(Some(ProfileKind::Local));
    let mut buffer = Buffer::empty(screen);
    let mut effects = UiEffects::new(true);
    let mut state = profiles_state();
    effects.begin_frame();
    effects.apply(&state, areas, &accent, &mut buffer);
    state = workspace_state(Vec::new(), Focus::Main);
    state.notice = Some(Notice {
        level: NoticeLevel::Info,
        text: "Copied".to_owned(),
    });
    effects.begin_frame();
    effects.mark_modal(areas.main);
    effects.mark_loading(LoadSlot::Main, areas.main);
    effects.apply(&state, areas, &accent, &mut buffer);
    assert!(effects.is_running());
    let mut disabled = UiEffects::new(false);
    disabled.mark_modal(areas.main);
    disabled.mark_loading(LoadSlot::Main, areas.main);
    disabled.apply(&state, areas, &accent, &mut buffer);
    assert!(!disabled.is_running());
}

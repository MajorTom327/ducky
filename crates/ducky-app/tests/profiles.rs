mod support;

use ducky_app::{
    AppState, Command, Modal, ModalMsg, Msg, PickerMode, ProfilePicker, ProfileRow, ProfilesMsg,
    Screen, profile_rows,
};
use ducky_core::{Profile, ProfileKind};
use support::{connection, profiles, send, storage, tools_available, workspace};
use tui_input::InputRequest;

fn profile(label: &str, project: Option<&str>, kind: ProfileKind, tags: &[&str]) -> Profile {
    Profile {
        label: label.to_owned(),
        url: format!(
            "postgres://app@{}.example.com:5432/app",
            label.to_lowercase()
        ),
        kind,
        project: project.map(str::to_owned),
        tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
    }
}

fn grouped_profiles() -> Vec<Profile> {
    vec![
        profile("scratch", None, ProfileKind::Local, &[]),
        profile("shop-local", Some("shop"), ProfileKind::Local, &[]),
        profile("b-local", Some("analytics"), ProfileKind::Local, &[]),
        profile(
            "shop-prod",
            Some("shop"),
            ProfileKind::Prod,
            &["eu", "readonly"],
        ),
        profile("shop-staging", Some("shop"), ProfileKind::Staging, &[]),
    ]
}

fn picker(state: &AppState) -> &ProfilePicker {
    match (&state.modal, &state.screen) {
        (Some(Modal::ProfilePicker(picker)), _) => picker,
        (_, Screen::Profiles(picker)) => picker,
        _ => panic!("expected a profile picker"),
    }
}

fn picker_index(state: &AppState) -> usize {
    picker(state).selected_profile_index
}

fn type_filter(state: &mut AppState, text: &str) {
    for typed_char in text.chars() {
        send(
            state,
            Msg::Profiles(ProfilesMsg::Filter(InputRequest::InsertChar(typed_char))),
        );
    }
}

fn project(name: &str) -> ProfileRow {
    ProfileRow::Project(name.to_owned())
}

#[test]
fn rows_group_profiles_by_project_in_order_of_first_appearance() {
    assert_eq!(
        profile_rows(&grouped_profiles(), ""),
        vec![
            project("shop"),
            ProfileRow::Profile(1),
            ProfileRow::Profile(3),
            ProfileRow::Profile(4),
            project("analytics"),
            ProfileRow::Profile(2),
            project("Other"),
            ProfileRow::Profile(0),
        ]
    );
}

#[test]
fn rows_without_any_project_have_no_header() {
    assert_eq!(
        profile_rows(&profiles(), ""),
        vec![ProfileRow::Profile(0), ProfileRow::Profile(1)]
    );
}

#[test]
fn filter_terms_match_project_label_kind_tag_and_target() {
    let profiles = grouped_profiles();
    assert_eq!(
        profile_rows(&profiles, "SHOP prod"),
        vec![project("shop"), ProfileRow::Profile(3)]
    );
    assert_eq!(
        profile_rows(&profiles, "readonly eu"),
        vec![project("shop"), ProfileRow::Profile(3)]
    );
    assert_eq!(
        profile_rows(&profiles, "analytics local"),
        vec![project("analytics"), ProfileRow::Profile(2)]
    );
    assert_eq!(
        profile_rows(&profiles, "scratch.example.com:5432"),
        vec![project("Other"), ProfileRow::Profile(0)]
    );
    assert_eq!(
        profile_rows(&profiles, "staging"),
        vec![project("shop"), ProfileRow::Profile(4)]
    );
    assert!(profile_rows(&profiles, "shop nothing").is_empty());
}

#[test]
fn initial_selection_is_the_first_visible_profile() {
    let state = AppState::new(grouped_profiles(), storage());
    assert_eq!(picker_index(&state), 1);
    assert_eq!(picker(&state).mode, PickerMode::Filter);
    assert_eq!(picker(&state).filter.value(), "");
}

#[test]
fn next_and_previous_skip_headers_and_clamp() {
    let mut state = AppState::new(grouped_profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Previous));
    assert_eq!(picker_index(&state), 1);
    let mut visited = Vec::new();
    for _ in 0..6 {
        send(&mut state, Msg::Profiles(ProfilesMsg::Next));
        visited.push(picker_index(&state));
    }
    assert_eq!(visited, [3, 4, 2, 0, 0, 0]);
    send(&mut state, Msg::Profiles(ProfilesMsg::First));
    assert_eq!(picker_index(&state), 1);
    send(&mut state, Msg::Profiles(ProfilesMsg::Last));
    assert_eq!(picker_index(&state), 0);
}

#[test]
fn filtering_reselects_the_first_match_or_keeps_a_visible_selection() {
    let mut state = AppState::new(grouped_profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    assert_eq!(picker_index(&state), 3);
    type_filter(&mut state, "shop");
    assert_eq!(picker_index(&state), 3);
    type_filter(&mut state, " stag");
    assert_eq!(picker_index(&state), 4);
    send(&mut state, Msg::Profiles(ProfilesMsg::ClearFilter));
    assert_eq!(picker(&state).filter.value(), "");
    assert_eq!(picker_index(&state), 4);
}

#[test]
fn no_match_leaves_nothing_to_connect() {
    let mut state = AppState::new(grouped_profiles(), storage());
    type_filter(&mut state, "zzz");
    assert_eq!(
        picker(&state).selected_visible_profile_index(&state.profiles),
        None
    );
    assert!(send(&mut state, Msg::Profiles(ProfilesMsg::Next)).is_empty());
    assert!(send(&mut state, Msg::Profiles(ProfilesMsg::Connect)).is_empty());
    let Screen::Profiles(picker) = &state.screen else {
        panic!("expected profiles");
    };
    assert_eq!(picker.connecting_profile_index, None);
}

#[test]
fn focus_messages_switch_the_mode() {
    let mut state = AppState::new(grouped_profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::FocusList));
    assert_eq!(picker(&state).mode, PickerMode::Navigate);
    send(&mut state, Msg::Profiles(ProfilesMsg::FocusFilter));
    assert_eq!(picker(&state).mode, PickerMode::Filter);
}

#[test]
fn connect_emits_connect_for_selected_profile() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    let commands = send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    assert_eq!(
        commands,
        vec![Command::Connect {
            profile_index: 1,
            profile: profiles()[1].clone(),
        }]
    );
    let Screen::Profiles(picker) = &state.screen else {
        panic!("expected profiles");
    };
    assert_eq!(picker.connecting_profile_index, Some(1));
}

#[test]
fn navigation_is_ignored_while_connecting() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    type_filter(&mut state, "prod");
    assert_eq!(picker_index(&state), 0);
    assert_eq!(picker(&state).filter.value(), "");
}

#[test]
fn connect_with_no_profiles_does_nothing() {
    let mut state = AppState::new(Vec::new(), storage());
    assert!(send(&mut state, Msg::Profiles(ProfilesMsg::Connect)).is_empty());
}

fn grouped_workspace_state() -> AppState {
    let mut state = AppState::new(grouped_profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    send(
        &mut state,
        Msg::Connected {
            profile_index: 1,
            result: Ok(connection(tools_available())),
        },
    );
    state
}

#[test]
fn open_profile_picker_selects_the_connected_profile() {
    let mut state = AppState::new(grouped_profiles(), storage());
    send(&mut state, Msg::OpenProfilePicker);
    assert!(state.modal.is_none());
    let mut state = grouped_workspace_state();
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    send(&mut state, Msg::OpenProfilePicker);
    let Some(Modal::ProfilePicker(picker)) = &state.modal else {
        panic!("expected the profile picker");
    };
    assert_eq!(picker.selected_profile_index, 1);
    assert_eq!(picker.filter.value(), "");
    assert_eq!(picker.mode, PickerMode::Filter);
    assert_eq!(workspace(&state).profile_index, 1);
}

#[test]
fn modal_connect_switches_through_the_switch_guards() {
    let mut state = grouped_workspace_state();
    send(&mut state, Msg::OpenProfilePicker);
    type_filter(&mut state, "staging");
    let commands = send(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    assert_eq!(
        commands,
        vec![Command::Connect {
            profile_index: 4,
            profile: grouped_profiles()[4].clone(),
        }]
    );
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).switching_profile_index, Some(4));

    let mut state = grouped_workspace_state();
    if let Screen::Workspace(workspace) = &mut state.screen {
        workspace.running_script_tab_id = Some(1);
    }
    send(&mut state, Msg::OpenProfilePicker);
    type_filter(&mut state, "staging");
    assert!(send(&mut state, Msg::Modal(ModalMsg::Submit)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).switching_profile_index, None);
    assert_eq!(
        state.notice.map(|notice| notice.text).as_deref(),
        Some("Cancel the running script before switching profile")
    );
}

#[test]
fn modal_connect_on_the_connected_profile_only_closes() {
    let mut state = grouped_workspace_state();
    send(&mut state, Msg::OpenProfilePicker);
    assert!(send(&mut state, Msg::Profiles(ProfilesMsg::Connect)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).switching_profile_index, None);
    assert!(state.notice.is_none());
}

#[test]
fn modal_navigation_and_cancel() {
    let mut state = grouped_workspace_state();
    send(&mut state, Msg::OpenProfilePicker);
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    assert_eq!(picker_index(&state), 3);
    send(&mut state, Msg::Profiles(ProfilesMsg::FocusList));
    assert_eq!(picker(&state).mode, PickerMode::Navigate);
    assert!(send(&mut state, Msg::Modal(ModalMsg::Cancel)).is_empty());
    assert!(state.modal.is_none());
    assert_eq!(workspace(&state).profile_index, 1);
}

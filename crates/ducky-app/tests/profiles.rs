mod support;

use ducky_app::{AppState, Command, Msg, ProfilesMsg, Screen};
use support::{profiles, send, storage};

fn picker_index(state: &AppState) -> usize {
    match &state.screen {
        Screen::Profiles(picker) => picker.selected_profile_index,
        Screen::Workspace(_) => panic!("expected profiles"),
    }
}

#[test]
fn next_and_previous_clamp() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Previous));
    assert_eq!(picker_index(&state), 0);
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    send(&mut state, Msg::Profiles(ProfilesMsg::Next));
    assert_eq!(picker_index(&state), 1);
}

#[test]
fn select_only_accepts_indexes_in_range() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(1)));
    assert_eq!(picker_index(&state), 1);
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(5)));
    assert_eq!(picker_index(&state), 1);
}

#[test]
fn connect_emits_connect_for_selected_profile() {
    let mut state = AppState::new(profiles(), storage());
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(1)));
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
    send(&mut state, Msg::Profiles(ProfilesMsg::Select(1)));
    assert_eq!(picker_index(&state), 0);
}

#[test]
fn connect_with_no_profiles_does_nothing() {
    let mut state = AppState::new(Vec::new(), storage());
    assert!(send(&mut state, Msg::Profiles(ProfilesMsg::Connect)).is_empty());
}

use ducky_core::Profile;

use crate::command::Command;
use crate::msg::ProfilesMsg;
use crate::state::{AppState, Modal, PickerMode, ProfilePicker, Screen};
use crate::update::step_index;
use crate::update::switch::switch_profile;

pub(crate) fn update(state: &mut AppState, msg: ProfilesMsg) -> Vec<Command> {
    let is_modal_picker = matches!(state.modal, Some(Modal::ProfilePicker(_)));
    if is_modal_picker && msg == ProfilesMsg::Connect {
        return connect_from_modal(state);
    }
    if let Some(Modal::ProfilePicker(picker)) = &mut state.modal {
        apply(picker, &state.profiles, msg);
        return Vec::new();
    }
    if state.modal.is_some() {
        return Vec::new();
    }
    let Screen::Profiles(picker) = &mut state.screen else {
        return Vec::new();
    };
    if picker.connecting_profile_index.is_some() {
        return Vec::new();
    }
    if msg != ProfilesMsg::Connect {
        apply(picker, &state.profiles, msg);
        return Vec::new();
    }
    let Some(profile_index) = picker.selected_visible_profile_index(&state.profiles) else {
        return Vec::new();
    };
    let Some(profile) = state.profiles.get(profile_index) else {
        return Vec::new();
    };
    picker.connecting_profile_index = Some(profile_index);
    picker.connection_error = None;
    vec![Command::Connect {
        profile_index,
        profile: profile.clone(),
    }]
}

pub(crate) fn open(state: &mut AppState) -> Vec<Command> {
    if let Some(workspace) = state.workspace() {
        let picker = ProfilePicker::new(&state.profiles, Some(workspace.profile_index));
        state.modal = Some(Modal::ProfilePicker(picker));
    }
    Vec::new()
}

pub(crate) fn connect_from_modal(state: &mut AppState) -> Vec<Command> {
    let Some(Modal::ProfilePicker(picker)) = state.modal.take() else {
        return Vec::new();
    };
    let Some(profile_index) = picker.selected_visible_profile_index(&state.profiles) else {
        state.modal = Some(Modal::ProfilePicker(picker));
        return Vec::new();
    };
    let is_connected_profile = state
        .workspace()
        .is_some_and(|workspace| workspace.profile_index == profile_index);
    if is_connected_profile {
        return Vec::new();
    }
    switch_profile(state, profile_index)
}

fn apply(picker: &mut ProfilePicker, profiles: &[Profile], msg: ProfilesMsg) {
    match msg {
        ProfilesMsg::Next | ProfilesMsg::Previous => {
            let visible_profile_indexes = picker.visible_profile_indexes(profiles);
            let Some(selected_position) = visible_profile_indexes
                .iter()
                .position(|profile_index| *profile_index == picker.selected_profile_index)
            else {
                picker.select_first(profiles);
                return;
            };
            let next_position = step_index(
                selected_position,
                visible_profile_indexes.len(),
                msg == ProfilesMsg::Next,
            );
            if let Some(profile_index) = visible_profile_indexes.get(next_position) {
                picker.selected_profile_index = *profile_index;
            }
        }
        ProfilesMsg::First => picker.select_first(profiles),
        ProfilesMsg::Last => {
            if let Some(profile_index) = picker.visible_profile_indexes(profiles).last() {
                picker.selected_profile_index = *profile_index;
            }
        }
        ProfilesMsg::Filter(request) => {
            picker.filter.handle(request);
            picker.reselect(profiles);
        }
        ProfilesMsg::ClearFilter => {
            picker.filter.reset();
            picker.reselect(profiles);
        }
        ProfilesMsg::FocusFilter => picker.mode = PickerMode::Filter,
        ProfilesMsg::FocusList => picker.mode = PickerMode::Navigate,
        ProfilesMsg::Connect => {}
    }
}

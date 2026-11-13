use crate::command::Command;
use crate::msg::ProfilesMsg;
use crate::state::{AppState, Screen};
use crate::update::step_index;

pub(crate) fn update(state: &mut AppState, msg: ProfilesMsg) -> Vec<Command> {
    let Screen::Profiles(picker) = &mut state.screen else {
        return Vec::new();
    };
    if picker.connecting_profile_index.is_some() {
        return Vec::new();
    }
    let profile_count = state.profiles.len();
    match msg {
        ProfilesMsg::Next | ProfilesMsg::Previous => {
            picker.selected_profile_index = step_index(
                picker.selected_profile_index,
                profile_count,
                msg == ProfilesMsg::Next,
            );
            Vec::new()
        }
        ProfilesMsg::Select(profile_index) => {
            if profile_index < profile_count {
                picker.selected_profile_index = profile_index;
            }
            Vec::new()
        }
        ProfilesMsg::Connect => {
            let profile_index = picker.selected_profile_index;
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
    }
}

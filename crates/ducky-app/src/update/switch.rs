use crate::command::Command;
use crate::loadable::Loadable;
use crate::msg::Connection;
use crate::state::{AppState, JobState, NoticeLevel, TableTab};
use crate::update::run_confirmation::forget_confirmations;
use crate::update::table::{reload_all_pages, table_views_mut};
use crate::update::{notice, workspace_mut};

pub(crate) fn switch_profile(state: &mut AppState, profile_index: usize) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    if workspace.running_script_tab_id.is_some() {
        state.notice = notice(
            NoticeLevel::Error,
            "Cancel the running script before switching profile",
        );
        return Vec::new();
    }
    if workspace.dump.job == JobState::Running || workspace.restore.job == JobState::Running {
        state.notice = notice(
            NoticeLevel::Error,
            "Wait for the running dump or restore before switching profile",
        );
        return Vec::new();
    }
    if profile_index == workspace.profile_index {
        return Vec::new();
    }
    let Some(profile) = state.profiles.get(profile_index) else {
        return Vec::new();
    };
    workspace.switching_profile_index = Some(profile_index);
    state.notice = notice(
        NoticeLevel::Info,
        format!("Connecting to {}…", profile.display_name()),
    );
    vec![Command::Connect {
        profile_index,
        profile: profile.clone(),
    }]
}

pub(crate) fn switched(
    state: &mut AppState,
    profile_index: usize,
    result: Result<Connection, String>,
) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    workspace.switching_profile_index = None;
    let Some(profile) = state.profiles.get(profile_index) else {
        return Vec::new();
    };
    let connection = match result {
        Ok(connection) => connection,
        Err(message) => {
            state.notice = notice(
                NoticeLevel::Error,
                format!("Could not connect to {}: {message}", profile.display_name()),
            );
            return Vec::new();
        }
    };
    workspace.profile_index = profile_index;
    workspace.profile = profile.clone();
    workspace.session = connection.session;
    workspace.tools = connection.tools;
    workspace.catalog = Loadable::Loading;
    forget_confirmations(workspace);
    let mut commands = vec![Command::LoadCatalog];
    for table in table_views_mut(workspace) {
        table.details = match table.tab {
            TableTab::Structure => {
                commands.push(Command::LoadDetails(table.relation.clone()));
                Loadable::Loading
            }
            TableTab::Data => Loadable::Idle,
        };
    }
    state.notice = notice(
        NoticeLevel::Success,
        format!("Switched to {}", profile.display_name()),
    );
    commands.extend(reload_all_pages(workspace));
    commands
}

#[cfg(test)]
mod tests;

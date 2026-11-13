use std::path::PathBuf;

use ducky_core::{PgTools, Profile, ProfileKind, SessionInfo, StoragePaths, ToolStatus};

use super::switch_profile;
use crate::msg::{Connection, Msg, ProfilesMsg};
use crate::state::AppState;
use crate::update::update;

fn connected_state() -> AppState {
    let profile = Profile {
        label: "Local".to_owned(),
        url: "postgres://localhost/a".to_owned(),
        kind: ProfileKind::Local,
        project: None,
        tags: Vec::new(),
    };
    let storage = StoragePaths {
        sql_dir: PathBuf::from("sql"),
        dump_dir: PathBuf::from("dumps"),
    };
    let mut state = AppState::new(vec![profile], storage);
    update(&mut state, Msg::Profiles(ProfilesMsg::Connect));
    update(
        &mut state,
        Msg::Connected {
            profile_index: 0,
            result: Ok(Connection {
                session: SessionInfo {
                    database: "ducky".to_owned(),
                    user: "ducky".to_owned(),
                    server_version: "18.0".to_owned(),
                },
                tools: PgTools {
                    pg_dump: ToolStatus::Missing(String::new()),
                    pg_restore: ToolStatus::Missing(String::new()),
                },
            }),
        },
    );
    state
}

#[test]
fn same_or_unknown_profile_index_does_nothing() {
    let mut state = connected_state();
    for profile_index in [0, 7] {
        assert!(switch_profile(&mut state, profile_index).is_empty());
        let workspace = state.workspace().expect("workspace");
        assert_eq!(workspace.switching_profile_index, None);
        assert_eq!(workspace.profile_index, 0);
        assert!(state.notice.is_none());
    }
}

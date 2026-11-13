use ducky_core::{PageRequest, Profile, RelationRef, RestoreOptions, TransactionMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Connect {
        profile_index: usize,
        profile: Profile,
    },
    Disconnect,
    LoadCatalog,
    LoadPage(PageRequest),
    LoadDetails(RelationRef),
    RunScript {
        sql: String,
        mode: TransactionMode,
    },
    CancelScript,
    ListSqlFiles,
    LoadFilePreview {
        name: String,
    },
    OpenFile {
        name: String,
        run_after_open: bool,
    },
    SaveFile {
        name: String,
        content: String,
        overwrite: bool,
    },
    ListDumps,
    StartDump {
        custom_file_name: Option<String>,
    },
    StartRestore {
        file_name: String,
        options: RestoreOptions,
    },
    CopyToClipboard(String),
}

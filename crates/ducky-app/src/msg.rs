use ducky_core::{
    Catalog, FileEntry, Page, PageRequest, PgTools, RelationDetails, RelationRef, ScriptReport,
    SessionInfo,
};
use tui_input::InputRequest;

use crate::state::{GridCursor, JobKind, Notice, TabId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Quit,
    DismissNotice,
    OpenHelp,
    ToggleFocus,
    FocusPane(PaneDirection),
    Reconnect,
    Disconnect,
    RefreshCatalog,
    OpenSchemaPicker,
    OpenPalette,
    OpenProfilePicker,
    Tabs(TabsMsg),
    Profiles(ProfilesMsg),
    Sidebar(SidebarMsg),
    Table(TableMsg),
    Query(QueryMsg),
    Files(FilesMsg),
    Dump(DumpMsg),
    Restore(RestoreMsg),
    Modal(ModalMsg),
    Connected {
        profile_index: usize,
        result: Result<Connection, String>,
    },
    CatalogLoaded(Result<Catalog, String>),
    PageLoaded {
        request: PageRequest,
        result: Result<Page, String>,
    },
    DetailsLoaded {
        relation: RelationRef,
        result: Result<RelationDetails, String>,
    },
    ScriptFinished {
        tab_id: TabId,
        report: ScriptReport,
    },
    SqlFilesListed(Result<Vec<FileEntry>, String>),
    FilePreviewLoaded {
        name: String,
        result: Result<String, String>,
    },
    FileOpened {
        name: String,
        run_after_open: bool,
        result: Result<String, String>,
    },
    FileSaved {
        name: String,
        result: Result<FileEntry, SaveFailure>,
    },
    DumpsListed(Result<Vec<FileEntry>, String>),
    JobLogged {
        job: JobKind,
        line: String,
    },
    JobFinished {
        job: JobKind,
        result: Result<String, String>,
    },
    CellEditsSaved {
        tab_id: TabId,
        result: Result<Vec<u64>, String>,
    },
    RowsDeleted {
        tab_id: TabId,
        result: Result<Vec<u64>, String>,
    },
    Notified(Notice),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub session: SessionInfo,
    pub tools: PgTools,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveFailure {
    AlreadyExists,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilesMsg {
    Next,
    Previous,
    First,
    Last,
    Connect,
    Select(usize),
    Filter(InputRequest),
    ClearFilter,
    FocusFilter,
    FocusList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabsMsg {
    Next,
    Previous,
    Select(usize),
    Close,
    NewQuery,
    OpenFiles,
    OpenDump,
    OpenRestore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMsg {
    Next,
    Previous,
    First,
    Last,
    Activate,
    StartFilter,
    ClearFilter,
    ToggleCollapsed,
    Widen,
    Narrow,
    SetWidth(u16),
    Select(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridMove {
    Left,
    Right,
    Up,
    Down,
    FirstRow,
    LastRow,
    FirstColumn,
    LastColumn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableMsg {
    ToggleTab,
    Move(GridMove),
    SelectCell(GridCursor),
    NextPage,
    PreviousPage,
    CycleSort,
    StartFilter,
    ClearFilter,
    Reload,
    OpenRowDetail,
    CopyCell,
    GoToReferenced,
    FindReferencing,
    ToggleRowMark,
    ToggleVisualSelection,
    ClearSelection,
    EditSelectedRows,
    DeleteSelectedRows,
    ChangeCell,
    SetCellNull,
    RevertCell,
    DiscardDrafts,
    SaveDrafts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMsg {
    Run { sql: String },
    Cancel,
    ToggleTransactionMode,
    Move(GridMove),
    SelectCell(GridCursor),
    NextResult,
    PreviousResult,
    OpenRowDetail,
    CopyCell,
    Save { content: String },
    SaveAs { content: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesMsg {
    Next,
    Previous,
    Open,
    Execute,
    Reload,
    Select(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpMsg {
    EditFileName,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreMsg {
    Next,
    Previous,
    ToggleClean,
    ToggleSkipOwnership,
    Start,
    Reload,
    Select(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalMsg {
    Input(InputRequest),
    Submit,
    Cancel,
    Next,
    Previous,
    Select(usize),
    CopyValue,
    ToggleRaw,
}

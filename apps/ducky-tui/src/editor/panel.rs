use ducky_app::Document;
use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};

pub struct EditorPanel {
    pub state: EditorState,
    pub event_handler: EditorEventHandler,
    pub loaded_revision: Option<u64>,
}

impl Default for EditorPanel {
    fn default() -> Self {
        EditorPanel::new()
    }
}

impl EditorPanel {
    pub fn new() -> EditorPanel {
        EditorPanel {
            state: EditorState::default(),
            event_handler: EditorEventHandler::default(),
            loaded_revision: None,
        }
    }

    pub fn text(&self) -> String {
        self.state.lines.to_string()
    }

    pub fn sync_document(&mut self, document: &Document) {
        if self.loaded_revision == Some(document.revision) {
            return;
        }
        self.state = EditorState::new(Lines::from(document.text.as_str()));
        self.loaded_revision = Some(document.revision);
    }

    pub fn is_insert_mode(&self) -> bool {
        self.state.mode == EditorMode::Insert
    }
}

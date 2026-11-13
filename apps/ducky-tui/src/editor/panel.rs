use ducky_app::Document;
use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};

use crate::editor::completion::CompletionPopup;
use crate::editor::highlight::refresh_highlights;

pub struct EditorPanel {
    pub state: EditorState,
    pub event_handler: EditorEventHandler,
    pub loaded_revision: Option<u64>,
    pub highlighted_text: String,
    pub completion: Option<CompletionPopup>,
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
            highlighted_text: String::new(),
            completion: None,
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
        self.highlighted_text.clear();
        self.completion = None;
        refresh_highlights(self);
    }

    pub fn is_insert_mode(&self) -> bool {
        self.state.mode == EditorMode::Insert
    }

    pub fn is_modified(&self, document: &Document) -> bool {
        normalized_text(&self.text()) != normalized_text(&document.text)
    }
}

fn normalized_text(text: &str) -> String {
    let joined_lines = text.lines().collect::<Vec<_>>().join("\n");
    joined_lines.trim_end_matches('\n').to_owned()
}

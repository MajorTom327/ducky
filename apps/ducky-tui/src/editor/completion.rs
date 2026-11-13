use ducky_app::Workspace;
use ducky_sql::{Completion, CompletionItem, TextPosition, complete, offset_at};
use edtui::actions::{DeleteChar, InsertChar};

use crate::editor::panel::EditorPanel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopup {
    pub completion: Completion,
    pub selected_item_index: usize,
}

impl CompletionPopup {
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.completion.items.get(self.selected_item_index)
    }

    pub fn select_next(&mut self) {
        let last_index = self.completion.items.len().saturating_sub(1);
        self.selected_item_index = (self.selected_item_index + 1).min(last_index);
    }

    pub fn select_previous(&mut self) {
        self.selected_item_index = self.selected_item_index.saturating_sub(1);
    }
}

pub fn refresh_completion(panel: &mut EditorPanel, workspace: &Workspace, is_forced: bool) {
    panel.completion = completion_popup(panel, workspace, is_forced);
}

pub fn accept_completion(panel: &mut EditorPanel) {
    let Some(popup) = panel.completion.take() else {
        return;
    };
    let Some(item) = popup.selected_item() else {
        return;
    };
    let text = panel.text();
    let cursor_offset = cursor_offset(panel, &text);
    let prefix_char_count = text
        .get(popup.completion.replace_start..cursor_offset)
        .map_or(0, |prefix| prefix.chars().count());
    if prefix_char_count > 0 {
        panel.state.execute(DeleteChar(prefix_char_count));
    }
    for character in item.label.chars() {
        panel.state.execute(InsertChar(character));
    }
}

fn completion_popup(
    panel: &EditorPanel,
    workspace: &Workspace,
    is_forced: bool,
) -> Option<CompletionPopup> {
    let catalog = workspace.catalog.ready()?;
    let text = panel.text();
    let cursor_offset = cursor_offset(panel, &text);
    let completion = complete(&text, cursor_offset, catalog, &workspace.schema_name);
    let has_prefix = completion.replace_start < cursor_offset;
    let follows_dot = text
        .get(..cursor_offset)
        .is_some_and(|text_before| text_before.ends_with('.'));
    let is_wanted = is_forced || has_prefix || follows_dot;
    match is_wanted && !completion.items.is_empty() {
        true => Some(CompletionPopup {
            completion,
            selected_item_index: 0,
        }),
        false => None,
    }
}

fn cursor_offset(panel: &EditorPanel, text: &str) -> usize {
    offset_at(
        text,
        TextPosition {
            line: panel.state.cursor.row,
            column: panel.state.cursor.col,
        },
    )
}

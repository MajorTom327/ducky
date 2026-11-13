use std::collections::HashMap;

use ducky_app::{TabContent, TabId, Workspace};

use crate::editor::panel::EditorPanel;
use crate::effects::UiEffects;
use crate::keys::sequence::KeyPrefix;
use crate::widgets::grid::GridViewport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Bar,
}

pub struct TuiState {
    pub effects: UiEffects,
    pub editors: HashMap<TabId, EditorPanel>,
    pub table_viewports: HashMap<TabId, GridViewport>,
    pub results_viewports: HashMap<TabId, GridViewport>,
    pub structure_scrolls: HashMap<TabId, u16>,
    pub preview_scroll: u16,
    pub job_log_scroll: Option<u16>,
    pub help_scroll: u16,
    pub cursor_shape: CursorShape,
    pub pending_prefix: Option<KeyPrefix>,
}

impl TuiState {
    pub fn new(are_effects_enabled: bool) -> TuiState {
        TuiState {
            effects: UiEffects::new(are_effects_enabled),
            editors: HashMap::new(),
            table_viewports: HashMap::new(),
            results_viewports: HashMap::new(),
            structure_scrolls: HashMap::new(),
            preview_scroll: 0,
            job_log_scroll: None,
            help_scroll: 0,
            cursor_shape: CursorShape::Bar,
            pending_prefix: None,
        }
    }

    pub fn editor(&mut self, tab_id: TabId) -> &mut EditorPanel {
        self.editors.entry(tab_id).or_default()
    }

    pub fn sync_workspace(&mut self, workspace: &Workspace) {
        let open_tab_ids: Vec<TabId> = workspace.tabs.iter().map(|tab| tab.id).collect();
        let is_open = |tab_id: &TabId| open_tab_ids.contains(tab_id);
        self.editors.retain(|tab_id, _| is_open(tab_id));
        self.table_viewports.retain(|tab_id, _| is_open(tab_id));
        self.results_viewports.retain(|tab_id, _| is_open(tab_id));
        self.structure_scrolls.retain(|tab_id, _| is_open(tab_id));
        for tab in &workspace.tabs {
            if let TabContent::Query(query) = &tab.content {
                self.editor(tab.id).sync_document(&query.document);
            }
        }
    }
}

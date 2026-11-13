use ducky_core::RelationRef;

use crate::editor::panel::EditorPanel;
use crate::effects::UiEffects;
use crate::widgets::grid::GridViewport;

pub struct TuiState {
    pub effects: UiEffects,
    pub editor: EditorPanel,
    pub table_viewport: GridViewport,
    pub results_viewport: GridViewport,
    pub shown_relation: Option<RelationRef>,
    pub structure_scroll: u16,
    pub preview_scroll: u16,
    pub job_log_scroll: Option<u16>,
}

impl TuiState {
    pub fn new(are_effects_enabled: bool) -> TuiState {
        TuiState {
            effects: UiEffects::new(are_effects_enabled),
            editor: EditorPanel::new(),
            table_viewport: GridViewport::default(),
            results_viewport: GridViewport::default(),
            shown_relation: None,
            structure_scroll: 0,
            preview_scroll: 0,
            job_log_scroll: None,
        }
    }
}

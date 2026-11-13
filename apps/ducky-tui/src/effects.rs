use std::time::{Duration, Instant};

use ducky_app::{AppState, Screen};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tachyonfx::{EffectManager, Interpolation, fx};

use crate::layout::FrameAreas;
use crate::theme::Palette;
use crate::widgets::loading::LoadSlot;

const FIRST_FRAME_STEP: Duration = Duration::from_millis(16);
const MAX_FRAME_STEP: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum EffectSlot {
    #[default]
    Startup,
    Workspace,
    Sidebar,
    Main,
    Modal,
    Notice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    is_workspace: bool,
    notice_text: Option<String>,
}

impl Snapshot {
    fn of(state: &AppState) -> Snapshot {
        Snapshot {
            is_workspace: matches!(state.screen, Screen::Workspace(_)),
            notice_text: state.notice.as_ref().map(|notice| notice.text.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FrameMarks {
    sidebar_loading_area: Option<Rect>,
    main_loading_area: Option<Rect>,
    modal_area: Option<Rect>,
}

impl FrameMarks {
    fn loading_area(&self, slot: LoadSlot) -> Option<Rect> {
        match slot {
            LoadSlot::Sidebar => self.sidebar_loading_area,
            LoadSlot::Main => self.main_loading_area,
        }
    }
}

pub struct UiEffects {
    is_enabled: bool,
    manager: EffectManager<EffectSlot>,
    last_frame_at: Option<Instant>,
    previous_snapshot: Option<Snapshot>,
    current_marks: FrameMarks,
    previous_marks: FrameMarks,
}

impl UiEffects {
    pub fn new(is_enabled: bool) -> UiEffects {
        UiEffects {
            is_enabled,
            manager: EffectManager::default(),
            last_frame_at: None,
            previous_snapshot: None,
            current_marks: FrameMarks::default(),
            previous_marks: FrameMarks::default(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_enabled && self.manager.is_running()
    }

    pub fn begin_frame(&mut self) {
        self.current_marks = FrameMarks::default();
    }

    pub fn mark_loading(&mut self, slot: LoadSlot, area: Rect) {
        if !self.is_enabled {
            return;
        }
        match slot {
            LoadSlot::Sidebar => self.current_marks.sidebar_loading_area = Some(area),
            LoadSlot::Main => self.current_marks.main_loading_area = Some(area),
        }
    }

    pub fn mark_modal(&mut self, area: Rect) {
        if self.is_enabled {
            self.current_marks.modal_area = Some(area);
        }
    }

    pub fn apply(
        &mut self,
        state: &AppState,
        areas: FrameAreas,
        palette: &Palette,
        buffer: &mut Buffer,
    ) {
        if !self.is_enabled {
            return;
        }
        let now = Instant::now();
        let elapsed = self
            .last_frame_at
            .replace(now)
            .map_or(FIRST_FRAME_STEP, |last_frame_at| {
                now.duration_since(last_frame_at).min(MAX_FRAME_STEP)
            });
        self.queue_effects(state, areas, palette);
        self.previous_marks = self.current_marks;
        self.manager
            .process_effects(elapsed.into(), buffer, areas.screen);
    }

    fn queue_effects(&mut self, state: &AppState, areas: FrameAreas, palette: &Palette) {
        let current_snapshot = Snapshot::of(state);
        let previous_snapshot = self.previous_snapshot.replace(current_snapshot.clone());
        match &previous_snapshot {
            None => self.manager.add_unique_effect(
                EffectSlot::Startup,
                fx::coalesce((500, Interpolation::QuadOut)).with_area(areas.screen),
            ),
            Some(previous_snapshot) => {
                if current_snapshot.is_workspace && !previous_snapshot.is_workspace {
                    self.manager.add_unique_effect(
                        EffectSlot::Workspace,
                        fx::fade_from_fg(palette.accent, (450, Interpolation::QuadOut))
                            .with_area(areas.screen),
                    );
                }
                if current_snapshot.notice_text.is_some()
                    && current_snapshot.notice_text != previous_snapshot.notice_text
                {
                    self.manager.add_unique_effect(
                        EffectSlot::Notice,
                        fx::fade_from_fg(palette.accent, (400, Interpolation::QuadOut))
                            .with_area(areas.status),
                    );
                }
            }
        }
        self.queue_loading_effect(LoadSlot::Sidebar, EffectSlot::Sidebar, palette);
        self.queue_loading_effect(LoadSlot::Main, EffectSlot::Main, palette);
        if let Some(modal_area) = self.current_marks.modal_area
            && self.previous_marks.modal_area.is_none()
        {
            self.manager.add_unique_effect(
                EffectSlot::Modal,
                fx::coalesce((200, Interpolation::QuadOut)).with_area(modal_area),
            );
        }
    }

    fn queue_loading_effect(&mut self, slot: LoadSlot, effect_slot: EffectSlot, palette: &Palette) {
        let current_area = self.current_marks.loading_area(slot);
        let previous_area = self.previous_marks.loading_area(slot);
        match (current_area, previous_area) {
            (Some(area), None) => self.manager.add_unique_effect(
                effect_slot,
                fx::repeating(fx::ping_pong(fx::fade_to_fg(
                    palette.accent,
                    (600, Interpolation::SineInOut),
                )))
                .with_area(area),
            ),
            (None, Some(area)) => self.manager.add_unique_effect(
                effect_slot,
                fx::coalesce((350, Interpolation::QuadOut)).with_area(area),
            ),
            (Some(_), Some(_)) | (None, None) => {}
        }
    }
}

use ducky_app::Sidebar;
use ratatui::layout::{Constraint, Layout, Rect};

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 20;
pub const MIN_MAIN_WIDTH: u16 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameAreas {
    pub screen: Rect,
    pub header: Rect,
    pub sidebar: Rect,
    pub tab_bar: Rect,
    pub main: Rect,
    pub status: Rect,
}

impl FrameAreas {
    pub fn new(screen: Rect, sidebar_width: u16) -> FrameAreas {
        let [header, body, status] = screen.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]));
        let [sidebar, main_column] = body.layout(&Layout::horizontal([
            Constraint::Length(sidebar_width),
            Constraint::Min(0),
        ]));
        let [tab_bar, main] = main_column.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
        ]));
        FrameAreas {
            screen,
            header,
            sidebar,
            tab_bar,
            main,
            status,
        }
    }

    pub fn body(&self) -> Rect {
        self.sidebar.union(self.tab_bar).union(self.main)
    }
}

pub fn sidebar_width(sidebar: &Sidebar, screen_width: u16) -> u16 {
    match sidebar.is_collapsed {
        true => 0,
        false => sidebar
            .width
            .min(screen_width.saturating_sub(MIN_MAIN_WIDTH)),
    }
}

pub fn is_too_small(screen: Rect) -> bool {
    screen.width < MIN_WIDTH || screen.height < MIN_HEIGHT
}

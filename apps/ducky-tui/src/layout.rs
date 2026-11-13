use ratatui::layout::{Constraint, Layout, Rect};

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 20;
pub const SIDEBAR_WIDTH: u16 = 32;

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
    pub fn new(screen: Rect) -> FrameAreas {
        let [header, body, status] = screen.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]));
        let [sidebar, main_column] = body.layout(&Layout::horizontal([
            Constraint::Length(SIDEBAR_WIDTH),
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

pub fn is_too_small(screen: Rect) -> bool {
    screen.width < MIN_WIDTH || screen.height < MIN_HEIGHT
}

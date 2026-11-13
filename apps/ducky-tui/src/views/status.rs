use ducky_app::{AppState, Focus, MainView, QueryPane, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::keys::help::hints;
use crate::theme::Palette;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let focus_name = focus_name(state);
    let focus_width = u16::try_from(focus_name.chars().count() + 1).unwrap_or(u16::MAX);
    let [message_area, focus_area] = area.layout(&Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(focus_width),
    ]));
    let message = match &state.notice {
        Some(notice) => Line::styled(
            format!(" {}", notice.text.lines().next().unwrap_or_default()),
            palette.notice(notice.level),
        ),
        None => Line::styled(format!(" {}", hints(state)), palette.muted()),
    };
    frame.render_widget(Paragraph::new(message), message_area);
    frame.render_widget(
        Paragraph::new(Line::styled(focus_name, palette.title())).right_aligned(),
        focus_area,
    );
}

fn focus_name(state: &AppState) -> &'static str {
    let workspace = match &state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => return "",
    };
    match (workspace.focus, workspace.view, workspace.query.pane) {
        (Focus::Sidebar, _, _) => "Sidebar",
        (Focus::Main, MainView::Query, QueryPane::Editor) => "Editor",
        (Focus::Main, MainView::Query, QueryPane::Results) => "Results",
        (Focus::Main, _, _) => "Main",
    }
}

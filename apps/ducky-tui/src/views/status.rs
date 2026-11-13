use ducky_app::{AppState, Focus, QueryPane, Screen, TabContent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::keys::help::{KeyHint, hints};
use crate::keys::sequence::{KeyPrefix, pending_hints};
use crate::theme::Palette;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    pending_prefix: Option<KeyPrefix>,
    palette: &Palette,
) {
    let focus_name = focus_name(state);
    let focus_width = u16::try_from(focus_name.chars().count() + 1).unwrap_or(u16::MAX);
    let [message_area, focus_area] = area.layout(&Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(focus_width),
    ]));
    let message = match (&state.notice, pending_prefix) {
        (_, Some(prefix)) => hint_line(pending_hints(prefix), palette),
        (Some(notice), None) => Line::styled(
            format!(" {}", notice.text.lines().next().unwrap_or_default()),
            palette.notice(notice.level),
        ),
        (None, None) => hint_line(hints(state), palette),
    };
    frame.render_widget(Paragraph::new(message), message_area);
    frame.render_widget(
        Paragraph::new(Line::styled(focus_name, palette.title())).right_aligned(),
        focus_area,
    );
}

pub fn hint_line(key_hints: &[KeyHint], palette: &Palette) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (hint_index, (key, description)) in key_hints.iter().enumerate() {
        if hint_index > 0 {
            spans.push(Span::styled(" · ", palette.muted()));
        }
        spans.push(Span::styled(*key, palette.title()));
        spans.push(Span::raw(format!(" {description}")));
    }
    Line::from(spans)
}

fn focus_name(state: &AppState) -> &'static str {
    let workspace = match &state.screen {
        Screen::Workspace(workspace) => workspace,
        Screen::Profiles(_) => return "",
    };
    if workspace.focus == Focus::Sidebar {
        return "Sidebar";
    }
    match workspace.active_tab().map(|tab| &tab.content) {
        Some(TabContent::Query(query)) => match query.pane {
            QueryPane::Editor => "Editor",
            QueryPane::Results => "Results",
        },
        Some(_) | None => "Main",
    }
}

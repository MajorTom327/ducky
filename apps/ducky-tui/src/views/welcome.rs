use ducky_app::Focus;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

use crate::tui_state::TuiState;
use crate::views::ViewContext;

const SHORTCUTS: [(&str, &str); 5] = [
    ("Ctrl+p", "go to anything"),
    ("Enter", "open the selected table"),
    ("Ctrl+n", "new SQL query"),
    ("Ctrl+o", "SQL files"),
    ("?", "all key bindings"),
];
const SHORTCUT_KEY_WIDTH: usize = 8;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, _tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let block = Block::bordered()
        .title(Span::styled(" Welcome ", palette.title()))
        .border_style(palette.border(workspace.focus == Focus::Main))
        .padding(Padding::left(1));
    let session_facts = [
        ("Database", workspace.session.database.clone()),
        ("User", workspace.session.user.clone()),
        (
            "Server",
            format!("PostgreSQL {}", workspace.session.server_version),
        ),
        ("Profile", workspace.profile.display_name()),
    ];
    let mut lines: Vec<Line> = session_facts
        .into_iter()
        .map(|(name, value)| {
            Line::from(vec![
                Span::styled(format!("{name:<10}"), palette.muted()),
                Span::raw(value),
            ])
        })
        .collect();
    lines.push(Line::default());
    lines.extend(SHORTCUTS.iter().map(|(key, description)| {
        Line::from(vec![
            Span::styled(format!("{key:<SHORTCUT_KEY_WIDTH$}"), palette.title()),
            Span::raw(*description),
        ])
    }));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

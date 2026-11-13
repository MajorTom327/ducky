use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use ducky_app::Focus;

use crate::tui_state::TuiState;
use crate::views::ViewContext;

const USEFUL_KEYS: [(&str, &str); 5] = [
    ("Enter", "open the selected table or action"),
    ("Tab", "switch focus between sidebar and main view"),
    ("/", "filter tables"),
    ("s", "choose another schema"),
    ("?", "show every key binding"),
];

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, _tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    let block = Block::bordered()
        .title(Span::styled(" Welcome ", palette.title()))
        .border_style(palette.border(workspace.focus == Focus::Main));
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Database  ", palette.muted()),
            Span::raw(workspace.session.database.clone()),
        ]),
        Line::from(vec![
            Span::styled("User      ", palette.muted()),
            Span::raw(workspace.session.user.clone()),
        ]),
        Line::from(vec![
            Span::styled("Server    ", palette.muted()),
            Span::raw(format!("PostgreSQL {}", workspace.session.server_version)),
        ]),
        Line::from(vec![
            Span::styled("Profile   ", palette.muted()),
            Span::raw(workspace.profile.label.clone()),
        ]),
        Line::default(),
    ];
    lines.extend(USEFUL_KEYS.iter().map(|(key, description)| {
        Line::from(vec![
            Span::styled(format!("{key:<7}"), palette.title()),
            Span::raw(*description),
        ])
    }));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

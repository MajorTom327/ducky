use ducky_app::AppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::theme::{Palette, kind_badge};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, palette: &Palette) {
    let text = match state.workspace() {
        Some(workspace) => format!(
            " ducky │ {}{} {} │ {}@{} │ PostgreSQL {} ",
            workspace
                .profile
                .project
                .as_ref()
                .map(|project| format!("{project} │ "))
                .unwrap_or_default(),
            kind_badge(workspace.profile.kind),
            workspace.profile.label,
            workspace.session.user,
            workspace.session.database,
            workspace.session.server_version,
        ),
        None => " ducky │ Profiles ".to_owned(),
    };
    frame.render_widget(Paragraph::new(text).style(palette.header()), area);
}

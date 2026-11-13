use ducky_app::NoticeLevel;
use ducky_core::ProfileKind;
use ducky_sql::TokenKind;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub accent: Color,
}

pub fn palette(kind: Option<ProfileKind>) -> Palette {
    let accent = match kind {
        Some(ProfileKind::Local) => Color::Green,
        Some(ProfileKind::Staging) => Color::Indexed(208),
        Some(ProfileKind::Prod) => Color::Red,
        None => Color::Cyan,
    };
    Palette { accent }
}

impl Palette {
    pub fn header(&self) -> Style {
        Style::new()
            .fg(Color::Black)
            .bg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border(&self, is_focused: bool) -> Style {
        match is_focused {
            true => Style::new().fg(self.accent),
            false => Style::new().fg(Color::DarkGray),
        }
    }

    pub fn selected(&self, is_focused: bool) -> Style {
        match is_focused {
            true => Style::new().fg(Color::Black).bg(self.accent),
            false => Style::new().fg(self.accent).add_modifier(Modifier::BOLD),
        }
    }

    pub fn title(&self) -> Style {
        Style::new().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn muted(&self) -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub fn warning(&self) -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn cursor_row(&self) -> Style {
        Style::new().fg(Color::White).bg(Color::DarkGray)
    }

    pub fn null_cell(&self) -> Style {
        Style::new()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn notice(&self, level: NoticeLevel) -> Style {
        match level {
            NoticeLevel::Info => Style::new(),
            NoticeLevel::Success => Style::new().fg(Color::Green),
            NoticeLevel::Error => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        }
    }
}

pub fn token_style(kind: TokenKind) -> Style {
    match kind {
        TokenKind::Keyword => Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        TokenKind::String => Style::new().fg(Color::Yellow),
        TokenKind::Number => Style::new().fg(Color::Cyan),
        TokenKind::Comment => Style::new()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
        TokenKind::QuotedIdentifier => Style::new().fg(Color::Blue),
        TokenKind::Identifier | TokenKind::Operator | TokenKind::Punctuation => Style::new(),
    }
}

pub fn kind_badge(kind: ProfileKind) -> &'static str {
    match kind {
        ProfileKind::Local => "[LOCAL]",
        ProfileKind::Staging => "[STAGING]",
        ProfileKind::Prod => "[PROD]",
    }
}

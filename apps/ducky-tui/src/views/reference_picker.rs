use ducky_core::{ForeignKeyLink, RelationRef};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::mouse::{MouseTarget, MouseTargets, list_row_targets};
use crate::theme::Palette;

const WIDTH_PERCENT: u16 = 60;
const FOOTER: &str = "Enter open · j/k move · Esc cancel";
const CHROME_HEIGHT: usize = 4;

pub fn render(
    frame: &mut Frame,
    screen: Rect,
    links: &[(RelationRef, ForeignKeyLink)],
    selected_link_index: usize,
    palette: &Palette,
    mouse_targets: &mut MouseTargets,
) -> Rect {
    let height = u16::try_from(links.len() + CHROME_HEIGHT).unwrap_or(u16::MAX);
    let area = screen.centered(
        Constraint::Percentage(WIDTH_PERCENT),
        Constraint::Length(height.min(screen.height)),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(Span::styled(" Referencing rows ", palette.title()))
        .border_style(palette.border(true));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let [list_area, _, footer_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner_area);
    let lines: Vec<Line> = links
        .iter()
        .enumerate()
        .map(|(link_index, (referencing, link))| {
            let line = Line::raw(format!(" {}", link_label(referencing, link)));
            match link_index == selected_link_index {
                true => line.style(palette.selected(true)),
                false => line,
            }
        })
        .collect();
    let scroll = (selected_link_index + 1).saturating_sub(usize::from(list_area.height));
    frame.render_widget(
        Paragraph::new(lines).scroll((u16::try_from(scroll).unwrap_or(0), 0)),
        list_area,
    );
    mouse_targets.extend(list_row_targets(
        list_area,
        scroll,
        links.len(),
        MouseTarget::ModalItem,
    ));
    frame.render_widget(
        Paragraph::new(Line::styled(FOOTER, palette.muted())),
        footer_area,
    );
    area
}

pub fn link_label(referencing: &RelationRef, link: &ForeignKeyLink) -> String {
    let referencing_name = match referencing.schema == link.referenced.schema {
        true => referencing.name.clone(),
        false => format!("{}.{}", referencing.schema, referencing.name),
    };
    format!(
        "{referencing_name}.{} → {}.{}",
        column_list(&link.columns),
        link.referenced.name,
        column_list(&link.referenced_columns)
    )
}

fn column_list(columns: &[String]) -> String {
    match columns {
        [single_column] => single_column.clone(),
        _ => format!("({})", columns.join(", ")),
    }
}

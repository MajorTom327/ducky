use ducky_app::{ParameterField, ParameterForm};
use ducky_sql::bind_parameters;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Padding, Paragraph};

use crate::theme::Palette;

const WIDTH_PERCENT: u16 = 70;
const CHROME_HEIGHT: u16 = 4;
const COLUMN_GAP: &str = "  ";
const RAW_BADGE: &str = " raw ";
const FOOTER: &str = "Tab next · Shift+Tab previous · Ctrl+r raw SQL · Enter run · Esc cancel";

pub fn render(frame: &mut Frame, screen: Rect, form: &ParameterForm, palette: &Palette) -> Rect {
    let field_count = u16::try_from(form.fields.len()).unwrap_or(u16::MAX);
    let height = field_count.saturating_add(CHROME_HEIGHT).min(screen.height);
    let area = screen.centered(
        Constraint::Percentage(WIDTH_PERCENT),
        Constraint::Length(height),
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .padding(Padding::horizontal(1))
        .title(Span::styled(" Query parameters ", palette.title()))
        .border_style(palette.border(true));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);
    let [fields_area, footer_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);
    let name_width = form
        .fields
        .iter()
        .map(|field| field.name.chars().count())
        .max()
        .unwrap_or(0);
    let value_width =
        usize::from(fields_area.width).saturating_sub(name_width + 2 * COLUMN_GAP.len()) / 2;
    let visible_count = usize::from(fields_area.height);
    let first_visible_index = (form.selected_field_index + 1).saturating_sub(visible_count);
    for (row_offset, (field_index, field)) in form
        .fields
        .iter()
        .enumerate()
        .skip(first_visible_index)
        .take(visible_count)
        .enumerate()
    {
        let is_selected = field_index == form.selected_field_index;
        let row_area = Rect {
            y: fields_area.y + u16::try_from(row_offset).unwrap_or(0),
            height: 1,
            ..fields_area
        };
        let input_scroll = field.input.visual_scroll(value_width.max(1));
        let line = field_line(
            field,
            is_selected,
            name_width,
            value_width,
            input_scroll,
            palette,
        );
        frame.render_widget(Paragraph::new(line), row_area);
        if is_selected {
            let cursor_column = name_width
                + COLUMN_GAP.len()
                + field.input.visual_cursor().saturating_sub(input_scroll);
            let cursor_x = row_area
                .x
                .saturating_add(u16::try_from(cursor_column).unwrap_or(u16::MAX));
            frame.set_cursor_position((cursor_x, row_area.y));
        }
    }
    frame.render_widget(
        Paragraph::new(Line::styled(FOOTER, palette.muted())),
        footer_area,
    );
    area
}

fn field_line(
    field: &ParameterField,
    is_selected: bool,
    name_width: usize,
    value_width: usize,
    input_scroll: usize,
    palette: &Palette,
) -> Line<'static> {
    let name_style = match is_selected {
        true => palette.selected(true).add_modifier(Modifier::BOLD),
        false => Style::new().add_modifier(Modifier::BOLD),
    };
    let visible_value: String = field
        .input
        .value()
        .chars()
        .skip(input_scroll)
        .take(value_width)
        .collect();
    let mut spans = vec![
        Span::styled(format!("{:<name_width$}", field.name), name_style),
        Span::raw(COLUMN_GAP),
        Span::raw(format!("{visible_value:<value_width$}")),
        Span::raw(COLUMN_GAP),
        Span::styled(format!("→ {}", inserted_sql(field)), palette.muted()),
    ];
    if field.is_raw {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(RAW_BADGE, palette.warning()));
    }
    Line::from(spans)
}

fn inserted_sql(field: &ParameterField) -> String {
    bind_parameters(&field.name, &[(field.name.clone(), field.value())])
}

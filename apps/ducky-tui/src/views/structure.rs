use ducky_core::{Column, RelationDetails};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Palette;
use crate::tui_state::TuiState;
use crate::views::ViewContext;
use crate::widgets::loading::{LoadSlot, render_loadable};
use crate::widgets::sql_text::highlighted_sql;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let Some(table) = &context.workspace.table else {
        return;
    };
    let palette = context.palette;
    let columns = context
        .workspace
        .catalog
        .ready()
        .and_then(|catalog| catalog.relation(&table.relation))
        .map(|relation| relation.columns.as_slice())
        .unwrap_or_default();
    let mut lines = column_lines(columns, palette);
    let content_area = match table.details.ready() {
        Some(details) => {
            lines.extend(detail_lines(details, palette));
            area
        }
        None => {
            let column_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
            let [columns_area, details_area] =
                Layout::vertical([Constraint::Length(column_height), Constraint::Min(0)])
                    .areas(area);
            render_loadable(
                frame,
                details_area,
                &table.details,
                "details",
                LoadSlot::Main,
                tui,
                palette,
            );
            columns_area
        }
    };
    let content_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    tui.structure_scroll = tui
        .structure_scroll
        .min(content_height.saturating_sub(content_area.height));
    frame.render_widget(
        Paragraph::new(lines).scroll((tui.structure_scroll, 0)),
        content_area,
    );
}

fn padded(text: &str, width: usize) -> String {
    format!("{text:<width$}  ")
}

fn column_lines(columns: &[Column], palette: &Palette) -> Vec<Line<'static>> {
    let name_width = columns
        .iter()
        .map(|column| column.name.chars().count())
        .max()
        .unwrap_or(0);
    let type_width = columns
        .iter()
        .map(|column| column.data_type.chars().count())
        .max()
        .unwrap_or(0);
    let mut lines = vec![Line::styled("Columns", palette.title())];
    lines.extend(columns.iter().map(|column| {
        let nullability = match column.is_nullable {
            true => "NULL",
            false => "NOT NULL",
        };
        let mut spans = vec![
            Span::raw(padded(&column.name, name_width)),
            Span::raw(padded(&column.data_type, type_width)),
            Span::raw(padded(nullability, "NOT NULL".len())),
        ];
        if column.is_primary_key {
            spans.push(Span::styled("PK  ", palette.title()));
        }
        if let Some(default_expression) = &column.default_expression {
            spans.push(Span::styled(
                format!("default {default_expression}"),
                palette.muted(),
            ));
        }
        Line::from(spans)
    }));
    lines
}

fn detail_lines(details: &RelationDetails, palette: &Palette) -> Vec<Line<'static>> {
    let mut lines = vec![Line::raw(""), Line::styled("Indexes", palette.title())];
    if details.indexes.is_empty() {
        lines.push(Line::styled("none", palette.muted()));
    }
    for index in &details.indexes {
        let mut spans = vec![Span::raw(format!("{}  ", index.name))];
        if index.is_primary {
            spans.push(Span::styled("PRIMARY  ", palette.title()));
        } else if index.is_unique {
            spans.push(Span::styled("UNIQUE  ", palette.title()));
        }
        spans.push(Span::styled(index.definition.clone(), palette.muted()));
        lines.push(Line::from(spans));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled("Foreign keys", palette.title()));
    if details.foreign_keys.is_empty() {
        lines.push(Line::styled("none", palette.muted()));
    }
    for foreign_key in &details.foreign_keys {
        lines.push(Line::from(vec![
            Span::raw(format!("{}  ", foreign_key.name)),
            Span::styled(foreign_key.definition.clone(), palette.muted()),
        ]));
    }
    if let Some(view_definition) = &details.view_definition {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Definition", palette.title()));
        lines.extend(highlighted_sql(view_definition).lines);
    }
    lines
}

use ducky_app::{TabId, TableView};
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

const INDENT: &str = "  ";
const NOT_NULL: &str = "NOT NULL";
const PRIMARY: &str = "PRIMARY";

pub fn render(
    frame: &mut Frame,
    area: Rect,
    tab_id: TabId,
    table: &TableView,
    context: &ViewContext,
    tui: &mut TuiState,
) {
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
            let column_height = u16::try_from(lines.len() + 1).unwrap_or(u16::MAX);
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
    let scroll = tui.structure_scrolls.entry(tab_id).or_default();
    *scroll = (*scroll).min(content_height.saturating_sub(content_area.height));
    frame.render_widget(Paragraph::new(lines).scroll((*scroll, 0)), content_area);
}

fn padded(text: &str, width: usize) -> String {
    format!("{text:<width$}  ")
}

fn widest<'a>(texts: impl Iterator<Item = &'a str>) -> usize {
    texts.map(|text| text.chars().count()).max().unwrap_or(0)
}

fn section_title(title: &'static str, palette: &Palette) -> Line<'static> {
    Line::styled(title, palette.title())
}

fn empty_section(palette: &Palette) -> Line<'static> {
    Line::styled(format!("{INDENT}none"), palette.muted())
}

fn column_lines(columns: &[Column], palette: &Palette) -> Vec<Line<'static>> {
    let name_width = widest(columns.iter().map(|column| column.name.as_str()));
    let type_width = widest(columns.iter().map(|column| column.data_type.as_str()));
    let mut lines = vec![section_title("Columns", palette)];
    if columns.is_empty() {
        lines.push(empty_section(palette));
    }
    lines.extend(columns.iter().map(|column| {
        let nullability = match column.is_nullable {
            true => "NULL",
            false => NOT_NULL,
        };
        let primary_key = match column.is_primary_key {
            true => "PK",
            false => "",
        };
        let mut spans = vec![
            Span::raw(INDENT),
            Span::raw(padded(&column.name, name_width)),
            Span::styled(padded(&column.data_type, type_width), palette.muted()),
            Span::raw(padded(nullability, NOT_NULL.len())),
            Span::styled(padded(primary_key, 2), palette.title()),
        ];
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
    let mut lines = vec![Line::raw(""), section_title("Indexes", palette)];
    if details.indexes.is_empty() {
        lines.push(empty_section(palette));
    }
    let index_name_width = widest(details.indexes.iter().map(|index| index.name.as_str()));
    for index in &details.indexes {
        let flag = match (index.is_primary, index.is_unique) {
            (true, _) => PRIMARY,
            (false, true) => "UNIQUE",
            (false, false) => "",
        };
        lines.push(Line::from(vec![
            Span::raw(INDENT),
            Span::raw(padded(&index.name, index_name_width)),
            Span::styled(padded(flag, PRIMARY.len()), palette.title()),
            Span::styled(index.definition.clone(), palette.muted()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(section_title("Foreign keys", palette));
    if details.foreign_keys.is_empty() {
        lines.push(empty_section(palette));
    }
    let foreign_key_name_width = widest(
        details
            .foreign_keys
            .iter()
            .map(|foreign_key| foreign_key.name.as_str()),
    );
    for foreign_key in &details.foreign_keys {
        lines.push(Line::from(vec![
            Span::raw(INDENT),
            Span::raw(padded(&foreign_key.name, foreign_key_name_width)),
            Span::styled(foreign_key.definition.clone(), palette.muted()),
        ]));
    }
    if let Some(view_definition) = &details.view_definition {
        lines.push(Line::raw(""));
        lines.push(section_title("Definition", palette));
        lines.extend(
            highlighted_sql(view_definition)
                .lines
                .into_iter()
                .map(|mut line| {
                    line.spans.insert(0, Span::raw(INDENT));
                    line
                }),
        );
    }
    lines
}

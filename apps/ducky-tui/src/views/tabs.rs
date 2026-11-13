use ducky_app::{Tab, TabContent, Workspace};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui_state::TuiState;
use crate::views::ViewContext;

const MARKER_WIDTH: u16 = 2;

pub fn render(frame: &mut Frame, area: Rect, context: &ViewContext, tui: &mut TuiState) {
    let workspace = context.workspace;
    let palette = context.palette;
    if workspace.tabs.is_empty() {
        let hint = Line::styled(" Ctrl+n new query · Ctrl+p go to", palette.muted());
        frame.render_widget(Paragraph::new(hint), area);
        return;
    }
    let labels: Vec<String> = workspace
        .tabs
        .iter()
        .enumerate()
        .map(|(tab_index, tab)| tab_label(tab_index, tab, workspace, tui))
        .collect();
    let label_widths: Vec<usize> = labels
        .iter()
        .map(|label| Span::raw(label).width())
        .collect();
    let active_tab_index = workspace.active_tab_index.min(labels.len() - 1);
    let total_width = label_widths.iter().sum::<usize>() + labels.len() - 1;
    let (visible_range, tabs_area, left_marker_area, right_marker_area) =
        match total_width <= usize::from(area.width) {
            true => (0..labels.len(), area, None, None),
            false => {
                let [left_area, middle_area, right_area] = area.layout(&Layout::horizontal([
                    Constraint::Length(MARKER_WIDTH),
                    Constraint::Min(0),
                    Constraint::Length(MARKER_WIDTH),
                ]));
                let visible_range = visible_tab_range(
                    &label_widths,
                    active_tab_index,
                    usize::from(middle_area.width),
                );
                (
                    visible_range,
                    middle_area,
                    Some(left_area),
                    Some(right_area),
                )
            }
        };
    if let Some(left_area) = left_marker_area.filter(|_| visible_range.start > 0) {
        frame.render_widget(
            Paragraph::new(Span::styled("‹", palette.title())),
            left_area,
        );
    }
    if let Some(right_area) = right_marker_area.filter(|_| visible_range.end < labels.len()) {
        frame.render_widget(
            Paragraph::new(Span::styled("›", palette.title())).right_aligned(),
            right_area,
        );
    }
    let mut spans = Vec::new();
    let visible_labels = labels
        .into_iter()
        .enumerate()
        .take(visible_range.end)
        .skip(visible_range.start);
    for (tab_index, label) in visible_labels {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        let style = match tab_index == active_tab_index {
            true => palette.selected(true),
            false => palette.muted(),
        };
        spans.push(Span::styled(label, style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), tabs_area);
}

fn tab_label(tab_index: usize, tab: &Tab, workspace: &Workspace, tui: &TuiState) -> String {
    let mut label = format!(" {} {}", tab_index + 1, tab.title());
    if workspace.running_script_tab_id == Some(tab.id) {
        label.push_str(" ●");
    }
    if let TabContent::Query(query) = &tab.content {
        let is_modified = tui
            .editors
            .get(&tab.id)
            .is_some_and(|editor| editor.is_modified(&query.document));
        if is_modified {
            label.push_str(" •");
        }
    }
    label.push(' ');
    label
}

fn visible_tab_range(
    label_widths: &[usize],
    active_tab_index: usize,
    available_width: usize,
) -> std::ops::Range<usize> {
    let mut first_index = active_tab_index;
    let mut used_width = label_widths[active_tab_index];
    while first_index > 0 && used_width + 1 + label_widths[first_index - 1] <= available_width {
        first_index -= 1;
        used_width += 1 + label_widths[first_index];
    }
    let mut end_index = active_tab_index + 1;
    while end_index < label_widths.len()
        && used_width + 1 + label_widths[end_index] <= available_width
    {
        used_width += 1 + label_widths[end_index];
        end_index += 1;
    }
    first_index..end_index
}

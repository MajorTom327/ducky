use std::path::PathBuf;

use ducky_app::{AppState, CommandPalette, Modal, Prompt, PromptKind};
use ducky_core::{Profile, ProfileKind, StoragePaths};
use ducky_sql::{Completion, CompletionItem, CompletionKind};
use ducky_tui::editor::completion::CompletionPopup;
use ducky_tui::editor::panel::EditorPanel;
use ducky_tui::theme::{Palette, palette};
use ducky_tui::tui_state::TuiState;
use ducky_tui::views::{completion, modal};
use edtui::{EditorState, EditorView, Lines};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use tui_input::Input;

fn item(label: &str, detail: Option<&str>) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: CompletionKind::Relation,
        detail: detail.map(str::to_owned),
    }
}

fn panel_with(items: Vec<CompletionItem>, selected_item_index: usize) -> EditorPanel {
    let mut panel = EditorPanel::new();
    panel.state = EditorState::new(Lines::from("select"));
    panel.completion = Some(CompletionPopup {
        completion: Completion {
            replace_start: 0,
            items,
        },
        selected_item_index,
    });
    panel
}

fn render_popup(panel: &mut EditorPanel, editor_area: Rect, theme: &Palette) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(60, 20)).expect("terminal");
    terminal
        .draw(|frame| {
            frame.render_widget(EditorView::new(&mut panel.state), editor_area);
            completion::render(frame, editor_area, panel, theme);
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol().to_owned())
        .collect()
}

fn column_of(buffer: &Buffer, y: u16, needle: &str) -> u16 {
    let line = row(buffer, y);
    let byte_index = line.find(needle).expect("needle in row");
    u16::try_from(line[..byte_index].chars().count()).expect("column")
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|y| row(buffer, *y).contains(needle))
}

#[test]
fn completion_popup_opens_below_cursor_with_details_and_selection() {
    let theme = palette(Some(ProfileKind::Local));
    let mut panel = panel_with(
        vec![
            item("customers", Some("table")),
            item("orders", Some("view")),
            item("plain", None),
        ],
        1,
    );
    let buffer = render_popup(&mut panel, Rect::new(0, 0, 60, 8), &theme);
    let customers_row = find_row(&buffer, "customers").expect("customers row");
    assert!(customers_row > 0, "popup must sit below the cursor row");
    assert!(row(&buffer, customers_row).contains("customers  table"));
    let orders_row = find_row(&buffer, "orders").expect("orders row");
    let orders_x = column_of(&buffer, orders_row, "orders");
    assert_eq!(buffer[(orders_x, orders_row)].bg, Color::Green);
    assert_eq!(buffer[(orders_x, customers_row)].bg, Color::Reset);
    let detail_x = column_of(&buffer, customers_row, "table");
    assert_eq!(buffer[(detail_x, customers_row)].fg, Color::DarkGray);
    assert!(find_row(&buffer, "plain").is_some());
}

#[test]
fn completion_popup_opens_above_cursor_without_room_below() {
    let theme = palette(None);
    let mut panel = panel_with(vec![item("customers", Some("table"))], 0);
    let buffer = render_popup(&mut panel, Rect::new(0, 18, 60, 2), &theme);
    let customers_row = find_row(&buffer, "customers").expect("customers row");
    assert!(customers_row < 18, "popup must sit above the cursor row");
}

#[test]
fn completion_popup_scrolls_to_keep_selection_visible() {
    let theme = palette(None);
    let items = (0..12)
        .map(|index| item(&format!("item{index:02}"), None))
        .collect();
    let mut panel = panel_with(items, 11);
    let buffer = render_popup(&mut panel, Rect::new(0, 0, 60, 4), &theme);
    assert!(find_row(&buffer, "item11").is_some());
    assert!(find_row(&buffer, "item00").is_none());
}

#[test]
fn completion_popup_without_items_or_popup_renders_nothing() {
    let theme = palette(None);
    let mut panel = panel_with(Vec::new(), 0);
    let buffer = render_popup(&mut panel, Rect::new(0, 0, 60, 4), &theme);
    assert!(find_row(&buffer, "┌").is_none());
    let mut closed_panel = EditorPanel::new();
    let buffer = render_popup(&mut closed_panel, Rect::new(0, 0, 60, 4), &theme);
    assert!(find_row(&buffer, "┌").is_none());
}

fn profiles_state() -> AppState {
    AppState::new(
        vec![Profile {
            label: "Dev local".to_owned(),
            url: "postgres://ducky@localhost/ducky".to_owned(),
            kind: ProfileKind::Local,
            project: None,
            tags: Vec::new(),
        }],
        StoragePaths {
            sql_dir: PathBuf::from("/tmp/ducky/sql"),
            dump_dir: PathBuf::from("/tmp/ducky/dumps"),
        },
    )
}

fn render_modal(state: &AppState, shown_modal: &Modal) -> (Buffer, Option<(u16, u16)>) {
    let theme = palette(None);
    let mut tui = TuiState::new(false);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| {
            modal::render(frame, frame.area(), state, shown_modal, &mut tui, &theme);
        })
        .expect("draw");
    let position = terminal.get_cursor_position().expect("cursor");
    let cursor = Some((position.x, position.y));
    (terminal.backend().buffer().clone(), cursor)
}

#[test]
fn prompt_modal_shows_title_value_and_cursor() {
    let state = profiles_state();
    let prompt = Prompt {
        kind: PromptKind::TableFilter,
        title: "WHERE".to_owned(),
        input: Input::new("id > 3".to_owned()),
    };
    let (buffer, cursor) = render_modal(&state, &Modal::Prompt(prompt));
    let title_row = find_row(&buffer, " WHERE ").expect("title");
    let value_row = find_row(&buffer, "id > 3").expect("value");
    assert_eq!(value_row, title_row + 1);
    let value_x = column_of(&buffer, value_row, "id > 3");
    assert_eq!(cursor, Some((value_x + 6, value_row)));
}

#[test]
fn schema_picker_without_workspace_renders_empty_list() {
    let state = profiles_state();
    let (buffer, _) = render_modal(
        &state,
        &Modal::SchemaPicker {
            selected_schema_index: 0,
        },
    );
    assert!(find_row(&buffer, " Schemas ").is_some());
}

#[test]
fn palette_modal_without_workspace_renders_nothing() {
    let state = profiles_state();
    let shown_modal = Modal::Palette(CommandPalette {
        input: Input::default(),
        selected_item_index: 0,
    });
    let (buffer, _) = render_modal(&state, &shown_modal);
    assert!((0..buffer.area.height).all(|y| row(&buffer, y).trim().is_empty()));
}

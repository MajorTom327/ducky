use std::io::Write;
use std::time::Duration;

use crossterm::cursor::SetCursorStyle;
use crossterm::event::{Event, KeyEventKind};
use crossterm::execute;
use ducky_app::{AppState, Msg, update};
use ducky_runtime::Runtime;
use futures::{Stream, StreamExt};
use ratatui::Terminal;
use ratatui::backend::Backend;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::tui_state::{CursorShape, TuiState};
use crate::{keys, mouse, views};

const ANIMATION_FRAME_PERIOD: Duration = Duration::from_millis(16);

pub struct TerminalIo<'a, B: Backend, E, W> {
    pub terminal: &'a mut Terminal<B>,
    pub events: E,
    pub cursor_writer: W,
}

pub async fn run<B, E, W>(
    terminal_io: TerminalIo<'_, B, E, W>,
    mut state: AppState,
    mut tui: TuiState,
    mut runtime: Runtime,
    mut messages: UnboundedReceiver<Msg>,
    startup_messages: Vec<Msg>,
) -> anyhow::Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    E: Stream<Item = std::io::Result<Event>> + Unpin,
    W: Write,
{
    let TerminalIo {
        terminal,
        events: mut terminal_events,
        mut cursor_writer,
    } = terminal_io;
    apply_messages(&mut state, &mut runtime, startup_messages);
    let mut applied_cursor_shape = None;
    while !state.should_quit {
        if let Some(workspace) = state.workspace() {
            tui.sync_workspace(workspace);
        }
        terminal.draw(|frame| views::draw(frame, &state, &mut tui))?;
        if applied_cursor_shape != Some(tui.cursor_shape) {
            execute!(cursor_writer, cursor_style(tui.cursor_shape))?;
            applied_cursor_shape = Some(tui.cursor_shape);
        }
        let incoming_messages = tokio::select! {
            terminal_event = terminal_events.next() => match terminal_event {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    keys::handle_key(&state, &mut tui, key)
                }
                Some(Ok(Event::Paste(pasted_text))) => keys::handle_paste(&state, &mut tui, pasted_text),
                Some(Ok(Event::Mouse(mouse_event))) => mouse::handle_mouse(&state, &mut tui, mouse_event),
                Some(Ok(_)) => Vec::new(),
                Some(Err(error)) => return Err(error.into()),
                None => return Ok(()),
            },
            Some(message) = messages.recv() => vec![message],
            _ = tokio::time::sleep(ANIMATION_FRAME_PERIOD), if tui.effects.is_running() => Vec::new(),
        };
        apply_messages(&mut state, &mut runtime, incoming_messages);
    }
    Ok(())
}

fn cursor_style(shape: CursorShape) -> SetCursorStyle {
    match shape {
        CursorShape::Block => SetCursorStyle::SteadyBlock,
        CursorShape::Bar => SetCursorStyle::SteadyBar,
    }
}

fn apply_messages(state: &mut AppState, runtime: &mut Runtime, messages: Vec<Msg>) {
    for message in messages {
        for command in update(state, message) {
            runtime.dispatch(command);
        }
    }
}

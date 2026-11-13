use crate::command::Command;
use crate::msg::{Msg, TabsMsg};
use crate::state::{AppState, CommandPalette, Focus, Modal, PaletteAction};
use crate::update::tabs::{open_relation, reveal_schema};
use crate::update::{update, workspace_mut};

pub(crate) fn open(state: &mut AppState) -> Vec<Command> {
    if state.workspace().is_some() {
        state.modal = Some(Modal::Palette(CommandPalette::default()));
    }
    Vec::new()
}

pub(crate) fn submit(state: &mut AppState, command_palette: &CommandPalette) -> Vec<Command> {
    let Some(selected_action) = state.workspace().and_then(|workspace| {
        workspace
            .palette_items(command_palette.input.value())
            .into_iter()
            .nth(command_palette.selected_item_index)
            .map(|item| item.action)
    }) else {
        return Vec::new();
    };
    let msg = match selected_action {
        PaletteAction::SwitchTab(tab_id) => {
            if let Some(workspace) = workspace_mut(&mut state.screen)
                && workspace.activate_tab(tab_id)
            {
                workspace.focus = Focus::Main;
            }
            return Vec::new();
        }
        PaletteAction::OpenRelation(relation) => {
            let page_size = state.page_size;
            let Some(workspace) = workspace_mut(&mut state.screen) else {
                return Vec::new();
            };
            reveal_schema(workspace, &relation.schema);
            return open_relation(workspace, relation, page_size);
        }
        PaletteAction::NewQuery => Msg::Tabs(TabsMsg::NewQuery),
        PaletteAction::SqlFiles => Msg::Tabs(TabsMsg::OpenFiles),
        PaletteAction::Dump => Msg::Tabs(TabsMsg::OpenDump),
        PaletteAction::Restore => Msg::Tabs(TabsMsg::OpenRestore),
        PaletteAction::ChooseSchema => Msg::OpenSchemaPicker,
        PaletteAction::RefreshCatalog => Msg::RefreshCatalog,
        PaletteAction::Reconnect => Msg::Reconnect,
        PaletteAction::Disconnect => Msg::Disconnect,
        PaletteAction::Help => Msg::OpenHelp,
        PaletteAction::Quit => Msg::Quit,
        PaletteAction::SwitchProfile => Msg::OpenProfilePicker,
    };
    update(state, msg)
}

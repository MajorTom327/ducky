use ducky_sql::{bind_parameters, find_parameters};
use tui_input::InputRequest;

use crate::command::Command;
use crate::state::{AppState, Modal, ParameterForm, TabId};
use crate::update::query::is_run_refused;
use crate::update::run_confirmation;
use crate::update::{step_index, workspace_mut};

pub(crate) fn request_run(state: &mut AppState, tab_id: TabId, sql: String) -> Vec<Command> {
    if is_run_refused(state, &sql) {
        return Vec::new();
    }
    let parameter_names = find_parameters(&sql);
    if parameter_names.is_empty() {
        return run_confirmation::request(state, tab_id, sql);
    }
    let Some(query) =
        workspace_mut(&mut state.screen).and_then(|workspace| workspace.query_mut(tab_id))
    else {
        return Vec::new();
    };
    let form = ParameterForm::new(tab_id, sql, parameter_names, &query.parameter_values);
    state.modal = Some(Modal::Parameters(form));
    Vec::new()
}

pub(crate) fn input(form: &mut ParameterForm, request: InputRequest) {
    if let Some(field) = form.selected_field_mut() {
        field.input.handle(request);
    }
}

pub(crate) fn step(form: &mut ParameterForm, is_forward: bool) {
    form.selected_field_index =
        step_index(form.selected_field_index, form.fields.len(), is_forward);
}

pub(crate) fn toggle_raw(form: &mut ParameterForm) {
    if let Some(field) = form.selected_field_mut() {
        field.is_raw = !field.is_raw;
    }
}

pub(crate) fn submit(state: &mut AppState, form: ParameterForm) -> Vec<Command> {
    let submitted_values = form.values();
    let Some(query) =
        workspace_mut(&mut state.screen).and_then(|workspace| workspace.query_mut(form.tab_id))
    else {
        return Vec::new();
    };
    query.parameter_values.retain(|(name, _)| {
        !submitted_values
            .iter()
            .any(|(submitted_name, _)| submitted_name == name)
    });
    query
        .parameter_values
        .extend(submitted_values.iter().cloned());
    let bound_sql = bind_parameters(&form.sql, &submitted_values);
    run_confirmation::request(state, form.tab_id, bound_sql)
}

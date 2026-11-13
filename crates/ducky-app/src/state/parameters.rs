use ducky_sql::ParameterValue;
use tui_input::Input;

use crate::state::tabs::TabId;

#[derive(Debug, Clone)]
pub struct ParameterForm {
    pub tab_id: TabId,
    pub sql: String,
    pub fields: Vec<ParameterField>,
    pub selected_field_index: usize,
}

#[derive(Debug, Clone)]
pub struct ParameterField {
    pub name: String,
    pub input: Input,
    pub is_raw: bool,
}

impl ParameterField {
    pub fn value(&self) -> ParameterValue {
        let entered_value = self.input.value().to_owned();
        match self.is_raw {
            true => ParameterValue::Raw(entered_value),
            false => ParameterValue::Text(entered_value),
        }
    }
}

impl ParameterForm {
    pub(crate) fn new(
        tab_id: TabId,
        sql: String,
        parameter_names: Vec<String>,
        remembered_values: &[(String, ParameterValue)],
    ) -> ParameterForm {
        let fields = parameter_names
            .into_iter()
            .map(|name| {
                let remembered_value = remembered_values
                    .iter()
                    .find(|(remembered_name, _)| *remembered_name == name)
                    .map(|(_, value)| value);
                let (initial_value, is_raw) = match remembered_value {
                    Some(ParameterValue::Text(text)) => (text.clone(), false),
                    Some(ParameterValue::Raw(raw_sql)) => (raw_sql.clone(), true),
                    None => (String::new(), false),
                };
                ParameterField {
                    name,
                    input: Input::new(initial_value),
                    is_raw,
                }
            })
            .collect();
        ParameterForm {
            tab_id,
            sql,
            fields,
            selected_field_index: 0,
        }
    }

    pub fn values(&self) -> Vec<(String, ParameterValue)> {
        self.fields
            .iter()
            .map(|field| (field.name.clone(), field.value()))
            .collect()
    }

    pub fn selected_field(&self) -> Option<&ParameterField> {
        self.fields.get(self.selected_field_index)
    }

    pub(crate) fn selected_field_mut(&mut self) -> Option<&mut ParameterField> {
        self.fields.get_mut(self.selected_field_index)
    }
}

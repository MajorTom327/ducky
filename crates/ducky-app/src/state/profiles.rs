use ducky_core::{Profile, ProfileKind};
use tui_input::Input;

const UNGROUPED_PROJECT_NAME: &str = "Other";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerMode {
    #[default]
    Filter,
    Navigate,
}

#[derive(Debug, Clone, Default)]
pub struct ProfilePicker {
    pub selected_profile_index: usize,
    pub connecting_profile_index: Option<usize>,
    pub connection_error: Option<String>,
    pub filter: Input,
    pub mode: PickerMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRow {
    Project(String),
    Profile(usize),
}

impl ProfilePicker {
    pub fn new(profiles: &[Profile], preferred_profile_index: Option<usize>) -> ProfilePicker {
        let mut picker = ProfilePicker::default();
        match preferred_profile_index {
            Some(profile_index) => {
                picker.selected_profile_index = profile_index;
                picker.reselect(profiles);
            }
            None => picker.select_first(profiles),
        }
        picker
    }

    pub fn rows(&self, profiles: &[Profile]) -> Vec<ProfileRow> {
        profile_rows(profiles, self.filter.value())
    }

    pub fn visible_profile_indexes(&self, profiles: &[Profile]) -> Vec<usize> {
        visible_profile_indexes(&self.rows(profiles))
    }

    pub fn selected_visible_profile_index(&self, profiles: &[Profile]) -> Option<usize> {
        self.visible_profile_indexes(profiles)
            .contains(&self.selected_profile_index)
            .then_some(self.selected_profile_index)
    }

    pub(crate) fn reselect(&mut self, profiles: &[Profile]) {
        if self.selected_visible_profile_index(profiles).is_none() {
            self.select_first(profiles);
        }
    }

    pub(crate) fn select_first(&mut self, profiles: &[Profile]) {
        if let Some(first_profile_index) = self.visible_profile_indexes(profiles).first() {
            self.selected_profile_index = *first_profile_index;
        }
    }
}

pub fn profile_rows(profiles: &[Profile], filter: &str) -> Vec<ProfileRow> {
    let lowercase_terms: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
    let matching_profiles: Vec<(usize, &Profile)> = profiles
        .iter()
        .enumerate()
        .filter(|(_, profile)| matches_terms(profile, &lowercase_terms))
        .collect();
    let mut project_names: Vec<&str> = Vec::new();
    for project_name in profiles
        .iter()
        .filter_map(|profile| profile.project.as_deref())
    {
        if !project_names.contains(&project_name) {
            project_names.push(project_name);
        }
    }
    let mut rows = Vec::new();
    for project_name in project_names.iter().copied() {
        push_group(
            &mut rows,
            Some(project_name),
            &matching_profiles,
            |profile| profile.project.as_deref() == Some(project_name),
        );
    }
    let ungrouped_header = (!project_names.is_empty()).then_some(UNGROUPED_PROJECT_NAME);
    push_group(&mut rows, ungrouped_header, &matching_profiles, |profile| {
        profile.project.is_none()
    });
    rows
}

pub fn profile_kind_name(kind: ProfileKind) -> &'static str {
    match kind {
        ProfileKind::Local => "local",
        ProfileKind::Staging => "staging",
        ProfileKind::Prod => "prod",
    }
}

fn push_group(
    rows: &mut Vec<ProfileRow>,
    header: Option<&str>,
    matching_profiles: &[(usize, &Profile)],
    belongs_to_group: impl Fn(&Profile) -> bool,
) {
    let member_rows: Vec<ProfileRow> = matching_profiles
        .iter()
        .filter(|(_, profile)| belongs_to_group(profile))
        .map(|(profile_index, _)| ProfileRow::Profile(*profile_index))
        .collect();
    if member_rows.is_empty() {
        return;
    }
    if let Some(header) = header {
        rows.push(ProfileRow::Project(header.to_owned()));
    }
    rows.extend(member_rows);
}

fn visible_profile_indexes(rows: &[ProfileRow]) -> Vec<usize> {
    rows.iter()
        .filter_map(|row| match row {
            ProfileRow::Profile(profile_index) => Some(*profile_index),
            ProfileRow::Project(_) => None,
        })
        .collect()
}

fn matches_terms(profile: &Profile, lowercase_terms: &[String]) -> bool {
    if lowercase_terms.is_empty() {
        return true;
    }
    let searchable_text = [
        profile.project.clone().unwrap_or_default(),
        profile.label.clone(),
        profile_kind_name(profile.kind).to_owned(),
        profile.tags.join(" "),
        profile.target(),
    ]
    .join(" ")
    .to_lowercase();
    lowercase_terms
        .iter()
        .all(|term| searchable_text.contains(term.as_str()))
}

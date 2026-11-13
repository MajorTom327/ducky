#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfilePicker {
    pub selected_profile_index: usize,
    pub connecting_profile_index: Option<usize>,
    pub connection_error: Option<String>,
}

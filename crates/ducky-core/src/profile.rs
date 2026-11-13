#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Local,
    Staging,
    Prod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub label: String,
    pub url: String,
    pub kind: ProfileKind,
}

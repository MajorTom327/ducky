#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub database: String,
    pub user: String,
    pub server_version: String,
}

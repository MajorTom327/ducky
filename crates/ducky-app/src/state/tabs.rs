use crate::state::query::QueryView;
use crate::state::table::TableView;

pub type TabId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabContent {
    Table(TableView),
    Query(QueryView),
    Files,
    Dump,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub content: TabContent,
}

impl Tab {
    pub fn title(&self) -> String {
        match &self.content {
            TabContent::Table(table) => table.relation.name.clone(),
            TabContent::Query(query) => query
                .document
                .name
                .clone()
                .unwrap_or_else(|| format!("Query {}", query.number)),
            TabContent::Files => "SQL files".to_owned(),
            TabContent::Dump => "Dump".to_owned(),
            TabContent::Restore => "Restore".to_owned(),
        }
    }
}

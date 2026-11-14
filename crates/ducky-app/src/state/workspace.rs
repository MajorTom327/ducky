use ducky_core::{Catalog, PgTools, Profile, RelationRef, SessionInfo, Sort, SortDirection};

use crate::loadable::Loadable;
use crate::msg::Connection;
use crate::state::files::FilesView;
use crate::state::jobs::{DumpView, RestoreView};
use crate::state::palette::{PaletteItem, palette_items};
use crate::state::query::{Document, PendingSave, QueryView};
use crate::state::sidebar::{Sidebar, SidebarItem};
use crate::state::table::TableView;
use crate::state::tabs::{Tab, TabContent, TabId};

const DEFAULT_SORT_COLUMN: &str = "id";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub profile_index: usize,
    pub profile: Profile,
    pub session: SessionInfo,
    pub tools: PgTools,
    pub focus: Focus,
    pub catalog: Loadable<Catalog>,
    pub schema_name: String,
    pub sidebar: Sidebar,
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
    pub next_tab_id: TabId,
    pub next_query_number: usize,
    pub pending_save: Option<PendingSave>,
    pub running_script_tab_id: Option<TabId>,
    pub switching_profile_index: Option<usize>,
    pub files: FilesView,
    pub dump: DumpView,
    pub restore: RestoreView,
}

impl Workspace {
    pub fn new(profile_index: usize, profile: Profile, connection: Connection) -> Workspace {
        Workspace {
            profile_index,
            profile,
            session: connection.session,
            tools: connection.tools,
            focus: Focus::Sidebar,
            catalog: Loadable::Loading,
            schema_name: String::new(),
            sidebar: Sidebar::default(),
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 1,
            next_query_number: 1,
            pending_save: None,
            running_script_tab_id: None,
            switching_profile_index: None,
            files: FilesView::default(),
            dump: DumpView::default(),
            restore: RestoreView::default(),
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn tab_index(&self, tab_id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == tab_id)
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let Some(schema) = self
            .catalog
            .ready()
            .and_then(|catalog| catalog.schema(&self.schema_name))
        else {
            return Vec::new();
        };
        let lowercase_filter = self.sidebar.filter.to_lowercase();
        let mut matching_relations: Vec<_> = schema
            .relations
            .iter()
            .filter(|relation| relation.name.to_lowercase().contains(&lowercase_filter))
            .collect();
        matching_relations.sort_by(|left, right| left.name.cmp(&right.name));
        matching_relations
            .into_iter()
            .map(|relation| SidebarItem {
                relation: RelationRef {
                    schema: schema.name.clone(),
                    name: relation.name.clone(),
                },
                kind: relation.kind,
                estimated_row_count: relation.estimated_row_count,
            })
            .collect()
    }

    pub fn palette_items(&self, query: &str) -> Vec<PaletteItem> {
        palette_items(self, query)
    }

    pub fn schema_names(&self) -> Vec<String> {
        self.catalog
            .ready()
            .map(|catalog| {
                catalog
                    .schemas
                    .iter()
                    .map(|schema| schema.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn query(&self, tab_id: TabId) -> Option<&QueryView> {
        self.tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .and_then(|tab| match &tab.content {
                TabContent::Query(query) => Some(query),
                _ => None,
            })
    }

    pub(crate) fn active_tab_id(&self) -> Option<TabId> {
        self.active_tab().map(|tab| tab.id)
    }

    pub(crate) fn active_content_mut(&mut self) -> Option<&mut TabContent> {
        self.tabs
            .get_mut(self.active_tab_index)
            .map(|tab| &mut tab.content)
    }

    pub(crate) fn active_table_mut(&mut self) -> Option<&mut TableView> {
        match self.active_content_mut() {
            Some(TabContent::Table(table)) => Some(table),
            _ => None,
        }
    }

    pub(crate) fn active_query_mut(&mut self) -> Option<(TabId, &mut QueryView)> {
        let tab = self.tabs.get_mut(self.active_tab_index)?;
        match &mut tab.content {
            TabContent::Query(query) => Some((tab.id, query)),
            _ => None,
        }
    }

    pub(crate) fn query_mut(&mut self, tab_id: TabId) -> Option<&mut QueryView> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .and_then(|tab| match &mut tab.content {
                TabContent::Query(query) => Some(query),
                _ => None,
            })
    }

    pub(crate) fn table_mut(&mut self, tab_id: TabId) -> Option<&mut TableView> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .and_then(|tab| match &mut tab.content {
                TabContent::Table(table) => Some(table),
                _ => None,
            })
    }

    pub(crate) fn is_active_content(&self, is_matching: fn(&TabContent) -> bool) -> bool {
        self.active_tab()
            .is_some_and(|tab| is_matching(&tab.content))
    }

    pub(crate) fn new_table_view(&self, relation: RelationRef, page_size: usize) -> TableView {
        let has_id_column = self
            .catalog
            .ready()
            .and_then(|catalog| catalog.relation(&relation))
            .is_some_and(|catalog_relation| {
                catalog_relation
                    .columns
                    .iter()
                    .any(|column| column.name == DEFAULT_SORT_COLUMN)
            });
        let default_sort = has_id_column.then(|| Sort {
            column: DEFAULT_SORT_COLUMN.to_owned(),
            direction: SortDirection::Descending,
        });
        TableView::first_page(relation, page_size, default_sort)
    }

    pub(crate) fn push_tab(&mut self, content: TabContent) -> TabId {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(Tab {
            id: tab_id,
            content,
        });
        self.active_tab_index = self.tabs.len() - 1;
        tab_id
    }

    pub(crate) fn push_query(&mut self, document: Document) -> TabId {
        let mut query = QueryView::new(self.next_query_number);
        query.document = document;
        self.next_query_number += 1;
        self.push_tab(TabContent::Query(query))
    }

    pub(crate) fn activate_tab(&mut self, tab_id: TabId) -> bool {
        match self.tab_index(tab_id) {
            Some(tab_index) => {
                self.active_tab_index = tab_index;
                true
            }
            None => false,
        }
    }
}

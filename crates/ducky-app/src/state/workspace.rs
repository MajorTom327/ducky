use ducky_core::{Catalog, PgTools, Profile, RelationRef, SessionInfo};

use crate::loadable::Loadable;
use crate::msg::Connection;
use crate::state::files::FilesView;
use crate::state::jobs::{DumpView, RestoreView};
use crate::state::query::QueryView;
use crate::state::sidebar::{Sidebar, SidebarAction, SidebarItem};
use crate::state::table::TableView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Welcome,
    Table,
    Query,
    Files,
    Dump,
    Restore,
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
    pub view: MainView,
    pub table: Option<TableView>,
    pub query: QueryView,
    pub files: FilesView,
    pub dump: DumpView,
    pub restore: RestoreView,
}

const SIDEBAR_ACTIONS: [SidebarAction; 4] = [
    SidebarAction::SqlCommand,
    SidebarAction::SqlFiles,
    SidebarAction::Dump,
    SidebarAction::Restore,
];

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
            view: MainView::Welcome,
            table: None,
            query: QueryView::default(),
            files: FilesView::default(),
            dump: DumpView::default(),
            restore: RestoreView::default(),
        }
    }

    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut sidebar_items: Vec<SidebarItem> = SIDEBAR_ACTIONS
            .into_iter()
            .map(SidebarItem::Action)
            .collect();
        let Some(schema) = self
            .catalog
            .ready()
            .and_then(|catalog| catalog.schema(&self.schema_name))
        else {
            return sidebar_items;
        };
        let lowercase_filter = self.sidebar.filter.to_lowercase();
        let mut matching_relations: Vec<_> = schema
            .relations
            .iter()
            .filter(|relation| relation.name.to_lowercase().contains(&lowercase_filter))
            .collect();
        matching_relations.sort_by(|left, right| left.name.cmp(&right.name));
        sidebar_items.extend(matching_relations.into_iter().map(|relation| {
            SidebarItem::Relation {
                relation: RelationRef {
                    schema: schema.name.clone(),
                    name: relation.name.clone(),
                },
                kind: relation.kind,
            }
        }));
        sidebar_items
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
}

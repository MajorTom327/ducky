#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    View,
    MaterializedView,
    ForeignTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationRef {
    pub schema: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_expression: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub name: String,
    pub kind: RelationKind,
    pub estimated_row_count: Option<i64>,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub schemas: Vec<Schema>,
}

impl Catalog {
    pub fn schema(&self, schema_name: &str) -> Option<&Schema> {
        self.schemas
            .iter()
            .find(|schema| schema.name == schema_name)
    }

    pub fn relation(&self, relation_ref: &RelationRef) -> Option<&Relation> {
        self.schema(&relation_ref.schema)?
            .relations
            .iter()
            .find(|relation| relation.name == relation_ref.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub definition: String,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKey {
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDetails {
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub view_definition: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_catalog() -> Catalog {
        Catalog {
            schemas: vec![Schema {
                name: "public".to_owned(),
                relations: vec![Relation {
                    name: "customers".to_owned(),
                    kind: RelationKind::Table,
                    estimated_row_count: Some(3),
                    columns: Vec::new(),
                }],
            }],
        }
    }

    fn relation_ref(schema: &str, name: &str) -> RelationRef {
        RelationRef {
            schema: schema.to_owned(),
            name: name.to_owned(),
        }
    }

    #[test]
    fn schema_finds_exact_name() {
        let catalog = fixture_catalog();
        assert_eq!(
            catalog.schema("public").map(|schema| schema.name.as_str()),
            Some("public")
        );
        assert!(catalog.schema("Public").is_none());
        assert!(catalog.schema("missing").is_none());
    }

    #[test]
    fn relation_resolves_schema_then_name() {
        let catalog = fixture_catalog();
        let found_relation = catalog.relation(&relation_ref("public", "customers"));
        assert_eq!(
            found_relation.map(|relation| relation.name.as_str()),
            Some("customers")
        );
    }

    #[test]
    fn relation_in_unknown_schema_is_none() {
        assert!(
            fixture_catalog()
                .relation(&relation_ref("analytics", "customers"))
                .is_none()
        );
    }

    #[test]
    fn unknown_relation_is_none() {
        assert!(
            fixture_catalog()
                .relation(&relation_ref("public", "orders"))
                .is_none()
        );
    }
}

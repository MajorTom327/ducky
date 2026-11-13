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
    pub foreign_keys: Vec<ForeignKeyLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignKeyLink {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced: RelationRef,
    pub referenced_columns: Vec<String>,
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

    pub fn referencing(&self, relation: &RelationRef) -> Vec<(RelationRef, ForeignKeyLink)> {
        let mut incoming_links: Vec<(RelationRef, ForeignKeyLink)> = self
            .schemas
            .iter()
            .flat_map(|schema| {
                schema.relations.iter().flat_map(move |candidate| {
                    candidate
                        .foreign_keys
                        .iter()
                        .filter(|link| link.referenced == *relation)
                        .map(move |link| {
                            (
                                RelationRef {
                                    schema: schema.name.clone(),
                                    name: candidate.name.clone(),
                                },
                                link.clone(),
                            )
                        })
                })
            })
            .collect();
        incoming_links.sort_by(|(left_relation, left_link), (right_relation, right_link)| {
            (&left_relation.schema, &left_relation.name, &left_link.name).cmp(&(
                &right_relation.schema,
                &right_relation.name,
                &right_link.name,
            ))
        });
        incoming_links
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
                    foreign_keys: Vec::new(),
                }],
            }],
        }
    }

    fn link(name: &str, columns: &[&str], referenced: RelationRef) -> ForeignKeyLink {
        ForeignKeyLink {
            name: name.to_owned(),
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
            referenced,
            referenced_columns: vec!["id".to_owned()],
        }
    }

    fn table_with_links(name: &str, foreign_keys: Vec<ForeignKeyLink>) -> Relation {
        Relation {
            name: name.to_owned(),
            kind: RelationKind::Table,
            estimated_row_count: None,
            columns: Vec::new(),
            foreign_keys,
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

    #[test]
    fn referencing_lists_incoming_links_by_schema_relation_and_name() {
        let customers = relation_ref("public", "customers");
        let catalog = Catalog {
            schemas: vec![
                Schema {
                    name: "analytics".to_owned(),
                    relations: vec![table_with_links(
                        "visits",
                        vec![link(
                            "visits_customer_fkey",
                            &["customer_id"],
                            customers.clone(),
                        )],
                    )],
                },
                Schema {
                    name: "public".to_owned(),
                    relations: vec![
                        table_with_links("customers", Vec::new()),
                        table_with_links(
                            "orders",
                            vec![
                                link("orders_payer_fkey", &["payer_id"], customers.clone()),
                                link("orders_buyer_fkey", &["buyer_id"], customers.clone()),
                                link(
                                    "orders_product_fkey",
                                    &["product_id"],
                                    relation_ref("public", "products"),
                                ),
                            ],
                        ),
                    ],
                },
            ],
        };
        let incoming_links: Vec<(String, String)> = catalog
            .referencing(&customers)
            .into_iter()
            .map(|(relation, link)| (format!("{}.{}", relation.schema, relation.name), link.name))
            .collect();
        assert_eq!(
            incoming_links,
            vec![
                (
                    "analytics.visits".to_owned(),
                    "visits_customer_fkey".to_owned()
                ),
                ("public.orders".to_owned(), "orders_buyer_fkey".to_owned()),
                ("public.orders".to_owned(), "orders_payer_fkey".to_owned()),
            ]
        );
        assert!(
            catalog
                .referencing(&relation_ref("public", "orders"))
                .is_empty()
        );
    }
}

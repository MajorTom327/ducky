use std::collections::HashMap;

use ducky_core::{Catalog, Column, Relation, RelationKind, Schema};
use tokio_postgres::{Client, Row};

use crate::error::{PgError, query_error};
use crate::rows::column_value;

const RELATIONS_SQL: &str = "
SELECT n.nspname::text, c.relname::text, c.relkind::text, c.reltuples::float8
FROM pg_catalog.pg_namespace n
LEFT JOIN pg_catalog.pg_class c
    ON c.relnamespace = n.oid AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg_toast%'
    AND n.nspname NOT LIKE 'pg_temp%'
ORDER BY n.nspname, c.relname";

const COLUMNS_SQL: &str = "
SELECT n.nspname::text, c.relname::text, a.attname::text,
    pg_catalog.format_type(a.atttypid, a.atttypmod),
    NOT a.attnotnull,
    pg_catalog.pg_get_expr(d.adbin, d.adrelid),
    EXISTS (
        SELECT 1 FROM pg_catalog.pg_index i
        WHERE i.indrelid = c.oid AND i.indisprimary AND a.attnum = ANY (i.indkey)
    )
FROM pg_catalog.pg_attribute a
JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
WHERE a.attnum > 0
    AND NOT a.attisdropped
    AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
    AND n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg_toast%'
    AND n.nspname NOT LIKE 'pg_temp%'
ORDER BY n.nspname, c.relname, a.attnum";

pub(crate) async fn load_catalog(client: &Client) -> Result<Catalog, PgError> {
    let (relation_rows, column_rows) = tokio::try_join!(
        client.query(RELATIONS_SQL, &[]),
        client.query(COLUMNS_SQL, &[])
    )
    .map_err(query_error)?;
    let mut schemas: Vec<Schema> = Vec::new();
    let mut relation_positions: HashMap<(String, String), (usize, usize)> = HashMap::new();
    for row in &relation_rows {
        let schema_name: String = column_value(row, 0)?;
        if schemas.last().map(|schema| &schema.name) != Some(&schema_name) {
            schemas.push(Schema {
                name: schema_name.clone(),
                relations: Vec::new(),
            });
        }
        let schema_index = schemas.len() - 1;
        let Some(relation) = relation_from_row(row)? else {
            continue;
        };
        let relations = &mut schemas[schema_index].relations;
        relation_positions.insert(
            (schema_name, relation.name.clone()),
            (schema_index, relations.len()),
        );
        relations.push(relation);
    }
    for row in &column_rows {
        let schema_name: String = column_value(row, 0)?;
        let relation_name: String = column_value(row, 1)?;
        if let Some(&(schema_index, relation_index)) =
            relation_positions.get(&(schema_name, relation_name))
        {
            schemas[schema_index].relations[relation_index]
                .columns
                .push(column_from_row(row)?);
        }
    }
    Ok(Catalog { schemas })
}

fn relation_from_row(row: &Row) -> Result<Option<Relation>, PgError> {
    let Some(relation_name) = column_value::<Option<String>>(row, 1)? else {
        return Ok(None);
    };
    let relation_kind_code: Option<String> = column_value(row, 2)?;
    let kind = match relation_kind_code.as_deref() {
        Some("v") => RelationKind::View,
        Some("m") => RelationKind::MaterializedView,
        Some("f") => RelationKind::ForeignTable,
        _ => RelationKind::Table,
    };
    let estimated_tuples: Option<f64> = column_value(row, 3)?;
    Ok(Some(Relation {
        name: relation_name,
        kind,
        estimated_row_count: estimated_tuples
            .filter(|tuples| *tuples >= 0.0)
            .map(|tuples| tuples as i64),
        columns: Vec::new(),
    }))
}

fn column_from_row(row: &Row) -> Result<Column, PgError> {
    Ok(Column {
        name: column_value(row, 2)?,
        data_type: column_value(row, 3)?,
        is_nullable: column_value(row, 4)?,
        default_expression: column_value(row, 5)?,
        is_primary_key: column_value(row, 6)?,
    })
}

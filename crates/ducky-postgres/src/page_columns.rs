use ducky_core::RelationRef;
use tokio_postgres::Client;

use crate::error::{PgError, query_error};
use crate::rows::column_value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageColumn {
    pub name: String,
    pub geo_function_schema: Option<String>,
}

const PAGE_COLUMNS_SQL: &str = "
SELECT a.attname::text, t.typname::text, type_namespace.nspname::text
FROM pg_catalog.pg_attribute a
JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
JOIN pg_catalog.pg_namespace type_namespace ON type_namespace.oid = t.typnamespace
WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped
ORDER BY a.attnum";

pub(crate) async fn load_page_columns(
    client: &Client,
    relation: &RelationRef,
) -> Result<Vec<PageColumn>, PgError> {
    let column_rows = client
        .query(PAGE_COLUMNS_SQL, &[&relation.schema, &relation.name])
        .await
        .map_err(query_error)?;
    column_rows
        .iter()
        .map(|row| {
            let type_name: String = column_value(row, 1)?;
            let type_schema: String = column_value(row, 2)?;
            let is_geo_type = type_name == "geometry" || type_name == "geography";
            Ok(PageColumn {
                name: column_value(row, 0)?,
                geo_function_schema: is_geo_type.then_some(type_schema),
            })
        })
        .collect()
}

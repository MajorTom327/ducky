use ducky_core::{ConstraintDefinition, Index, RelationDetails, RelationRef};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Client, Row};

use crate::error::{PgError, query_error};
use crate::rows::column_value;

const INDEXES_SQL: &str = "
SELECT index_class.relname::text, pg_catalog.pg_get_indexdef(i.indexrelid), i.indisunique, i.indisprimary
FROM pg_catalog.pg_index i
JOIN pg_catalog.pg_class c ON c.oid = i.indrelid
JOIN pg_catalog.pg_class index_class ON index_class.oid = i.indexrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = $1 AND c.relname = $2
ORDER BY index_class.relname";

const CONSTRAINTS_SQL: &str = "
SELECT con.conname::text, pg_catalog.pg_get_constraintdef(con.oid, true)
FROM pg_catalog.pg_constraint con
JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE con.contype::text = $3 AND n.nspname = $1 AND c.relname = $2
ORDER BY con.conname";

const FOREIGN_KEY_TYPE: &str = "f";
const CHECK_TYPE: &str = "c";

const VIEW_DEFINITION_SQL: &str = "
SELECT CASE WHEN c.relkind IN ('v', 'm') THEN pg_catalog.pg_get_viewdef(c.oid, true) END
FROM pg_catalog.pg_class c
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = $1 AND c.relname = $2";

pub(crate) async fn load_relation_details(
    client: &Client,
    relation: &RelationRef,
) -> Result<RelationDetails, PgError> {
    let parameters: [&(dyn ToSql + Sync); 2] = [&relation.schema, &relation.name];
    let foreign_key_parameters: [&(dyn ToSql + Sync); 3] =
        [&relation.schema, &relation.name, &FOREIGN_KEY_TYPE];
    let check_parameters: [&(dyn ToSql + Sync); 3] =
        [&relation.schema, &relation.name, &CHECK_TYPE];
    let (index_rows, foreign_key_rows, check_rows, view_definition_row) = tokio::try_join!(
        client.query(INDEXES_SQL, &parameters),
        client.query(CONSTRAINTS_SQL, &foreign_key_parameters),
        client.query(CONSTRAINTS_SQL, &check_parameters),
        client.query_opt(VIEW_DEFINITION_SQL, &parameters)
    )
    .map_err(query_error)?;
    let indexes = index_rows
        .iter()
        .map(|row| {
            Ok(Index {
                name: column_value(row, 0)?,
                definition: column_value(row, 1)?,
                is_unique: column_value(row, 2)?,
                is_primary: column_value(row, 3)?,
            })
        })
        .collect::<Result<Vec<_>, PgError>>()?;
    let foreign_keys = constraints_from_rows(&foreign_key_rows)?;
    let check_constraints = constraints_from_rows(&check_rows)?;
    let view_definition = match &view_definition_row {
        Some(row) => column_value::<Option<String>>(row, 0)?,
        None => None,
    };
    Ok(RelationDetails {
        indexes,
        foreign_keys,
        check_constraints,
        view_definition,
    })
}

fn constraints_from_rows(rows: &[Row]) -> Result<Vec<ConstraintDefinition>, PgError> {
    rows.iter()
        .map(|row| {
            Ok(ConstraintDefinition {
                name: column_value(row, 0)?,
                definition: column_value(row, 1)?,
            })
        })
        .collect()
}

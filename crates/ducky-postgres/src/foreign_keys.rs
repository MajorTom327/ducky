use ducky_core::{ForeignKeyLink, RelationRef};
use tokio_postgres::Row;

use crate::error::PgError;
use crate::rows::column_value;

pub(crate) const FOREIGN_KEY_LINKS_SQL: &str = "
SELECT con.conname::text, n.nspname::text, c.relname::text, a.attname::text,
    referenced_namespace.nspname::text, referenced_class.relname::text, referenced_attribute.attname::text
FROM pg_catalog.pg_constraint con
JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
JOIN pg_catalog.pg_class referenced_class ON referenced_class.oid = con.confrelid
JOIN pg_catalog.pg_namespace referenced_namespace ON referenced_namespace.oid = referenced_class.relnamespace
CROSS JOIN LATERAL unnest(con.conkey, con.confkey) WITH ORDINALITY
    AS key_pair(attnum, referenced_attnum, key_position)
JOIN pg_catalog.pg_attribute a
    ON a.attrelid = con.conrelid AND a.attnum = key_pair.attnum
JOIN pg_catalog.pg_attribute referenced_attribute
    ON referenced_attribute.attrelid = con.confrelid AND referenced_attribute.attnum = key_pair.referenced_attnum
WHERE con.contype = 'f'
    AND n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg_toast%'
    AND n.nspname NOT LIKE 'pg_temp%'
ORDER BY n.nspname, c.relname, con.conname, key_pair.key_position";

pub(crate) struct OwnedLink {
    pub(crate) owner: RelationRef,
    pub(crate) link: ForeignKeyLink,
}

pub(crate) fn links_from_rows(rows: &[Row]) -> Result<Vec<OwnedLink>, PgError> {
    let mut owned_links: Vec<OwnedLink> = Vec::new();
    for row in rows {
        let constraint_name: String = column_value(row, 0)?;
        let owner = RelationRef {
            schema: column_value(row, 1)?,
            name: column_value(row, 2)?,
        };
        let column_name: String = column_value(row, 3)?;
        let referenced_column_name: String = column_value(row, 6)?;
        let continues_last_link = owned_links.last().is_some_and(|last_link| {
            last_link.owner == owner && last_link.link.name == constraint_name
        });
        if !continues_last_link {
            owned_links.push(OwnedLink {
                owner,
                link: ForeignKeyLink {
                    name: constraint_name,
                    columns: Vec::new(),
                    referenced: RelationRef {
                        schema: column_value(row, 4)?,
                        name: column_value(row, 5)?,
                    },
                    referenced_columns: Vec::new(),
                },
            });
        }
        if let Some(last_link) = owned_links.last_mut() {
            last_link.link.columns.push(column_name);
            last_link
                .link
                .referenced_columns
                .push(referenced_column_name);
        }
    }
    Ok(owned_links)
}

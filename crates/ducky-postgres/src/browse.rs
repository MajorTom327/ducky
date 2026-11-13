use ducky_core::{Page, PageRequest, ResultSet, SortDirection, StatementOutcome};
use ducky_sql::quote_identifier;
use tokio_postgres::Client;

use crate::error::{PgError, query_error};
use crate::rows::run_statement;

pub fn build_page_sql(request: &PageRequest) -> String {
    let mut page_sql = format!(
        "SELECT * FROM {}.{}",
        quote_identifier(&request.relation.schema),
        quote_identifier(&request.relation.name)
    );
    if let Some(filter) = &request.filter {
        page_sql.push_str(&format!(" WHERE ({filter})"));
    }
    if let Some(sort) = &request.sort {
        let direction = match sort.direction {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };
        page_sql.push_str(&format!(
            " ORDER BY {} {direction}",
            quote_identifier(&sort.column)
        ));
    }
    page_sql.push_str(&format!(
        " LIMIT {} OFFSET {}",
        request.page_size.saturating_add(1),
        request.page_index.saturating_mul(request.page_size)
    ));
    page_sql
}

pub(crate) async fn load_page(client: &Client, request: &PageRequest) -> Result<Page, PgError> {
    let outcome = run_statement(client, &build_page_sql(request))
        .await
        .map_err(query_error)?;
    let mut result_set = match outcome {
        StatementOutcome::Rows(result_set) => result_set,
        StatementOutcome::Affected(_) => ResultSet {
            columns: Vec::new(),
            rows: Vec::new(),
            is_truncated: false,
        },
    };
    let has_next_page = result_set.rows.len() > request.page_size;
    result_set.rows.truncate(request.page_size);
    Ok(Page {
        result_set,
        has_next_page,
    })
}

#[cfg(test)]
mod tests {
    use ducky_core::{RelationRef, Sort};

    use super::*;

    fn request(schema: &str, name: &str) -> PageRequest {
        PageRequest {
            relation: RelationRef {
                schema: schema.to_owned(),
                name: name.to_owned(),
            },
            page_index: 0,
            page_size: 100,
            sort: None,
            filter: None,
        }
    }

    #[test]
    fn plain_page_selects_first_page_with_one_extra_row() {
        assert_eq!(
            build_page_sql(&request("public", "customers")),
            r#"SELECT * FROM "public"."customers" LIMIT 101 OFFSET 0"#
        );
    }

    #[test]
    fn filter_sort_and_page_index_are_applied() {
        let page_request = PageRequest {
            page_index: 2,
            sort: Some(Sort {
                column: "created_at".to_owned(),
                direction: SortDirection::Descending,
            }),
            filter: Some("status = 'paid'".to_owned()),
            ..request("public", "orders")
        };
        assert_eq!(
            build_page_sql(&page_request),
            r#"SELECT * FROM "public"."orders" WHERE (status = 'paid') ORDER BY "created_at" DESC LIMIT 101 OFFSET 200"#
        );
    }

    #[test]
    fn ascending_sort_uses_asc() {
        let page_request = PageRequest {
            sort: Some(Sort {
                column: "id".to_owned(),
                direction: SortDirection::Ascending,
            }),
            ..request("public", "orders")
        };
        assert!(build_page_sql(&page_request).contains(r#"ORDER BY "id" ASC"#));
    }

    #[test]
    fn identifiers_with_double_quotes_are_escaped() {
        let page_request = PageRequest {
            sort: Some(Sort {
                column: r#"we"ird"#.to_owned(),
                direction: SortDirection::Ascending,
            }),
            ..request(r#"my"schema"#, r#"odd "table""#)
        };
        assert_eq!(
            build_page_sql(&page_request),
            r#"SELECT * FROM "my""schema"."odd ""table""" ORDER BY "we""ird" ASC LIMIT 101 OFFSET 0"#
        );
    }
}

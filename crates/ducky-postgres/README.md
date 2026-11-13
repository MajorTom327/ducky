# ducky-postgres

PostgreSQL access for ducky. A `PgSession` holds two connections to the same database:

- `browse` runs catalog introspection and table pages;
- `script` runs user SQL, so a long script or an open transaction never blocks browsing.

Every query goes through the simple-query protocol, so values arrive as text (`Option<String>`, `None` for SQL `NULL`) without per-type decoding.

## Public API

- `PgSession::connect(database_url)` opens both connections. The URL's `sslmode` is read by ducky: `disable`; absent, `allow` or `prefer` (TLS when offered); `require` (TLS, certificate not verified, as libpq does); `verify-ca` and `verify-full` (TLS with the platform's native root certificates). Any other value is `PgError::InvalidUrl`. The application name is `ducky` and the connect timeout 10 s.
- `PgSession::info()` returns the current database, user and server version.
- `PgSession::load_catalog()` lists every non-system schema (empty ones included) with its tables, partitioned tables, views, materialized views and foreign tables, their estimated row count and their columns (type, nullability, default, primary-key flag) and their outgoing foreign keys (`Relation.foreign_keys`: constraint name, columns and referenced relation and columns in key order, read from `pg_constraint` with `unnest(conkey, confkey) WITH ORDINALITY`).
- `PgSession::load_relation_details(relation)` returns indexes, foreign keys and, for views and materialized views, the view definition.
- `PgSession::load_page(request)` loads one page of a relation through `build_page_sql`; `has_next_page` is detected by fetching one extra row.
- `PgSession::run_script(statements, mode)` runs already split statements in order and stops at the first failure. `TransactionMode::Transaction` wraps them in `BEGIN` / `COMMIT` and rolls back on failure (a failing `COMMIT` is reported with `statement_index == statements.len()`); `AutoCommit` sends them as they are. A result keeps at most `MAX_RESULT_ROWS` (10 000) rows and sets `is_truncated` beyond that.
- `PgSession::cancel_script()` asks the server to cancel the statement running on the script connection.
- `PageColumn { name, geo_function_schema }` describes one browsed column; `geo_function_schema` is the schema of a PostGIS `geometry` / `geography` type.
- `build_page_sql(request, columns)` builds `SELECT <select list> FROM "schema"."name" [WHERE (filter)] [ORDER BY "column" ASC|DESC] LIMIT page_size + 1 OFFSET page_index * page_size`. The select list is `*` unless a column is `geometry` / `geography`; then every column is listed and geo columns become `"<type schema>".st_asgeojson("col") AS "col"`, so they read as GeoJSON. `load_page` reads the columns (attnum order, type and type schema) before each page. The filter is inserted verbatim: it is the user's own SQL.
- `PgError` (`InvalidUrl`, `Connect`, `Query`) displays its inner text. Database errors read `<SEVERITY>: <message>` followed by `DETAIL:` and `HINT:` lines when present.

## Tests

```sh
cargo test -p ducky-postgres
just db-up
DUCKY_TEST_DATABASE_URL=postgres://ducky:ducky@localhost:5440/ducky cargo test -p ducky-postgres -- --ignored
```

The ignored tests need a reachable database; each creates and drops its own `ducky_it_<nanos>_<n>` schema.

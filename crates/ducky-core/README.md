# ducky-core

Domain vocabulary shared by every ducky crate. No dependencies, no IO.

## Public API

- Profiles: `ProfileKind`, `Profile { label, url, kind, project, tags }` with `target()` (`host[:port]/database` read from the url text, `${VAR}` kept)
- Session: `SessionInfo`
- Catalog: `RelationKind`, `RelationRef`, `Column`, `Relation` (with its outgoing `foreign_keys`), `ForeignKeyLink { name, columns, referenced, referenced_columns }`, `Schema`, `Catalog` (`schema`, `relation` lookups; `referencing(relation)` lists incoming links as `(referencing relation, link)` ordered by schema, relation, constraint name), `Index`, `ForeignKey`, `RelationDetails`
- Data: `Cell`, `ResultSet`, `SortDirection`, `Sort`, `PageRequest`, `Page`, `DEFAULT_PAGE_SIZE` (50), `MAX_PAGE_SIZE` (5000)
- Scripts: `TransactionMode` (default `Transaction`), `StatementOutcome`, `StatementResult`, `ScriptFailure`, `ScriptReport`
- Files: `FileEntry`, `ToolStatus`, `PgTools`, `RestoreOptions` (default all false), `StoragePaths`

## Tests

```sh
cargo test -p ducky-core
```
